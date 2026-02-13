//! IProgress COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender};

use crate::ffi::{IProgressVtable, IID_IPROGRESS, K_NOT_IMPLEMENTED, K_RESULT_OK};

/// Progress event from the plugin.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Progress started.
    Started {
        id: u64,
        progress_type: u32,
        description: String,
    },
    /// Progress updated (0.0 to 1.0).
    Updated { id: u64, progress: f64 },
    /// Progress finished.
    Finished { id: u64 },
}

/// Handles progress reporting from plugins during long operations.
#[repr(C)]
pub struct ProgressHandler {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IProgressVtable,
    ref_count: AtomicU32,
    next_id: AtomicU64,
    event_sender: Sender<ProgressEvent>,
}

unsafe impl Send for ProgressHandler {}
unsafe impl Sync for ProgressHandler {}

impl ProgressHandler {
    /// Create a new progress handler, returning it and a receiver for progress events.
    pub fn new() -> (Box<Self>, Receiver<ProgressEvent>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let handler = Box::new(ProgressHandler {
            vtable: &PROGRESS_VTABLE,
            ref_count: AtomicU32::new(1),
            next_id: AtomicU64::new(1),
            event_sender: tx,
        });
        (handler, rx)
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const ProgressHandler as *mut c_void
    }
}

static PROGRESS_VTABLE: IProgressVtable = IProgressVtable {
    query_interface: progress_query_interface,
    add_ref: progress_add_ref,
    release: progress_release,
    start: progress_start,
    update: progress_update,
    finish: progress_finish,
};

unsafe extern "system" fn progress_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IPROGRESS {
        *obj = this;
        progress_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn progress_add_ref(this: *mut c_void) -> u32 {
    let handler = &*(this as *const ProgressHandler);
    handler.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn progress_release(this: *mut c_void) -> u32 {
    let handler = &*(this as *const ProgressHandler);
    let count = handler.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut ProgressHandler);
    }
    count
}

unsafe extern "system" fn progress_start(
    this: *mut c_void,
    progress_type: u32,
    description: *const u16,
    out_id: *mut u64,
) -> i32 {
    let handler = &*(this as *const ProgressHandler);

    let id = handler.next_id.fetch_add(1, Ordering::SeqCst);

    let desc = if description.is_null() {
        String::new()
    } else {
        let mut chars = Vec::new();
        let mut ptr = description;
        while *ptr != 0 {
            chars.push(*ptr);
            ptr = ptr.add(1);
        }
        String::from_utf16_lossy(&chars)
    };

    let _ = handler.event_sender.send(ProgressEvent::Started {
        id,
        progress_type,
        description: desc,
    });

    if !out_id.is_null() {
        *out_id = id;
    }

    K_RESULT_OK
}

unsafe extern "system" fn progress_update(this: *mut c_void, id: u64, progress: f64) -> i32 {
    let handler = &*(this as *const ProgressHandler);
    let _ = handler
        .event_sender
        .send(ProgressEvent::Updated { id, progress });
    K_RESULT_OK
}

unsafe extern "system" fn progress_finish(this: *mut c_void, id: u64) -> i32 {
    let handler = &*(this as *const ProgressHandler);
    let _ = handler.event_sender.send(ProgressEvent::Finished { id });
    K_RESULT_OK
}
