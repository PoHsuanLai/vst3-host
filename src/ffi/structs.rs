//! VST3 C-compatible struct definitions.
//!
//! These structs have `#[repr(C)]` layout to match the VST3 SDK.

use std::ffi::c_void;


/// Factory information returned by IPluginFactory::getFactoryInfo.
#[repr(C)]
#[derive(Clone)]
pub struct PFactoryInfo {
    pub vendor: [i8; 64],
    pub url: [i8; 256],
    pub email: [i8; 128],
    pub flags: i32,
}

impl Default for PFactoryInfo {
    fn default() -> Self {
        Self {
            vendor: [0; 64],
            url: [0; 256],
            email: [0; 128],
            flags: 0,
        }
    }
}

/// Class information returned by IPluginFactory::getClassInfo.
#[repr(C)]
#[derive(Clone)]
pub struct PClassInfo {
    pub cid: [u8; 16],
    pub cardinality: i32,
    pub category: [i8; 32],
    pub name: [i8; 64],
}

impl Default for PClassInfo {
    fn default() -> Self {
        Self {
            cid: [0; 16],
            cardinality: 0,
            category: [0; 32],
            name: [0; 64],
        }
    }
}


/// Process setup information passed to IAudioProcessor::setupProcessing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessSetup {
    pub process_mode: i32,
    pub symbolic_sample_size: i32,
    pub max_samples_per_block: i32,
    pub sample_rate: f64,
}

/// Audio bus buffer descriptor.
#[repr(C)]
pub struct AudioBusBuffers {
    pub num_channels: i32,
    pub silence_flags: u64,
    pub buffers: *mut *mut c_void,
}

/// Bus information returned by IComponent::getBusInfo.
#[repr(C)]
#[derive(Clone)]
pub struct BusInfo {
    /// Media type (K_AUDIO or K_EVENT).
    pub media_type: i32,
    /// Direction (K_INPUT or K_OUTPUT).
    pub direction: i32,
    /// Number of channels (for audio buses).
    pub channel_count: i32,
    /// Bus name (UTF-16, null-terminated).
    pub name: [u16; 128],
    /// Bus type (main or aux).
    pub bus_type: i32,
    /// Flags.
    pub flags: u32,
}

impl Default for BusInfo {
    fn default() -> Self {
        Self {
            media_type: 0,
            direction: 0,
            channel_count: 0,
            name: [0; 128],
            bus_type: 0,
            flags: 0,
        }
    }
}

impl BusInfo {
    /// Get the bus name as a Rust String.
    pub fn name_string(&self) -> String {
        utf16_to_string(&self.name)
    }
}

/// Main processing data passed to IAudioProcessor::process.
#[repr(C)]
pub struct ProcessData {
    pub process_mode: i32,
    pub symbolic_sample_size: i32,
    pub num_samples: i32,
    pub num_inputs: i32,
    pub num_outputs: i32,
    pub inputs: *mut AudioBusBuffers,
    pub outputs: *mut AudioBusBuffers,
    pub input_param_changes: *mut c_void,
    pub output_param_changes: *mut c_void,
    pub input_events: *mut c_void,
    pub output_events: *mut c_void,
    pub context: *mut ProcessContext,
}

/// Transport and timing context for processing.
#[repr(C)]
pub struct ProcessContext {
    /// Transport and validity state flags.
    pub state: u32,
    /// Current sample rate.
    pub sample_rate: f64,
    /// Project time in samples (always valid).
    pub project_time_samples: i64,
    /// System time in nanoseconds (optional).
    pub system_time: i64,
    /// Continuous time in samples without loop (optional).
    pub continuous_time_samples: i64,
    /// Musical position in quarter notes (optional).
    pub project_time_music: f64,
    /// Last bar start position in quarter notes (optional).
    pub bar_position_music: f64,
    /// Cycle/loop start in quarter notes (optional).
    pub cycle_start_music: f64,
    /// Cycle/loop end in quarter notes (optional).
    pub cycle_end_music: f64,
    /// Tempo in BPM (optional).
    pub tempo: f64,
    /// Time signature numerator (optional).
    pub time_sig_numerator: i32,
    /// Time signature denominator (optional).
    pub time_sig_denominator: i32,
    /// Musical chord info (optional, simplified).
    pub chord: [u8; 12],
    /// SMPTE frame offset (optional).
    pub smpte_offset_subframes: i32,
    /// Video frame rate (optional).
    pub frame_rate: i32,
    /// Samples to next MIDI clock (24 ppq).
    pub samples_to_next_clock: i32,
}

impl Default for ProcessContext {
    fn default() -> Self {
        Self {
            state: 0,
            sample_rate: 44100.0,
            project_time_samples: 0,
            system_time: 0,
            continuous_time_samples: 0,
            project_time_music: 0.0,
            bar_position_music: 0.0,
            cycle_start_music: 0.0,
            cycle_end_music: 0.0,
            tempo: 120.0,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
            chord: [0; 12],
            smpte_offset_subframes: 0,
            frame_rate: 0,
            samples_to_next_clock: 0,
        }
    }
}

// View/Editor Structures

/// View rectangle for plugin GUI dimensions.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ViewRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ViewRect {
    /// Get the width of the rectangle.
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    /// Get the height of the rectangle.
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}


/// Common header for all VST3 events.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHeader {
    pub bus_index: i32,
    pub sample_offset: i32,
    pub ppq_position: f64,
    pub flags: u16,
    pub event_type: u16,
}

