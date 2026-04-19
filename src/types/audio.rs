//! Audio buffer types for plugin processing.

use std::ffi::c_void;

use vst3::Steinberg::Vst::SymbolicSampleSizes_;

pub(crate) const K_SAMPLE_32_INT: i32 = SymbolicSampleSizes_::kSample32 as i32;
pub(crate) const K_SAMPLE_64_INT: i32 = SymbolicSampleSizes_::kSample64 as i32;

/// Marker trait for VST3-compatible sample types (f32, f64).
///
/// The `prepare_ffi_buffers` method lets generic code (`process<T>`) fill the
/// correct pre-allocated pointer arrays without runtime branching in the
/// monomorphised output — the compiler generates one version per concrete type.
pub trait Sample: Copy + Default + Send + 'static {
    const VST3_SYMBOLIC_SIZE: i32;

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

pub struct BufferPtrs<T> {
    pub input: Vec<*mut T>,
    pub output: Vec<*mut T>,
}

unsafe impl<T> Send for BufferPtrs<T> {}
unsafe impl<T> Sync for BufferPtrs<T> {}

impl<T> BufferPtrs<T> {
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            input: vec![std::ptr::null_mut(); num_inputs],
            output: vec![std::ptr::null_mut(); num_outputs],
        }
    }

    pub fn resize_inputs(&mut self, count: usize) {
        self.input = vec![std::ptr::null_mut(); count];
    }

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

pub struct AudioBuffer<'a, T: Sample = f32> {
    pub inputs: &'a [&'a [T]],
    pub outputs: &'a mut [&'a mut [T]],
    pub num_samples: usize,
    /// Hz
    pub sample_rate: f64,
}

impl<'a, T: Sample> AudioBuffer<'a, T> {
    /// `num_samples` is derived from the first output channel's length, or the
    /// first input channel's length if there are no outputs. Panics if both are empty.
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

    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    pub fn clear_outputs(&mut self) {
        for output in self.outputs.iter_mut() {
            output.fill(T::default());
        }
    }
}
