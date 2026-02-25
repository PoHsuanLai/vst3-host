//! VST3 C-compatible struct definitions.
//!
//! These structs have `#[repr(C)]` layout to match the VST3 SDK.

use std::ffi::c_void;

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessSetup {
    pub process_mode: i32,
    pub symbolic_sample_size: i32,
    pub max_samples_per_block: i32,
    pub sample_rate: f64,
}

#[repr(C)]
pub struct AudioBusBuffers {
    pub num_channels: i32,
    pub silence_flags: u64,
    pub buffers: *mut *mut c_void,
}

#[repr(C)]
#[derive(Clone)]
pub struct BusInfo {
    pub media_type: i32,
    pub direction: i32,
    pub channel_count: i32,
    pub name: [u16; 128],
    pub bus_type: i32,
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
    pub fn name_string(&self) -> String {
        utf16_to_string(&self.name)
    }
}

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

/// Validity of optional fields is indicated by K_*_VALID flags in `state`.
#[repr(C)]
pub struct ProcessContext {
    pub state: u32,
    pub sample_rate: f64,
    pub project_time_samples: i64,
    /// Nanoseconds.
    pub system_time: i64,
    pub continuous_time_samples: i64,
    /// Quarter notes.
    pub project_time_music: f64,
    /// Quarter notes.
    pub bar_position_music: f64,
    /// Quarter notes.
    pub cycle_start_music: f64,
    /// Quarter notes.
    pub cycle_end_music: f64,
    /// BPM.
    pub tempo: f64,
    pub time_sig_numerator: i32,
    pub time_sig_denominator: i32,
    pub chord: [u8; 12],
    pub smpte_offset_subframes: i32,
    pub frame_rate: i32,
    /// 24 ppq.
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

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ViewRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ViewRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHeader {
    pub bus_index: i32,
    pub sample_offset: i32,
    pub ppq_position: f64,
    pub flags: u16,
    pub event_type: u16,
}

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataEvent {
    pub header: EventHeader,
    pub size: u32,
    pub event_type: u32,
    pub bytes: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolyPressureEvent {
    pub header: EventHeader,
    pub channel: i16,
    pub pitch: i16,
    pub pressure: f32,
    pub note_id: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NoteExpressionValueEvent {
    pub header: EventHeader,
    pub note_id: i32,
    /// 0=volume, 1=pan, 2=tuning, 3=vibrato, 4=brightness.
    pub type_id: u32,
    /// 0.0 to 1.0, meaning depends on type_id.
    pub value: f64,
}

#[derive(Clone, Copy)]
pub enum Vst3Event {
    NoteOn(NoteOnEvent),
    NoteOff(NoteOffEvent),
    Data(DataEvent),
    PolyPressure(PolyPressureEvent),
    NoteExpression(NoteExpressionValueEvent),
}

impl Vst3Event {
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

pub mod parameter_flags {
    pub const CAN_AUTOMATE: i32 = 1 << 0;
    pub const IS_READ_ONLY: i32 = 1 << 1;
    pub const IS_WRAP: i32 = 1 << 2;
    /// Discrete values.
    pub const IS_LIST: i32 = 1 << 3;
    pub const IS_HIDDEN: i32 = 1 << 4;
    pub const IS_PROGRAM_CHANGE: i32 = 1 << 15;
    pub const IS_BYPASS: i32 = 1 << 16;
}

#[repr(C)]
#[derive(Clone)]
pub struct Vst3ParameterInfo {
    pub id: u32,
    pub title: [u16; 128],
    pub short_title: [u16; 128],
    pub units: [u16; 128],
    /// 0 = continuous.
    pub step_count: i32,
    /// 0.0-1.0.
    pub default_normalized_value: f64,
    pub unit_id: i32,
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
    pub fn title_string(&self) -> String {
        utf16_to_string(&self.title)
    }

    pub fn short_title_string(&self) -> String {
        utf16_to_string(&self.short_title)
    }

    pub fn units_string(&self) -> String {
        utf16_to_string(&self.units)
    }

    pub fn can_automate(&self) -> bool {
        (self.flags & parameter_flags::CAN_AUTOMATE) != 0
    }

    pub fn is_read_only(&self) -> bool {
        (self.flags & parameter_flags::IS_READ_ONLY) != 0
    }

    pub fn is_hidden(&self) -> bool {
        (self.flags & parameter_flags::IS_HIDDEN) != 0
    }

    pub fn is_bypass(&self) -> bool {
        (self.flags & parameter_flags::IS_BYPASS) != 0
    }

    pub fn is_wrap(&self) -> bool {
        (self.flags & parameter_flags::IS_WRAP) != 0
    }
}

pub fn utf16_to_string(bytes: &[u16]) -> String {
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    String::from_utf16_lossy(&bytes[..end])
}

pub fn c_str_to_string(bytes: &[i8]) -> String {
    let bytes: Vec<u8> = bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as u8)
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}
