//! IParameterChanges COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use super::{com_add_ref, com_release, HasRefCount};
use crate::ffi::{IParameterChangesVtable, K_RESULT_OK};
use crate::types::ParameterChanges;

use super::param_queue::ParamValueQueueImpl;

#[repr(C)]
pub struct ParameterChangesImpl {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IParameterChangesVtable,
    ref_count: AtomicU32,
    #[allow(clippy::vec_box)] // Box needed for stable pointers in COM interface
    queues: Vec<Box<ParamValueQueueImpl>>,
}

unsafe impl Send for ParameterChangesImpl {}
unsafe impl Sync for ParameterChangesImpl {}

impl HasRefCount for ParameterChangesImpl {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl ParameterChangesImpl {
    pub fn from_changes(changes: &ParameterChanges) -> Box<Self> {
        let queues: Vec<Box<ParamValueQueueImpl>> = changes
            .queues
            .iter()
            .map(ParamValueQueueImpl::from_queue)
            .collect();

        Box::new(ParameterChangesImpl {
            vtable: &PARAMETER_CHANGES_VTABLE,
            ref_count: AtomicU32::new(1),
            queues,
        })
    }

    pub fn new_empty() -> Box<Self> {
        Box::new(ParameterChangesImpl {
            vtable: &PARAMETER_CHANGES_VTABLE,
            ref_count: AtomicU32::new(1),
            queues: Vec::with_capacity(32),
        })
    }

    pub fn to_changes(&self) -> ParameterChanges {
        let mut changes = ParameterChanges::new();
        for queue in &self.queues {
            let q = queue.to_queue();
            for point in &q.points {
                changes.add_change(q.param_id, point.sample_offset, point.value);
            }
        }
        changes
    }

    pub fn len(&self) -> usize {
        self.queues.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }

    pub fn as_ptr(&mut self) -> *mut c_void {
        self as *mut ParameterChangesImpl as *mut c_void
    }
}

static PARAMETER_CHANGES_VTABLE: IParameterChangesVtable = IParameterChangesVtable {
    query_interface: param_changes_query_interface,
    add_ref: param_changes_add_ref,
    release: param_changes_release,
    get_parameter_count: param_changes_get_parameter_count,
    get_parameter_data: param_changes_get_parameter_data,
    add_parameter_data: param_changes_add_parameter_data,
};

unsafe extern "system" fn param_changes_query_interface(
    this: *mut c_void,
    _iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    *obj = this;
    param_changes_add_ref(this);
    K_RESULT_OK
}

unsafe extern "system" fn param_changes_add_ref(this: *mut c_void) -> u32 {
    com_add_ref::<ParameterChangesImpl>(this)
}

unsafe extern "system" fn param_changes_release(this: *mut c_void) -> u32 {
    com_release::<ParameterChangesImpl>(this)
}

unsafe extern "system" fn param_changes_get_parameter_count(this: *mut c_void) -> i32 {
    let changes = &*(this as *const ParameterChangesImpl);
    changes.queues.len() as i32
}

unsafe extern "system" fn param_changes_get_parameter_data(
    this: *mut c_void,
    index: i32,
) -> *mut c_void {
    let changes = &*(this as *const ParameterChangesImpl);
    if index < 0 || index >= changes.queues.len() as i32 {
        return std::ptr::null_mut();
    }

    &*changes.queues[index as usize] as *const ParamValueQueueImpl as *mut c_void
}

unsafe extern "system" fn param_changes_add_parameter_data(
    this: *mut c_void,
    param_id: *const u32,
    index: *mut i32,
) -> *mut c_void {
    let changes = &mut *(this as *mut ParameterChangesImpl);
    let new_queue = ParamValueQueueImpl::new_empty(*param_id);
    let queue_ptr = &*new_queue as *const ParamValueQueueImpl as *mut c_void;
    changes.queues.push(new_queue);
    *index = (changes.queues.len() - 1) as i32;
    queue_ptr
}
