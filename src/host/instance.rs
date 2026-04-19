//! Active VST3 processing state. Embeds a [`Vst3Loaded`] plus the scratch
//! buffers needed for [`process`](Vst3Instance::process), and owns the
//! `setActive(1)` + `setProcessing(1)` lifecycle.
//!
//! All non-processing methods (parameters, editor, state, latency) are
//! inherited from [`Vst3Loaded`] via [`Deref`] / [`DerefMut`] — see
//! [`crate::host::loaded`] for that surface.
//!
//! To create one: `Vst3Instance::load(path, rate, block)` or
//! `Vst3Loaded::load(path)?.activate(rate, block)?`. To drop back to
//! non-processing state: [`Vst3Instance::deactivate`].

use std::ops::{Deref, DerefMut};
use std::path::Path;

use smallvec::SmallVec;
use vst3::Steinberg::{
    kResultFalse, kResultOk,
    Vst::{
        IAudioProcessorTrait, IComponentTrait, IEventList, IParameterChanges,
        MediaTypes_::kEvent, ProcessModes_::kRealtime, ProcessSetup,
    },
};

use crate::com::{EventList, ParameterChangesImpl};
use crate::error::{LoadStage, Result, Vst3Error};
use crate::types::{
    AudioBuffer, BufferPtrs, MidiEvent, NoteExpressionValue, ParameterChanges, PluginInfo,
    ProcessOutput, Sample, TransportState,
};

use super::loaded::{get_bus_channel_count, Vst3Loaded, K_INPUT, K_OUTPUT};

pub(super) const K_EVENT: i32 = kEvent as i32;
const K_REALTIME: i32 = kRealtime as i32;
const MIN_PTR_COUNT: usize = 2;

/// Scratch buffers + event lists the realtime `process()` loop needs.
/// Separated from [`Vst3Loaded`] so GUI-only hosting doesn't pay the cost.
struct AudioIO {
    sample_rate: f64,
    block_size: usize,
    use_f64: bool,
    num_input_channels: usize,
    num_output_channels: usize,
    ptrs_f32: BufferPtrs<f32>,
    ptrs_f64: BufferPtrs<f64>,
    input_events: vst3::ComWrapper<EventList>,
    output_events: vst3::ComWrapper<EventList>,
}

/// Fully-active VST3 plugin ready to `process()` audio. Embeds a
/// [`Vst3Loaded`] — all parameter, editor, state, and metadata methods are
/// available via [`Deref`].
pub struct Vst3Instance {
    loaded: Vst3Loaded,
    audio: AudioIO,
}

impl Vst3Instance {
    /// Lightweight probe: load library, read factory and bus metadata, return
    /// without calling init() or setActive(). Safe for plugins with license dialogs.
    pub fn probe(path: &Path) -> Result<PluginInfo> {
        Vst3Loaded::probe(path)
    }

    /// Load a VST3 plugin and bring it to the active processing state.
    ///
    /// For GUI-only hosting, prefer [`Vst3Loaded::load`] — it skips the
    /// `setActive(1) + setProcessing(1)` cost.
    pub fn load(path: &Path, sample_rate: f64, block_size: usize) -> Result<Self> {
        let loaded = Vst3Loaded::load_with_info(path)?;
        Self::from_loaded(loaded, sample_rate, block_size)
    }

    /// Called by [`Vst3Loaded::activate`]. Runs `setupProcessing`, activates
    /// buses, calls `setActive(1)` + `setProcessing(1)`.
    pub(super) fn from_loaded(
        loaded: Vst3Loaded,
        sample_rate: f64,
        block_size: usize,
    ) -> Result<Self> {
        let num_input_channels = loaded.info.num_inputs;
        let num_output_channels = loaded.info.num_outputs;
        let input_ptr_count = num_input_channels.max(MIN_PTR_COUNT);
        let output_ptr_count = num_output_channels.max(MIN_PTR_COUNT);

        let audio = AudioIO {
            sample_rate,
            block_size,
            use_f64: false,
            num_input_channels,
            num_output_channels,
            ptrs_f32: BufferPtrs::new(input_ptr_count, output_ptr_count),
            ptrs_f64: BufferPtrs::new(input_ptr_count, output_ptr_count),
            input_events: EventList::new(),
            output_events: EventList::new(),
        };

        let mut instance = Self { loaded, audio };
        instance.apply_process_setup()?;
        instance.activate_buses()?;
        instance.set_active(true)?;
        Ok(instance)
    }

