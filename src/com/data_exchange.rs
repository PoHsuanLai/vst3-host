//! IDataExchangeHandler COM implementation.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use crate::ffi::{
    DataExchangeBlock, IDataExchangeHandlerVtable, IID_IDATA_EXCHANGE_HANDLER, K_NOT_IMPLEMENTED,
    K_RESULT_OK,
};

/// Data block received from the audio processor.
#[derive(Debug, Clone)]
pub struct DataBlock {
    pub user_context_id: u32,
    pub data: Vec<u8>,
}

struct Queue {
    block_size: u32,
    #[allow(dead_code)]
    num_blocks: u32,
    #[allow(dead_code)]
    alignment: u32,
    blocks: Vec<Vec<u8>>,
    next_block_id: u32,
    locked_blocks: HashMap<u32, Vec<u8>>,
}

impl Queue {
    fn new(block_size: u32, num_blocks: u32, alignment: u32) -> Self {
        let mut blocks = Vec::with_capacity(num_blocks as usize);
        for _ in 0..num_blocks {
            blocks.push(vec![0u8; block_size as usize]);
        }
        Queue {
            block_size,
            num_blocks,
            alignment,
            blocks,
            next_block_id: 0,
            locked_blocks: HashMap::new(),
        }
    }
}

/// Enables direct, thread-safe data transfer from audio processor to edit controller
/// for visualization purposes (e.g., waveform displays, spectrum analyzers).
#[repr(C)]
pub struct DataExchangeHandler {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IDataExchangeHandlerVtable,
    ref_count: AtomicU32,
    next_queue_id: AtomicU32,
    queues: Mutex<HashMap<u32, Queue>>,
    data_sender: Sender<DataBlock>,
}

unsafe impl Send for DataExchangeHandler {}
unsafe impl Sync for DataExchangeHandler {}

impl DataExchangeHandler {
    /// Create a new data exchange handler, returning it and a receiver for data blocks.
    pub fn new() -> (Box<Self>, Receiver<DataBlock>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let handler = Box::new(DataExchangeHandler {
            vtable: &DATA_EXCHANGE_HANDLER_VTABLE,
            ref_count: AtomicU32::new(1),
            next_queue_id: AtomicU32::new(1),
            queues: Mutex::new(HashMap::new()),
            data_sender: tx,
        });
        (handler, rx)
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const DataExchangeHandler as *mut c_void
    }
}

static DATA_EXCHANGE_HANDLER_VTABLE: IDataExchangeHandlerVtable = IDataExchangeHandlerVtable {
    query_interface: handler_query_interface,
    add_ref: handler_add_ref,
    release: handler_release,
    open_queue: handler_open_queue,
    close_queue: handler_close_queue,
    lock_block: handler_lock_block,
    free_block: handler_free_block,
};

unsafe extern "system" fn handler_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IDATA_EXCHANGE_HANDLER {
        *obj = this;
        handler_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn handler_add_ref(this: *mut c_void) -> u32 {
    let handler = &*(this as *const DataExchangeHandler);
    handler.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn handler_release(this: *mut c_void) -> u32 {
    let handler = &*(this as *const DataExchangeHandler);
    let count = handler.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut DataExchangeHandler);
    }
    count
}

unsafe extern "system" fn handler_open_queue(
    this: *mut c_void,
    _processor: *mut c_void,
    block_size: u32,
    num_blocks: u32,
    alignment: u32,
    user_context_id: u32,
    out_queue_id: *mut u32,
) -> i32 {
    let handler = &*(this as *const DataExchangeHandler);

    let queue_id = handler.next_queue_id.fetch_add(1, Ordering::SeqCst);
    let queue = Queue::new(block_size, num_blocks, alignment);
    handler.queues.lock().insert(user_context_id, queue);

    if !out_queue_id.is_null() {
        *out_queue_id = queue_id;
    }

    K_RESULT_OK
}

unsafe extern "system" fn handler_close_queue(this: *mut c_void, queue_id: u32) -> i32 {
    let handler = &*(this as *const DataExchangeHandler);
    handler.queues.lock().remove(&queue_id);
    K_RESULT_OK
}

unsafe extern "system" fn handler_lock_block(
    this: *mut c_void,
    queue_id: u32,
    block: *mut DataExchangeBlock,
) -> i32 {
    let handler = &*(this as *const DataExchangeHandler);

    let mut queues = handler.queues.lock();
    if let Some(queue) = queues.get_mut(&queue_id) {
        if let Some(mut data) = queue.blocks.pop() {
            let block_id = queue.next_block_id;
            queue.next_block_id += 1;

            (*block).data = data.as_mut_ptr() as *mut c_void;
            (*block).size = queue.block_size;
            (*block).block_id = block_id;

            queue.locked_blocks.insert(block_id, data);

            return K_RESULT_OK;
        }
    }

    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn handler_free_block(
    this: *mut c_void,
    queue_id: u32,
    block_id: u32,
    send_to_controller: u8,
) -> i32 {
    let handler = &*(this as *const DataExchangeHandler);

    let mut queues = handler.queues.lock();
    if let Some(queue) = queues.get_mut(&queue_id) {
        if let Some(data) = queue.locked_blocks.remove(&block_id) {
            if send_to_controller != 0 {
                let _ = handler.data_sender.send(DataBlock {
                    user_context_id: queue_id,
                    data: data.clone(),
                });
            }

            queue.blocks.push(data);

            return K_RESULT_OK;
        }
    }

    K_NOT_IMPLEMENTED
}
