//! Type-safe Rust library for hosting VST3 audio plugins via their native COM
//! interfaces.
//!
//! The public surface is organised around a three-stage lifecycle encoded in the
//! type system: a [`Vst3Library`] holds the loaded DSO and factory, a
//! [`Vst3Loaded`] is a `initialize()`'d plugin suitable for GUI/parameter work,
//! and a [`Vst3Instance`] adds the activation state required for
//! [`Vst3Instance::process`]. Transitions between stages move ownership, so the
//! compiler rejects calls that would be invalid for the current state.
//!
//! See the crate's `README.md` for a worked example.

#[allow(dead_code)]
pub(crate) mod com;
pub mod error;
pub(crate) mod helpers;
pub mod host;
pub mod types;

pub use error::{LoadStage, Result, Vst3Error};
pub use host::{Vst3Instance, Vst3Library, Vst3Loaded};
pub use types::{
    parameter_flags, vst3_event_from_midi, vst3_to_midi_event, AudioBuffer, BusInfo, EditorSize,
    MidiEvent, NoteExpressionType, NoteExpressionValue, ParameterChanges, ParameterPoint,
    ParameterQueue, PluginInfo, ProcessOutput, Sample, TransportState, Vst3ParameterInfo,
    WindowHandle,
};

pub use com::{ParameterEditEvent, ProgressEvent, UnitEvent};

/// Tagged-enum wrappers over VST3's typed event structs, plus the event-type
/// discriminant constants.
///
/// Re-exported here so downstream crates that need to construct
/// `NoteOnEvent`/`NoteOffEvent`/etc. literally (rather than going through the
/// [`vst3_to_midi_event`] / [`vst3_event_from_midi`] helpers) can do so without
/// depending on the raw `vst3` crate.
pub mod events {
    pub use crate::types::{
        DataEvent, EventHeader, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent,
        PolyPressureEvent, Vst3Event, K_DATA_EVENT, K_NOTE_EXPRESSION_VALUE_EVENT,
        K_NOTE_OFF_EVENT, K_NOTE_ON_EVENT, K_POLY_PRESSURE_EVENT,
    };
}
