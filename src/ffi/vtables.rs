//! VST3 COM interface vtable definitions.

use std::ffi::c_void;

use super::structs::{PClassInfo, PFactoryInfo, ProcessData, ProcessSetup, ViewRect};

#[repr(C)]
pub struct IUnknownVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
pub struct IPluginFactoryVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_factory_info: unsafe extern "system" fn(*mut c_void, *mut PFactoryInfo) -> i32,
    pub count_classes: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_class_info: unsafe extern "system" fn(*mut c_void, i32, *mut PClassInfo) -> i32,
    pub create_instance: unsafe extern "system" fn(
        *mut c_void,
        *const [u8; 16],
        *const [u8; 16],
        *mut *mut c_void,
    ) -> i32,
}

#[repr(C)]
pub struct IPluginBaseVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
pub struct IComponentVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_controller_class_id: unsafe extern "system" fn(*mut c_void, *mut [u8; 16]) -> i32,
    pub set_io_mode: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub get_bus_count: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    pub get_bus_info: unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut c_void) -> i32,
    pub get_routing_info: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
    pub activate_bus: unsafe extern "system" fn(*mut c_void, i32, i32, i32, u8) -> i32,
    pub set_active: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    pub set_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub get_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
}

#[repr(C)]
pub struct IAudioProcessorVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub set_bus_arrangements:
        unsafe extern "system" fn(*mut c_void, *mut u64, i32, *mut u64, i32) -> i32,
    pub get_bus_arrangement: unsafe extern "system" fn(*mut c_void, i32, i32, *mut u64) -> i32,
    pub can_process_sample_size: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub get_latency_samples: unsafe extern "system" fn(*mut c_void) -> u32,
    pub setup_processing: unsafe extern "system" fn(*mut c_void, *const ProcessSetup) -> i32,
    pub set_processing: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    pub process: unsafe extern "system" fn(*mut c_void, *mut ProcessData) -> i32,
    pub get_tail_samples: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
pub struct IEditControllerVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
    pub set_component_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub set_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub get_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub get_parameter_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_parameter_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub get_param_string_by_value:
        unsafe extern "system" fn(*mut c_void, u32, f64, *mut c_void) -> i32,
    pub get_param_value_by_string:
        unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut f64) -> i32,
    pub normalized_param_to_plain: unsafe extern "system" fn(*mut c_void, u32, f64) -> f64,
    pub plain_param_to_normalized: unsafe extern "system" fn(*mut c_void, u32, f64) -> f64,
    pub get_param_normalized: unsafe extern "system" fn(*mut c_void, u32) -> f64,
    pub set_param_normalized: unsafe extern "system" fn(*mut c_void, u32, f64) -> i32,
    pub set_component_handler: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub create_view: unsafe extern "system" fn(*mut c_void, *const i8) -> *mut c_void,
}

#[repr(C)]
pub struct IPlugViewVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub is_platform_type_supported: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    pub attached: unsafe extern "system" fn(*mut c_void, *mut c_void, *const i8) -> i32,
    pub removed: unsafe extern "system" fn(*mut c_void) -> i32,
    pub on_wheel: unsafe extern "system" fn(*mut c_void, f32) -> i32,
    pub on_key_down: unsafe extern "system" fn(*mut c_void, i16, i16, i16) -> i32,
    pub on_key_up: unsafe extern "system" fn(*mut c_void, i16, i16, i16) -> i32,
    pub get_size: unsafe extern "system" fn(*mut c_void, *mut ViewRect) -> i32,
    pub on_size: unsafe extern "system" fn(*mut c_void, *mut ViewRect) -> i32,
    pub on_focus: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    pub set_frame: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub can_resize: unsafe extern "system" fn(*mut c_void) -> i32,
    pub check_size_constraint: unsafe extern "system" fn(*mut c_void, *mut ViewRect) -> i32,
}

#[repr(C)]
pub struct IEventListVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_event_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_event: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub add_event: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
}

#[repr(C)]
pub struct IParamValueQueueVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_parameter_id: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_point_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_point: unsafe extern "system" fn(*mut c_void, i32, *mut i32, *mut f64) -> i32,
    pub add_point: unsafe extern "system" fn(*mut c_void, i32, f64, *mut i32) -> i32,
}

#[repr(C)]
pub struct IParameterChangesVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_parameter_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_parameter_data: unsafe extern "system" fn(*mut c_void, i32) -> *mut c_void,
    pub add_parameter_data:
        unsafe extern "system" fn(*mut c_void, *const u32, *mut i32) -> *mut c_void,
}

#[repr(C)]
pub struct IHostApplicationVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_name: unsafe extern "system" fn(*mut c_void, *mut [u16; 128]) -> i32,
    pub create_instance: unsafe extern "system" fn(
        *mut c_void,
        *const [u8; 16],
        *const [u8; 16],
        *mut *mut c_void,
    ) -> i32,
}

#[repr(C)]
pub struct IComponentHandlerVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub begin_edit: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub perform_edit: unsafe extern "system" fn(*mut c_void, u32, f64) -> i32,
    pub end_edit: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub restart_component: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

#[repr(C)]
pub struct IComponentHandler2Vtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub set_dirty: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    pub request_open_editor: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    pub start_group_edit: unsafe extern "system" fn(*mut c_void) -> i32,
    pub finish_group_edit: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
pub struct IComponentHandler3Vtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub create_context_menu:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *const u32) -> *mut c_void,
}

#[repr(C)]
pub struct IComponentHandlerBusActivationVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub request_bus_activation: unsafe extern "system" fn(*mut c_void, i32, i32, i32, u8) -> i32,
}

