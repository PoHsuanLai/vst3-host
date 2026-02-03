//! Parameter automation types.

use smallvec::SmallVec;

/// A single automation point for a parameter.
#[derive(Debug, Clone, Copy)]
pub struct ParameterPoint {
    /// Sample offset within the processing block.
    pub sample_offset: i32,
    /// Normalized parameter value (0.0 to 1.0).
    pub value: f64,
}

/// Automation queue for a single parameter.
#[derive(Debug, Clone)]
pub struct ParameterQueue {
    /// VST3 parameter ID.
    pub param_id: u32,
    /// Automation points sorted by sample offset.
    pub points: SmallVec<[ParameterPoint; 8]>,
}

impl ParameterQueue {
    /// Create a new empty parameter queue.
    pub fn new(param_id: u32) -> Self {
        Self {
            param_id,
            points: SmallVec::new(),
        }
    }

    /// Add an automation point.
    pub fn add_point(&mut self, sample_offset: i32, value: f64) {
        self.points.push(ParameterPoint {
            sample_offset,
            value,
        });
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get the number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Clear all points.
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

/// Collection of parameter automation queues.
#[derive(Debug, Clone, Default)]
pub struct ParameterChanges {
    /// Parameter queues (one per automated parameter).
    pub queues: SmallVec<[ParameterQueue; 16]>,
}

impl ParameterChanges {
    /// Create a new empty parameter changes collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a parameter change point.
    ///
    /// Creates a new queue for the parameter if one doesn't exist.
    pub fn add_change(&mut self, param_id: u32, sample_offset: i32, value: f64) {
        // Find existing queue or create new one
        if let Some(queue) = self.queues.iter_mut().find(|q| q.param_id == param_id) {
            queue.add_point(sample_offset, value);
        } else {
            let mut queue = ParameterQueue::new(param_id);
            queue.add_point(sample_offset, value);
            self.queues.push(queue);
        }
    }

    /// Check if there are any parameter changes.
    pub fn is_empty(&self) -> bool {
        self.queues.is_empty() || self.queues.iter().all(|q| q.is_empty())
    }

    /// Get the number of parameters with changes.
    pub fn len(&self) -> usize {
        self.queues.len()
    }

    /// Clear all parameter changes.
    pub fn clear(&mut self) {
        self.queues.clear();
    }

    /// Get a queue by parameter ID.
    pub fn get_queue(&self, param_id: u32) -> Option<&ParameterQueue> {
        self.queues.iter().find(|q| q.param_id == param_id)
    }

    /// Get a mutable queue by parameter ID.
    pub fn get_queue_mut(&mut self, param_id: u32) -> Option<&mut ParameterQueue> {
        self.queues.iter_mut().find(|q| q.param_id == param_id)
    }
}
