//! Post-`initialize()` VST3 state. Audio processing is **not** active here —
//! [`Vst3Loaded::activate`] transitions to [`Vst3Instance`] for that.
//!
//! `Vst3Loaded` is what you want for GUI-only hosting, offline parameter
//! inspection, and state save/restore. `process()` lives exclusively on
//! [`Vst3Instance`]; the type system enforces that you can't call it here.

use std::path::Path;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use vst3::com_scrape_types::Unknown;
use vst3::{ComPtr, ComWrapper};
use vst3::Steinberg::{
    kResultFalse, kResultOk, FUnknown, IBStream, IPluginBaseTrait, IPlugView, IPlugViewTrait,
    ViewRect,
    Vst::{
        BusDirections_::{kInput, kOutput},
        IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentTrait, IConnectionPoint,
        IConnectionPointTrait, IEditController, IEditControllerTrait,
        MediaTypes_::kAudio,
    },
};

#[cfg(target_os = "macos")]
use vst3::Steinberg::kPlatformTypeNSView;
#[cfg(target_os = "windows")]
use vst3::Steinberg::kPlatformTypeHWND;
#[cfg(target_os = "linux")]
use vst3::Steinberg::kPlatformTypeX11EmbedWindowID;

use crate::com::{
    BStream, ComponentHandler, HostApplication, ParameterEditEvent, ProgressEvent, UnitEvent,
};
use crate::error::{LoadStage, Result, Vst3Error};
use crate::types::{
    BusInfo as BusInfoWrap, EditorSize, PluginInfo, Vst3ParameterInfo, WindowHandle,
};

use super::instance::Vst3Instance;
use super::library::Vst3Library;

const DEFAULT_EDITOR_SIZE: (u32, u32) = (800, 600);

// Re-express the handful of int-constant references we use in plugin API calls
// as `i32` for `media_type`/`direction` arguments.
pub(super) const K_AUDIO: i32 = kAudio as i32;
pub(super) const K_INPUT: i32 = kInput as i32;
pub(super) const K_OUTPUT: i32 = kOutput as i32;

pub(super) fn cid_to_string(cid_bytes: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        cid_bytes[0], cid_bytes[1], cid_bytes[2], cid_bytes[3],
        cid_bytes[4], cid_bytes[5], cid_bytes[6], cid_bytes[7],
        cid_bytes[8], cid_bytes[9], cid_bytes[10], cid_bytes[11],
        cid_bytes[12], cid_bytes[13], cid_bytes[14], cid_bytes[15]
    )
}

