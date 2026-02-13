//! VST3 constants and flags.

use std::ffi::c_void;

/// Function signature for VST3 module entry point.
pub type GetPluginFactoryFn = unsafe extern "system" fn() -> *mut c_void;

/// Operation succeeded.
pub const K_RESULT_OK: i32 = 0;
/// Operation succeeded (alias for K_RESULT_OK).
pub const K_RESULT_TRUE: i32 = 0;
/// Operation failed (false result).
pub const K_RESULT_FALSE: i32 = 1;
/// Invalid argument.
pub const K_INVALID_ARGUMENT: i32 = 2;
/// Not implemented.
pub const K_NOT_IMPLEMENTED: i32 = 3;
/// Internal error.
pub const K_INTERNAL_ERROR: i32 = 4;
/// Not initialized.
pub const K_NOT_INITIALIZED: i32 = 5;
/// Already initialized.
pub const K_OUT_OF_MEMORY: i32 = 6;

/// 32-bit float samples.
pub const K_SAMPLE_32: i32 = 0;
/// 64-bit double samples.
pub const K_SAMPLE_64: i32 = 1;

/// Real-time processing mode.
pub const K_REALTIME: i32 = 0;
/// Prefetch processing mode.
pub const K_PREFETCH: i32 = 1;
/// Offline processing mode.
pub const K_OFFLINE: i32 = 2;

/// Audio media type.
pub const K_AUDIO: i32 = 0;
/// Event (MIDI) media type.
pub const K_EVENT: i32 = 1;

/// Input bus direction.
pub const K_INPUT: i32 = 0;
/// Output bus direction.
pub const K_OUTPUT: i32 = 1;

/// Note on event.
pub const K_NOTE_ON_EVENT: u16 = 0;
/// Note off event.
pub const K_NOTE_OFF_EVENT: u16 = 1;
/// Data event (MIDI CC, etc.).
pub const K_DATA_EVENT: u16 = 2;
/// Polyphonic pressure (aftertouch) event.
pub const K_POLY_PRESSURE_EVENT: u16 = 3;
/// Note expression value event.
pub const K_NOTE_EXPRESSION_VALUE_EVENT: u16 = 4;
/// Note expression text event.
pub const K_NOTE_EXPRESSION_TEXT_EVENT: u16 = 5;
/// Chord event.
pub const K_CHORD_EVENT: u16 = 6;
/// Scale event.
pub const K_SCALE_EVENT: u16 = 7;
/// Legacy MIDI CC output event.
pub const K_LEGACY_MIDI_CC_OUT_EVENT: u16 = 65535;

/// Transport is playing.
pub const K_PLAYING: u32 = 1 << 1;
/// Cycle/loop is active.
pub const K_CYCLE_ACTIVE: u32 = 1 << 2;
/// Recording is active.
pub const K_RECORDING: u32 = 1 << 3;
/// System time is valid.
pub const K_SYSTEM_TIME_VALID: u32 = 1 << 8;
/// Project time in musical position is valid.
pub const K_PROJECT_TIME_MUSIC_VALID: u32 = 1 << 9;
/// Tempo is valid.
pub const K_TEMPO_VALID: u32 = 1 << 10;
/// Bar position is valid.
pub const K_BAR_POSITION_VALID: u32 = 1 << 11;
/// Cycle start/end positions are valid.
pub const K_CYCLE_VALID: u32 = 1 << 12;
/// Time signature is valid.
pub const K_TIME_SIG_VALID: u32 = 1 << 13;
/// SMPTE offset is valid.
pub const K_SMPTE_VALID: u32 = 1 << 14;
/// Clock position is valid.
pub const K_CLOCK_VALID: u32 = 1 << 15;
/// Continuous time samples is valid.
pub const K_CONT_TIME_VALID: u32 = 1 << 17;
/// Chord information is valid.
pub const K_CHORD_VALID: u32 = 1 << 18;

/// Volume expression (0.0 = -oo dB, 0.5 = 0dB, 1.0 = +6dB).
pub const K_VOLUME_TYPE_ID: u32 = 0;
/// Pan expression (0.0 = left, 0.5 = center, 1.0 = right).
pub const K_PAN_TYPE_ID: u32 = 1;
/// Tuning expression in semitones (-120.0 to +120.0 mapped to 0.0-1.0).
pub const K_TUNING_TYPE_ID: u32 = 2;
/// Vibrato expression (0.0 = none, 1.0 = max).
pub const K_VIBRATO_TYPE_ID: u32 = 3;
/// Brightness/filter cutoff expression.
pub const K_BRIGHTNESS_TYPE_ID: u32 = 4;

// RestartFlags (for IComponentHandler::restartComponent)

/// Reload component (all settings).
pub const K_RELOAD_COMPONENT: i32 = 1 << 0;
/// IO configuration has changed (bus count, etc.).
pub const K_IO_CHANGED: i32 = 1 << 1;
/// Parameter definitions have changed.
pub const K_PARAM_VALUES_CHANGED: i32 = 1 << 2;
/// Latency has changed.
pub const K_LATENCY_CHANGED: i32 = 1 << 3;
/// Parameter titles/units have changed.
pub const K_PARAM_TITLES_CHANGED: i32 = 1 << 4;
/// MIDI CC assignments have changed.
pub const K_MIDI_CC_ASSIGNMENT_CHANGED: i32 = 1 << 5;
/// Note expression has changed.
pub const K_NOTE_EXPRESSION_CHANGED: i32 = 1 << 6;
/// IO titles have changed.
pub const K_IO_TITLES_CHANGED: i32 = 1 << 7;
/// Prefetchable support has changed.
pub const K_PREFETCHABLE_SUPPORT_CHANGED: i32 = 1 << 8;
/// Routing info has changed.
pub const K_ROUTING_INFO_CHANGED: i32 = 1 << 9;
/// Keyswitches have changed.
pub const K_KEYSWITCHES_CHANGED: i32 = 1 << 10;

/// Seek from beginning.
pub const K_IB_SEEK_SET: i32 = 0;
/// Seek from current position.
pub const K_IB_SEEK_CUR: i32 = 1;
/// Seek from end.
pub const K_IB_SEEK_END: i32 = 2;

/// Async state restoration progress.
pub const K_PROGRESS_ASYNC_STATE_RESTORATION: u32 = 0;
/// UI background task progress.
pub const K_PROGRESS_UI_BACKGROUND_TASK: u32 = 1;

// Knob Modes (for IEditController2)

/// Circular knob mode.
pub const K_KNOB_CIRCULAR_MODE: i32 = 0;
/// Relative circular knob mode.
pub const K_KNOB_RELATIVE_CIRCULAR_MODE: i32 = 1;
/// Linear knob mode.
pub const K_KNOB_LINEAR_MODE: i32 = 2;
