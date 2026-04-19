//! VST3 plugin instance management.

use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use smallvec::SmallVec;

use crate::com::{
    BStream, ComponentHandler, EventList, HostApplication, ParameterChangesImpl,
    ParameterEditEvent, ProgressEvent, UnitEvent,
};
use crate::error::{LoadStage, Result, Vst3Error};
use crate::ffi::{
    AudioBusBuffers, BusInfo, IAudioProcessorVtable, IComponentVtable, IConnectionPointVtable,
    IEditControllerVtable, IPlugViewVtable, IUnknownVtable, ProcessData, ProcessSetup, ViewRect,
    IID_IAUDIO_PROCESSOR, IID_ICOMPONENT, IID_ICONNECTION_POINT, IID_IEDIT_CONTROLLER, K_AUDIO,
    K_EVENT, K_INPUT, K_OUTPUT, K_REALTIME, K_RESULT_OK, K_RESULT_TRUE, K_SAMPLE_32, K_SAMPLE_64,
};
use crate::types::{
    AudioBuffer, BufferPtrs, EditorSize, MidiEvent, NoteExpressionValue, ParameterChanges,
    PluginInfo, ProcessOutput, Sample, TransportState, WindowHandle,
};

use super::library::Vst3Library;

pub(crate) struct PluginInterfaces {
    pub component: *mut c_void,
    pub component_vtable: *const IComponentVtable,
    pub processor: Option<*mut c_void>,
    pub processor_vtable: Option<*const IAudioProcessorVtable>,
    pub controller: Option<*mut c_void>,
    pub controller_vtable: Option<*const IEditControllerVtable>,
    pub separate_controller: bool,
}

unsafe impl Send for PluginInterfaces {}
unsafe impl Sync for PluginInterfaces {}

impl PluginInterfaces {
    /// Call `f` with the processor pointer and vtable if both are present.
    fn with_processor<R>(
        &self,
        default: R,
        f: impl FnOnce(*mut c_void, *const IAudioProcessorVtable) -> R,
    ) -> R {
        match (self.processor, self.processor_vtable) {
            (Some(proc), Some(vtable)) => f(proc, vtable),
            _ => default,
        }
    }

    /// Call `f` with the controller pointer and vtable if both are present.
    fn with_controller<R>(
        &self,
        default: R,
        f: impl FnOnce(*mut c_void, *const IEditControllerVtable) -> R,
    ) -> R {
        match (self.controller, self.controller_vtable) {
            (Some(ctrl), Some(vtable)) => f(ctrl, vtable),
            _ => default,
        }
    }
}

pub(crate) struct HostContext {
    pub application: Box<HostApplication>,
    pub handler: Box<ComponentHandler>,
    pub param_event_rx: Receiver<ParameterEditEvent>,
    pub progress_event_rx: Receiver<ProgressEvent>,
    pub unit_event_rx: Receiver<UnitEvent>,
}

pub(crate) struct AudioIO {
    pub sample_rate: f64,
    pub block_size: usize,
    pub use_f64: bool,
    pub num_input_channels: usize,
    pub num_output_channels: usize,
    pub ptrs_f32: BufferPtrs<f32>,
    pub ptrs_f64: BufferPtrs<f64>,
    pub input_events: Option<Box<EventList>>,
    pub output_events: Option<Box<EventList>>,
}

pub(crate) struct EditorState {
    pub view: Option<*mut c_void>,
    pub view_vtable: Option<*const IPlugViewVtable>,
    pub size: (u32, u32),
}

unsafe impl Send for EditorState {}
unsafe impl Sync for EditorState {}

/// Minimum pointer array size — ensures stereo even if plugin reports mono.
const MIN_PTR_COUNT: usize = 2;

/// Default editor dimensions when the plugin doesn't report a size.
const DEFAULT_EDITOR_SIZE: (u32, u32) = (800, 600);

