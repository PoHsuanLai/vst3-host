//! VST3 plugin instance management.

use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use smallvec::SmallVec;

use crate::com::{
    BStream, ComponentHandler, EventList, HostApplication, ParameterChangesImpl, ParameterEditEvent,
};
use crate::error::{LoadStage, Result, Vst3Error};
use crate::ffi::{
    AudioBusBuffers, BusInfo, IAudioProcessorVtable, IComponentVtable, IConnectionPointVtable,
    IEditControllerVtable, IPlugViewVtable, IUnknownVtable, ProcessData, ProcessSetup, ViewRect,
    IID_IAUDIO_PROCESSOR, IID_ICOMPONENT, IID_ICONNECTION_POINT, IID_IEDIT_CONTROLLER, K_AUDIO,
    K_INPUT, K_OUTPUT, K_REALTIME, K_RESULT_OK, K_RESULT_TRUE, K_SAMPLE_32, K_SAMPLE_64,
};
use crate::types::{
    AudioBuffer, MidiEvent, NoteExpressionValue, ParameterChanges, PluginInfo, Sample,
    TransportState, Vst3MidiEvent,
};

use super::library::Vst3Library;

/// A loaded VST3 plugin instance.
///
/// This struct manages the lifecycle of a VST3 plugin, including initialization,
/// audio processing, parameter control, and cleanup.
///
/// # Example
///
/// ```ignore
/// use vst3_host::{Vst3Instance, AudioBuffer, MidiEvent, TransportState};
///
/// // Load the plugin
/// let mut plugin = Vst3Instance::load("/path/to/plugin.vst3", 44100.0, 512)?;
///
/// // Process audio
/// let midi = vec![MidiEvent::note_on(0, 0, 60, 0.8)];
/// let transport = TransportState::new().tempo(120.0).playing(true);
/// plugin.process(&mut buffer, &midi, &transport)?;
/// ```
pub struct Vst3Instance {
    _library: Arc<Vst3Library>,
    component: *mut c_void,
    component_vtable: *const IComponentVtable,
    processor: *mut c_void,
    processor_vtable: *const IAudioProcessorVtable,
    controller: Option<*mut c_void>,
    controller_vtable: Option<*const IEditControllerVtable>,
    view: Option<*mut c_void>,
    view_vtable: Option<*const IPlugViewVtable>,
    #[allow(dead_code)]
    host_application: Box<HostApplication>,
    #[allow(dead_code)]
    component_handler: Box<ComponentHandler>,
    param_event_rx: Receiver<ParameterEditEvent>,
    info: PluginInfo,
    sample_rate: f64,
    is_active: bool,
    block_size: usize,
    num_input_channels: usize,
    num_output_channels: usize,
    input_buffer_ptrs: Vec<*mut f32>,
    output_buffer_ptrs: Vec<*mut f32>,
    input_buffer_ptrs_f64: Vec<*mut f64>,
    output_buffer_ptrs_f64: Vec<*mut f64>,
    use_f64: bool,
    editor_size: (u32, u32),
    /// Reused across process calls to avoid allocation.
    input_event_list: Option<Box<EventList>>,
    output_event_list: Option<Box<EventList>>,
    separate_controller: bool,
}

unsafe impl Send for Vst3Instance {}
unsafe impl Sync for Vst3Instance {}

impl Vst3Instance {
    /// Load a VST3 plugin from a bundle path.
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

        let processor = {
            let vtable = unsafe { *(component as *const *const IUnknownVtable) };
            let mut proc_ptr: *mut c_void = std::ptr::null_mut();
            let result = unsafe {
                ((*vtable).query_interface)(component, &IID_IAUDIO_PROCESSOR, &mut proc_ptr)
            };
            if result == K_RESULT_OK && !proc_ptr.is_null() {
                proc_ptr
            } else {
                return Err(Vst3Error::LoadFailed {
                    path: path.to_path_buf(),
                    stage: LoadStage::Instantiation,
                    reason: "VST3 plugin does not support IAudioProcessor".to_string(),
                });
            }
        };

