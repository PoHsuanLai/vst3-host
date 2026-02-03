//! Audio buffer types for plugin processing.

use crate::ffi::{K_SAMPLE_32, K_SAMPLE_64};

/// Trait for supported audio sample types.
///
/// Implemented for `f32` and `f64` to support both 32-bit and 64-bit processing.
pub trait Sample: Copy + Default + Send + 'static {
    /// The VST3 symbolic sample size constant for this type.
    const VST3_SYMBOLIC_SIZE: i32;
}

impl Sample for f32 {
    const VST3_SYMBOLIC_SIZE: i32 = K_SAMPLE_32;
}

impl Sample for f64 {
    const VST3_SYMBOLIC_SIZE: i32 = K_SAMPLE_64;
}

/// Audio buffer for plugin processing.
///
/// This struct provides a view into input and output audio buffers.
/// The lifetime `'a` ensures the buffers remain valid during processing.
///
/// # Example
///
/// ```ignore
/// let inputs: [&[f32]; 2] = [&input_left, &input_right];
/// let mut outputs: [&mut [f32]; 2] = [&mut output_left, &mut output_right];
///
/// let buffer = AudioBuffer {
///     inputs: &inputs,
///     outputs: &mut outputs,
///     num_samples: 512,
///     sample_rate: 44100.0,
/// };
/// ```
pub struct AudioBuffer<'a, T: Sample = f32> {
    /// Input channel buffers (read-only).
    pub inputs: &'a [&'a [T]],
    /// Output channel buffers (writable).
    pub outputs: &'a mut [&'a mut [T]],
    /// Number of samples in each buffer.
    pub num_samples: usize,
    /// Sample rate in Hz.
    pub sample_rate: f64,
}

impl<'a, T: Sample> AudioBuffer<'a, T> {
    /// Create a new audio buffer.
    pub fn new(
        inputs: &'a [&'a [T]],
        outputs: &'a mut [&'a mut [T]],
        num_samples: usize,
        sample_rate: f64,
    ) -> Self {
        Self {
            inputs,
            outputs,
            num_samples,
            sample_rate,
        }
    }

    /// Get the number of input channels.
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Get the number of output channels.
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    /// Clear all output buffers to zero.
    pub fn clear_outputs(&mut self) {
        for output in self.outputs.iter_mut() {
            output.fill(T::default());
        }
    }
}