/// Release a COM object through its IUnknown vtable.
///
/// # Safety
///
/// `obj` must be a valid COM object pointer.
unsafe fn release_com(obj: *mut c_void) {
    let vtable = *(obj as *const *const IUnknownVtable);
    ((*vtable).release)(obj);
}

/// Query a COM interface from an object. Returns `None` if the interface is not supported.
///
/// # Safety
///
/// `object` must be a valid COM object pointer.
unsafe fn query_interface(object: *mut c_void, iid: &[u8; 16]) -> Option<*mut c_void> {
    let vtable = *(object as *const *const IUnknownVtable);
    let mut out: *mut c_void = std::ptr::null_mut();
    let result = ((*vtable).query_interface)(object, iid, &mut out);
    if result == K_RESULT_OK && !out.is_null() {
        Some(out)
    } else {
        None
    }
}

fn cid_to_string(cid: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        cid[0], cid[1], cid[2], cid[3],
        cid[4], cid[5], cid[6], cid[7],
        cid[8], cid[9], cid[10], cid[11],
        cid[12], cid[13], cid[14], cid[15]
    )
}

/// Query the channel count of the first audio bus in the given direction.
/// Returns `None` if there are no buses of that direction.
///
/// # Safety
///
/// `component` and `vtable` must be valid pointers to an initialized IComponent.
unsafe fn get_bus_channel_count(
    component: *mut c_void,
    vtable: *const IComponentVtable,
    direction: i32,
    min_channels: i32,
) -> Option<usize> {
    let num_buses = ((*vtable).get_bus_count)(component, K_AUDIO, direction);
    if num_buses <= 0 {
        return None;
    }
    let mut bus_info = BusInfo::default();
    let result = ((*vtable).get_bus_info)(
        component,
        K_AUDIO,
        direction,
        0,
        &mut bus_info as *mut _ as *mut c_void,
    );
    if result == K_RESULT_OK {
        Some(bus_info.channel_count.max(min_channels) as usize)
    } else {
        None
    }
}

pub struct Vst3Instance {
    _library: Arc<Vst3Library>,
    interfaces: PluginInterfaces,
    host: HostContext,
    audio: AudioIO,
    editor: EditorState,
    info: PluginInfo,
    is_active: bool,
}

impl Vst3Instance {
    /// Lightweight probe: load library, read factory and bus metadata, return
    /// without calling init() or setActive(). Safe for plugins with license dialogs.
    pub fn probe(path: &Path) -> Result<PluginInfo> {
        if !path.exists() {
            return Err(Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Scanning,
                reason: "Plugin file not found".to_string(),
            });
        }