pub(super) fn get_bus_channel_count(
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

pub(super) struct PluginInterfaces {
    pub component: ComPtr<IComponent>,
    /// Always present: [`Vst3Loaded::load_with_info`] errors out at load time
    /// if the component doesn't expose `IAudioProcessor`.
    pub processor: ComPtr<IAudioProcessor>,
    pub controller: Controller,
}

unsafe impl Send for PluginInterfaces {}
unsafe impl Sync for PluginInterfaces {}

/// The editor controller. Three states — encodes the invariant that
/// "separate controller" plugins always have a controller.
pub(super) enum Controller {
    /// Component and controller are the same COM object (common single-component
    /// plugins). No extra `initialize()`/connection wiring needed.
    Same(ComPtr<IEditController>),
    /// Controller is a distinct COM object created from a separate CID. We
    /// [`initialize`](IEditControllerTrait) it and wire the connection points.
    Separate(ComPtr<IEditController>),
    /// Plugin has no editor controller (no parameters, no UI).
    None,
}

impl Controller {
    pub fn as_ref(&self) -> Option<&ComPtr<IEditController>> {
        match self {
            Controller::Same(c) | Controller::Separate(c) => Some(c),
            Controller::None => None,
        }
    }
}

pub(super) struct HostContext {
    pub application: ComWrapper<HostApplication>,
    pub handler: ComWrapper<ComponentHandler>,
    pub param_event_rx: Receiver<ParameterEditEvent>,
    pub progress_event_rx: Receiver<ProgressEvent>,
    pub unit_event_rx: Receiver<UnitEvent>,
}

/// Editor window state. `Open` owns the attached view; `Drop`-like close is
/// via [`Vst3Loaded::close_editor`].
pub(super) enum EditorState {
    Closed,
    Open(ComPtr<IPlugView>),
}

unsafe impl Send for EditorState {}
unsafe impl Sync for EditorState {}

/// Plugin instance that has been `initialize()`'d and has usable parameter /
/// editor / state surfaces, but is **not** processing audio.
///
/// Transition to [`Vst3Instance`] via [`Vst3Loaded::activate`] to enable
/// `process()`. For GUI-only hosting (no audio ever), stay here — skip the
/// `setActive(1) + setProcessing(1)` cost entirely.
pub struct Vst3Loaded {
    /// Kept alive to keep the DSO loaded for the plugin's lifetime.
    pub(super) _library: Arc<Vst3Library>,
    pub(super) interfaces: PluginInterfaces,
    pub(super) host: HostContext,
    pub(super) editor: EditorState,
    pub(super) info: PluginInfo,
}

impl Vst3Loaded {
    /// Lightweight probe: load library, read factory and bus metadata, return
    /// without calling init() or setActive(). Safe for plugins with license dialogs.
    pub fn probe(path: &Path) -> Result<PluginInfo> {
        check_exists(path)?;
        let library = Vst3Library::load(path)?;
        ensure_has_classes(&library, path)?;

        let class = find_audio_class(&library, path)?;
        let component: ComPtr<IComponent> = library.create_instance(&class.cid)?;
        let processor = component.cast::<IAudioProcessor>();
        Ok(build_plugin_info_raw(&library, &component, processor.as_ref(), &class))
    }

    /// Load a VST3 plugin for GUI/editor use only — no audio processing will
    /// ever happen on the returned value. Stays in `Loaded` state, skipping
    /// the activation cost.
    pub fn load(path: &Path) -> Result<Self> {
        Self::load_with_info(path)
    }

    /// Shared constructor used by both [`Vst3Loaded::load`] and
    /// [`Vst3Instance::load`]. Returns `Loaded` state; the caller decides
    /// whether to [`activate`](Vst3Loaded::activate) it.
    pub(super) fn load_with_info(path: &Path) -> Result<Self> {
        check_exists(path)?;
        let library = Vst3Library::load(path)?;
        ensure_has_classes(&library, path)?;

        let class = find_audio_class(&library, path)?;
        let component: ComPtr<IComponent> = library.create_instance(&class.cid)?;
        let processor = component.cast::<IAudioProcessor>().ok_or_else(|| {
            Vst3Error::LoadFailed {
                path: path.to_path_buf(),
                stage: LoadStage::Instantiation,
                reason: "VST3 plugin does not support IAudioProcessor".to_string(),
            }
        })?;
        let controller = query_controller(&component, &library);
        let info = build_plugin_info(&library, &component, &processor, &class);

        let mut loaded = Self::assemble(library, component, processor, controller, info);
        loaded.initialize()?;
        Ok(loaded)
    }

    /// Build `Self` from already-queried interfaces. No side effects — the
    /// caller runs [`initialize`](Self::initialize).
    fn assemble(
        library: Arc<Vst3Library>,
        component: ComPtr<IComponent>,
        processor: ComPtr<IAudioProcessor>,
        controller: Controller,
        info: PluginInfo,
    ) -> Self {
        let host_application = HostApplication::new("vst3-host");
        let (component_handler, param_event_rx, progress_event_rx, unit_event_rx) =
            ComponentHandler::new();

        Self {
            _library: library,
            interfaces: PluginInterfaces {
                component,
                processor,
                controller,
            },
            host: HostContext {
                application: host_application,
                handler: component_handler,
                param_event_rx,
                progress_event_rx,
                unit_event_rx,
            },
            editor: EditorState::Closed,
            info,
        }
    }

    /// Transition to the processing state. Runs `setupProcessing`, activates
    /// buses, calls `setActive(1)` and `setProcessing(1)`. Returns a
    /// [`Vst3Instance`] that exposes `process()`.
    pub fn activate(self, sample_rate: f64, block_size: usize) -> Result<Vst3Instance> {
        Vst3Instance::from_loaded(self, sample_rate, block_size)
    }

    // ── read-only metadata ──

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    pub fn supports_f64(&self) -> bool {
        self.info.supports_f64
    }

    pub fn get_latency_samples(&self) -> u32 {
        unsafe { self.interfaces.processor.getLatencySamples() }
    }

    // ── parameters ──

    pub fn parameter_count(&self) -> u32 {
        match self.interfaces.controller.as_ref() {
            Some(c) => unsafe { c.getParameterCount() as u32 },
            None => 0,
        }
    }

    pub fn parameter(&self, index: u32) -> f64 {
        match self.interfaces.controller.as_ref() {
            Some(c) => unsafe { c.getParamNormalized(index) },
            None => 0.0,
        }
    }

    pub fn set_parameter(&mut self, index: u32, value: f64) -> &mut Self {
        if let Some(c) = self.interfaces.controller.as_ref() {
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

    // ── event receivers ──

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

    // ── state ──

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

        if let Some(ctrl) = self.interfaces.controller.as_ref() {
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

    // ── editor ──

    pub fn has_editor(&self) -> bool {
        self.interfaces.controller.as_ref().is_some()
    }

    pub fn open_editor(&mut self, parent: WindowHandle) -> Result<EditorSize> {
        let ctrl = self.interfaces.controller.as_ref().ok_or(
            Vst3Error::NotSupported("Plugin has no editor controller".to_string()),
        )?;

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

        let (width, height) = query_view_size(&view).unwrap_or(DEFAULT_EDITOR_SIZE);
        self.editor = EditorState::Open(view);

        Ok(EditorSize { width, height })
    }

    pub fn close_editor(&mut self) -> &mut Self {
        if let EditorState::Open(view) =
            std::mem::replace(&mut self.editor, EditorState::Closed)
        {
            unsafe { view.removed(); }
        }
        self
    }

    // ── internal ──

    /// Wire the component's connection point to the separate controller's.
    /// No-op unless both ends expose `IConnectionPoint`.
    fn connect_separate_controller(&self, ctrl: &ComPtr<IEditController>) {
        let Some(comp_conn) = self.interfaces.component.cast::<IConnectionPoint>() else {
            return;
        };
        let Some(ctrl_conn) = ctrl.cast::<IConnectionPoint>() else {
            return;
        };
        unsafe {
            comp_conn.connect(ctrl_conn.as_ptr());
            ctrl_conn.connect(comp_conn.as_ptr());
        }
    }

    fn initialize(&mut self) -> Result<()> {
        let host_ptr = self.host_context_ptr()?;

        let result = unsafe { self.interfaces.component.initialize(host_ptr) };
        if result != kResultOk && result != kResultFalse {
            unsafe { FUnknown::release(host_ptr) };
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: result,
            });
        }

        self.reconcile_bus_counts();

        if let Controller::Separate(ctrl) = &self.interfaces.controller {
            unsafe { let _ = ctrl.initialize(host_ptr); }
            let ctrl = ctrl.clone();
            self.connect_separate_controller(&ctrl);
        }

        self.attach_component_handler();
        Ok(())
    }

    /// `IHostApplication` upcast to `FUnknown`, with a +1 refcount that the
    /// plugin assumes ownership of via `IComponent::initialize`.
    fn host_context_ptr(&self) -> Result<*mut FUnknown> {
        Ok(self
            .host
            .application
            .to_com_ptr::<vst3::Steinberg::Vst::IHostApplication>()
            .ok_or(Vst3Error::PluginError {
                stage: LoadStage::Initialization,
                code: 0,
            })?
            .upcast::<FUnknown>()
            .into_raw())
    }

    /// Re-query bus counts from the component — `initialize` may have changed
    /// them (some plugins don't declare bus counts until after init).
    fn reconcile_bus_counts(&mut self) {
        if let Some(ch) = get_bus_channel_count(&self.interfaces.component, K_INPUT, 0) {
            if ch != self.info.num_inputs {
                self.info = self.info.clone().audio_io(ch, self.info.num_outputs);
            }
        }
        if let Some(ch) = get_bus_channel_count(&self.interfaces.component, K_OUTPUT, 1) {
            if ch != self.info.num_outputs {
                self.info = self.info.clone().audio_io(self.info.num_inputs, ch);
            }
        }
    }

    /// Hand the controller our `IComponentHandler` so it can report param
    /// edits, bus-activation requests, etc.
    fn attach_component_handler(&self) {
        let Some(ctrl) = self.interfaces.controller.as_ref() else {
            return;
        };
        let handler_ptr = self
            .host
            .handler
            .as_com_ref::<vst3::Steinberg::Vst::IComponentHandler>()
            .map(|r| r.as_ptr())
            .unwrap_or(std::ptr::null_mut());
        unsafe { let _ = ctrl.setComponentHandler(handler_ptr); }
    }
}

impl Drop for Vst3Loaded {
    fn drop(&mut self) {
        self.close_editor();
        unsafe {
            self.interfaces.component.terminate();
        }
        if let Some(ctrl) = self.interfaces.controller.as_ref() {
            unsafe {
                ctrl.terminate();
            }
        }
    }
}

// ── helpers ──

fn check_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Vst3Error::LoadFailed {
            path: path.to_path_buf(),
            stage: LoadStage::Scanning,
            reason: "Plugin file not found".to_string(),
        });
    }
    Ok(())
}

