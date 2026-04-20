//! Audio buffer types for plugin processing.

use std::ffi::c_void;

use vst3::Steinberg::Vst::SymbolicSampleSizes_;

pub(crate) const K_SAMPLE_32_INT: i32 = SymbolicSampleSizes_::kSample32 as i32;
pub(crate) const K_SAMPLE_64_INT: i32 = SymbolicSampleSizes_::kSample64 as i32;

/// Marker trait for VST3-compatible sample types (f32, f64).
///
/// [`Sample::prepare_ffi_buffers`] lets generic code such as
/// [`Vst3Instance::process`](crate::Vst3Instance::process) dispatch to the
/// correct pre-allocated pointer array without runtime branching — the compiler
/// generates one monomorphized version per concrete type.
pub trait Sample: Copy + Default + Send + 'static {
    /// The `kSample32` or `kSample64` constant the plugin expects in
    /// `ProcessData::symbolicSampleSize`.
    const VST3_SYMBOLIC_SIZE: i32;

    /// Fill the appropriate [`BufferPtrs`] from the caller's input/output
    /// slices and return the two `*mut *mut c_void` arrays that the VST3 C
    /// API requires.
    fn prepare_ffi_buffers(
        ptrs_f32: &mut BufferPtrs<f32>,
        ptrs_f64: &mut BufferPtrs<f64>,
        inputs: &[&[Self]],
        outputs: &mut [&mut [Self]],
    ) -> (*mut *mut c_void, *mut *mut c_void);
}

impl Sample for f32 {
    const VST3_SYMBOLIC_SIZE: i32 = K_SAMPLE_32_INT;

    fn prepare_ffi_buffers(
        ptrs_f32: &mut BufferPtrs<f32>,
        _ptrs_f64: &mut BufferPtrs<f64>,
        inputs: &[&[Self]],
        outputs: &mut [&mut [Self]],
    ) -> (*mut *mut c_void, *mut *mut c_void) {
        ptrs_f32.prepare(inputs, outputs)
    }
}

impl Sample for f64 {
    const VST3_SYMBOLIC_SIZE: i32 = K_SAMPLE_64_INT;

    fn prepare_ffi_buffers(
        _ptrs_f32: &mut BufferPtrs<f32>,
        ptrs_f64: &mut BufferPtrs<f64>,
        inputs: &[&[Self]],
        outputs: &mut [&mut [Self]],
    ) -> (*mut *mut c_void, *mut *mut c_void) {
        ptrs_f64.prepare(inputs, outputs)
    }
}

/// Pair of pre-allocated pointer arrays handed to the VST3 C API on each
/// `process()` call, one per bus direction.
///
/// Allocated once per [`Vst3Instance`](crate::Vst3Instance); reused for every
/// audio block so the realtime path is allocation-free.
pub struct BufferPtrs<T> {
    /// Raw channel pointers for the input bus.
    pub input: Vec<*mut T>,
    /// Raw channel pointers for the output bus.
    pub output: Vec<*mut T>,
}

unsafe impl<T> Send for BufferPtrs<T> {}
unsafe impl<T> Sync for BufferPtrs<T> {}

impl<T> BufferPtrs<T> {
    /// Allocate pointer arrays sized for `num_inputs` and `num_outputs`
    /// channels. Slots start out null and are filled in by [`Self::prepare`].
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            input: vec![std::ptr::null_mut(); num_inputs],
            output: vec![std::ptr::null_mut(); num_outputs],
        }
    }

    /// Reallocate the input pointer array to hold `count` channels.
    pub fn resize_inputs(&mut self, count: usize) {
        self.input = vec![std::ptr::null_mut(); count];
    }

    /// Reallocate the output pointer array to hold `count` channels.
    pub fn resize_outputs(&mut self, count: usize) {
        self.output = vec![std::ptr::null_mut(); count];
    }

    /// Fill pointer arrays from buffer slices, return raw `*mut *mut c_void` for FFI.
    ///
    /// Input slices are cast to `*mut T` to satisfy the VST3 C API which uses
    /// `*mut *mut c_void` for both inputs and outputs. Well-behaved plugins
    /// must not mutate input buffers.
    pub fn prepare(
        &mut self,
        inputs: &[&[T]],
        outputs: &mut [&mut [T]],
    ) -> (*mut *mut c_void, *mut *mut c_void) {
        for (i, input_slice) in inputs.iter().enumerate() {
            if i < self.input.len() {
                self.input[i] = input_slice.as_ptr() as *mut T;
            }
        }
        for (i, output_slice) in outputs.iter_mut().enumerate() {
            if i < self.output.len() {
                self.output[i] = output_slice.as_mut_ptr();
            }
        }
        (
            self.input.as_mut_ptr() as *mut *mut c_void,
            self.output.as_mut_ptr() as *mut *mut c_void,
        )
    }
}

/// A block of deinterleaved audio handed to
/// [`Vst3Instance::process`](crate::Vst3Instance::process).
///
/// `inputs` and `outputs` are borrowed channel-slice arrays — the host owns
/// the underlying buffers. `T` picks 32-bit or 64-bit processing.
pub struct AudioBuffer<'a, T: Sample = f32> {
    /// One slice per input channel; all slices must have the same length
    /// (`num_samples`).
    pub inputs: &'a [&'a [T]],
    /// One slice per output channel; all slices must have the same length
    /// (`num_samples`).
    pub outputs: &'a mut [&'a mut [T]],
    /// Frames per channel in this block.
    pub num_samples: usize,
    /// Sample rate in Hz.
    pub sample_rate: f64,
}

impl<'a, T: Sample> AudioBuffer<'a, T> {
    /// Construct a buffer. `num_samples` is derived from the first output
    /// channel's length, or the first input channel's length if there are no
    /// outputs.
    ///
    /// # Panics
    ///
    /// Panics if both `inputs` and `outputs` are empty.
    pub fn new(inputs: &'a [&'a [T]], outputs: &'a mut [&'a mut [T]], sample_rate: f64) -> Self {
        let num_samples = outputs
            .first()
            .map(|s| s.len())
            .or_else(|| inputs.first().map(|s| s.len()))
            .expect("AudioBuffer requires at least one input or output channel");
        Self {
            inputs,
            outputs,
            num_samples,
            sample_rate,
        }
    }

    /// Number of input channels.
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Number of output channels.
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    /// Zero every output channel.
    pub fn clear_outputs(&mut self) {
        for output in self.outputs.iter_mut() {
            output.fill(T::default());
        }
    }
}
