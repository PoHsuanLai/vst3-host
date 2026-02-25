//! IProgress COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender};

use super::{com_add_ref, com_release, HasRefCount};
use crate::ffi::{utf16_to_string, IProgressVtable, IID_IPROGRESS, K_NOT_IMPLEMENTED, K_RESULT_OK};

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Started {
        id: u64,
        progress_type: u32,
        description: String,
    },
    /// 0.0 to 1.0.
    Updated {
        id: u64,
        progress: f64,
    },
    Finished {
        id: u64,
    },
}

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

impl HasRefCount for ProgressHandler {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl ProgressHandler {
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
    com_add_ref::<ProgressHandler>(this)
}

unsafe extern "system" fn progress_release(this: *mut c_void) -> u32 {
    com_release::<ProgressHandler>(this)
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
        let mut len = 0;
        let mut ptr = description;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
        }
        utf16_to_string(std::slice::from_raw_parts(description, len))
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
