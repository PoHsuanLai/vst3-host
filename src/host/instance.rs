//! VST3 plugin instance management.

use std::path::Path;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use smallvec::SmallVec;
use vst3::com_scrape_types::Unknown;
use vst3::{ComPtr, ComWrapper};
use vst3::Steinberg::{
    kResultFalse, kResultOk, FUnknown, IBStream, IPluginBaseTrait, IPlugView, IPlugViewTrait,
    ViewRect,
    Vst::{
        BusDirections_::{kInput, kOutput},
        IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentTrait, IConnectionPoint,
        IConnectionPointTrait, IEditController, IEditControllerTrait, IEventList,
        IParameterChanges,
        MediaTypes_::{kAudio, kEvent},
        ProcessModes_::kRealtime,
        ProcessSetup,
    },
};

#[cfg(target_os = "macos")]
use vst3::Steinberg::kPlatformTypeNSView;
#[cfg(target_os = "windows")]
use vst3::Steinberg::kPlatformTypeHWND;
#[cfg(target_os = "linux")]
use vst3::Steinberg::kPlatformTypeX11EmbedWindowID;

use crate::com::{
    BStream, ComponentHandler, EventList, HostApplication, ParameterChangesImpl,
    ParameterEditEvent, ProgressEvent, UnitEvent,
};
use crate::error::{LoadStage, Result, Vst3Error};
use crate::types::{
    AudioBuffer, BufferPtrs, BusInfo as BusInfoWrap, EditorSize, MidiEvent, NoteExpressionValue,
    ParameterChanges, PluginInfo, ProcessOutput, Sample, TransportState, Vst3ParameterInfo,
    WindowHandle,
};

use super::library::Vst3Library;

// Re-express the handful of int-constant references we use in plugin API calls
// as `i32` for `media_type`/`direction` arguments.
const K_AUDIO: i32 = kAudio as i32;
const K_EVENT: i32 = kEvent as i32;
const K_INPUT: i32 = kInput as i32;
const K_OUTPUT: i32 = kOutput as i32;
const K_REALTIME: i32 = kRealtime as i32;

pub(crate) struct PluginInterfaces {
    pub component: ComPtr<IComponent>,
    pub processor: Option<ComPtr<IAudioProcessor>>,
    pub controller: Option<ComPtr<IEditController>>,
    pub separate_controller: bool,
}

unsafe impl Send for PluginInterfaces {}
unsafe impl Sync for PluginInterfaces {}

pub(crate) struct HostContext {
    pub application: ComWrapper<HostApplication>,
    pub handler: ComWrapper<ComponentHandler>,
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
    pub input_events: ComWrapper<EventList>,
    pub output_events: ComWrapper<EventList>,
}

pub(crate) struct EditorState {
    pub view: Option<ComPtr<IPlugView>>,
    pub size: (u32, u32),
}

unsafe impl Send for EditorState {}
unsafe impl Sync for EditorState {}

const MIN_PTR_COUNT: usize = 2;
const DEFAULT_EDITOR_SIZE: (u32, u32) = (800, 600);

fn cid_to_string(cid_bytes: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        cid_bytes[0], cid_bytes[1], cid_bytes[2], cid_bytes[3],
        cid_bytes[4], cid_bytes[5], cid_bytes[6], cid_bytes[7],
        cid_bytes[8], cid_bytes[9], cid_bytes[10], cid_bytes[11],
        cid_bytes[12], cid_bytes[13], cid_bytes[14], cid_bytes[15]
    )
}