    /// Leave the active state, returning to [`Vst3Loaded`]. Reverses
    /// `setProcessing(1)` + `setActive(1)`.
    pub fn deactivate(mut self) -> Vst3Loaded {
        self.stop_processing();
        let _ = self.set_active(false);
        // Move `loaded` out without running `Vst3Instance::Drop` (which would
        // deactivate a second time).
        let loaded =
            unsafe { std::ptr::read(&self.loaded as *const Vst3Loaded) };
        std::mem::forget(self);
        loaded
    }

    // ── configuration (re-runs setupProcessing) ──

    pub fn sample_rate(&self) -> f64 {
        self.audio.sample_rate
    }

    pub fn set_sample_rate(&mut self, rate: f64) -> &mut Self {
        self.audio.sample_rate = rate;
        let _ = self.apply_process_setup();
        self
    }

    pub fn block_size(&self) -> usize {
        self.audio.block_size
    }

    pub fn set_block_size(&mut self, size: usize) -> &mut Self {
        self.audio.block_size = size;
        let _ = self.apply_process_setup();
        self
    }

    pub fn num_input_channels(&self) -> usize {
        self.audio.num_input_channels
    }

    pub fn num_output_channels(&self) -> usize {
        self.audio.num_output_channels
    }

    pub fn set_use_f64(&mut self, use_f64: bool) -> Result<&mut Self> {
        if use_f64 && !self.loaded.info.supports_f64 {
            return Err(Vst3Error::NotSupported(
                "Plugin does not support 64-bit processing".to_string(),
            ));
        }
        self.audio.use_f64 = use_f64;
        self.apply_process_setup()?;
        Ok(self)
    }

    // ── processing ──

    pub fn process<T: Sample>(
        &mut self,
        buffer: &mut AudioBuffer<T>,
        midi_events: &[MidiEvent],
        param_changes: Option<&ParameterChanges>,
        note_expressions: &[NoteExpressionValue],
        transport: &TransportState,
    ) -> ProcessOutput {
        let empty_result = ProcessOutput {
            midi_events: SmallVec::new(),
            parameter_changes: ParameterChanges::new(),
        };

        let processor = match &self.loaded.interfaces.processor {
            Some(p) => p.clone(),
            None => return empty_result,
        };

        if T::VST3_SYMBOLIC_SIZE == crate::types::K_SAMPLE_64_INT
            && !self.loaded.info.supports_f64
        {
            return empty_result;
        }

        let num_samples = buffer.num_samples;
        if num_samples == 0 {
            return empty_result;
        }

        let (input_ptrs, output_ptrs) = T::prepare_ffi_buffers(
            &mut self.audio.ptrs_f32,
            &mut self.audio.ptrs_f64,
            buffer.inputs,
            buffer.outputs,
        );

        let mut input_bus: vst3::Steinberg::Vst::AudioBusBuffers = unsafe { std::mem::zeroed() };
        input_bus.numChannels = buffer.inputs.len() as i32;
        input_bus.silenceFlags = 0;
        input_bus.__field0.channelBuffers32 = input_ptrs as *mut *mut f32;

        let mut output_bus: vst3::Steinberg::Vst::AudioBusBuffers = unsafe { std::mem::zeroed() };
        output_bus.numChannels = buffer.outputs.len() as i32;
        output_bus.silenceFlags = 0;
        output_bus.__field0.channelBuffers32 = output_ptrs as *mut *mut f32;

        let has_events = !midi_events.is_empty() || !note_expressions.is_empty();
        if has_events {
            self.audio
                .input_events
                .update_from_midi_and_expression(midi_events, note_expressions);
        } else {
            self.audio.input_events.clear();
        }
        self.audio.output_events.clear();

        let input_events_ptr: *mut IEventList = if has_events {
            self.audio
                .input_events
                .as_com_ref::<IEventList>()
                .map(|r| r.as_ptr())
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        };

        let output_events_ptr: *mut IEventList = self
            .audio
            .output_events
            .as_com_ref::<IEventList>()
            .map(|r| r.as_ptr())
            .unwrap_or(std::ptr::null_mut());

        let input_param_changes = param_changes
            .filter(|pc| !pc.is_empty())
            .map(ParameterChangesImpl::from_changes);
        let input_param_changes_ptr = input_param_changes
            .as_ref()
            .and_then(|c| c.as_com_ref::<IParameterChanges>().map(|r| r.as_ptr()))
            .unwrap_or(std::ptr::null_mut());

        let output_param_changes = ParameterChangesImpl::new_empty();
        let output_param_changes_ptr = output_param_changes
            .as_com_ref::<IParameterChanges>()
            .map(|r| r.as_ptr())
            .unwrap_or(std::ptr::null_mut());

        let mut process_context = transport.to_process_context();
        process_context.sampleRate = buffer.sample_rate;

        let mut process_data: vst3::Steinberg::Vst::ProcessData = unsafe { std::mem::zeroed() };
        process_data.processMode = K_REALTIME;
        process_data.symbolicSampleSize = T::VST3_SYMBOLIC_SIZE;
        process_data.numSamples = num_samples as i32;
        process_data.numInputs = 1;
        process_data.numOutputs = 1;
        process_data.inputs = &mut input_bus;
        process_data.outputs = &mut output_bus;
        process_data.inputParameterChanges = input_param_changes_ptr;
        process_data.outputParameterChanges = output_param_changes_ptr;
        process_data.inputEvents = input_events_ptr;
        process_data.outputEvents = output_events_ptr;
        process_data.processContext = &mut process_context;

        let result = unsafe { processor.process(&mut process_data) };

        if result != kResultOk {
            buffer.clear_outputs();
            return empty_result;
        }

        let midi_out = self.audio.output_events.to_midi_events();
        let param_out = output_param_changes.to_changes();

        ProcessOutput {
            midi_events: midi_out,
            parameter_changes: param_out,
        }
    }

