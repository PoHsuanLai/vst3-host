//! IParamValueQueue COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::ffi::{IParamValueQueueVtable, K_RESULT_OK};
use crate::types::{ParameterPoint, ParameterQueue};

/// IParamValueQueue COM implementation.
///
/// Provides parameter automation points for a single parameter.
#[repr(C)]
pub struct ParamValueQueueImpl {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IParamValueQueueVtable,
    ref_count: AtomicU32,
    param_id: u32,
    points: Vec<ParameterPoint>,
}

// Safety: ParamValueQueueImpl only contains thread-safe types
unsafe impl Send for ParamValueQueueImpl {}
unsafe impl Sync for ParamValueQueueImpl {}

impl ParamValueQueueImpl {
    /// Create from a ParameterQueue.
    pub fn from_queue(queue: &ParameterQueue) -> Box<Self> {
        Box::new(ParamValueQueueImpl {
            vtable: &PARAM_VALUE_QUEUE_VTABLE,
            ref_count: AtomicU32::new(1),
            param_id: queue.param_id,
            points: queue.points.to_vec(),
        })
    }

    /// Create an empty queue for output.
    pub fn new_empty(param_id: u32) -> Box<Self> {
        Box::new(ParamValueQueueImpl {
            vtable: &PARAM_VALUE_QUEUE_VTABLE,
            ref_count: AtomicU32::new(1),
            param_id,
            points: Vec::with_capacity(16),
        })
    }

    /// Convert to a ParameterQueue.
    pub fn to_queue(&self) -> ParameterQueue {
        let mut queue = ParameterQueue::new(self.param_id);
        for point in &self.points {
            queue.add_point(point.sample_offset, point.value);
        }
        queue
    }

    /// Get the parameter ID.
    pub fn param_id(&self) -> u32 {
        self.param_id
    }

    /// Get the number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}


static PARAM_VALUE_QUEUE_VTABLE: IParamValueQueueVtable = IParamValueQueueVtable {
    query_interface: param_queue_query_interface,
    add_ref: param_queue_add_ref,
    release: param_queue_release,
    get_parameter_id: param_queue_get_parameter_id,
    get_point_count: param_queue_get_point_count,
    get_point: param_queue_get_point,
    add_point: param_queue_add_point,
};

unsafe extern "system" fn param_queue_query_interface(
    this: *mut c_void,
    _iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    *obj = this;
    param_queue_add_ref(this);
    K_RESULT_OK
}

unsafe extern "system" fn param_queue_add_ref(this: *mut c_void) -> u32 {
    let queue = &*(this as *const ParamValueQueueImpl);
    queue.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn param_queue_release(this: *mut c_void) -> u32 {
    let queue = &*(this as *const ParamValueQueueImpl);
    let count = queue.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut ParamValueQueueImpl);
    }
    count
}

unsafe extern "system" fn param_queue_get_parameter_id(this: *mut c_void) -> u32 {
    let queue = &*(this as *const ParamValueQueueImpl);
    queue.param_id
}

unsafe extern "system" fn param_queue_get_point_count(this: *mut c_void) -> i32 {
    let queue = &*(this as *const ParamValueQueueImpl);
    queue.points.len() as i32
}

unsafe extern "system" fn param_queue_get_point(
    this: *mut c_void,
    index: i32,
    sample_offset: *mut i32,
    value: *mut f64,
) -> i32 {
    let queue = &*(this as *const ParamValueQueueImpl);
    if index < 0 || index >= queue.points.len() as i32 {
        return -1;
    }

    let point = &queue.points[index as usize];
    *sample_offset = point.sample_offset;
    *value = point.value;
    K_RESULT_OK
}

unsafe extern "system" fn param_queue_add_point(
    this: *mut c_void,
    sample_offset: i32,
    value: f64,
    index: *mut i32,
) -> i32 {
    let queue = &mut *(this as *mut ParamValueQueueImpl);
    queue.points.push(ParameterPoint {
        sample_offset,
        value,
    });
    *index = (queue.points.len() - 1) as i32;
    K_RESULT_OK
}
