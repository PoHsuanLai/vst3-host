//! VST3 constants and flags.

use std::ffi::c_void;

pub type GetPluginFactoryFn = unsafe extern "system" fn() -> *mut c_void;

pub const K_RESULT_OK: i32 = 0;
pub const K_RESULT_TRUE: i32 = 0;
pub const K_RESULT_FALSE: i32 = 1;
pub const K_INVALID_ARGUMENT: i32 = 2;
pub const K_NOT_IMPLEMENTED: i32 = 3;
pub const K_INTERNAL_ERROR: i32 = 4;
pub const K_NOT_INITIALIZED: i32 = 5;
pub const K_OUT_OF_MEMORY: i32 = 6;

pub const K_SAMPLE_32: i32 = 0;
pub const K_SAMPLE_64: i32 = 1;

pub const K_REALTIME: i32 = 0;
pub const K_PREFETCH: i32 = 1;
pub const K_OFFLINE: i32 = 2;

pub const K_AUDIO: i32 = 0;
pub const K_EVENT: i32 = 1;

pub const K_INPUT: i32 = 0;
pub const K_OUTPUT: i32 = 1;

pub const K_NOTE_ON_EVENT: u16 = 0;
pub const K_NOTE_OFF_EVENT: u16 = 1;
/// MIDI CC, pitch bend, etc.
pub const K_DATA_EVENT: u16 = 2;
/// Polyphonic aftertouch.
pub const K_POLY_PRESSURE_EVENT: u16 = 3;
pub const K_NOTE_EXPRESSION_VALUE_EVENT: u16 = 4;
pub const K_NOTE_EXPRESSION_TEXT_EVENT: u16 = 5;
pub const K_CHORD_EVENT: u16 = 6;
pub const K_SCALE_EVENT: u16 = 7;
pub const K_LEGACY_MIDI_CC_OUT_EVENT: u16 = 65535;

pub const K_PLAYING: u32 = 1 << 1;
pub const K_CYCLE_ACTIVE: u32 = 1 << 2;
pub const K_RECORDING: u32 = 1 << 3;
pub const K_SYSTEM_TIME_VALID: u32 = 1 << 8;
pub const K_PROJECT_TIME_MUSIC_VALID: u32 = 1 << 9;
pub const K_TEMPO_VALID: u32 = 1 << 10;
pub const K_BAR_POSITION_VALID: u32 = 1 << 11;
pub const K_CYCLE_VALID: u32 = 1 << 12;
pub const K_TIME_SIG_VALID: u32 = 1 << 13;
pub const K_SMPTE_VALID: u32 = 1 << 14;
pub const K_CLOCK_VALID: u32 = 1 << 15;
pub const K_CONT_TIME_VALID: u32 = 1 << 17;
pub const K_CHORD_VALID: u32 = 1 << 18;

/// 0.0 = -inf dB, 0.5 = 0dB, 1.0 = +6dB.
pub const K_VOLUME_TYPE_ID: u32 = 0;
/// 0.0 = left, 0.5 = center, 1.0 = right.
pub const K_PAN_TYPE_ID: u32 = 1;
/// Semitones: -120.0 to +120.0 mapped to 0.0-1.0.
pub const K_TUNING_TYPE_ID: u32 = 2;
/// 0.0 = none, 1.0 = max.
pub const K_VIBRATO_TYPE_ID: u32 = 3;
/// Filter cutoff: 0.0 = dark, 1.0 = bright.
pub const K_BRIGHTNESS_TYPE_ID: u32 = 4;

// RestartFlags (for IComponentHandler::restartComponent)

pub const K_RELOAD_COMPONENT: i32 = 1 << 0;
pub const K_IO_CHANGED: i32 = 1 << 1;
pub const K_PARAM_VALUES_CHANGED: i32 = 1 << 2;
pub const K_LATENCY_CHANGED: i32 = 1 << 3;
pub const K_PARAM_TITLES_CHANGED: i32 = 1 << 4;
pub const K_MIDI_CC_ASSIGNMENT_CHANGED: i32 = 1 << 5;
pub const K_NOTE_EXPRESSION_CHANGED: i32 = 1 << 6;
pub const K_IO_TITLES_CHANGED: i32 = 1 << 7;
pub const K_PREFETCHABLE_SUPPORT_CHANGED: i32 = 1 << 8;
pub const K_ROUTING_INFO_CHANGED: i32 = 1 << 9;
pub const K_KEYSWITCHES_CHANGED: i32 = 1 << 10;

pub const K_IB_SEEK_SET: i32 = 0;
pub const K_IB_SEEK_CUR: i32 = 1;
pub const K_IB_SEEK_END: i32 = 2;

pub const K_PROGRESS_ASYNC_STATE_RESTORATION: u32 = 0;
pub const K_PROGRESS_UI_BACKGROUND_TASK: u32 = 1;

// Knob Modes (for IEditController2)

pub const K_KNOB_CIRCULAR_MODE: i32 = 0;
pub const K_KNOB_RELATIVE_CIRCULAR_MODE: i32 = 1;
pub const K_KNOB_LINEAR_MODE: i32 = 2;
