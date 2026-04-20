//! IParameterChanges COM implementation.
//!
//! # Real-time safety
//!
//! The inner `Vec<ComWrapper<ParamValueQueueImpl>>` lives in an
//! [`AudioThreadCell`], mirroring
//! [`super::event_list::EventList`] and
//! [`super::param_queue::ParamValueQueueImpl`].
//!
//! [`refill_from_changes`](ParameterChangesImpl::refill_from_changes)
//! lets a host reuse a single `ComWrapper<ParameterChangesImpl>` across
//! buffers — clearing the internal queues, recycling their backing
//! `ComWrapper<ParamValueQueueImpl>` storage, and refilling in place —
//! so the RT path makes no new heap allocations once the process loop
//! has warmed up.

use vst3::{Class, ComWrapper};
use vst3::Steinberg::Vst::{IParameterChanges, IParameterChangesTrait, IParamValueQueue};

use super::param_queue::ParamValueQueueImpl;
use crate::rt_cell::AudioThreadCell;
use crate::types::ParameterChanges;

pub struct ParameterChangesImpl {
    queues: AudioThreadCell<Vec<ComWrapper<ParamValueQueueImpl>>>,
}

impl Class for ParameterChangesImpl {
    type Interfaces = (IParameterChanges,);
}

impl ParameterChangesImpl {
    /// Build a changes list from an existing [`ParameterChanges`]. Each
    /// call allocates a fresh `ComWrapper<ParamValueQueueImpl>` per
    /// queue; **not RT-safe** — use [`new_empty`] +
    /// [`refill_from_changes`] on the hot path.
    pub fn from_changes(changes: &ParameterChanges) -> ComWrapper<Self> {
        let mut queues: Vec<_> = Vec::with_capacity(changes.queues.len().max(32));
        for q in &changes.queues {
            queues.push(ParamValueQueueImpl::from_queue(q));
        }
        ComWrapper::new(Self {
            queues: AudioThreadCell::new(queues),
        })
    }

    pub fn new_empty() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            queues: AudioThreadCell::new(Vec::with_capacity(32)),
        })
    }

    /// Refill this changes list from `changes` in place. Reuses existing
    /// `ComWrapper<ParamValueQueueImpl>` slots; grows once (off the hot
    /// path in practice) if `changes` carries more queues than we've
    /// seen before, and keeps the larger capacity thereafter.
    ///
    /// # Real-time
    ///
    /// Allocation-free once the host has seen at least as many distinct
    /// parameter queues as `changes.queues.len()`. The first few blocks
    /// may grow; stable-state automation is alloc-free.
    pub fn refill_from_changes(&self, changes: &ParameterChanges) {
        let queues = self.queues.borrow_mut();
        // Grow (off-the-hot-path) if needed.
        while queues.len() < changes.queues.len() {
            // Placeholder param_id — we immediately refill it below.
            queues.push(ParamValueQueueImpl::new_empty(0));
        }
        // Refill the live slots in place.
        for (slot, q) in queues.iter().zip(changes.queues.iter()) {
            slot.refill_from_queue(q);
        }
        // Trim leftover slots from a larger previous block. Drop only
        // drops the ComWrapper (refcount--); the queue storage returns
        // to the allocator off-RT if no other reference holds it.
        if queues.len() > changes.queues.len() {
            queues.truncate(changes.queues.len());
        }
    }

    /// Clear all queues in place (reuse-friendly variant of drop-and-refill).
    pub fn clear_in_place(&self) {
        self.queues.borrow_mut().clear();
    }

    pub fn to_changes(&self) -> ParameterChanges {
        let mut changes = ParameterChanges::new();
        for queue in self.queues.borrow().iter() {
            let q = queue.to_queue();
            for point in &q.points {
                changes.add_change(q.param_id, point.sample_offset, point.value);
            }
        }
        changes
    }

    pub fn len(&self) -> usize {
        self.queues.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queues.borrow().is_empty()
    }

    /// Reset the audio-thread owner (see [`AudioThreadCell::reset_owner`]).
    /// Also resets the owner on every contained queue.
    pub fn reset_owner(&self) {
        self.queues.reset_owner();
        // borrow() after reset re-claims ownership on the current thread,
        // which is fine — this method is called off-RT during device swap.
        for queue in self.queues.borrow().iter() {
            queue.reset_owner();
        }
    }
}

impl IParameterChangesTrait for ParameterChangesImpl {
    unsafe fn getParameterCount(&self) -> i32 {
        self.queues.borrow().len() as i32
    }

    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        let queues = self.queues.borrow();
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
        let queues = self.queues.borrow_mut();
        queues.push(new_queue);
        if !index.is_null() {
            *index = (queues.len() - 1) as i32;
        }
        queue_ptr
    }
}
