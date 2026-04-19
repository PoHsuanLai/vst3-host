//! IParameterChanges COM implementation.

use parking_lot::Mutex;
use vst3::{Class, ComWrapper};
use vst3::Steinberg::Vst::{IParameterChanges, IParameterChangesTrait, IParamValueQueue};

use super::param_queue::ParamValueQueueImpl;
use crate::types::ParameterChanges;

pub struct ParameterChangesImpl {
    queues: Mutex<Vec<ComWrapper<ParamValueQueueImpl>>>,
}

impl Class for ParameterChangesImpl {
    type Interfaces = (IParameterChanges,);
}

impl ParameterChangesImpl {
    pub fn from_changes(changes: &ParameterChanges) -> ComWrapper<Self> {
        let queues: Vec<_> = changes
            .queues
            .iter()
            .map(ParamValueQueueImpl::from_queue)
            .collect();

        ComWrapper::new(Self {
            queues: Mutex::new(queues),
        })
    }

    pub fn new_empty() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            queues: Mutex::new(Vec::with_capacity(32)),
        })
    }

    pub fn to_changes(&self) -> ParameterChanges {
        let mut changes = ParameterChanges::new();
        for queue in self.queues.lock().iter() {
            let q = queue.to_queue();
            for point in &q.points {
                changes.add_change(q.param_id, point.sample_offset, point.value);
            }
        }
        changes
    }

    pub fn len(&self) -> usize {
        self.queues.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queues.lock().is_empty()
    }
}

impl IParameterChangesTrait for ParameterChangesImpl {
    unsafe fn getParameterCount(&self) -> i32 {
        self.queues.lock().len() as i32
    }

    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        let queues = self.queues.lock();
        if index < 0 || index >= queues.len() as i32 {
            return std::ptr::null_mut();
        }
        // Borrowed pointer: VST3 plugins do not release queues returned here.
        match queues[index as usize].as_com_ref::<IParamValueQueue>() {
            Some(r) => r.as_ptr(),
            None => std::ptr::null_mut(),
        }
    }

    unsafe fn addParameterData(&self, id: *const u32, index: *mut i32) -> *mut IParamValueQueue {
        if id.is_null() {
            return std::ptr::null_mut();
        }
        let param_id = *id;
        let new_queue = ParamValueQueueImpl::new_empty(param_id);
        let queue_ptr = new_queue
            .as_com_ref::<IParamValueQueue>()
            .map(|r| r.as_ptr())
            .unwrap_or(std::ptr::null_mut());
        let mut queues = self.queues.lock();
        queues.push(new_queue);
        if !index.is_null() {
            *index = (queues.len() - 1) as i32;
        }
        queue_ptr
    }
}
