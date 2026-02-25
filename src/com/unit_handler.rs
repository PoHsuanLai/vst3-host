//! IUnitHandler COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use crossbeam_channel::{Receiver, Sender};

use super::{com_add_ref, com_release, container_of, HasRefCount};
use crate::ffi::{
    IUnitHandler2Vtable, IUnitHandlerVtable, IID_IUNIT_HANDLER, IID_IUNIT_HANDLER2,
    K_NOT_IMPLEMENTED, K_RESULT_OK,
};

#[derive(Debug, Clone)]
pub enum UnitEvent {
    UnitSelected(i32),
    ProgramListChanged { list_id: i32, program_index: i32 },
    UnitByBusChanged,
}

#[repr(C)]
pub struct UnitHandler {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IUnitHandlerVtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable2: *const IUnitHandler2Vtable,
    ref_count: AtomicU32,
    event_sender: Sender<UnitEvent>,
}

unsafe impl Send for UnitHandler {}
unsafe impl Sync for UnitHandler {}

impl HasRefCount for UnitHandler {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl UnitHandler {
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
    com_add_ref::<UnitHandler>(this)
}

unsafe extern "system" fn unit_handler_release(this: *mut c_void) -> u32 {
    com_release::<UnitHandler>(this)
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
    let parent = container_of!(this, UnitHandler, vtable2) as *mut c_void;
    unit_handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn unit_handler2_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, UnitHandler, vtable2) as *mut c_void;
    unit_handler_add_ref(parent)
}

unsafe extern "system" fn unit_handler2_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, UnitHandler, vtable2) as *mut c_void;
    unit_handler_release(parent)
}

unsafe extern "system" fn unit_handler2_notify_by_bus(this: *mut c_void) -> i32 {
    let handler = &*container_of!(this, UnitHandler, vtable2);
    let _ = handler.event_sender.send(UnitEvent::UnitByBusChanged);
    K_RESULT_OK
}
