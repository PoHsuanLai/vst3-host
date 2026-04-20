//! Transport state for plugin processing.

use vst3::Steinberg::Vst::ProcessContext_::StatesAndFlags_;

/// Host transport snapshot passed to the plugin on every
/// [`process`](crate::Vst3Instance::process) call via `ProcessContext`.
///
/// Construct via [`TransportState::new`] + builder methods; the
/// [`TransportState::to_process_context`] method encodes the correct validity
/// flags VST3 plugins expect.
#[derive(Debug, Clone)]
pub struct TransportState {
    /// True if the transport is playing.
    pub playing: bool,
    /// True if the transport is recording.
    pub recording: bool,
    /// True if loop / cycle region is active.
    pub cycle_active: bool,
    /// BPM
    pub tempo: f64,
    /// Time-signature numerator (top number).
    pub time_sig_numerator: i32,
    /// Time-signature denominator (bottom number, always a power of 2).
    pub time_sig_denominator: i32,
    /// Project time in samples.
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
    /// Create a stopped transport at 120 BPM, 4/4, 44.1 kHz.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the playing flag.
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set the recording flag.
    pub fn recording(mut self, recording: bool) -> Self {
        self.recording = recording;
        self
    }

    /// Set the cycle-active flag.
    pub fn cycle_active(mut self, active: bool) -> Self {
        self.cycle_active = active;
        self
    }

    /// Set tempo in BPM.
    pub fn tempo(mut self, bpm: f64) -> Self {
        self.tempo = bpm;
        self
    }

    /// Set time signature (e.g. `.time_signature(7, 8)` for 7/8).
    pub fn time_signature(mut self, numerator: i32, denominator: i32) -> Self {
        self.time_sig_numerator = numerator;
        self.time_sig_denominator = denominator;
        self
    }

    /// Set position in samples.
    pub fn position_samples(mut self, samples: i64) -> Self {
        self.position_samples = samples;
        self
    }

    /// Set position in quarter notes from the project start.
    pub fn position_beats(mut self, beats: f64) -> Self {
        self.position_beats = beats;
        self
    }

    /// Set the position of the current bar in quarter notes.
    pub fn bar_position_beats(mut self, beats: f64) -> Self {
        self.bar_position_beats = beats;
        self
    }

    /// Set the cycle/loop start and end in quarter notes.
    pub fn cycle_range(mut self, start: f64, end: f64) -> Self {
        self.cycle_start_beats = start;
        self.cycle_end_beats = end;
        self
    }

    /// Set sample rate in Hz.
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Build the VST3 `ProcessContext` with appropriate state bits and
    /// validity flags set (`kProjectTimeMusicValid`, `kBarPositionValid`,
    /// `kTempoValid`, `kTimeSigValid`).
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
                state |=
                    (StatesAndFlags_::kCycleActive as u32) | (StatesAndFlags_::kCycleValid as u32);
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