    /// Tell the plugin's audio processor to idle. Safe to call repeatedly;
    /// `Drop` calls this automatically.
    pub fn stop_processing(&mut self) {
        if let Some(p) = &self.loaded.interfaces.processor {
            unsafe {
                p.setProcessing(0);
            }
        }
    }

    // ── internal ──

    fn apply_process_setup(&mut self) -> Result<()> {
        let symbolic_sample_size = if self.audio.use_f64 {
            crate::types::K_SAMPLE_64_INT
        } else {
            crate::types::K_SAMPLE_32_INT
        };
        let mut setup = ProcessSetup {
            processMode: K_REALTIME,
            symbolicSampleSize: symbolic_sample_size,
            maxSamplesPerBlock: self.audio.block_size as i32,
            sampleRate: self.audio.sample_rate,
        };
        if let Some(ref p) = self.loaded.interfaces.processor {
            let result = unsafe { p.setupProcessing(&mut setup) };
            if result != kResultOk && result != kResultFalse {
                return Err(Vst3Error::PluginError {
                    stage: LoadStage::Setup,
                    code: result,
                });
            }
        }
        Ok(())
    }

    fn activate_buses(&mut self) -> Result<()> {
        const K_AUDIO: i32 = super::loaded::K_AUDIO;
        unsafe {
            let component = &self.loaded.interfaces.component;
            for i in 0..component.getBusCount(K_AUDIO, K_INPUT) {
                component.activateBus(K_AUDIO, K_INPUT, i, 1);
            }
            for i in 0..component.getBusCount(K_AUDIO, K_OUTPUT) {
                component.activateBus(K_AUDIO, K_OUTPUT, i, 1);
            }

            // Re-sync channel counts after bus activation.
            if let Some(ch) = get_bus_channel_count(component, K_INPUT, 0) {
                if ch != self.audio.num_input_channels {
                    self.audio.num_input_channels = ch;
                    self.audio.ptrs_f32.resize_inputs(ch.max(MIN_PTR_COUNT));
                    self.audio.ptrs_f64.resize_inputs(ch.max(MIN_PTR_COUNT));
                }
            }
            if let Some(ch) = get_bus_channel_count(component, K_OUTPUT, 1) {
                if ch != self.audio.num_output_channels {
                    self.audio.num_output_channels = ch;
                    self.audio.ptrs_f32.resize_outputs(ch.max(MIN_PTR_COUNT));
                    self.audio.ptrs_f64.resize_outputs(ch.max(MIN_PTR_COUNT));
                }
            }
        }
        Ok(())
    }

    fn set_active(&mut self, active: bool) -> Result<()> {
        let flag: vst3::Steinberg::TBool = if active { 1 } else { 0 };
        let result = unsafe { self.loaded.interfaces.component.setActive(flag) };
        if result != kResultOk && result != kResultFalse {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Activation,
                code: result,
            });
        }
        if active {
            if let Some(p) = &self.loaded.interfaces.processor {
                let result = unsafe { p.setProcessing(1) };
                if result != kResultOk && result != kResultFalse {
                    return Err(Vst3Error::PluginError {
                        stage: LoadStage::Activation,
                        code: result,
                    });
                }
            }
        }
        Ok(())
    }
}

impl Deref for Vst3Instance {
    type Target = Vst3Loaded;
    fn deref(&self) -> &Vst3Loaded {
        &self.loaded
    }
}

impl DerefMut for Vst3Instance {
    fn deref_mut(&mut self) -> &mut Vst3Loaded {
        &mut self.loaded
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        self.stop_processing();
        let _ = self.set_active(false);
        // Drop order then terminates via Vst3Loaded's Drop.
    }
}