        let component_vtable = unsafe { *(component as *const *const IComponentVtable) };

        // Try single-component model first (IEditController from same object)
        let mut controller: Option<*mut c_void> = {
            let vtable = unsafe { *(component as *const *const IUnknownVtable) };
            let mut ctrl_ptr: *mut c_void = std::ptr::null_mut();
            let result = unsafe {
                ((*vtable).query_interface)(component, &IID_IEDIT_CONTROLLER, &mut ctrl_ptr)
            };
            if result == K_RESULT_OK && !ctrl_ptr.is_null() {
                Some(ctrl_ptr)
            } else {
                None
            }
        };

        let mut separate_controller = false;

        // Fall back to separate controller model
        if controller.is_none() {
            let mut controller_cid = [0u8; 16];
            let result = unsafe {
                ((*component_vtable).get_controller_class_id)(component, &mut controller_cid)
            };

            if result == K_RESULT_OK && controller_cid != [0u8; 16] {
                if let Ok(ctrl_ptr) =
                    library.create_instance(&controller_cid, &IID_IEDIT_CONTROLLER)
                {
                    controller = Some(ctrl_ptr);
                    separate_controller = true;
                }
            }
        }
        let processor_vtable = unsafe { *(processor as *const *const IAudioProcessorVtable) };
        let controller_vtable =
            controller.map(|c| unsafe { *(c as *const *const IEditControllerVtable) });

        let unique_id = format!(
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
            class_info[0], class_info[1], class_info[2], class_info[3],
            class_info[4], class_info[5], class_info[6], class_info[7],
            class_info[8], class_info[9], class_info[10], class_info[11],
            class_info[12], class_info[13], class_info[14], class_info[15]
        );

        let supports_f64 = {
            let vtable = unsafe { &*processor_vtable };
            let result = unsafe { (vtable.can_process_sample_size)(processor, K_SAMPLE_64) };
            result == K_RESULT_OK
        };

        let (num_input_channels, num_output_channels) = unsafe {
            let vtable = &*component_vtable;

            let num_input_buses = (vtable.get_bus_count)(component, K_AUDIO, K_INPUT);
            let input_channels = if num_input_buses > 0 {
                let mut bus_info = BusInfo::default();
                let result = (vtable.get_bus_info)(
                    component,
                    K_AUDIO,
                    K_INPUT,
                    0,
                    &mut bus_info as *mut _ as *mut c_void,
                );
                if result == K_RESULT_OK {
                    bus_info.channel_count.max(0) as usize
                } else {
                    2
                }
            } else {
                0
            };

            let num_output_buses = (vtable.get_bus_count)(component, K_AUDIO, K_OUTPUT);
            let output_channels = if num_output_buses > 0 {
                let mut bus_info = BusInfo::default();
                let result = (vtable.get_bus_info)(
                    component,
                    K_AUDIO,
                    K_OUTPUT,
                    0,
                    &mut bus_info as *mut _ as *mut c_void,
                );
                if result == K_RESULT_OK {
                    bus_info.channel_count.max(1) as usize
                } else {
                    2
                }
            } else {
                2
            };

            (input_channels, output_channels)
        };

        let info = PluginInfo::new(format!("vst3.{}", unique_id), name.clone())
            .vendor(vendor)
            .version("1.0.0".to_string())
            .audio_io(num_input_channels, num_output_channels)
            .midi(true)
            .f64_support(supports_f64);

        let host_application = HostApplication::new("vst3-host");
        let (component_handler, param_event_rx) = ComponentHandler::new();

        // At least 2 channels to handle common stereo cases
        let input_ptr_count = num_input_channels.max(2);
        let output_ptr_count = num_output_channels.max(2);

