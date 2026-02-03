//! Transport state for plugin processing.

use crate::ffi::{
    ProcessContext, K_BAR_POSITION_VALID, K_CYCLE_ACTIVE, K_CYCLE_VALID, K_PLAYING,
    K_PROJECT_TIME_MUSIC_VALID, K_RECORDING, K_TEMPO_VALID, K_TIME_SIG_VALID,
};

/// Transport state for plugin processing.
///
/// This struct provides a builder-style API for constructing transport
/// information to pass to plugins during processing.
///
/// # Example
///
/// ```
/// use vst3_host::TransportState;
///
/// let transport = TransportState::new()
///     .tempo(128.0)
///     .time_signature(4, 4)
///     .playing(true)
///     .position_samples(44100) // 1 second at 44.1kHz
///     .position_beats(2.0);    // 2 quarter notes in
/// ```
#[derive(Debug, Clone)]
pub struct TransportState {
    /// Whether playback is active.
    pub playing: bool,
    /// Whether recording is active.
    pub recording: bool,
    /// Whether loop/cycle is active.
    pub cycle_active: bool,
    /// Tempo in BPM.
    pub tempo: f64,
    /// Time signature numerator.
    pub time_sig_numerator: i32,
    /// Time signature denominator.
    pub time_sig_denominator: i32,
    /// Position in samples.
    pub position_samples: i64,
    /// Position in quarter notes (beats).
    pub position_beats: f64,
    /// Bar start position in quarter notes.
    pub bar_position_beats: f64,
    /// Cycle start position in quarter notes.
    pub cycle_start_beats: f64,
    /// Cycle end position in quarter notes.
    pub cycle_end_beats: f64,
    /// Sample rate (set automatically during processing).
    pub sample_rate: f64,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            playing: false,
            recording: false,
            cycle_active: false,
            tempo: 120.0,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
            position_samples: 0,
            position_beats: 0.0,
            bar_position_beats: 0.0,
            cycle_start_beats: 0.0,
            cycle_end_beats: 0.0,
            sample_rate: 44100.0,
        }
    }
}

impl TransportState {
    /// Create a new transport state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether playback is active.
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set whether recording is active.
    pub fn recording(mut self, recording: bool) -> Self {
        self.recording = recording;
        self
    }

    /// Set whether loop/cycle is active.
    pub fn cycle_active(mut self, active: bool) -> Self {
        self.cycle_active = active;
        self
    }

    /// Set the tempo in BPM.
    pub fn tempo(mut self, bpm: f64) -> Self {
        self.tempo = bpm;
        self
    }

    /// Set the time signature.
    pub fn time_signature(mut self, numerator: i32, denominator: i32) -> Self {
        self.time_sig_numerator = numerator;
        self.time_sig_denominator = denominator;
        self
    }

    /// Set the position in samples.
    pub fn position_samples(mut self, samples: i64) -> Self {
        self.position_samples = samples;
        self
    }

    /// Set the position in quarter notes (beats).
    pub fn position_beats(mut self, beats: f64) -> Self {
        self.position_beats = beats;
        self
    }

    /// Set the bar start position in quarter notes.
    pub fn bar_position_beats(mut self, beats: f64) -> Self {
        self.bar_position_beats = beats;
        self
    }

    /// Set the cycle/loop range in quarter notes.
    pub fn cycle_range(mut self, start: f64, end: f64) -> Self {
        self.cycle_start_beats = start;
        self.cycle_end_beats = end;
        self
    }

    /// Set the sample rate.
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Convert to a VST3 ProcessContext.
    pub fn to_process_context(&self) -> ProcessContext {
        let mut state = 0u32;

        // Set transport state flags
        if self.playing {
            state |= K_PLAYING;
        }
        if self.recording {
            state |= K_RECORDING;
        }
        if self.cycle_active {
            state |= K_CYCLE_ACTIVE | K_CYCLE_VALID;
        }

        // Set validity flags for fields we populate
        state |= K_PROJECT_TIME_MUSIC_VALID | K_BAR_POSITION_VALID | K_TEMPO_VALID | K_TIME_SIG_VALID;

        ProcessContext {
            state,
            sample_rate: self.sample_rate,
            project_time_samples: self.position_samples,
            system_time: 0,
            continuous_time_samples: self.position_samples,
            project_time_music: self.position_beats,
            bar_position_music: self.bar_position_beats,
            cycle_start_music: self.cycle_start_beats,
            cycle_end_music: self.cycle_end_beats,
            tempo: self.tempo,
            time_sig_numerator: self.time_sig_numerator,
            time_sig_denominator: self.time_sig_denominator,
            chord: [0; 12],
            smpte_offset_subframes: 0,
            frame_rate: 0,
            samples_to_next_clock: 0,
        }
    }
}
