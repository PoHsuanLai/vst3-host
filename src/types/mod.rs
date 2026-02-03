//! Public types for vst3-host.
//!
//! This module contains the high-level types used by the vst3-host API.
//! These types abstract over the low-level FFI types in [`crate::ffi`].

mod audio;
mod events;
mod params;
mod transport;

pub use audio::{AudioBuffer, Sample};
pub use events::{
    vst3_to_midi_event, vst3_to_note_expression, MidiData, MidiEvent, NoteExpressionType,
    NoteExpressionValue, Vst3MidiEvent,
};
pub use params::{ParameterChanges, ParameterPoint, ParameterQueue};
pub use transport::TransportState;

/// Information about a loaded VST3 plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique plugin identifier (formatted class ID).
    pub id: String,
    /// Plugin name.
    pub name: String,
    /// Plugin vendor/author.
    pub vendor: String,
    /// Plugin version string.
    pub version: String,
    /// Number of audio input channels.
    pub num_inputs: usize,
    /// Number of audio output channels.
    pub num_outputs: usize,
    /// Whether the plugin accepts MIDI input.
    pub has_midi_input: bool,
    /// Whether the plugin supports 64-bit (f64) processing.
    pub supports_f64: bool,
}

impl PluginInfo {
    /// Create a new PluginInfo with the given ID and name.
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

    /// Set the vendor.
    pub fn vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = vendor.into();
        self
    }

    /// Set the version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the audio I/O configuration.
    pub fn audio_io(mut self, inputs: usize, outputs: usize) -> Self {
        self.num_inputs = inputs;
        self.num_outputs = outputs;
        self
    }

    /// Set whether the plugin has MIDI input.
    pub fn midi(mut self, has_midi: bool) -> Self {
        self.has_midi_input = has_midi;
        self
    }

    /// Set whether the plugin supports f64 processing.
    pub fn f64_support(mut self, supports: bool) -> Self {
        self.supports_f64 = supports;
        self
    }
}
