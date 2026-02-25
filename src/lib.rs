//! Rust library for hosting VST3 audio plugins via their COM interfaces.

pub mod com;
pub mod error;
pub mod ffi;
pub mod host;
pub mod types;

pub use error::{LoadStage, Result, Vst3Error};
pub use ffi::{parameter_flags, BusInfo, Vst3ParameterInfo};
pub use host::{Vst3Instance, Vst3Library};
pub use types::{
    AudioBuffer, MidiData, MidiEvent, NoteExpressionType, NoteExpressionValue, ParameterChanges,
    ParameterPoint, ParameterQueue, PluginInfo, Sample, TransportState, Vst3MidiEvent,
};

pub use com::{BStream, ComponentHandler, EventList, ParameterChangesImpl, ParameterEditEvent};
