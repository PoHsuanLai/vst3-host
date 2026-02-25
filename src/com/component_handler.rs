//! IComponentHandler COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use crossbeam_channel::{Receiver, Sender};

use super::{com_add_ref, com_release, container_of, HasRefCount};
use crate::ffi::{
    IComponentHandler2Vtable, IComponentHandler3Vtable, IComponentHandlerBusActivationVtable,
    IComponentHandlerVtable, IID_ICOMPONENT_HANDLER, IID_ICOMPONENT_HANDLER2,
    IID_ICOMPONENT_HANDLER3, IID_ICOMPONENT_HANDLER_BUS_ACTIVATION, K_NOT_IMPLEMENTED, K_RESULT_OK,
};

#[derive(Debug, Clone)]
pub enum ParameterEditEvent {
    BeginEdit(u32),
    PerformEdit {
        param_id: u32,
        value: f64,
    },
    EndEdit(u32),
    RestartComponent(i32),
    SetDirty(bool),
    RequestOpenEditor,
    StartGroupEdit,
    FinishGroupEdit,
    RequestBusActivation {
        media_type: i32,
        direction: i32,
        index: i32,
        state: bool,
    },
}

#[repr(C)]
pub struct ComponentHandler {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IComponentHandlerVtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable2: *const IComponentHandler2Vtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable3: *const IComponentHandler3Vtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable_bus: *const IComponentHandlerBusActivationVtable,
    ref_count: AtomicU32,
    event_sender: Sender<ParameterEditEvent>,
}

unsafe impl Send for ComponentHandler {}
unsafe impl Sync for ComponentHandler {}

impl HasRefCount for ComponentHandler {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl ComponentHandler {
    pub fn new() -> (Box<Self>, Receiver<ParameterEditEvent>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let handler = Box::new(ComponentHandler {
            vtable: &COMPONENT_HANDLER_VTABLE,
            vtable2: &COMPONENT_HANDLER2_VTABLE,
            vtable3: &COMPONENT_HANDLER3_VTABLE,
            vtable_bus: &COMPONENT_HANDLER_BUS_ACTIVATION_VTABLE,
            ref_count: AtomicU32::new(1),
            event_sender: tx,
        });
        (handler, rx)
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const ComponentHandler as *mut c_void
    }
}

static COMPONENT_HANDLER_VTABLE: IComponentHandlerVtable = IComponentHandlerVtable {
    query_interface: handler_query_interface,
    add_ref: handler_add_ref,
    release: handler_release,
    begin_edit: handler_begin_edit,
    perform_edit: handler_perform_edit,
    end_edit: handler_end_edit,
    restart_component: handler_restart_component,
};

unsafe extern "system" fn handler_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;

    if *iid_ref == IID_ICOMPONENT_HANDLER {
        *obj = this;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_ICOMPONENT_HANDLER2 {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable2 as *const _ as *mut c_void;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_ICOMPONENT_HANDLER3 {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable3 as *const _ as *mut c_void;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_ICOMPONENT_HANDLER_BUS_ACTIVATION {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable_bus as *const _ as *mut c_void;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn handler_add_ref(this: *mut c_void) -> u32 {
    com_add_ref::<ComponentHandler>(this)
}

unsafe extern "system" fn handler_release(this: *mut c_void) -> u32 {
    com_release::<ComponentHandler>(this)
}

unsafe extern "system" fn handler_begin_edit(this: *mut c_void, param_id: u32) -> i32 {
    let handler = &*(this as *const ComponentHandler);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::BeginEdit(param_id));
    K_RESULT_OK
}

unsafe extern "system" fn handler_perform_edit(
    this: *mut c_void,
    param_id: u32,
    value: f64,
) -> i32 {
    let handler = &*(this as *const ComponentHandler);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::PerformEdit { param_id, value });
    K_RESULT_OK
}

unsafe extern "system" fn handler_end_edit(this: *mut c_void, param_id: u32) -> i32 {
    let handler = &*(this as *const ComponentHandler);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::EndEdit(param_id));
    K_RESULT_OK
}

unsafe extern "system" fn handler_restart_component(this: *mut c_void, flags: i32) -> i32 {
    let handler = &*(this as *const ComponentHandler);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::RestartComponent(flags));
    K_RESULT_OK
}

// ---------------------------------------------------------------------------
// IComponentHandler2
// ---------------------------------------------------------------------------

static COMPONENT_HANDLER2_VTABLE: IComponentHandler2Vtable = IComponentHandler2Vtable {
    query_interface: handler2_query_interface,
    add_ref: handler2_add_ref,
    release: handler2_release,
    set_dirty: handler2_set_dirty,
    request_open_editor: handler2_request_open_editor,
    start_group_edit: handler2_start_group_edit,
    finish_group_edit: handler2_finish_group_edit,
};

/// Recover the parent `ComponentHandler` from a secondary vtable pointer.
unsafe fn handler_from_vtable2(this: *mut c_void) -> &'static ComponentHandler {
    &*container_of!(this, ComponentHandler, vtable2)
}

unsafe extern "system" fn handler2_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable2) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler2_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable2) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler2_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable2) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler2_set_dirty(this: *mut c_void, state: u8) -> i32 {
    let handler = handler_from_vtable2(this);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::SetDirty(state != 0));
    K_RESULT_OK
}

unsafe extern "system" fn handler2_request_open_editor(this: *mut c_void, _name: *const i8) -> i32 {
    let handler = handler_from_vtable2(this);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::RequestOpenEditor);
    K_RESULT_OK
}

unsafe extern "system" fn handler2_start_group_edit(this: *mut c_void) -> i32 {
    let handler = handler_from_vtable2(this);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::StartGroupEdit);
    K_RESULT_OK
}

unsafe extern "system" fn handler2_finish_group_edit(this: *mut c_void) -> i32 {
    let handler = handler_from_vtable2(this);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::FinishGroupEdit);
    K_RESULT_OK
}

// ---------------------------------------------------------------------------
// IComponentHandler3
// ---------------------------------------------------------------------------

static COMPONENT_HANDLER3_VTABLE: IComponentHandler3Vtable = IComponentHandler3Vtable {
    query_interface: handler3_query_interface,
    add_ref: handler3_add_ref,
    release: handler3_release,
    create_context_menu: handler3_create_context_menu,
};

unsafe extern "system" fn handler3_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable3) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler3_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable3) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler3_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable3) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler3_create_context_menu(
    _this: *mut c_void,
    _plug_view: *mut c_void,
    _param_id: *const u32,
) -> *mut c_void {
    std::ptr::null_mut()
}

// ---------------------------------------------------------------------------
// IComponentHandlerBusActivation
// ---------------------------------------------------------------------------

static COMPONENT_HANDLER_BUS_ACTIVATION_VTABLE: IComponentHandlerBusActivationVtable =
    IComponentHandlerBusActivationVtable {
        query_interface: handler_bus_query_interface,
        add_ref: handler_bus_add_ref,
        release: handler_bus_release,
        request_bus_activation: handler_bus_request_activation,
    };

unsafe extern "system" fn handler_bus_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable_bus) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler_bus_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_bus) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler_bus_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_bus) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler_bus_request_activation(
    this: *mut c_void,
    media_type: i32,
    direction: i32,
    index: i32,
    state: u8,
) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_bus);
    let _ = handler
        .event_sender
        .send(ParameterEditEvent::RequestBusActivation {
            media_type,
            direction,
            index,
            state: state != 0,
        });
    K_RESULT_OK
}