/// One factory class — the handful of fields we need to keep together when
/// walking the `IPluginFactory`. Returned by [`find_audio_class`].
pub(super) struct AudioClass {
    /// Steinberg-signed class id, used for `IPluginFactory::createInstance`.
    pub cid: [i8; 16],
    /// Unsigned byte form of the cid — used for human-readable IDs only.
    pub cid_bytes: [u8; 16],
    /// Display name from `PClassInfo::name`.
    pub name: String,
}

fn ensure_has_classes(library: &Vst3Library, path: &Path) -> Result<()> {
    if library.count_classes() == 0 {
        return Err(Vst3Error::LoadFailed {
            path: path.to_path_buf(),
            stage: LoadStage::Factory,
            reason: "VST3 factory contains no classes".to_string(),
        });
    }
    Ok(())
}

/// Read the plug-view's `getSize()` and translate it into our `(width, height)`
/// tuple. Returns `None` if the view refuses — callers fall back to a default.
fn query_view_size(view: &ComPtr<IPlugView>) -> Option<(u32, u32)> {
    let mut rect = ViewRect { left: 0, top: 0, right: 0, bottom: 0 };
    let result = unsafe { view.getSize(&mut rect) };
    if result == kResultOk {
        Some((
            (rect.right - rect.left) as u32,
            (rect.bottom - rect.top) as u32,
        ))
    } else {
        None
    }
}

