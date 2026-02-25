//! High-level types over the raw FFI layer.

mod audio;
mod events;
mod params;
mod transport;

pub use audio::{AudioBuffer, BufferPtrs, Sample};
pub use events::{
    vst3_to_midi_event, vst3_to_note_expression, MidiData, MidiEvent, NoteExpressionType,
    NoteExpressionValue, Vst3MidiEvent,
};
pub use params::{ParameterChanges, ParameterPoint, ParameterQueue};
pub use transport::TransportState;

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