fn get_bus_channel_count(
    component: &ComPtr<IComponent>,
    direction: i32,
    min_channels: i32,
) -> Option<usize> {
    unsafe {
        let num_buses = component.getBusCount(K_AUDIO, direction);
        if num_buses <= 0 {
            return None;
        }
        let mut bus = BusInfoWrap::default();
        if component.getBusInfo(K_AUDIO, direction, 0, bus.as_mut_inner()) == kResultOk {
            Some(bus.channel_count().max(min_channels) as usize)
        } else {
            None
        }
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

        let (class_cid, class_cid_bytes, name) = (0..count)
            .find_map(|i| {
                let info = library.get_class_info(i).ok()?;
                if info.category.contains("Audio") {
                    Some((info.cid, info.cid_bytes, info.name))
                } else {
                    None
                }
            })
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "No audio processor classes found in VST3".to_string(),
            })?;

        let component: ComPtr<IComponent> = library.create_instance(&class_cid)?;

        let (num_inputs, num_outputs) = (
            get_bus_channel_count(&component, K_INPUT, 0).unwrap_or(0),
            get_bus_channel_count(&component, K_OUTPUT, 1).unwrap_or(2),
        );

        let supports_f64 = component
            .cast::<IAudioProcessor>()
            .map(|proc| unsafe {
                proc.canProcessSampleSize(crate::types::K_SAMPLE_64_INT) == kResultOk
            })
            .unwrap_or(false);

        let unique_id = cid_to_string(&class_cid_bytes);
        let receives_midi = unsafe { component.getBusCount(K_EVENT, K_INPUT) > 0 };

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

        let (class_cid, class_cid_bytes, name) = (0..count)
            .find_map(|i| {
                let info = library.get_class_info(i).ok()?;
                if info.category.contains("Audio") {
                    Some((info.cid, info.cid_bytes, info.name))
                } else {
                    None
                }
            })
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "No audio processor classes found in VST3".to_string(),
            })?;

        let component: ComPtr<IComponent> = library.create_instance(&class_cid)?;

        let processor = component
            .cast::<IAudioProcessor>()
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Instantiation,
                reason: "VST3 plugin does not support IAudioProcessor".to_string(),
            })?;

        // Try single-component model first, fall back to separate controller.
        let (controller, separate_controller) = match component.cast::<IEditController>() {
            Some(ctrl) => (Some(ctrl), false),
            None => {
                let mut controller_cid = [0i8; 16];
                let result = unsafe { component.getControllerClassId(&mut controller_cid) };
                if result == kResultOk && controller_cid != [0i8; 16] {
                    match library.create_instance::<IEditController>(&controller_cid) {
                        Ok(ctrl) => (Some(ctrl), true),
                        Err(_) => (None, false),
                    }
                } else {
                    (None, false)
                }
            }
        };

        let supports_f64 = unsafe {
            processor.canProcessSampleSize(crate::types::K_SAMPLE_64_INT) == kResultOk
        };

        let num_input_channels = get_bus_channel_count(&component, K_INPUT, 0).unwrap_or(0);
        let num_output_channels = get_bus_channel_count(&component, K_OUTPUT, 1).unwrap_or(2);

        let unique_id = cid_to_string(&class_cid_bytes);
        let info = PluginInfo::new(format!("vst3.{}", unique_id), name.clone())
            .vendor(vendor)
            .version("1.0.0".to_string())
            .audio_io(num_input_channels, num_output_channels)
            .midi(true)
            .f64_support(supports_f64);

        let host_application = HostApplication::new("vst3-host");
        let (component_handler, param_event_rx, progress_event_rx, unit_event_rx) =
            ComponentHandler::new();

        let input_ptr_count = num_input_channels.max(MIN_PTR_COUNT);
        let output_ptr_count = num_output_channels.max(MIN_PTR_COUNT);

        let mut instance = Self {
            _library: library,
            interfaces: PluginInterfaces {
                component,
                processor: Some(processor),
                controller,
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
                input_events: EventList::new(),
                output_events: EventList::new(),
            },
            editor: EditorState {
                view: None,
                size: DEFAULT_EDITOR_SIZE,
            },
            info,
            is_active: false,
        };

        instance.initialize()?;

        Ok(instance)
    }

    /// Load a VST3 plugin for GUI/editor use only (no audio processing).
    pub fn load_gui_only(path: &Path) -> Result<Self> {
        Self::load(path, 44100.0, 512)
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    pub fn get_latency_samples(&self) -> u32 {
        match &self.interfaces.processor {
            Some(p) => unsafe { p.getLatencySamples() },
            None => 0,
        }
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
        if let Some(ref p) = self.interfaces.processor {
            unsafe {
                p.setupProcessing(&mut setup);
            }
        }
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

        let processor = match &self.interfaces.processor {
            Some(p) => p.clone(),
            None => return empty_result,
        };

        if !self.is_active
            || (T::VST3_SYMBOLIC_SIZE == crate::types::K_SAMPLE_64_INT && !self.info.supports_f64)
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

        let mut process_data: vst3::Steinberg::Vst::ProcessData =
            unsafe { std::mem::zeroed() };
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

    pub fn parameter_count(&self) -> u32 {
        match &self.interfaces.controller {
            Some(c) => unsafe { c.getParameterCount() as u32 },
            None => 0,
        }
    }

    pub fn parameter(&self, index: u32) -> f64 {
        match &self.interfaces.controller {
            Some(c) => unsafe { c.getParamNormalized(index) },
            None => 0.0,
        }
    }

    pub fn set_parameter(&mut self, index: u32, value: f64) -> &mut Self {
        if let Some(c) = &self.interfaces.controller {
            unsafe {
                c.setParamNormalized(index, value);
            }
        }
        self
    }

    pub fn parameter_info(&self, index: u32) -> Option<Vst3ParameterInfo> {
        let controller = self.interfaces.controller.as_ref()?;
        let mut raw: vst3::Steinberg::Vst::ParameterInfo = unsafe { std::mem::zeroed() };
        let result = unsafe { controller.getParameterInfo(index as i32, &mut raw) };
        (result == kResultOk).then(|| Vst3ParameterInfo::from_c(&raw))
    }

    pub fn param_event_receiver(&self) -> &Receiver<ParameterEditEvent> {
        &self.host.param_event_rx
    }

    pub fn poll_param_events(&self) -> Vec<ParameterEditEvent> {
        self.host.param_event_rx.try_iter().collect()
    }

    pub fn progress_event_receiver(&self) -> &Receiver<ProgressEvent> {
        &self.host.progress_event_rx
    }

    pub fn poll_progress_events(&self) -> Vec<ProgressEvent> {
        self.host.progress_event_rx.try_iter().collect()
    }

    pub fn unit_event_receiver(&self) -> &Receiver<UnitEvent> {
        &self.host.unit_event_rx
    }

    pub fn poll_unit_events(&self) -> Vec<UnitEvent> {
        self.host.unit_event_rx.try_iter().collect()
    }

    pub fn state(&self) -> Result<Vec<u8>> {
        let stream = BStream::new();
        let stream_ptr = stream
            .as_com_ref::<IBStream>()
            .ok_or_else(|| Vst3Error::StateError("Failed to wrap BStream".into()))?;

        let result = unsafe { self.interfaces.component.getState(stream_ptr.as_ptr()) };

        if result != kResultOk && result != kResultFalse {
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

        let stream = BStream::from_data(data.to_vec());
        let stream_ptr = stream
            .as_com_ref::<IBStream>()
            .ok_or_else(|| Vst3Error::StateError("Failed to wrap BStream".into()))?;

        let result = unsafe { self.interfaces.component.setState(stream_ptr.as_ptr()) };

        if result != kResultOk && result != kResultFalse {
            return self.set_state_fallback(data);
        }

        if let Some(ctrl) = &self.interfaces.controller {
            let ctrl_stream = BStream::from_data(data.to_vec());
            if let Some(ctrl_stream_ref) = ctrl_stream.as_com_ref::<IBStream>() {
                unsafe {
                    let _ = ctrl.setComponentState(ctrl_stream_ref.as_ptr());
                }
            }
        }

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
        let ctrl = self
            .interfaces
            .controller
            .as_ref()
            .ok_or(Vst3Error::NotSupported(
                "Plugin has no editor controller".to_string(),
            ))?;

        let view_raw = unsafe { ctrl.createView(c"editor".as_ptr()) };
        let view = unsafe { ComPtr::from_raw(view_raw) }.ok_or(Vst3Error::NotSupported(
            "Failed to create plugin view".to_string(),
        ))?;

        #[cfg(target_os = "macos")]
        let platform_type = kPlatformTypeNSView;
        #[cfg(target_os = "windows")]
        let platform_type = kPlatformTypeHWND;
        #[cfg(target_os = "linux")]
        let platform_type = kPlatformTypeX11EmbedWindowID;

        let result = unsafe { view.attached(parent.as_ptr(), platform_type) };
        if result != kResultOk {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: result,
            });
        }

        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let size_result = unsafe { view.getSize(&mut rect) };

        let (width, height) = if size_result == kResultOk {
            (
                (rect.right - rect.left) as u32,
                (rect.bottom - rect.top) as u32,
            )
        } else {
            self.editor.size
        };

        self.editor.view = Some(view);
        self.editor.size = (width, height);

        Ok(EditorSize { width, height })
    }

    pub fn stop_processing(&mut self) {
        if self.is_active {
            if let Some(p) = &self.interfaces.processor {
                unsafe {
                    p.setProcessing(0);
                }
            }
        }
    }

    pub fn close_editor(&mut self) -> &mut Self {
        if let Some(view) = self.editor.view.take() {
            unsafe {
                view.removed();
            }
        }
        self
    }

    fn connect_component_and_controller(&self) {
        let ctrl = match &self.interfaces.controller {
            Some(c) => c,
            None => return,
        };

        let comp_conn = match self.interfaces.component.cast::<IConnectionPoint>() {
            Some(p) => p,
            None => return,
        };
        let ctrl_conn = match ctrl.cast::<IConnectionPoint>() {
            Some(p) => p,
            None => return,
        };

        unsafe {
            comp_conn.connect(ctrl_conn.as_ptr());
            ctrl_conn.connect(comp_conn.as_ptr());
        }
    }

    fn initialize(&mut self) -> Result<()> {
        let host_ptr: *mut FUnknown = self
            .host
            .application
            .to_com_ptr::<vst3::Steinberg::Vst::IHostApplication>()
            .ok_or(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: 0,
            })?
            .upcast::<FUnknown>()
            .into_raw();

        let result = unsafe { self.interfaces.component.initialize(host_ptr) };
        if result != kResultOk && result != kResultFalse {
            unsafe {
                FUnknown::release(host_ptr);
            }
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: result,
            });
        }

        if let Some(ch) = get_bus_channel_count(&self.interfaces.component, K_INPUT, 0) {
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
        if let Some(ch) = get_bus_channel_count(&self.interfaces.component, K_OUTPUT, 1) {
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

        let separate = self.interfaces.separate_controller;
        if separate {
            if let Some(ctrl) = &self.interfaces.controller {
                unsafe {
                    let _ = ctrl.initialize(host_ptr);
                }
            }
            self.connect_component_and_controller();
        }

        if let Some(ctrl) = &self.interfaces.controller {
            let handler_ptr: *mut vst3::Steinberg::Vst::IComponentHandler = self
                .host
                .handler
                .as_com_ref::<vst3::Steinberg::Vst::IComponentHandler>()
                .map(|r| r.as_ptr())
                .unwrap_or(std::ptr::null_mut());
            unsafe {
                let _ = ctrl.setComponentHandler(handler_ptr);
            }
        }

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

        if let Some(p) = &self.interfaces.processor {
            let result = unsafe { p.setupProcessing(&mut setup) };
            if result != kResultOk && result != kResultFalse {
                return Err(Vst3Error::PluginError {
                    stage: LoadStage::Setup,
                    code: result,
                });
            }
        }

        self.activate_buses()?;

        let result = unsafe { self.interfaces.component.setActive(1) };
        if result != kResultOk && result != kResultFalse {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Activation,
                code: result,
            });
        }

        if let Some(p) = &self.interfaces.processor {
            let result = unsafe { p.setProcessing(1) };
            if result != kResultOk && result != kResultFalse {
                return Err(Vst3Error::PluginError {
                    stage: LoadStage::Activation,
                    code: result,
                });
            }
        }

        self.is_active = true;
        Ok(())
    }

    fn activate_buses(&mut self) -> Result<()> {
        unsafe {
            let num_input_buses = self.interfaces.component.getBusCount(K_AUDIO, K_INPUT);
            for i in 0..num_input_buses {
                self.interfaces
                    .component
                    .activateBus(K_AUDIO, K_INPUT, i, 1);
            }
            let num_output_buses = self.interfaces.component.getBusCount(K_AUDIO, K_OUTPUT);
            for i in 0..num_output_buses {
                self.interfaces
                    .component
                    .activateBus(K_AUDIO, K_OUTPUT, i, 1);
            }
        }
        Ok(())
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        self.close_editor();

        if self.is_active {
            if let Some(p) = &self.interfaces.processor {
                unsafe {
                    p.setProcessing(0);
                }
            }
            unsafe {
                self.interfaces.component.setActive(0);
            }
        }
        unsafe {
            self.interfaces.component.terminate();
        }
        if let Some(ctrl) = &self.interfaces.controller {
            unsafe {
                ctrl.terminate();
            }
        }
    }
}
