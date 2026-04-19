//! Transport state for plugin processing.

use vst3::Steinberg::Vst::ProcessContext_::StatesAndFlags_;

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

    pub fn to_process_context(&self) -> vst3::Steinberg::Vst::ProcessContext {
        // `StatesAndFlags_::*` is `u32` on unix, `c_int` (i32) on Windows — the casts
        // below are no-ops on one target and sign-changes on the other. Cleanest is
        // to normalise to `u32` at the edge.
        #[allow(clippy::unnecessary_cast)]
        let state_bits = {
            let mut state = 0u32;
            if self.playing {
                state |= StatesAndFlags_::kPlaying as u32;
            }
            if self.recording {
                state |= StatesAndFlags_::kRecording as u32;
            }
            if self.cycle_active {
                state |= (StatesAndFlags_::kCycleActive as u32)
                    | (StatesAndFlags_::kCycleValid as u32);
            }
            state |= (StatesAndFlags_::kProjectTimeMusicValid as u32)
                | (StatesAndFlags_::kBarPositionValid as u32)
                | (StatesAndFlags_::kTempoValid as u32)
                | (StatesAndFlags_::kTimeSigValid as u32);
            state
        };
        let state = state_bits;

        let mut ctx: vst3::Steinberg::Vst::ProcessContext = unsafe { std::mem::zeroed() };
        ctx.state = state;
        ctx.sampleRate = self.sample_rate;
        ctx.projectTimeSamples = self.position_samples;
        ctx.systemTime = 0;
        ctx.continousTimeSamples = self.position_samples;
        ctx.projectTimeMusic = self.position_beats;
        ctx.barPositionMusic = self.bar_position_beats;
        ctx.cycleStartMusic = self.cycle_start_beats;
        ctx.cycleEndMusic = self.cycle_end_beats;
        ctx.tempo = self.tempo;
        ctx.timeSigNumerator = self.time_sig_numerator;
        ctx.timeSigDenominator = self.time_sig_denominator;
        ctx.smpteOffsetSubframes = 0;
        ctx.samplesToNextClock = 0;
        ctx
    }
}