        let mut instance = Self {
            _library: library,
            component,
            component_vtable,
            processor,
            processor_vtable,
            controller,
            controller_vtable,
            view: None,
            view_vtable: None,
            host_application,
            component_handler,
            param_event_rx,
            info,
            sample_rate,
            is_active: false,
            block_size,
            num_input_channels,
            num_output_channels,
            input_buffer_ptrs: vec![std::ptr::null_mut(); input_ptr_count],
            output_buffer_ptrs: vec![std::ptr::null_mut(); output_ptr_count],
            input_buffer_ptrs_f64: vec![std::ptr::null_mut(); input_ptr_count],
            output_buffer_ptrs_f64: vec![std::ptr::null_mut(); output_ptr_count],
            use_f64: false,
            editor_size: (800, 600),
            input_event_list: Some(EventList::new()),
            output_event_list: Some(EventList::new()),
            separate_controller,
        };

        instance.initialize()?;

        Ok(instance)
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    pub fn supports_f64(&self) -> bool {
        self.info.supports_f64
    }

    /// Returns an error if the plugin doesn't support f64.
    pub fn set_use_f64(&mut self, use_f64: bool) -> Result<()> {
        if use_f64 && !self.info.supports_f64 {
            return Err(Vst3Error::NotSupported(
                "Plugin does not support 64-bit processing".to_string(),
            ));
        }
        self.use_f64 = use_f64;
        Ok(())
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, rate: f64) {
        self.sample_rate = rate;
        self.apply_process_setup();
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn set_block_size(&mut self, size: usize) {
        self.block_size = size;
        self.apply_process_setup();
    }

    pub fn num_input_channels(&self) -> usize {
        self.num_input_channels
    }

    pub fn num_output_channels(&self) -> usize {
        self.num_output_channels
    }

    fn apply_process_setup(&mut self) {
        let symbolic_sample_size = if self.use_f64 {
            K_SAMPLE_64
        } else {
            K_SAMPLE_32
        };
        let setup = ProcessSetup {
            process_mode: K_REALTIME,
            symbolic_sample_size,
            max_samples_per_block: self.block_size as i32,
            sample_rate: self.sample_rate,
        };

        unsafe {
            ((*self.processor_vtable).setup_processing)(self.processor, &setup);
        }
    }

    /// Process audio through the plugin. Returns (output MIDI events, output parameter changes).
    pub fn process<T: Sample, E: Vst3MidiEvent>(
        &mut self,
        buffer: &mut AudioBuffer<T>,
        midi_events: &[E],
        param_changes: Option<&ParameterChanges>,
        note_expressions: &[NoteExpressionValue],
        transport: &TransportState,
    ) -> (SmallVec<[MidiEvent; 64]>, ParameterChanges) {
        let empty_result = (SmallVec::new(), ParameterChanges::new());

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

        // SAFETY: VST3's AudioBusBuffers requires `*mut *mut c_void` for both inputs and outputs.
        // We cast input `*const T` to `*mut T` to satisfy the C API. Well-behaved plugins must
        // not mutate input buffers. Copying inputs to scratch buffers would add per-block overhead.
        let (input_ptrs, output_ptrs): (*mut *mut c_void, *mut *mut c_void) =
            if T::VST3_SYMBOLIC_SIZE == K_SAMPLE_64 {
                for (i, input_slice) in buffer.inputs.iter().enumerate() {
                    if i < self.input_buffer_ptrs_f64.len() {
                        self.input_buffer_ptrs_f64[i] = input_slice.as_ptr() as *mut f64;
                    }
                }
                for (i, output_slice) in buffer.outputs.iter_mut().enumerate() {
                    if i < self.output_buffer_ptrs_f64.len() {
                        self.output_buffer_ptrs_f64[i] = output_slice.as_mut_ptr() as *mut f64;
                    }
                }
                (
                    self.input_buffer_ptrs_f64.as_mut_ptr() as *mut *mut c_void,
                    self.output_buffer_ptrs_f64.as_mut_ptr() as *mut *mut c_void,
                )
            } else {
                for (i, input_slice) in buffer.inputs.iter().enumerate() {
                    if i < self.input_buffer_ptrs.len() {
                        self.input_buffer_ptrs[i] = input_slice.as_ptr() as *mut f32;
                    }
                }
                for (i, output_slice) in buffer.outputs.iter_mut().enumerate() {
                    if i < self.output_buffer_ptrs.len() {
                        self.output_buffer_ptrs[i] = output_slice.as_mut_ptr() as *mut f32;
                    }
                }
                (
                    self.input_buffer_ptrs.as_mut_ptr() as *mut *mut c_void,
                    self.output_buffer_ptrs.as_mut_ptr() as *mut *mut c_void,
                )
            };

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

        let mut input_event_list = self.input_event_list.take().unwrap();
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

        let mut output_event_list = self.output_event_list.take().unwrap();
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

        let result =
            unsafe { ((*self.processor_vtable).process)(self.processor, &mut process_data) };

        if result != K_RESULT_OK {
            buffer.clear_outputs();
            self.input_event_list = Some(input_event_list);
            self.output_event_list = Some(output_event_list);
            return empty_result;
        }

        let midi_out = output_event_list.to_midi_events();
        let param_out = output_param_changes.to_changes();

        self.input_event_list = Some(input_event_list);
        self.output_event_list = Some(output_event_list);

        (midi_out, param_out)
    }

    pub fn get_parameter_count(&self) -> i32 {
        if let Some(ctrl) = self.controller {
            if let Some(vtable) = self.controller_vtable {
                return unsafe { ((*vtable).get_parameter_count)(ctrl) };
            }
        }
        0
    }

    pub fn get_parameter(&self, index: u32) -> f64 {
        if let Some(ctrl) = self.controller {
            if let Some(vtable) = self.controller_vtable {
                return unsafe { ((*vtable).get_param_normalized)(ctrl, index) };
            }
        }
        0.0
    }

    pub fn set_parameter(&mut self, index: u32, value: f64) {
        if let Some(ctrl) = self.controller {
            if let Some(vtable) = self.controller_vtable {
                unsafe {
                    ((*vtable).set_param_normalized)(ctrl, index, value);
                }
            }
        }
    }

    pub fn get_parameter_info(&self, index: i32) -> Option<crate::ffi::Vst3ParameterInfo> {
        if let Some(ctrl) = self.controller {
            if let Some(vtable) = self.controller_vtable {
                let mut info = crate::ffi::Vst3ParameterInfo::default();
                let result = unsafe {
                    ((*vtable).get_parameter_info)(
                        ctrl,
                        index,
                        &mut info as *mut _ as *mut std::ffi::c_void,
                    )
                };
                if result == crate::ffi::K_RESULT_OK {
                    return Some(info);
                }
            }
        }
        None
    }

    pub fn param_event_receiver(&self) -> &Receiver<ParameterEditEvent> {
        &self.param_event_rx
    }

    pub fn poll_param_events(&self) -> Vec<ParameterEditEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.param_event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn get_state(&self) -> Result<Vec<u8>> {
        let mut stream = BStream::new();

        let result =
            unsafe { ((*self.component_vtable).get_state)(self.component, stream.as_ptr()) };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return self.get_state_fallback();
        }

        Ok(stream.data())
    }

