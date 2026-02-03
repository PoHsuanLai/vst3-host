//! VST3 COM interface vtable definitions.
//!
//! These structs define the virtual function tables for VST3 COM interfaces.
//! They are used for calling methods on plugin objects.

use std::ffi::c_void;

use super::structs::{PClassInfo, PFactoryInfo, ProcessData, ProcessSetup, ViewRect};

// IUnknown (Base COM Interface)

/// IUnknown vtable - base interface for all COM objects.
#[repr(C)]
pub struct IUnknownVtable {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
}


/// IPluginFactory vtable - creates plugin instances.
#[repr(C)]
pub struct IPluginFactoryVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPluginFactory
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


/// IPluginBase vtable - base interface for plugin components.
#[repr(C)]
pub struct IPluginBaseVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPluginBase
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
}


/// IComponent vtable - main plugin component interface.
#[repr(C)]
pub struct IComponentVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPluginBase
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
    // IComponent
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


/// IAudioProcessor vtable - audio processing interface.
#[repr(C)]
pub struct IAudioProcessorVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IAudioProcessor
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


/// IEditController vtable - parameter and UI controller interface.
#[repr(C)]
pub struct IEditControllerVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPluginBase
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    pub terminate: unsafe extern "system" fn(*mut c_void) -> i32,
    // IEditController
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


/// IPlugView vtable - plugin GUI window interface.
#[repr(C)]
pub struct IPlugViewVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPlugView
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


/// IEventList vtable - MIDI event list interface.
#[repr(C)]
pub struct IEventListVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IEventList
    pub get_event_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_event: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    pub add_event: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
}


/// IParamValueQueue vtable - parameter automation queue interface.
#[repr(C)]
pub struct IParamValueQueueVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IParamValueQueue
    pub get_parameter_id: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_point_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_point: unsafe extern "system" fn(*mut c_void, i32, *mut i32, *mut f64) -> i32,
    pub add_point: unsafe extern "system" fn(*mut c_void, i32, f64, *mut i32) -> i32,
}


/// IParameterChanges vtable - collection of parameter automation queues.
#[repr(C)]
pub struct IParameterChangesVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IParameterChanges
    pub get_parameter_count: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_parameter_data: unsafe extern "system" fn(*mut c_void, i32) -> *mut c_void,
    pub add_parameter_data:
        unsafe extern "system" fn(*mut c_void, *const u32, *mut i32) -> *mut c_void,
}


/// IHostApplication vtable - host application interface provided to plugins.
#[repr(C)]
pub struct IHostApplicationVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IHostApplication
    /// Get the host application name (String128 = 128 UTF-16 chars)
    pub get_name: unsafe extern "system" fn(*mut c_void, *mut [u16; 128]) -> i32,
    /// Create a host object (e.g., IMessage)
    pub create_instance:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *const [u8; 16], *mut *mut c_void) -> i32,
}


/// IComponentHandler vtable - host callback interface for edit controller.
#[repr(C)]
pub struct IComponentHandlerVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IComponentHandler
    /// Called before a performEdit (e.g., on mouse-click-down)
    pub begin_edit: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    /// Called between beginEdit and endEdit to inform about parameter changes
    pub perform_edit: unsafe extern "system" fn(*mut c_void, u32, f64) -> i32,
    /// Called after a performEdit (e.g., on mouse-click-up)
    pub end_edit: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    /// Instructs host to restart the component (see RestartFlags)
    pub restart_component: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

// IComponentHandler2

/// IComponentHandler2 vtable - extended host callback interface.
#[repr(C)]
pub struct IComponentHandler2Vtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IComponentHandler2
    /// Mark the plug-in state as dirty
    pub set_dirty: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    /// Request the host to open the plug-in editor
    pub request_open_editor: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    /// Start a group of parameter edits (for automation grouping)
    pub start_group_edit: unsafe extern "system" fn(*mut c_void) -> i32,
    /// Finish a group of parameter edits
    pub finish_group_edit: unsafe extern "system" fn(*mut c_void) -> i32,
}

// IComponentHandler3

/// IComponentHandler3 vtable - context menu support.
#[repr(C)]
pub struct IComponentHandler3Vtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IComponentHandler3
    /// Create a context menu for a parameter
    pub create_context_menu:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *const u32) -> *mut c_void,
}


/// IComponentHandlerBusActivation vtable - bus activation callbacks.
#[repr(C)]
pub struct IComponentHandlerBusActivationVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IComponentHandlerBusActivation
    /// Request host to activate/deactivate a bus
    pub request_bus_activation:
        unsafe extern "system" fn(*mut c_void, i32, i32, i32, u8) -> i32,
}