#[repr(C)]
pub struct IProgressVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub start: unsafe extern "system" fn(*mut c_void, u32, *const u16, *mut u64) -> i32,
    pub update: unsafe extern "system" fn(*mut c_void, u64, f64) -> i32,
    pub finish: unsafe extern "system" fn(*mut c_void, u64) -> i32,
}

#[repr(C)]
pub struct IBStreamVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub read: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut i32) -> i32,
    pub write: unsafe extern "system" fn(*mut c_void, *const c_void, i32, *mut i32) -> i32,
    /// mode: 0=set, 1=cur, 2=end
    pub seek: unsafe extern "system" fn(*mut c_void, i64, i32, *mut i64) -> i32,
    pub tell: unsafe extern "system" fn(*mut c_void, *mut i64) -> i32,
}

#[repr(C)]
pub struct IConnectionPointVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub connect: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub disconnect: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub notify: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
}

#[repr(C)]
pub struct IMessageVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_message_id: unsafe extern "system" fn(*mut c_void) -> *const i8,
    pub set_message_id: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    pub get_attributes: unsafe extern "system" fn(*mut c_void) -> *mut c_void,
}

#[repr(C)]
pub struct IAttributeListVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub set_int: unsafe extern "system" fn(*mut c_void, *const i8, i64) -> i32,
    pub get_int: unsafe extern "system" fn(*mut c_void, *const i8, *mut i64) -> i32,
    pub set_float: unsafe extern "system" fn(*mut c_void, *const i8, f64) -> i32,
    pub get_float: unsafe extern "system" fn(*mut c_void, *const i8, *mut f64) -> i32,
    pub set_string: unsafe extern "system" fn(*mut c_void, *const i8, *const u16) -> i32,
    pub get_string: unsafe extern "system" fn(*mut c_void, *const i8, *mut u16, u32) -> i32,
    pub set_binary: unsafe extern "system" fn(*mut c_void, *const i8, *const c_void, u32) -> i32,
    pub get_binary:
        unsafe extern "system" fn(*mut c_void, *const i8, *mut *const c_void, *mut u32) -> i32,
}

#[repr(C)]
pub struct IUnitInfoVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_unit_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_unit_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub get_program_list_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_program_list_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub get_program_name: unsafe extern "system" fn(*mut c_void, i32, i32, *mut u16) -> i32,
    pub get_program_info:
        unsafe extern "system" fn(*mut c_void, i32, i32, *const i8, *mut u16) -> i32,
    pub has_program_pitch_names: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    pub get_program_pitch_name:
        unsafe extern "system" fn(*mut c_void, i32, i32, i16, *mut u16) -> i32,
    pub get_selected_unit: unsafe extern "system" fn(*mut c_void) -> i32,
    pub select_unit: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub get_unit_by_bus:
        unsafe extern "system" fn(*mut c_void, i32, i32, i32, i32, *mut i32) -> i32,
    pub set_unit_program_data: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
}

#[repr(C)]
pub struct IUnitHandlerVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub notify_unit_selection: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub notify_program_list_change: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
}

#[repr(C)]
pub struct IUnitHandler2Vtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub notify_unit_by_bus_change: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
pub struct IProgramListDataVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub program_data_supported: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub get_program_data: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
    pub set_program_data: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
}

#[repr(C)]
pub struct IUnitDataVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub unit_data_supported: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub get_unit_data: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub set_unit_data: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
}

#[repr(C)]
pub struct IMidiMappingVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_midi_controller_assignment:
        unsafe extern "system" fn(*mut c_void, i32, i16, i16, *mut u32) -> i32,
}

#[repr(C)]
pub struct IMidiLearnVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub on_live_midi_controller_input: unsafe extern "system" fn(*mut c_void, i32, i16, i16) -> i32,
}

#[repr(C)]
pub struct IContextMenuVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_item_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_item: unsafe extern "system" fn(*mut c_void, i32, *mut c_void, *mut *mut c_void) -> i32,
    pub add_item: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
    pub remove_item: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
    pub popup: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
}

#[repr(C)]
pub struct IContextMenuTargetVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub execute_menu_item: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

#[repr(C)]
pub struct IEditController2Vtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub set_knob_mode: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    pub open_help: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    pub open_about_box: unsafe extern "system" fn(*mut c_void, u8) -> i32,
}

#[repr(C)]
pub struct IEditControllerHostEditingVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub begin_edit_from_host: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub end_edit_from_host: unsafe extern "system" fn(*mut c_void, u32) -> i32,
}

#[repr(C)]
pub struct DataExchangeBlock {
    pub data: *mut c_void,
    pub size: u32,
    pub block_id: u32,
}

/// VST 3.7+: thread-safe data transfer from audio processor to edit controller.
#[repr(C)]
pub struct IDataExchangeHandlerVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub open_queue:
        unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32, u32, u32, *mut u32) -> i32,
    pub close_queue: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub lock_block: unsafe extern "system" fn(*mut c_void, u32, *mut DataExchangeBlock) -> i32,
    pub free_block: unsafe extern "system" fn(*mut c_void, u32, u32, u8) -> i32,
}

/// VST 3.7+: plugin-side receiver for data blocks from the audio processor.
#[repr(C)]
pub struct IDataExchangeReceiverVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub queue_opened: unsafe extern "system" fn(*mut c_void, u32, u32, *mut u8) -> i32,
    pub queue_closed: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub on_data_exchange_blocks_received:
        unsafe extern "system" fn(*mut c_void, u32, u32, *const DataExchangeBlock, u8) -> i32,
}
