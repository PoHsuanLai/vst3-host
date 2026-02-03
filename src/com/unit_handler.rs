//! IUnitHandler COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use crossbeam_channel::{Receiver, Sender};

use crate::ffi::{
    IUnitHandler2Vtable, IUnitHandlerVtable, K_NOT_IMPLEMENTED, K_RESULT_OK,
    IID_IUNIT_HANDLER, IID_IUNIT_HANDLER2,
};


/// Unit-related event from the plugin.
#[derive(Debug, Clone)]
pub enum UnitEvent {
    /// Unit selection changed.
    UnitSelected(i32),
    /// Program list changed (list_id, program_index).
    ProgramListChanged { list_id: i32, program_index: i32 },
    /// Unit by bus info changed.
    UnitByBusChanged,
}


/// IUnitHandler COM implementation.
///
/// Receives unit change notifications from the plugin.
#[repr(C)]
pub struct UnitHandler {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IUnitHandlerVtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable2: *const IUnitHandler2Vtable,
    ref_count: AtomicU32,
    event_sender: Sender<UnitEvent>,
}

// Safety: UnitHandler only contains thread-safe types
unsafe impl Send for UnitHandler {}
unsafe impl Sync for UnitHandler {}

impl UnitHandler {
    /// Create a new unit handler.
    ///
    /// Returns the handler and a receiver for unit events.
    pub fn new() -> (Box<Self>, Receiver<UnitEvent>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let handler = Box::new(UnitHandler {
            vtable: &UNIT_HANDLER_VTABLE,
            vtable2: &UNIT_HANDLER2_VTABLE,
            ref_count: AtomicU32::new(1),
            event_sender: tx,
        });
        (handler, rx)
    }

    /// Get a raw pointer suitable for passing to VST3 APIs.
    pub fn as_ptr(&self) -> *mut c_void {
        self as *const UnitHandler as *mut c_void
    }
}


static UNIT_HANDLER_VTABLE: IUnitHandlerVtable = IUnitHandlerVtable {
    query_interface: unit_handler_query_interface,
    add_ref: unit_handler_add_ref,
    release: unit_handler_release,
    notify_unit_selection: unit_handler_notify_selection,
    notify_program_list_change: unit_handler_notify_program_list,
};

unsafe extern "system" fn unit_handler_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;

    if *iid_ref == IID_IUNIT_HANDLER {
        *obj = this;
        unit_handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_IUNIT_HANDLER2 {
        let handler = &*(this as *const UnitHandler);
        *obj = &handler.vtable2 as *const _ as *mut c_void;
        unit_handler_add_ref(this);
        return K_RESULT_OK;
    }

    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn unit_handler_add_ref(this: *mut c_void) -> u32 {
    let handler = &*(this as *const UnitHandler);
    handler.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn unit_handler_release(this: *mut c_void) -> u32 {
    let handler = &*(this as *const UnitHandler);
    let count = handler.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut UnitHandler);
    }
    count
}

unsafe extern "system" fn unit_handler_notify_selection(this: *mut c_void, unit_id: i32) -> i32 {
    let handler = &*(this as *const UnitHandler);
    let _ = handler.event_sender.send(UnitEvent::UnitSelected(unit_id));
    K_RESULT_OK
}

unsafe extern "system" fn unit_handler_notify_program_list(
    this: *mut c_void,
    list_id: i32,
    program_index: i32,
) -> i32 {
    let handler = &*(this as *const UnitHandler);
    let _ = handler.event_sender.send(UnitEvent::ProgramListChanged {
        list_id,
        program_index,
    });
    K_RESULT_OK
}

// IUnitHandler2 Vtable

static UNIT_HANDLER2_VTABLE: IUnitHandler2Vtable = IUnitHandler2Vtable {
    query_interface: unit_handler2_query_interface,
    add_ref: unit_handler2_add_ref,
    release: unit_handler2_release,
    notify_unit_by_bus_change: unit_handler2_notify_by_bus,
};

unsafe extern "system" fn unit_handler2_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let vtable2_ptr = this as *const *const IUnitHandler2Vtable;
    let handler_ptr = (vtable2_ptr as *const u8)
        .sub(std::mem::offset_of!(UnitHandler, vtable2)) as *mut UnitHandler;
    unit_handler_query_interface(handler_ptr as *mut c_void, iid, obj)
}

unsafe extern "system" fn unit_handler2_add_ref(this: *mut c_void) -> u32 {
    let vtable2_ptr = this as *const *const IUnitHandler2Vtable;
    let handler_ptr = (vtable2_ptr as *const u8)
        .sub(std::mem::offset_of!(UnitHandler, vtable2)) as *mut UnitHandler;
    unit_handler_add_ref(handler_ptr as *mut c_void)
}

unsafe extern "system" fn unit_handler2_release(this: *mut c_void) -> u32 {
    let vtable2_ptr = this as *const *const IUnitHandler2Vtable;
    let handler_ptr = (vtable2_ptr as *const u8)
        .sub(std::mem::offset_of!(UnitHandler, vtable2)) as *mut UnitHandler;
    unit_handler_release(handler_ptr as *mut c_void)
}

unsafe extern "system" fn unit_handler2_notify_by_bus(this: *mut c_void) -> i32 {
    let vtable2_ptr = this as *const *const IUnitHandler2Vtable;
    let handler_ptr = (vtable2_ptr as *const u8)
        .sub(std::mem::offset_of!(UnitHandler, vtable2)) as *const UnitHandler;
    let handler = &*handler_ptr;
    let _ = handler.event_sender.send(UnitEvent::UnitByBusChanged);
    K_RESULT_OK
}