    fn get_state_fallback(&self) -> Result<Vec<u8>> {
        let param_count = self.get_parameter_count();
        let mut state = Vec::with_capacity(4 + (param_count as usize * 8));
        state.extend_from_slice(&param_count.to_le_bytes());
        for i in 0..param_count as u32 {
            let value = self.get_parameter(i);
            state.extend_from_slice(&value.to_le_bytes());
        }

        Ok(state)
    }

    pub fn set_state(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Err(Vst3Error::StateError("Empty state data".to_string()));
        }

        let mut stream = BStream::from_data(data.to_vec());

        let result =
            unsafe { ((*self.component_vtable).set_state)(self.component, stream.as_ptr()) };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return self.set_state_fallback(data);
        }

        if let (Some(ctrl), Some(vtable)) = (self.controller, self.controller_vtable) {
            let mut stream = BStream::from_data(data.to_vec());
            let _ = unsafe { ((*vtable).set_component_state)(ctrl, stream.as_ptr()) };
        }

        Ok(())
    }

    fn set_state_fallback(&mut self, data: &[u8]) -> Result<()> {
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

        Ok(())
    }

    pub fn has_editor(&self) -> bool {
        self.controller.is_some()
    }

    /// # Safety
    ///
    /// `parent` must be a valid window handle for the target platform.
    pub unsafe fn open_editor(&mut self, parent: *mut c_void) -> Result<(u32, u32)> {
        let ctrl = self.controller.ok_or(Vst3Error::NotSupported(
            "Plugin has no editor controller".to_string(),
        ))?;

        let ctrl_vtable = self.controller_vtable.ok_or(Vst3Error::NotSupported(
            "Controller vtable missing".to_string(),
        ))?;

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

        let result = ((*view_vtable).attached)(view_ptr, parent, platform_type);

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
            self.editor_size
        };

        self.view = Some(view_ptr);
        self.view_vtable = Some(view_vtable);
        self.editor_size = (width, height);

        Ok((width, height))
    }

    pub fn close_editor(&mut self) {
        if let (Some(view), Some(vtable)) = (self.view, self.view_vtable) {
            unsafe {
                ((*vtable).removed)(view);

                let view_unknown = *(view as *const *const IUnknownVtable);
                ((*view_unknown).release)(view);
            }

            self.view = None;
            self.view_vtable = None;
        }
    }

    /// Required for separate controller model.
    fn connect_component_and_controller(&self) {
        let ctrl = match self.controller {
            Some(c) => c,
            None => return,
        };

        let comp_unknown = unsafe { *(self.component as *const *const IUnknownVtable) };
        let mut comp_conn_ptr: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            ((*comp_unknown).query_interface)(
                self.component,
                &IID_ICONNECTION_POINT,
                &mut comp_conn_ptr,
            )
        };

        if result != K_RESULT_OK || comp_conn_ptr.is_null() {
            return;
        }

        let ctrl_unknown = unsafe { *(ctrl as *const *const IUnknownVtable) };
        let mut ctrl_conn_ptr: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            ((*ctrl_unknown).query_interface)(ctrl, &IID_ICONNECTION_POINT, &mut ctrl_conn_ptr)
        };

        if result != K_RESULT_OK || ctrl_conn_ptr.is_null() {
            unsafe {
                let comp_unknown = *(comp_conn_ptr as *const *const IUnknownVtable);
                ((*comp_unknown).release)(comp_conn_ptr);
            }
            return;
        }

        let comp_conn_vtable = unsafe { *(comp_conn_ptr as *const *const IConnectionPointVtable) };
        let ctrl_conn_vtable = unsafe { *(ctrl_conn_ptr as *const *const IConnectionPointVtable) };

        unsafe { ((*comp_conn_vtable).connect)(comp_conn_ptr, ctrl_conn_ptr) };
        unsafe { ((*ctrl_conn_vtable).connect)(ctrl_conn_ptr, comp_conn_ptr) };

        // Connection points hold internal refs after connect
        unsafe {
            let comp_unknown = *(comp_conn_ptr as *const *const IUnknownVtable);
            ((*comp_unknown).release)(comp_conn_ptr);
            let ctrl_unknown = *(ctrl_conn_ptr as *const *const IUnknownVtable);
            ((*ctrl_unknown).release)(ctrl_conn_ptr);
        }
    }

    fn initialize(&mut self) -> Result<()> {
        let host_ptr = self.host_application.as_ptr();
        let result = unsafe { ((*self.component_vtable).initialize)(self.component, host_ptr) };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: result,
            });
        }

        // Re-query bus info after IComponent::initialize() — some plugins (e.g. Voxengo
        // Boogex) don't report their input buses until after initialization.
        unsafe {
            let vtable = &*self.component_vtable;

            let num_input_buses = (vtable.get_bus_count)(self.component, K_AUDIO, K_INPUT);
            if num_input_buses > 0 {
                let mut bus_info = BusInfo::default();
                let r = (vtable.get_bus_info)(
                    self.component,
                    K_AUDIO,
                    K_INPUT,
                    0,
                    &mut bus_info as *mut _ as *mut c_void,
                );
                if r == K_RESULT_OK {
                    let ch = bus_info.channel_count.max(0) as usize;
                    if ch != self.num_input_channels {
                        self.num_input_channels = ch;
                        self.info = self.info.clone().audio_io(ch, self.num_output_channels);
                        self.input_buffer_ptrs = vec![std::ptr::null_mut(); ch.max(2)];
                        self.input_buffer_ptrs_f64 = vec![std::ptr::null_mut(); ch.max(2)];
                    }
                }
            }

            let num_output_buses = (vtable.get_bus_count)(self.component, K_AUDIO, K_OUTPUT);
            if num_output_buses > 0 {
                let mut bus_info = BusInfo::default();
                let r = (vtable.get_bus_info)(
                    self.component,
                    K_AUDIO,
                    K_OUTPUT,
                    0,
                    &mut bus_info as *mut _ as *mut c_void,
                );
                if r == K_RESULT_OK {
                    let ch = bus_info.channel_count.max(1) as usize;
                    if ch != self.num_output_channels {
                        self.num_output_channels = ch;
                        self.info = self.info.clone().audio_io(self.num_input_channels, ch);
                        self.output_buffer_ptrs = vec![std::ptr::null_mut(); ch.max(2)];
                        self.output_buffer_ptrs_f64 = vec![std::ptr::null_mut(); ch.max(2)];
                    }
                }
            }
        }

        if let (Some(ctrl), Some(vtable)) = (self.controller, self.controller_vtable) {
            if self.separate_controller {
                let _ = unsafe { ((*vtable).initialize)(ctrl, host_ptr) };

                self.connect_component_and_controller();
            }

            let handler_ptr = self.component_handler.as_ptr();
            let _ = unsafe { ((*vtable).set_component_handler)(ctrl, handler_ptr) };
        }

        let symbolic_sample_size = if self.use_f64 {
            K_SAMPLE_64
        } else {
            K_SAMPLE_32
        };
        let setup = ProcessSetup {
            process_mode: K_REALTIME,
            symbolic_sample_size,
            max_samples_per_block: self.block_size as i32,
            sample_rate: self.sample_rate,
        };

        let result = unsafe { ((*self.processor_vtable).setup_processing)(self.processor, &setup) };

        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Setup,
                code: result,
            });
        }

        self.activate_buses()?;

        // VST3 spec requires: setActive(true) before setProcessing(true)
        let result = unsafe { ((*self.component_vtable).set_active)(self.component, 1) };
        if result != K_RESULT_OK && result != K_RESULT_TRUE {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Activation,
                code: result,
            });
        }

        let result = unsafe { ((*self.processor_vtable).set_processing)(self.processor, 1) };
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
        let num_input_buses =
            unsafe { ((*self.component_vtable).get_bus_count)(self.component, K_AUDIO, K_INPUT) };

        for i in 0..num_input_buses {
            unsafe {
                ((*self.component_vtable).activate_bus)(self.component, K_AUDIO, K_INPUT, i, 1);
            }
        }

        let num_output_buses =
            unsafe { ((*self.component_vtable).get_bus_count)(self.component, K_AUDIO, K_OUTPUT) };

        for i in 0..num_output_buses {
            unsafe {
                ((*self.component_vtable).activate_bus)(self.component, K_AUDIO, K_OUTPUT, i, 1);
            }
        }

        Ok(())
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        self.close_editor();

        if self.is_active {
            unsafe {
                ((*self.processor_vtable).set_processing)(self.processor, 0);
                ((*self.component_vtable).set_active)(self.component, 0);
            }
        }

        unsafe {
            ((*self.component_vtable).terminate)(self.component);
        }

        if let (Some(ctrl), Some(vtable)) = (self.controller, self.controller_vtable) {
            unsafe {
                ((*vtable).terminate)(ctrl);
            }
        }

        unsafe {
            let vtable = *(self.component as *const *const IUnknownVtable);
            ((*vtable).release)(self.component);

            let vtable = *(self.processor as *const *const IUnknownVtable);
            ((*vtable).release)(self.processor);

            if let Some(ctrl) = self.controller {
                let vtable = *(ctrl as *const *const IUnknownVtable);
                ((*vtable).release)(ctrl);
            }
        }
    }
}
