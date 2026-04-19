//! High-level types over the vst3 crate's raw FFI layer.

use std::ffi::c_void;

use smallvec::SmallVec;

mod audio;
mod events;
mod info;
mod params;
mod transport;

pub use audio::{AudioBuffer, BufferPtrs, Sample};
pub(crate) use audio::{K_SAMPLE_32_INT, K_SAMPLE_64_INT};
pub use events::{
    vst3_event_from_midi, vst3_to_midi_event, vst3_to_note_expression, DataEvent, EventHeader,
    MidiEvent, NoteExpressionType, NoteExpressionValue, NoteExpressionValueEvent, NoteOffEvent,
    NoteOnEvent, PolyPressureEvent, Vst3Event, K_DATA_EVENT, K_NOTE_EXPRESSION_VALUE_EVENT,
    K_NOTE_OFF_EVENT, K_NOTE_ON_EVENT, K_POLY_PRESSURE_EVENT,
};
pub(crate) use events::{from_c_event, to_c_event};
pub use info::{parameter_flags, BusInfo, Vst3ParameterInfo};
pub use params::{ParameterChanges, ParameterPoint, ParameterQueue};
pub use transport::TransportState;

pub struct ProcessOutput {
    pub midi_events: SmallVec<[MidiEvent; 64]>,
    pub parameter_changes: ParameterChanges,
}

/// Pixel dimensions of a plugin editor window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorSize {
    pub width: u32,
    pub height: u32,
}

/// A platform-specific parent window handle for embedding plugin editors.
///
/// Construct via [`WindowHandle::from_raw`], then pass to
/// [`Vst3Instance::open_editor`](crate::Vst3Instance::open_editor).
pub struct WindowHandle(*mut c_void);

impl WindowHandle {
    /// Create a `WindowHandle` from a raw platform pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid window handle for the current platform:
    /// - **macOS:** `NSView*`
    /// - **Windows:** `HWND`
    /// - **Linux:** X11 window ID cast to pointer
    pub unsafe fn from_raw(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub has_midi_input: bool,
    pub supports_f64: bool,
}

impl PluginInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            vendor: String::new(),
            version: String::new(),
            num_inputs: 2,
            num_outputs: 2,
            has_midi_input: false,
            supports_f64: false,
        }
    }

    pub fn vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = vendor.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn audio_io(mut self, inputs: usize, outputs: usize) -> Self {
        self.num_inputs = inputs;
        self.num_outputs = outputs;
        self
    }

    pub fn midi(mut self, has_midi: bool) -> Self {
        self.has_midi_input = has_midi;
        self
    }

    pub fn f64_support(mut self, supports: bool) -> Self {
        self.supports_f64 = supports;
        self
    }
}
