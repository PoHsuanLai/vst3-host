//! IParamValueQueue COM implementation.

use parking_lot::Mutex;
use vst3::{Class, ComWrapper};
use vst3::Steinberg::{
    kInvalidArgument, kResultOk, tresult,
    Vst::{IParamValueQueue, IParamValueQueueTrait},
};

use crate::types::{ParameterPoint, ParameterQueue};

pub struct ParamValueQueueImpl {
    param_id: u32,
    points: Mutex<Vec<ParameterPoint>>,
}

impl Class for ParamValueQueueImpl {
    type Interfaces = (IParamValueQueue,);
}

impl ParamValueQueueImpl {
    pub fn from_queue(queue: &ParameterQueue) -> ComWrapper<Self> {
        ComWrapper::new(Self {
            param_id: queue.param_id,
            points: Mutex::new(queue.points.to_vec()),
        })
    }

    pub fn new_empty(param_id: u32) -> ComWrapper<Self> {
        ComWrapper::new(Self {
            param_id,
            points: Mutex::new(Vec::with_capacity(16)),
        })
    }

    pub fn to_queue(&self) -> ParameterQueue {
        let mut queue = ParameterQueue::new(self.param_id);
        for point in self.points.lock().iter() {
            queue.add_point(point.sample_offset, point.value);
        }
        queue
    }

    pub fn param_id(&self) -> u32 {
        self.param_id
    }

    pub fn len(&self) -> usize {
        self.points.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.lock().is_empty()
    }
}

impl IParamValueQueueTrait for ParamValueQueueImpl {
    unsafe fn getParameterId(&self) -> u32 {
        self.param_id
    }

    unsafe fn getPointCount(&self) -> i32 {
        self.points.lock().len() as i32
    }

    unsafe fn getPoint(&self, index: i32, sample_offset: *mut i32, value: *mut f64) -> tresult {
        let points = self.points.lock();
        if index < 0 || index >= points.len() as i32 {
            return kInvalidArgument;
        }
        let point = &points[index as usize];
        if !sample_offset.is_null() {
            *sample_offset = point.sample_offset;
        }
        if !value.is_null() {
            *value = point.value;
        }
        kResultOk
    }

    unsafe fn addPoint(
        &self,
        sample_offset: i32,
        value: f64,
        index: *mut i32,
    ) -> tresult {
        let mut points = self.points.lock();
        points.push(ParameterPoint {
            sample_offset,
            value,
        });
        if !index.is_null() {
            *index = (points.len() - 1) as i32;
        }
        kResultOk
    }
}
