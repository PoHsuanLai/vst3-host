//! Rust library for hosting VST3 audio plugins via their COM interfaces.

#[allow(dead_code)]
pub(crate) mod com;
pub mod error;
#[allow(dead_code)]
pub(crate) mod ffi;
pub mod host;
pub mod types;

pub use error::{LoadStage, Result, Vst3Error};
pub use ffi::{parameter_flags, BusInfo, Vst3ParameterInfo};
pub use host::{Vst3Instance, Vst3Library};
pub use types::{
    AudioBuffer, EditorSize, MidiData, MidiEvent, NoteExpressionType, NoteExpressionValue,
    ParameterChanges, ParameterPoint, ParameterQueue, PluginInfo, ProcessOutput, Sample,
    TransportState, Vst3MidiEvent, WindowHandle,
};

// COM event types for consumers that poll host events from Vst3Instance.
pub use com::{ParameterEditEvent, ProgressEvent, UnitEvent};

/// FFI event types for custom [`Vst3MidiEvent`] implementations.
pub mod events {
    pub use crate::ffi::{
        DataEvent, EventHeader, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent,
        PolyPressureEvent, Vst3Event, K_DATA_EVENT, K_NOTE_EXPRESSION_VALUE_EVENT,
        K_NOTE_OFF_EVENT, K_NOTE_ON_EVENT, K_POLY_PRESSURE_EVENT,
    };
}