/// Note on event data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NoteOnEvent {
    pub header: EventHeader,
    pub channel: i16,
    pub pitch: i16,
    pub tuning: f32,
    pub velocity: f32,
    pub length: i32,
    pub note_id: i32,
}

/// Note off event data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NoteOffEvent {
    pub header: EventHeader,
    pub channel: i16,
    pub pitch: i16,
    pub velocity: f32,
    pub note_id: i32,
    pub tuning: f32,
}

/// Data event (for MIDI CC, pitch bend, etc.).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataEvent {
    pub header: EventHeader,
    pub size: u32,
    pub event_type: u32,
    pub bytes: [u8; 16],
}

/// Polyphonic pressure (aftertouch) event.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolyPressureEvent {
    pub header: EventHeader,
    pub channel: i16,
    pub pitch: i16,
    pub pressure: f32,
    pub note_id: i32,
}

/// Note expression value event (per-note modulation).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NoteExpressionValueEvent {
    pub header: EventHeader,
    /// Note ID (unique identifier for the note).
    pub note_id: i32,
    /// Expression type ID (0=volume, 1=pan, 2=tuning, 3=vibrato, 4=brightness).
    pub type_id: u32,
    /// Normalized value (0.0 to 1.0, meaning depends on type_id).
    pub value: f64,
}

/// Union-like enum for all VST3 event types.
#[derive(Clone, Copy)]
pub enum Vst3Event {
    NoteOn(NoteOnEvent),
    NoteOff(NoteOffEvent),
    Data(DataEvent),
    PolyPressure(PolyPressureEvent),
    NoteExpression(NoteExpressionValueEvent),
}

impl Vst3Event {
    /// Get the sample offset of this event.
    pub fn sample_offset(&self) -> i32 {
        match self {
            Vst3Event::NoteOn(e) => e.header.sample_offset,
            Vst3Event::NoteOff(e) => e.header.sample_offset,
            Vst3Event::Data(e) => e.header.sample_offset,
            Vst3Event::PolyPressure(e) => e.header.sample_offset,
            Vst3Event::NoteExpression(e) => e.header.sample_offset,
        }
    }
}


/// VST3 parameter info flags.
pub mod parameter_flags {
    /// Parameter can be automated.
    pub const CAN_AUTOMATE: i32 = 1 << 0;
    /// Parameter is read-only.
    pub const IS_READ_ONLY: i32 = 1 << 1;
    /// Parameter wraps around.
    pub const IS_WRAP: i32 = 1 << 2;
    /// Parameter is a list (discrete values).
    pub const IS_LIST: i32 = 1 << 3;
    /// Parameter is hidden.
    pub const IS_HIDDEN: i32 = 1 << 4;
    /// This is a program change parameter.
    pub const IS_PROGRAM_CHANGE: i32 = 1 << 15;
    /// Parameter is a bypass switch.
    pub const IS_BYPASS: i32 = 1 << 16;
}

/// Parameter information returned by IEditController::getParameterInfo.
#[repr(C)]
#[derive(Clone)]
pub struct Vst3ParameterInfo {
    /// Unique parameter ID.
    pub id: u32,
    /// Parameter title (UTF-16, null-terminated).
    pub title: [u16; 128],
    /// Parameter short title (UTF-16, null-terminated).
    pub short_title: [u16; 128],
    /// Parameter unit (UTF-16, null-terminated).
    pub units: [u16; 128],
    /// Number of discrete steps (0 = continuous).
    pub step_count: i32,
    /// Default normalized value (0.0-1.0).
    pub default_normalized_value: f64,
    /// Unit ID for grouping.
    pub unit_id: i32,
    /// Parameter flags (see parameter_flags module).
    pub flags: i32,
}

impl Default for Vst3ParameterInfo {
    fn default() -> Self {
        Self {
            id: 0,
            title: [0; 128],
            short_title: [0; 128],
            units: [0; 128],
            step_count: 0,
            default_normalized_value: 0.0,
            unit_id: 0,
            flags: 0,
        }
    }
}

impl Vst3ParameterInfo {
    /// Get the parameter title as a Rust String.
    pub fn title_string(&self) -> String {
        utf16_to_string(&self.title)
    }

    /// Get the parameter short title as a Rust String.
    pub fn short_title_string(&self) -> String {
        utf16_to_string(&self.short_title)
    }

    /// Get the parameter unit as a Rust String.
    pub fn units_string(&self) -> String {
        utf16_to_string(&self.units)
    }

    /// Check if parameter can be automated.
    pub fn can_automate(&self) -> bool {
        (self.flags & parameter_flags::CAN_AUTOMATE) != 0
    }

    /// Check if parameter is read-only.
    pub fn is_read_only(&self) -> bool {
        (self.flags & parameter_flags::IS_READ_ONLY) != 0
    }

    /// Check if parameter is hidden.
    pub fn is_hidden(&self) -> bool {
        (self.flags & parameter_flags::IS_HIDDEN) != 0
    }

    /// Check if parameter is bypass.
    pub fn is_bypass(&self) -> bool {
        (self.flags & parameter_flags::IS_BYPASS) != 0
    }

    /// Check if parameter wraps around.
    pub fn is_wrap(&self) -> bool {
        (self.flags & parameter_flags::IS_WRAP) != 0
    }
}


/// Convert a null-terminated UTF-16 array to a Rust String.
pub fn utf16_to_string(bytes: &[u16]) -> String {
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    String::from_utf16_lossy(&bytes[..end])
}

/// Convert a null-terminated i8 array to a Rust String.
pub fn c_str_to_string(bytes: &[i8]) -> String {
    let bytes: Vec<u8> = bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as u8)
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}