/// IProgress vtable - progress reporting interface.
#[repr(C)]
pub struct IProgressVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IProgress
    /// Start a new progress (returns progress ID)
    pub start: unsafe extern "system" fn(*mut c_void, u32, *const u16, *mut u64) -> i32,
    /// Update progress (0.0 to 1.0)
    pub update: unsafe extern "system" fn(*mut c_void, u64, f64) -> i32,
    /// Finish a progress
    pub finish: unsafe extern "system" fn(*mut c_void, u64) -> i32,
}


/// IBStream vtable - binary stream interface for state serialization.
#[repr(C)]
pub struct IBStreamVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IBStream
    /// Read data from stream
    pub read: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut i32) -> i32,
    /// Write data to stream
    pub write: unsafe extern "system" fn(*mut c_void, *const c_void, i32, *mut i32) -> i32,
    /// Seek to position (mode: 0=set, 1=cur, 2=end)
    pub seek: unsafe extern "system" fn(*mut c_void, i64, i32, *mut i64) -> i32,
    /// Get current position
    pub tell: unsafe extern "system" fn(*mut c_void, *mut i64) -> i32,
}


/// IConnectionPoint vtable - component connection interface.
#[repr(C)]
pub struct IConnectionPointVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IConnectionPoint
    /// Connect to another connection point
    pub connect: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    /// Disconnect from connection point
    pub disconnect: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    /// Notify about something
    pub notify: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
}


/// IMessage vtable - message interface for processor/controller communication.
#[repr(C)]
pub struct IMessageVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IMessage
    /// Get message ID
    pub get_message_id: unsafe extern "system" fn(*mut c_void) -> *const i8,
    /// Set message ID
    pub set_message_id: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    /// Get attribute list
    pub get_attributes: unsafe extern "system" fn(*mut c_void) -> *mut c_void,
}


/// IAttributeList vtable - key-value attribute storage.
#[repr(C)]
pub struct IAttributeListVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IAttributeList
    /// Set integer attribute
    pub set_int: unsafe extern "system" fn(*mut c_void, *const i8, i64) -> i32,
    /// Get integer attribute
    pub get_int: unsafe extern "system" fn(*mut c_void, *const i8, *mut i64) -> i32,
    /// Set float attribute
    pub set_float: unsafe extern "system" fn(*mut c_void, *const i8, f64) -> i32,
    /// Get float attribute
    pub get_float: unsafe extern "system" fn(*mut c_void, *const i8, *mut f64) -> i32,
    /// Set string attribute (UTF-16)
    pub set_string: unsafe extern "system" fn(*mut c_void, *const i8, *const u16) -> i32,
    /// Get string attribute (UTF-16)
    pub get_string: unsafe extern "system" fn(*mut c_void, *const i8, *mut u16, u32) -> i32,
    /// Set binary attribute
    pub set_binary: unsafe extern "system" fn(*mut c_void, *const i8, *const c_void, u32) -> i32,
    /// Get binary attribute
    pub get_binary: unsafe extern "system" fn(*mut c_void, *const i8, *mut *const c_void, *mut u32) -> i32,
}


/// IUnitInfo vtable - unit/program structure interface.
#[repr(C)]
pub struct IUnitInfoVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IUnitInfo
    /// Get number of units
    pub get_unit_count: unsafe extern "system" fn(*mut c_void) -> i32,
    /// Get unit info by index
    pub get_unit_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    /// Get number of program lists
    pub get_program_list_count: unsafe extern "system" fn(*mut c_void) -> i32,
    /// Get program list info
    pub get_program_list_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    /// Get program name
    pub get_program_name: unsafe extern "system" fn(*mut c_void, i32, i32, *mut u16) -> i32,
    /// Get program info (attribute value)
    pub get_program_info:
        unsafe extern "system" fn(*mut c_void, i32, i32, *const i8, *mut u16) -> i32,
    /// Check if program has pitch names
    pub has_program_pitch_names: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    /// Get pitch name for program
    pub get_program_pitch_name:
        unsafe extern "system" fn(*mut c_void, i32, i32, i16, *mut u16) -> i32,
    /// Get selected unit
    pub get_selected_unit: unsafe extern "system" fn(*mut c_void) -> i32,
    /// Select unit by ID
    pub select_unit: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// Get unit by bus
    pub get_unit_by_bus:
        unsafe extern "system" fn(*mut c_void, i32, i32, i32, i32, *mut i32) -> i32,
    /// Set unit program data (preset)
    pub set_unit_program_data:
        unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
}


/// IUnitHandler vtable - host callback for unit changes.
#[repr(C)]
pub struct IUnitHandlerVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IUnitHandler
    /// Notify host about unit selection change
    pub notify_unit_selection: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// Notify host about program list change
    pub notify_program_list_change: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
}

// IUnitHandler2

/// IUnitHandler2 vtable - extended unit handler.
#[repr(C)]
pub struct IUnitHandler2Vtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IUnitHandler2
    /// Notify host about unit info change
    pub notify_unit_by_bus_change: unsafe extern "system" fn(*mut c_void) -> i32,
}