        let library = Vst3Library::load(path)?;
        let count = library.count_classes();
        if count == 0 {
            return Err(Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "VST3 factory contains no classes".to_string(),
            });
        }

        let factory_info = library.get_factory_info();
        let vendor = factory_info.map(|info| info.vendor).unwrap_or_default();

        let (class_cid, name) = (0..count)
            .find_map(|i| {
                let info = library.get_class_info(i).ok()?;
                if info.category.contains("Audio") {
                    Some((info.cid, info.name))
                } else {
                    None
                }
            })
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "No audio processor classes found in VST3".to_string(),
            })?;

        let component = library.create_instance(&class_cid, &IID_ICOMPONENT)?;
        let component_vtable = unsafe { *(component as *const *const IComponentVtable) };

        // Query bus info (available before init on most plugins)
        let (num_inputs, num_outputs) = unsafe {
            let inputs = get_bus_channel_count(component, component_vtable, K_INPUT, 0).unwrap_or(0);
            let outputs = get_bus_channel_count(component, component_vtable, K_OUTPUT, 1).unwrap_or(2);
            (inputs, outputs)
        };

        // Check for audio processor and f64 support
        let supports_f64 = unsafe {
            if let Some(processor) = query_interface(component, &IID_IAUDIO_PROCESSOR) {
                let vtable = *(processor as *const *const IAudioProcessorVtable);
                ((*vtable).can_process_sample_size)(processor, K_SAMPLE_64) == K_RESULT_OK
            } else {
                false
            }
        };

        let unique_id = cid_to_string(&class_cid);
        // VST3 MIDI: check if there are event input buses
        let receives_midi = unsafe {
            let event_bus_count = ((*component_vtable).get_bus_count)(component, K_EVENT, K_INPUT);
            event_bus_count > 0
        };

        Ok(PluginInfo::new(format!("vst3.{}", unique_id), name)
            .vendor(vendor)
            .version("1.0.0".to_string())
            .audio_io(num_inputs, num_outputs)
            .midi(receives_midi)
            .f64_support(supports_f64))
    }

    pub fn load(path: &Path, sample_rate: f64, block_size: usize) -> Result<Self> {
        if !path.exists() {
            return Err(Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Scanning,
                reason: "Plugin file not found".to_string(),
            });
        }

        let library = Vst3Library::load(path)?;
        let count = library.count_classes();

        if count == 0 {
            return Err(Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "VST3 factory contains no classes".to_string(),
            });
        }

        let factory_info = library.get_factory_info();
        let vendor = factory_info.map(|info| info.vendor).unwrap_or_default();

        let (class_info, name) = (0..count)
            .find_map(|i| {
                let info = library.get_class_info(i).ok()?;
                if info.category.contains("Audio") {
                    Some((info.cid, info.name))
                } else {
                    None
                }
            })
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "No audio processor classes found in VST3".to_string(),
            })?;

        let component = library.create_instance(&class_info, &IID_ICOMPONENT)?;

        let processor =
            unsafe { query_interface(component, &IID_IAUDIO_PROCESSOR) }.ok_or_else(|| {
                Vst3Error::LoadFailed {
                    path: path.to_path_buf(),
                    stage: LoadStage::Instantiation,
                    reason: "VST3 plugin does not support IAudioProcessor".to_string(),
                }
            })?;

        let component_vtable = unsafe { *(component as *const *const IComponentVtable) };

        // Try single-component model first, fall back to separate controller
        let (controller, separate_controller) = unsafe {
            if let Some(ctrl) = query_interface(component, &IID_IEDIT_CONTROLLER) {
                (Some(ctrl), false)
            } else {
                let mut controller_cid = [0u8; 16];
                let result =
                    ((*component_vtable).get_controller_class_id)(component, &mut controller_cid);
                if result == K_RESULT_OK && controller_cid != [0u8; 16] {
                    if let Ok(ctrl) =
                        library.create_instance(&controller_cid, &IID_IEDIT_CONTROLLER)
                    {
                        (Some(ctrl), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (None, false)
                }
            }
        };

        let processor_vtable = unsafe { *(processor as *const *const IAudioProcessorVtable) };
        let controller_vtable =
            controller.map(|c| unsafe { *(c as *const *const IEditControllerVtable) });

        let unique_id = cid_to_string(&class_info);

        let supports_f64 = unsafe {
            ((*processor_vtable).can_process_sample_size)(processor, K_SAMPLE_64) == K_RESULT_OK
        };

        let (num_input_channels, num_output_channels) = unsafe {
            let input_channels =
                get_bus_channel_count(component, component_vtable, K_INPUT, 0).unwrap_or(0);
            let output_channels =
                get_bus_channel_count(component, component_vtable, K_OUTPUT, 1).unwrap_or(2);
            (input_channels, output_channels)
        };

        let info = PluginInfo::new(format!("vst3.{}", unique_id), name.clone())
            .vendor(vendor)
            .version("1.0.0".to_string())
            .audio_io(num_input_channels, num_output_channels)
            .midi(true)
            .f64_support(supports_f64);

        let host_application = HostApplication::new("vst3-host");
        let (component_handler, param_event_rx, progress_event_rx, unit_event_rx) =
            ComponentHandler::new();

        // At least 2 channels to handle common stereo cases
        let input_ptr_count = num_input_channels.max(MIN_PTR_COUNT);
        let output_ptr_count = num_output_channels.max(MIN_PTR_COUNT);

        let mut instance = Self {
            _library: library,
            interfaces: PluginInterfaces {
                component,
                component_vtable,
                processor: Some(processor),
                processor_vtable: Some(processor_vtable),
                controller,
                controller_vtable,
                separate_controller,
            },
            host: HostContext {
                application: host_application,
                handler: component_handler,
                param_event_rx,
                progress_event_rx,
                unit_event_rx,
            },
            audio: AudioIO {
                sample_rate,
                block_size,
                use_f64: false,
                num_input_channels,
                num_output_channels,
                ptrs_f32: BufferPtrs::new(input_ptr_count, output_ptr_count),
                ptrs_f64: BufferPtrs::new(input_ptr_count, output_ptr_count),
                input_events: Some(EventList::new()),
                output_events: Some(EventList::new()),
            },
            editor: EditorState {
                view: None,
                view_vtable: None,
                size: DEFAULT_EDITOR_SIZE,
            },
            info,
            is_active: false,
        };

        instance.initialize()?;

        Ok(instance)
    }

    /// Load a VST3 plugin for GUI/editor use only (no audio processing).
    ///
    /// Performs full initialization (including IAudioProcessor setup, bus
    /// activation, and setActive) but skips setProcessing. Many plugins
    /// require the component to be active before their editor works.
    pub fn load_gui_only(path: &Path) -> Result<Self> {
        // Keep full initialization including setProcessing(true) — some plugins
        // (e.g. TAL-NoiseMaker) require the audio processor to be active for
        // their editor to work. We never actually process audio on this instance.
        Self::load(path, 44100.0, 512)
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Returns the plugin's reported processing latency in samples.
    /// Returns 0 if the processor interface is unavailable.
    ///
    /// TODO(plugin-latency-runtime): VST3 plugins signal latency changes via
    /// `IComponentHandler::restartComponent(kLatencyChanged)`. We don't
    /// implement `IHostApplication::restartComponent` yet, so runtime latency
    /// updates are invisible — only the initial value queried here is seen.
    /// To support runtime updates: implement a restart handler on the host
    /// context that flips an atomic flag, poll it in the plugin-server loop
    /// (next to `poll_latency_changes`), and send `BridgeMessage::LatencyChanged`.
    pub fn get_latency_samples(&self) -> u32 {
        self.interfaces.with_processor(0, |proc, vt| unsafe {
            ((*vt).get_latency_samples)(proc)
        })
    }

    pub fn supports_f64(&self) -> bool {
        self.info.supports_f64
    }

    pub fn set_use_f64(&mut self, use_f64: bool) -> Result<&mut Self> {
        if use_f64 && !self.info.supports_f64 {
            return Err(Vst3Error::NotSupported(
                "Plugin does not support 64-bit processing".to_string(),
            ));
        }
        self.audio.use_f64 = use_f64;
        Ok(self)
    }

    pub fn sample_rate(&self) -> f64 {
        self.audio.sample_rate
    }

    pub fn set_sample_rate(&mut self, rate: f64) -> &mut Self {
        self.audio.sample_rate = rate;
        self.apply_process_setup();
        self
    }

    pub fn block_size(&self) -> usize {
        self.audio.block_size
    }

    pub fn set_block_size(&mut self, size: usize) -> &mut Self {
        self.audio.block_size = size;
        self.apply_process_setup();
        self
    }

    pub fn num_input_channels(&self) -> usize {
        self.audio.num_input_channels
    }

    pub fn num_output_channels(&self) -> usize {
        self.audio.num_output_channels
    }

    fn apply_process_setup(&mut self) {
        let symbolic_sample_size = if self.audio.use_f64 {
            K_SAMPLE_64
        } else {
            K_SAMPLE_32
        };
        let setup = ProcessSetup {
            process_mode: K_REALTIME,
            symbolic_sample_size,
            max_samples_per_block: self.audio.block_size as i32,
            sample_rate: self.audio.sample_rate,
        };

        self.interfaces.with_processor((), |proc, vt| unsafe {
            ((*vt).setup_processing)(proc, &setup);
        });
    }

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

        if !self.is_active {
            return empty_result;
        }

        if T::VST3_SYMBOLIC_SIZE == K_SAMPLE_64 && !self.info.supports_f64 {
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

        let mut input_bus = AudioBusBuffers {
            num_channels: buffer.inputs.len() as i32,
            silence_flags: 0,
            buffers: input_ptrs,
        };

        let mut output_bus = AudioBusBuffers {
            num_channels: buffer.outputs.len() as i32,
            silence_flags: 0,
            buffers: output_ptrs,
        };

        let mut input_event_list = self.audio.input_events.take().unwrap();
        let has_events = !midi_events.is_empty() || !note_expressions.is_empty();
        if has_events {
            input_event_list.update_from_midi_and_expression(midi_events, note_expressions);
        } else {
            input_event_list.clear();
        }

        let input_events = if has_events {
            input_event_list.as_ptr()
        } else {
            std::ptr::null_mut()
        };

        let mut output_event_list = self.audio.output_events.take().unwrap();
        output_event_list.clear();

        let mut input_param_changes_box: Option<Box<ParameterChangesImpl>> = param_changes
            .and_then(|pc| {
                if !pc.is_empty() {
                    Some(ParameterChangesImpl::from_changes(pc))
                } else {
                    None
                }
            });

        let input_param_changes_ptr = input_param_changes_box
            .as_mut()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null_mut());

        let mut output_param_changes = ParameterChangesImpl::new_empty();

        let mut process_context = transport.to_process_context();
        process_context.sample_rate = buffer.sample_rate;

        let mut process_data = ProcessData {
            process_mode: K_REALTIME,
            symbolic_sample_size: T::VST3_SYMBOLIC_SIZE,
            num_samples: num_samples as i32,
            num_inputs: 1,
            num_outputs: 1,
            inputs: &mut input_bus,
            outputs: &mut output_bus,
            input_param_changes: input_param_changes_ptr,
            output_param_changes: output_param_changes.as_ptr(),
            input_events,
            output_events: output_event_list.as_ptr(),
            context: &mut process_context,
        };

        let result = self
            .interfaces
            .with_processor(K_RESULT_OK, |proc, vt| unsafe {
                ((*vt).process)(proc, &mut process_data)
            });

        if result != K_RESULT_OK {
            buffer.clear_outputs();
            self.audio.input_events = Some(input_event_list);
            self.audio.output_events = Some(output_event_list);
            return empty_result;
        }

        let midi_out = output_event_list.to_midi_events();
        let param_out = output_param_changes.to_changes();

        self.audio.input_events = Some(input_event_list);
        self.audio.output_events = Some(output_event_list);

        ProcessOutput {
            midi_events: midi_out,
            parameter_changes: param_out,
        }
    }

    pub fn parameter_count(&self) -> u32 {
        self.interfaces
            .with_controller(0, |ctrl, vt| unsafe { ((*vt).get_parameter_count)(ctrl) })
            as u32
    }

    pub fn parameter(&self, index: u32) -> f64 {
        self.interfaces.with_controller(0.0, |ctrl, vt| unsafe {
            ((*vt).get_param_normalized)(ctrl, index)
        })
    }

    pub fn set_parameter(&mut self, index: u32, value: f64) -> &mut Self {
        self.interfaces.with_controller((), |ctrl, vt| unsafe {
            ((*vt).set_param_normalized)(ctrl, index, value);
        });
        self
    }

    pub fn parameter_info(&self, index: u32) -> Option<crate::ffi::Vst3ParameterInfo> {
        self.interfaces.with_controller(None, |ctrl, vt| {
            let mut info = crate::ffi::Vst3ParameterInfo::default();
            let result = unsafe {
                ((*vt).get_parameter_info)(ctrl, index as i32, &mut info as *mut _ as *mut c_void)
            };
            if result == K_RESULT_OK {
                Some(info)
            } else {
                None
            }
        })
    }

    pub fn param_event_receiver(&self) -> &Receiver<ParameterEditEvent> {
        &self.host.param_event_rx
    }

    pub fn poll_param_events(&self) -> Vec<ParameterEditEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.host.param_event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn progress_event_receiver(&self) -> &Receiver<ProgressEvent> {
        &self.host.progress_event_rx
    }

    pub fn poll_progress_events(&self) -> Vec<ProgressEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.host.progress_event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn unit_event_receiver(&self) -> &Receiver<UnitEvent> {
        &self.host.unit_event_rx
    }

    pub fn poll_unit_events(&self) -> Vec<UnitEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.host.unit_event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn state(&self) -> Result<Vec<u8>> {
        let mut stream = BStream::new();

        let result = unsafe {
            ((*self.interfaces.component_vtable).get_state)(
                self.interfaces.component,
                stream.as_ptr(),
            )
        };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return self.state_fallback();
        }

        Ok(stream.data())
    }

    fn state_fallback(&self) -> Result<Vec<u8>> {
        let param_count = self.parameter_count();
        let mut state = Vec::with_capacity(4 + (param_count as usize * 8));
        state.extend_from_slice(&param_count.to_le_bytes());
        for i in 0..param_count {
            let value = self.parameter(i);
            state.extend_from_slice(&value.to_le_bytes());
        }

        Ok(state)
    }

    pub fn set_state(&mut self, data: &[u8]) -> Result<&mut Self> {
        if data.is_empty() {
            return Err(Vst3Error::StateError("Empty state data".to_string()));
        }

        let mut stream = BStream::from_data(data.to_vec());

        let result = unsafe {
            ((*self.interfaces.component_vtable).set_state)(
                self.interfaces.component,
                stream.as_ptr(),
            )
        };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return self.set_state_fallback(data);
        }

        self.interfaces.with_controller((), |ctrl, vt| {
            let mut stream = BStream::from_data(data.to_vec());
            let _ = unsafe { ((*vt).set_component_state)(ctrl, stream.as_ptr()) };
        });

        Ok(self)
    }

    fn set_state_fallback(&mut self, data: &[u8]) -> Result<&mut Self> {
        if data.len() < 4 {
            return Err(Vst3Error::StateError("Invalid state data".to_string()));
        }

        let param_count = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if param_count < 0 {
            return Err(Vst3Error::StateError(format!(
                "Invalid param count: {}",
                param_count
            )));
        }
        let expected_size = 4usize.saturating_add((param_count as usize).saturating_mul(8));

        if data.len() != expected_size {
            return Err(Vst3Error::StateError(format!(
                "State size mismatch: expected {}, got {}",
                expected_size,
                data.len()
            )));
        }

        for i in 0..param_count {
            let offset = 4 + (i as usize * 8);
            let value = f64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            self.set_parameter(i as u32, value);
        }

        Ok(self)
    }

    pub fn has_editor(&self) -> bool {
        self.interfaces.controller.is_some()
    }

    pub fn open_editor(&mut self, parent: WindowHandle) -> Result<EditorSize> {
        let ctrl = self.interfaces.controller.ok_or(Vst3Error::NotSupported(
            "Plugin has no editor controller".to_string(),
        ))?;

        let ctrl_vtable = self
            .interfaces
            .controller_vtable
            .ok_or(Vst3Error::NotSupported(
                "Controller vtable missing".to_string(),
            ))?;

        unsafe {
            let view_type = c"editor".as_ptr();
            let view_ptr = ((*ctrl_vtable).create_view)(ctrl, view_type);

            if view_ptr.is_null() {
                return Err(Vst3Error::NotSupported(
                    "Failed to create plugin view".to_string(),
                ));
            }

            let view_vtable = *(view_ptr as *const *const IPlugViewVtable);

            #[cfg(target_os = "macos")]
            let platform_type = c"NSView".as_ptr();
            #[cfg(target_os = "windows")]
            let platform_type = c"HWND".as_ptr();
            #[cfg(target_os = "linux")]
            let platform_type = c"X11EmbedWindowID".as_ptr();

            let result = ((*view_vtable).attached)(view_ptr, parent.as_ptr(), platform_type);

            if result != K_RESULT_OK {
                return Err(Vst3Error::PluginError {
                    stage: LoadStage::Initialization,
                    code: result,
                });
            }

            let mut rect = ViewRect::default();
            let result = ((*view_vtable).get_size)(view_ptr, &mut rect);

            let (width, height) = if result == K_RESULT_OK {
                (rect.width() as u32, rect.height() as u32)
            } else {
                self.editor.size
            };

            self.editor.view = Some(view_ptr);
            self.editor.view_vtable = Some(view_vtable);
            self.editor.size = (width, height);

            Ok(EditorSize { width, height })
        }
    }

    /// Stop audio processing without deactivating.
    /// Safe to call multiple times. Drop handles full deactivation.
    pub fn stop_processing(&mut self) {
        if self.is_active {
            self.interfaces.with_processor((), |proc, vt| unsafe {
                ((*vt).set_processing)(proc, 0);
            });
        }
    }

    pub fn close_editor(&mut self) -> &mut Self {
        if let (Some(view), Some(vtable)) = (self.editor.view, self.editor.view_vtable) {
            unsafe {
                ((*vtable).removed)(view);
                release_com(view);
            }

            self.editor.view = None;
            self.editor.view_vtable = None;
        }
        self
    }

    fn connect_component_and_controller(&self) {
        let ctrl = match self.interfaces.controller {
            Some(c) => c,
            None => return,
        };

        let comp_conn =
            unsafe { query_interface(self.interfaces.component, &IID_ICONNECTION_POINT) };
        let comp_conn = match comp_conn {
            Some(ptr) => ptr,
            None => return,
        };

        let ctrl_conn = unsafe { query_interface(ctrl, &IID_ICONNECTION_POINT) };
        let ctrl_conn = match ctrl_conn {
            Some(ptr) => ptr,
            None => {
                unsafe { release_com(comp_conn) };
                return;
            }
        };

        let comp_conn_vtable = unsafe { *(comp_conn as *const *const IConnectionPointVtable) };
        let ctrl_conn_vtable = unsafe { *(ctrl_conn as *const *const IConnectionPointVtable) };

        unsafe { ((*comp_conn_vtable).connect)(comp_conn, ctrl_conn) };
        unsafe { ((*ctrl_conn_vtable).connect)(ctrl_conn, comp_conn) };

        // Connection points hold internal refs after connect
        unsafe {
            release_com(comp_conn);
            release_com(ctrl_conn);
        }
    }

    fn initialize(&mut self) -> Result<()> {
        let host_ptr = self.host.application.as_ptr();
        let result = unsafe {
            ((*self.interfaces.component_vtable).initialize)(self.interfaces.component, host_ptr)
        };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: result,
            });
        }

        // Re-query bus info after IComponent::initialize() — some plugins (e.g. Voxengo
        // Boogex) don't report their input buses until after initialization.
        unsafe {
            if let Some(ch) = get_bus_channel_count(
                self.interfaces.component,
                self.interfaces.component_vtable,
                K_INPUT,
                0,
            ) {
                if ch != self.audio.num_input_channels {
                    self.audio.num_input_channels = ch;
                    self.info = self
                        .info
                        .clone()
                        .audio_io(ch, self.audio.num_output_channels);
                    self.audio.ptrs_f32.resize_inputs(ch.max(MIN_PTR_COUNT));
                    self.audio.ptrs_f64.resize_inputs(ch.max(MIN_PTR_COUNT));
                }
            }

            if let Some(ch) = get_bus_channel_count(
                self.interfaces.component,
                self.interfaces.component_vtable,
                K_OUTPUT,
                1,
            ) {
                if ch != self.audio.num_output_channels {
                    self.audio.num_output_channels = ch;
                    self.info = self
                        .info
                        .clone()
                        .audio_io(self.audio.num_input_channels, ch);
                    self.audio.ptrs_f32.resize_outputs(ch.max(MIN_PTR_COUNT));
                    self.audio.ptrs_f64.resize_outputs(ch.max(MIN_PTR_COUNT));
                }
            }
        }

        let separate = self.interfaces.separate_controller;
        let handler_ptr = self.host.handler.as_ptr();
        self.interfaces.with_controller((), |ctrl, vt| {
            if separate {
                let _ = unsafe { ((*vt).initialize)(ctrl, host_ptr) };
            }
        });

        if separate {
            self.connect_component_and_controller();
        }

        self.interfaces.with_controller((), |ctrl, vt| {
            let _ = unsafe { ((*vt).set_component_handler)(ctrl, handler_ptr) };
        });

        let symbolic_sample_size = if self.audio.use_f64 {
            K_SAMPLE_64
        } else {
            K_SAMPLE_32
        };
        let setup = ProcessSetup {
            process_mode: K_REALTIME,
            symbolic_sample_size,
            max_samples_per_block: self.audio.block_size as i32,
            sample_rate: self.audio.sample_rate,
        };

        let result = self
            .interfaces
            .with_processor(K_RESULT_OK, |proc, vt| unsafe {
                ((*vt).setup_processing)(proc, &setup)
            });

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Setup,
                code: result,
            });
        }

        self.activate_buses()?;

        // VST3 spec requires: setActive(true) before setProcessing(true)
        let result = unsafe {
            ((*self.interfaces.component_vtable).set_active)(self.interfaces.component, 1)
        };
        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Activation,
                code: result,
            });
        }

        let result = self
            .interfaces
            .with_processor(K_RESULT_OK, |proc, vt| unsafe {
                ((*vt).set_processing)(proc, 1)
            });
        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Activation,
                code: result,
            });
        }

        self.is_active = true;
        Ok(())
    }

    fn activate_buses(&mut self) -> Result<()> {
        let num_input_buses = unsafe {
            ((*self.interfaces.component_vtable).get_bus_count)(
                self.interfaces.component,
                K_AUDIO,
                K_INPUT,
            )
        };

        for i in 0..num_input_buses {
            unsafe {
                ((*self.interfaces.component_vtable).activate_bus)(
                    self.interfaces.component,
                    K_AUDIO,
                    K_INPUT,
                    i,
                    1,
                );
            }
        }

        let num_output_buses = unsafe {
            ((*self.interfaces.component_vtable).get_bus_count)(
                self.interfaces.component,
                K_AUDIO,
                K_OUTPUT,
            )
        };

        for i in 0..num_output_buses {
            unsafe {
                ((*self.interfaces.component_vtable).activate_bus)(
                    self.interfaces.component,
                    K_AUDIO,
                    K_OUTPUT,
                    i,
                    1,
                );
            }
        }

        Ok(())
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        self.close_editor();

        if self.is_active {
            self.interfaces.with_processor((), |proc, vt| unsafe {
                ((*vt).set_processing)(proc, 0);
            });
            unsafe {
                ((*self.interfaces.component_vtable).set_active)(self.interfaces.component, 0);
            }
        }

        unsafe {
            ((*self.interfaces.component_vtable).terminate)(self.interfaces.component);
        }

        self.interfaces.with_controller((), |ctrl, vt| unsafe {
            ((*vt).terminate)(ctrl);
        });

        unsafe {
            release_com(self.interfaces.component);

            if let Some(proc) = self.interfaces.processor {
                release_com(proc);
            }
            if let Some(ctrl) = self.interfaces.controller {
                release_com(ctrl);
            }
        }
    }
}
