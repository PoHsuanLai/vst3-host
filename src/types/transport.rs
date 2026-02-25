//! Transport state for plugin processing.

use crate::ffi::{
    ProcessContext, K_BAR_POSITION_VALID, K_CYCLE_ACTIVE, K_CYCLE_VALID, K_PLAYING,
    K_PROJECT_TIME_MUSIC_VALID, K_RECORDING, K_TEMPO_VALID, K_TIME_SIG_VALID,
};

#[derive(Debug, Clone)]
pub struct TransportState {
    pub playing: bool,
    pub recording: bool,
    pub cycle_active: bool,
    /// BPM
    pub tempo: f64,
    pub time_sig_numerator: i32,
    pub time_sig_denominator: i32,
    pub position_samples: i64,
    /// Quarter notes
    pub position_beats: f64,
    /// Quarter notes
    pub bar_position_beats: f64,
    /// Quarter notes
    pub cycle_start_beats: f64,
    /// Quarter notes
    pub cycle_end_beats: f64,
    /// Hz. Set automatically during processing.
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn recording(mut self, recording: bool) -> Self {
        self.recording = recording;
        self
    }

    pub fn cycle_active(mut self, active: bool) -> Self {
        self.cycle_active = active;
        self
    }

    pub fn tempo(mut self, bpm: f64) -> Self {
        self.tempo = bpm;
        self
    }

    pub fn time_signature(mut self, numerator: i32, denominator: i32) -> Self {
        self.time_sig_numerator = numerator;
        self.time_sig_denominator = denominator;
        self
    }

    pub fn position_samples(mut self, samples: i64) -> Self {
        self.position_samples = samples;
        self
    }

    pub fn position_beats(mut self, beats: f64) -> Self {
        self.position_beats = beats;
        self
    }

    pub fn bar_position_beats(mut self, beats: f64) -> Self {
        self.bar_position_beats = beats;
        self
    }

    pub fn cycle_range(mut self, start: f64, end: f64) -> Self {
        self.cycle_start_beats = start;
        self.cycle_end_beats = end;
        self
    }

    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate;
        self
    }

    pub fn to_process_context(&self) -> ProcessContext {
        let mut state = 0u32;

        if self.playing {
            state |= K_PLAYING;
        }
        if self.recording {
            state |= K_RECORDING;
        }
        if self.cycle_active {
            state |= K_CYCLE_ACTIVE | K_CYCLE_VALID;
        }

        state |=
            K_PROJECT_TIME_MUSIC_VALID | K_BAR_POSITION_VALID | K_TEMPO_VALID | K_TIME_SIG_VALID;

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
