//! Parameter automation types.

use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub struct ParameterPoint {
    pub sample_offset: i32,
    /// Normalized (0.0 to 1.0).
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct ParameterQueue {
    pub param_id: u32,
    pub points: SmallVec<[ParameterPoint; 8]>,
}

impl ParameterQueue {
    pub fn new(param_id: u32) -> Self {
        Self {
            param_id,
            points: SmallVec::new(),
        }
    }

    pub fn add_point(&mut self, sample_offset: i32, value: f64) {
        self.points.push(ParameterPoint {
            sample_offset,
            value,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParameterChanges {
    pub queues: SmallVec<[ParameterQueue; 16]>,
}

impl ParameterChanges {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_change(&mut self, param_id: u32, sample_offset: i32, value: f64) {
        if let Some(queue) = self.queues.iter_mut().find(|q| q.param_id == param_id) {
            queue.add_point(sample_offset, value);
        } else {
            let mut queue = ParameterQueue::new(param_id);
            queue.add_point(sample_offset, value);
            self.queues.push(queue);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queues.is_empty() || self.queues.iter().all(|q| q.is_empty())
    }

    pub fn len(&self) -> usize {
        self.queues.len()
    }

    pub fn clear(&mut self) {
        self.queues.clear();
    }

    pub fn get_queue(&self, param_id: u32) -> Option<&ParameterQueue> {
        self.queues.iter().find(|q| q.param_id == param_id)
    }

    pub fn get_queue_mut(&mut self, param_id: u32) -> Option<&mut ParameterQueue> {
        self.queues.iter_mut().find(|q| q.param_id == param_id)
    }
}