/// IProgramListData vtable - program list data persistence.
#[repr(C)]
pub struct IProgramListDataVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IProgramListData
    /// Check if program data is supported
    pub program_data_supported: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// Get program data
    pub get_program_data: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
    /// Set program data
    pub set_program_data: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
}


/// IUnitData vtable - unit data persistence.
#[repr(C)]
pub struct IUnitDataVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IUnitData
    /// Check if unit data is supported
    pub unit_data_supported: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// Get unit data
    pub get_unit_data: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    /// Set unit data
    pub set_unit_data: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
}


/// IMidiMapping vtable - MIDI CC to parameter mapping.
#[repr(C)]
pub struct IMidiMappingVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IMidiMapping
    /// Get MIDI controller assignment for parameter
    pub get_midi_controller_assignment:
        unsafe extern "system" fn(*mut c_void, i32, i16, i16, *mut u32) -> i32,
}


/// IMidiLearn vtable - MIDI learn functionality.
#[repr(C)]
pub struct IMidiLearnVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IMidiLearn
    /// Called on live MIDI CC input
    pub on_live_midi_controller_input:
        unsafe extern "system" fn(*mut c_void, i32, i16, i16) -> i32,
}


/// IContextMenu vtable - context menu interface.
#[repr(C)]
pub struct IContextMenuVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IContextMenu
    /// Get number of items
    pub get_item_count: unsafe extern "system" fn(*mut c_void) -> i32,
    /// Get item at index
    pub get_item: unsafe extern "system" fn(*mut c_void, i32, *mut c_void, *mut *mut c_void) -> i32,
    /// Add item to menu
    pub add_item: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
    /// Remove item from menu
    pub remove_item: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
    /// Show the menu
    pub popup: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
}


/// IContextMenuTarget vtable - context menu action target.
#[repr(C)]
pub struct IContextMenuTargetVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IContextMenuTarget
    /// Execute menu item action
    pub execute_menu_item: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

// IEditController2

/// IEditController2 vtable - extended edit controller.
#[repr(C)]
pub struct IEditController2Vtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IEditController2
    /// Set knob mode (circular, relative circular, linear)
    pub set_knob_mode: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    /// Open help documentation
    pub open_help: unsafe extern "system" fn(*mut c_void, u8) -> i32,
    /// Open about box
    pub open_about_box: unsafe extern "system" fn(*mut c_void, u8) -> i32,
}


/// IEditControllerHostEditing vtable - host-controlled parameter editing.
#[repr(C)]
pub struct IEditControllerHostEditingVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IEditControllerHostEditing
    /// Begin editing from host
    pub begin_edit_from_host: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    /// End editing from host
    pub end_edit_from_host: unsafe extern "system" fn(*mut c_void, u32) -> i32,
}

// IDataExchangeHandler (VST 3.7+)

/// Data exchange block for waveform/visualization data.
#[repr(C)]
pub struct DataExchangeBlock {
    /// Pointer to block data
    pub data: *mut c_void,
    /// Size of data in bytes
    pub size: u32,
    /// Block ID for freeing
    pub block_id: u32,
}

/// IDataExchangeHandler vtable - host-side data exchange interface.
///
/// Enables direct, thread-safe data transfer from audio processor to edit controller
/// for visualization purposes (e.g., waveform displays).
#[repr(C)]
pub struct IDataExchangeHandlerVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IDataExchangeHandler
    /// Open a data exchange queue
    pub open_queue: unsafe extern "system" fn(
        *mut c_void,      // this
        *mut c_void,      // processor
        u32,              // block_size
        u32,              // num_blocks
        u32,              // alignment
        u32,              // user_context_id
        *mut u32,         // out_queue_id
    ) -> i32,
    /// Close a data exchange queue
    pub close_queue: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    /// Lock a block for writing (called from audio thread)
    pub lock_block: unsafe extern "system" fn(*mut c_void, u32, *mut DataExchangeBlock) -> i32,
    /// Free a block after writing (called from audio thread)
    pub free_block: unsafe extern "system" fn(*mut c_void, u32, u32, u8) -> i32,
}

// IDataExchangeReceiver (VST 3.7+)

/// IDataExchangeReceiver vtable - plugin-side data exchange interface.
///
/// Implemented by plugins to receive data blocks from the audio processor.
#[repr(C)]
pub struct IDataExchangeReceiverVtable {
    // IUnknown
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IDataExchangeReceiver
    /// Called when queue is opened (returns whether to dispatch on background thread)
    pub queue_opened: unsafe extern "system" fn(*mut c_void, u32, u32, *mut u8) -> i32,
    /// Called when queue is closed
    pub queue_closed: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    /// Called when data blocks are received
    pub on_data_exchange_blocks_received: unsafe extern "system" fn(
        *mut c_void,              // this
        u32,                      // user_context_id
        u32,                      // num_blocks
        *const DataExchangeBlock, // blocks
        u8,                       // on_background_thread
    ) -> i32,
}