/// Assemble `PluginInfo` from already-queried interfaces. Used by both
/// [`Vst3Loaded::probe`] (which may not own an `IAudioProcessor`) and
/// [`Vst3Loaded::load_with_info`] (which does).
fn build_plugin_info_raw(
    library: &Vst3Library,
    component: &ComPtr<IComponent>,
    processor: Option<&ComPtr<IAudioProcessor>>,
    class: &AudioClass,
) -> PluginInfo {
    let vendor = library
        .get_factory_info()
        .map(|info| info.vendor)
        .unwrap_or_default();
    let num_inputs = get_bus_channel_count(component, K_INPUT, 0).unwrap_or(0);
    let num_outputs = get_bus_channel_count(component, K_OUTPUT, 1).unwrap_or(2);
    let supports_f64 = processor
        .map(|p| unsafe {
            p.canProcessSampleSize(crate::types::K_SAMPLE_64_INT) == kResultOk
        })
        .unwrap_or(false);
    let receives_midi =
        unsafe { component.getBusCount(crate::host::instance::K_EVENT, K_INPUT) > 0 };

    PluginInfo::new(
        format!("vst3.{}", cid_to_string(&class.cid_bytes)),
        class.name.clone(),
    )
    .vendor(vendor)
    .version("1.0.0".to_string())
    .audio_io(num_inputs, num_outputs)
    .midi(receives_midi)
    .f64_support(supports_f64)
}

/// Convenience wrapper for the load path where we always have a processor.
fn build_plugin_info(
    library: &Vst3Library,
    component: &ComPtr<IComponent>,
    processor: &ComPtr<IAudioProcessor>,
    class: &AudioClass,
) -> PluginInfo {
    build_plugin_info_raw(library, component, Some(processor), class)
}

fn find_audio_class(library: &Vst3Library, path: &Path) -> Result<AudioClass> {
    let count = library.count_classes();
    (0..count)
        .find_map(|i| {
            let info = library.get_class_info(i).ok()?;
            if !info.category.contains("Audio") {
                return None;
            }
            Some(AudioClass {
                cid: info.cid,
                cid_bytes: info.cid_bytes,
                name: info.name,
            })
        })
        .ok_or_else(|| Vst3Error::LoadFailed {
            path: path.to_path_buf(),
            stage: LoadStage::Factory,
            reason: "No audio processor classes found in VST3".to_string(),
        })
}

fn query_controller(component: &ComPtr<IComponent>, library: &Vst3Library) -> Controller {
    if let Some(ctrl) = component.cast::<IEditController>() {
        return Controller::Same(ctrl);
    }
    let mut controller_cid = [0i8; 16];
    let result = unsafe { component.getControllerClassId(&mut controller_cid) };
    if result == kResultOk && controller_cid != [0i8; 16] {
        match library.create_instance::<IEditController>(&controller_cid) {
            Ok(ctrl) => Controller::Separate(ctrl),
            Err(_) => Controller::None,
        }
    } else {
        Controller::None
    }
}
