//! Parameter automation types.
//!
//! A [`ParameterChanges`] bundle is the host's side of the
//! `inputParameterChanges` / `outputParameterChanges` fields on
//! `ProcessData` — one [`ParameterQueue`] per parameter, each with a series of
//! sample-accurate [`ParameterPoint`]s.

use smallvec::SmallVec;

/// One automation sample: a normalized value at a specific frame inside the
/// current processing block.
#[derive(Debug, Clone, Copy)]
pub struct ParameterPoint {
    /// Frame offset within the processing block.
    pub sample_offset: i32,
    /// Normalized (0.0 to 1.0).
    pub value: f64,
}

/// Ordered list of [`ParameterPoint`]s for a single parameter id within one
/// block. Uses `SmallVec` so short automation runs don't allocate.
#[derive(Debug, Clone)]
pub struct ParameterQueue {
    /// Parameter id this queue targets.
    pub param_id: u32,
    /// Points in ascending `sample_offset` order (the caller is responsible
    /// for maintaining order).
    pub points: SmallVec<[ParameterPoint; 8]>,
}

impl ParameterQueue {
    /// Create an empty queue targeting `param_id`.
    pub fn new(param_id: u32) -> Self {
        Self {
            param_id,
            points: SmallVec::new(),
        }
    }

    /// Append a `(sample_offset, value)` point.
    pub fn add_point(&mut self, sample_offset: i32, value: f64) {
        self.points.push(ParameterPoint {
            sample_offset,
            value,
        });
    }

    /// True if this queue has no points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Number of points currently in the queue.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Remove all points.
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

/// Collection of [`ParameterQueue`]s, one per parameter id, passed into or
/// returned from [`Vst3Instance::process`](crate::Vst3Instance::process).
#[derive(Debug, Clone, Default)]
pub struct ParameterChanges {
    /// One queue per parameter; the first 16 live inline.
    pub queues: SmallVec<[ParameterQueue; 16]>,
}

impl ParameterChanges {
    /// Create an empty `ParameterChanges`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a point to the queue for `param_id`, creating the queue if it
    /// doesn't exist yet.
    pub fn add_change(&mut self, param_id: u32, sample_offset: i32, value: f64) {
        if let Some(queue) = self.queues.iter_mut().find(|q| q.param_id == param_id) {
            queue.add_point(sample_offset, value);
        } else {
            let mut queue = ParameterQueue::new(param_id);
            queue.add_point(sample_offset, value);
            self.queues.push(queue);
        }
    }

    /// True if there are no points across any queue.
    pub fn is_empty(&self) -> bool {
        self.queues.is_empty() || self.queues.iter().all(|q| q.is_empty())
    }

    /// Number of queues (not points).
    pub fn len(&self) -> usize {
        self.queues.len()
    }

    /// Remove all queues.
    pub fn clear(&mut self) {
        self.queues.clear();
    }

    /// Borrow the queue for `param_id`, if present.
    pub fn get_queue(&self, param_id: u32) -> Option<&ParameterQueue> {
        self.queues.iter().find(|q| q.param_id == param_id)
    }

    /// Mutably borrow the queue for `param_id`, if present.
    pub fn get_queue_mut(&mut self, param_id: u32) -> Option<&mut ParameterQueue> {
        self.queues.iter_mut().find(|q| q.param_id == param_id)
    }
}
