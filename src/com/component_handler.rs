//! IComponentHandler COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender};

use super::{com_add_ref, com_release, container_of, HasRefCount};
use crate::ffi::{
    utf16_to_string, IComponentHandler2Vtable, IComponentHandler3Vtable,
    IComponentHandlerBusActivationVtable, IComponentHandlerVtable, IProgressVtable,
    IUnitHandler2Vtable, IUnitHandlerVtable, IID_ICOMPONENT_HANDLER, IID_ICOMPONENT_HANDLER2,
    IID_ICOMPONENT_HANDLER3, IID_ICOMPONENT_HANDLER_BUS_ACTIVATION, IID_IPROGRESS,
    IID_IUNIT_HANDLER, IID_IUNIT_HANDLER2, K_NOT_IMPLEMENTED, K_RESULT_OK,
};

pub use super::progress::ProgressEvent;
pub use super::unit_handler::UnitEvent;

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
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable_progress: *const IProgressVtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable_unit: *const IUnitHandlerVtable,
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable_unit2: *const IUnitHandler2Vtable,
    ref_count: AtomicU32,
    event_sender: Sender<ParameterEditEvent>,
    next_progress_id: AtomicU64,
    progress_sender: Sender<ProgressEvent>,
    unit_sender: Sender<UnitEvent>,
}

unsafe impl Send for ComponentHandler {}
unsafe impl Sync for ComponentHandler {}

impl HasRefCount for ComponentHandler {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl ComponentHandler {
    pub fn new() -> (
        Box<Self>,
        Receiver<ParameterEditEvent>,
        Receiver<ProgressEvent>,
        Receiver<UnitEvent>,
    ) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (progress_tx, progress_rx) = crossbeam_channel::unbounded();
        let (unit_tx, unit_rx) = crossbeam_channel::unbounded();
        let handler = Box::new(ComponentHandler {
            vtable: &COMPONENT_HANDLER_VTABLE,
            vtable2: &COMPONENT_HANDLER2_VTABLE,
            vtable3: &COMPONENT_HANDLER3_VTABLE,
            vtable_bus: &COMPONENT_HANDLER_BUS_ACTIVATION_VTABLE,
            vtable_progress: &HANDLER_PROGRESS_VTABLE,
            vtable_unit: &HANDLER_UNIT_VTABLE,
            vtable_unit2: &HANDLER_UNIT2_VTABLE,
            ref_count: AtomicU32::new(1),
            event_sender: tx,
            next_progress_id: AtomicU64::new(1),
            progress_sender: progress_tx,
            unit_sender: unit_tx,
        });
        (handler, rx, progress_rx, unit_rx)
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

    if *iid_ref == IID_IPROGRESS {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable_progress as *const _ as *mut c_void;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_IUNIT_HANDLER {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable_unit as *const _ as *mut c_void;
        handler_add_ref(this);
        return K_RESULT_OK;
    }

    if *iid_ref == IID_IUNIT_HANDLER2 {
        let handler = &*(this as *const ComponentHandler);
        *obj = &handler.vtable_unit2 as *const _ as *mut c_void;
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

static COMPONENT_HANDLER2_VTABLE: IComponentHandler2Vtable = IComponentHandler2Vtable {
    query_interface: handler2_query_interface,
    add_ref: handler2_add_ref,
    release: handler2_release,
    set_dirty: handler2_set_dirty,
    request_open_editor: handler2_request_open_editor,
    start_group_edit: handler2_start_group_edit,
    finish_group_edit: handler2_finish_group_edit,
};

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

static HANDLER_PROGRESS_VTABLE: IProgressVtable = IProgressVtable {
    query_interface: handler_progress_query_interface,
    add_ref: handler_progress_add_ref,
    release: handler_progress_release,
    start: handler_progress_start,
    update: handler_progress_update,
    finish: handler_progress_finish,
};

unsafe extern "system" fn handler_progress_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable_progress) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler_progress_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_progress) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler_progress_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_progress) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler_progress_start(
    this: *mut c_void,
    progress_type: u32,
    description: *const u16,
    out_id: *mut u64,
) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_progress);
    let id = handler.next_progress_id.fetch_add(1, Ordering::SeqCst);

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

    let _ = handler.progress_sender.send(ProgressEvent::Started {
        id,
        progress_type,
        description: desc,
    });

    if !out_id.is_null() {
        *out_id = id;
    }

    K_RESULT_OK
}

unsafe extern "system" fn handler_progress_update(
    this: *mut c_void,
    id: u64,
    progress: f64,
) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_progress);
    let _ = handler
        .progress_sender
        .send(ProgressEvent::Updated { id, progress });
    K_RESULT_OK
}

unsafe extern "system" fn handler_progress_finish(this: *mut c_void, id: u64) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_progress);
    let _ = handler.progress_sender.send(ProgressEvent::Finished { id });
    K_RESULT_OK
}

static HANDLER_UNIT_VTABLE: IUnitHandlerVtable = IUnitHandlerVtable {
    query_interface: handler_unit_query_interface,
    add_ref: handler_unit_add_ref,
    release: handler_unit_release,
    notify_unit_selection: handler_unit_notify_selection,
    notify_program_list_change: handler_unit_notify_program_list,
};

unsafe extern "system" fn handler_unit_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler_unit_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler_unit_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler_unit_notify_selection(this: *mut c_void, unit_id: i32) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_unit);
    let _ = handler.unit_sender.send(UnitEvent::UnitSelected(unit_id));
    K_RESULT_OK
}

unsafe extern "system" fn handler_unit_notify_program_list(
    this: *mut c_void,
    list_id: i32,
    program_index: i32,
) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_unit);
    let _ = handler.unit_sender.send(UnitEvent::ProgramListChanged {
        list_id,
        program_index,
    });
    K_RESULT_OK
}

static HANDLER_UNIT2_VTABLE: IUnitHandler2Vtable = IUnitHandler2Vtable {
    query_interface: handler_unit2_query_interface,
    add_ref: handler_unit2_add_ref,
    release: handler_unit2_release,
    notify_unit_by_bus_change: handler_unit2_notify_by_bus,
};

unsafe extern "system" fn handler_unit2_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit2) as *mut c_void;
    handler_query_interface(parent, iid, obj)
}

unsafe extern "system" fn handler_unit2_add_ref(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit2) as *mut c_void;
    handler_add_ref(parent)
}

unsafe extern "system" fn handler_unit2_release(this: *mut c_void) -> u32 {
    let parent = container_of!(this, ComponentHandler, vtable_unit2) as *mut c_void;
    handler_release(parent)
}

unsafe extern "system" fn handler_unit2_notify_by_bus(this: *mut c_void) -> i32 {
    let handler = &*container_of!(this, ComponentHandler, vtable_unit2);
    let _ = handler.unit_sender.send(UnitEvent::UnitByBusChanged);
    K_RESULT_OK
}
