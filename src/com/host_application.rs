//! IHostApplication COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::ffi::{
    IAttributeListVtable, IHostApplicationVtable, IMessageVtable, K_NOT_IMPLEMENTED, K_RESULT_OK,
    IID_IATTRIBUTE_LIST, IID_IHOST_APPLICATION, IID_IMESSAGE,
};


/// IHostApplication COM implementation.
///
/// Provides host information to plugins during initialization.
#[repr(C)]
pub struct HostApplication {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IHostApplicationVtable,
    ref_count: AtomicU32,
    name: [u16; 128],
}

// Safety: HostApplication only contains thread-safe types
unsafe impl Send for HostApplication {}
unsafe impl Sync for HostApplication {}

impl HostApplication {
    /// Create a new host application with the given name.
    pub fn new(name: &str) -> Box<Self> {
        let mut name_utf16 = [0u16; 128];
        for (i, c) in name.encode_utf16().take(127).enumerate() {
            name_utf16[i] = c;
        }

        Box::new(HostApplication {
            vtable: &HOST_APPLICATION_VTABLE,
            ref_count: AtomicU32::new(1),
            name: name_utf16,
        })
    }

    /// Get a raw pointer suitable for passing to VST3 APIs.
    pub fn as_ptr(&self) -> *mut c_void {
        self as *const HostApplication as *mut c_void
    }
}

static HOST_APPLICATION_VTABLE: IHostApplicationVtable = IHostApplicationVtable {
    query_interface: host_app_query_interface,
    add_ref: host_app_add_ref,
    release: host_app_release,
    get_name: host_app_get_name,
    create_instance: host_app_create_instance,
};

unsafe extern "system" fn host_app_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IHOST_APPLICATION {
        *obj = this;
        host_app_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn host_app_add_ref(this: *mut c_void) -> u32 {
    let app = &*(this as *const HostApplication);
    app.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn host_app_release(this: *mut c_void) -> u32 {
    let app = &*(this as *const HostApplication);
    let count = app.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut HostApplication);
    }
    count
}

unsafe extern "system" fn host_app_get_name(this: *mut c_void, name: *mut [u16; 128]) -> i32 {
    let app = &*(this as *const HostApplication);
    *name = app.name;
    K_RESULT_OK
}

unsafe extern "system" fn host_app_create_instance(
    _this: *mut c_void,
    cid: *const [u8; 16],
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let cid_ref = &*cid;
    let iid_ref = &*iid;

    // Create IMessage if requested
    if *cid_ref == IID_IMESSAGE && *iid_ref == IID_IMESSAGE {
        let message = Message::new();
        *obj = Box::into_raw(message) as *mut c_void;
        return K_RESULT_OK;
    }

    // Create IAttributeList if requested
    if *cid_ref == IID_IATTRIBUTE_LIST && *iid_ref == IID_IATTRIBUTE_LIST {
        let attrs = AttributeList::new();
        *obj = Box::into_raw(attrs) as *mut c_void;
        return K_RESULT_OK;
    }

    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}


/// IMessage COM implementation.
#[repr(C)]
pub struct Message {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IMessageVtable,
    ref_count: AtomicU32,
    message_id: Option<std::ffi::CString>,
    attributes: Box<AttributeList>,
}

unsafe impl Send for Message {}
unsafe impl Sync for Message {}

impl Message {
    /// Create a new empty message.
    pub fn new() -> Box<Self> {
        Box::new(Message {
            vtable: &MESSAGE_VTABLE,
            ref_count: AtomicU32::new(1),
            message_id: None,
            attributes: AttributeList::new(),
        })
    }
}

impl Default for Message {
    fn default() -> Self {
        Message {
            vtable: &MESSAGE_VTABLE,
            ref_count: AtomicU32::new(1),
            message_id: None,
            attributes: AttributeList::new(),
        }
    }
}

static MESSAGE_VTABLE: IMessageVtable = IMessageVtable {
    query_interface: message_query_interface,
    add_ref: message_add_ref,
    release: message_release,
    get_message_id: message_get_id,
    set_message_id: message_set_id,
    get_attributes: message_get_attributes,
};

unsafe extern "system" fn message_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IMESSAGE {
        *obj = this;
        message_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn message_add_ref(this: *mut c_void) -> u32 {
    let msg = &*(this as *const Message);
    msg.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn message_release(this: *mut c_void) -> u32 {
    let msg = &*(this as *const Message);
    let count = msg.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut Message);
    }
    count
}

unsafe extern "system" fn message_get_id(this: *mut c_void) -> *const i8 {
    let msg = &*(this as *const Message);
    msg.message_id
        .as_ref()
        .map(|s| s.as_ptr())
        .unwrap_or(std::ptr::null())
}

unsafe extern "system" fn message_set_id(this: *mut c_void, id: *const i8) -> i32 {
    let msg = &mut *(this as *mut Message);
    if id.is_null() {
        msg.message_id = None;
    } else {
        let c_str = std::ffi::CStr::from_ptr(id);
        msg.message_id = Some(c_str.to_owned());
    }
    K_RESULT_OK
}

unsafe extern "system" fn message_get_attributes(this: *mut c_void) -> *mut c_void {
    let msg = &*(this as *const Message);
    &*msg.attributes as *const AttributeList as *mut c_void
}


use std::collections::HashMap;
use parking_lot::Mutex;

/// Attribute value storage.
#[derive(Clone)]
enum AttributeValue {
    Int(i64),
    Float(f64),
    String(Vec<u16>),
    Binary(Vec<u8>),
}

/// IAttributeList COM implementation.
#[repr(C)]
pub struct AttributeList {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IAttributeListVtable,
    ref_count: AtomicU32,
    attributes: Mutex<HashMap<String, AttributeValue>>,
}

unsafe impl Send for AttributeList {}
unsafe impl Sync for AttributeList {}

impl AttributeList {
    /// Create a new empty attribute list.
    pub fn new() -> Box<Self> {
        Box::new(AttributeList {
            vtable: &ATTRIBUTE_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            attributes: Mutex::new(HashMap::new()),
        })
    }
}

impl Default for AttributeList {
    fn default() -> Self {
        AttributeList {
            vtable: &ATTRIBUTE_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            attributes: Mutex::new(HashMap::new()),
        }
    }
}

static ATTRIBUTE_LIST_VTABLE: IAttributeListVtable = IAttributeListVtable {
    query_interface: attr_query_interface,
    add_ref: attr_add_ref,
    release: attr_release,
    set_int: attr_set_int,
    get_int: attr_get_int,
    set_float: attr_set_float,
    get_float: attr_get_float,
    set_string: attr_set_string,
    get_string: attr_get_string,
    set_binary: attr_set_binary,
    get_binary: attr_get_binary,
};

unsafe extern "system" fn attr_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IATTRIBUTE_LIST {
        *obj = this;
        attr_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn attr_add_ref(this: *mut c_void) -> u32 {
    let attrs = &*(this as *const AttributeList);
    attrs.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn attr_release(this: *mut c_void) -> u32 {
    let attrs = &*(this as *const AttributeList);
    let count = attrs.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut AttributeList);
    }
    count
}

fn key_from_ptr(key: *const i8) -> Option<String> {
    if key.is_null() {
        return None;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(key);
        c_str.to_str().ok().map(|s| s.to_string())
    }
}

unsafe extern "system" fn attr_set_int(this: *mut c_void, key: *const i8, value: i64) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        attrs.attributes.lock().insert(k, AttributeValue::Int(value));
        K_RESULT_OK
    } else {
        K_NOT_IMPLEMENTED
    }
}

unsafe extern "system" fn attr_get_int(this: *mut c_void, key: *const i8, value: *mut i64) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        if let Some(AttributeValue::Int(v)) = attrs.attributes.lock().get(&k) {
            *value = *v;
            return K_RESULT_OK;
        }
    }
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn attr_set_float(this: *mut c_void, key: *const i8, value: f64) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        attrs
            .attributes
            .lock()
            .insert(k, AttributeValue::Float(value));
        K_RESULT_OK
    } else {
        K_NOT_IMPLEMENTED
    }
}

unsafe extern "system" fn attr_get_float(this: *mut c_void, key: *const i8, value: *mut f64) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        if let Some(AttributeValue::Float(v)) = attrs.attributes.lock().get(&k) {
            *value = *v;
            return K_RESULT_OK;
        }
    }
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn attr_set_string(
    this: *mut c_void,
    key: *const i8,
    value: *const u16,
) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        let mut string = Vec::new();
        let mut ptr = value;
        while !ptr.is_null() && *ptr != 0 {
            string.push(*ptr);
            ptr = ptr.add(1);
        }
        string.push(0); // null terminator
        attrs
            .attributes
            .lock()
            .insert(k, AttributeValue::String(string));
        K_RESULT_OK
    } else {
        K_NOT_IMPLEMENTED
    }
}

unsafe extern "system" fn attr_get_string(
    this: *mut c_void,
    key: *const i8,
    value: *mut u16,
    size: u32,
) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        if let Some(AttributeValue::String(v)) = attrs.attributes.lock().get(&k) {
            let copy_len = std::cmp::min(v.len(), size as usize);
            std::ptr::copy_nonoverlapping(v.as_ptr(), value, copy_len);
            return K_RESULT_OK;
        }
    }
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn attr_set_binary(
    this: *mut c_void,
    key: *const i8,
    data: *const c_void,
    size: u32,
) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
        attrs
            .attributes
            .lock()
            .insert(k, AttributeValue::Binary(slice.to_vec()));
        K_RESULT_OK
    } else {
        K_NOT_IMPLEMENTED
    }
}

unsafe extern "system" fn attr_get_binary(
    this: *mut c_void,
    key: *const i8,
    data: *mut *const c_void,
    size: *mut u32,
) -> i32 {
    let attrs = &*(this as *const AttributeList);
    if let Some(k) = key_from_ptr(key) {
        if let Some(AttributeValue::Binary(v)) = attrs.attributes.lock().get(&k) {
            *data = v.as_ptr() as *const c_void;
            *size = v.len() as u32;
            return K_RESULT_OK;
        }
    }
    K_NOT_IMPLEMENTED
}
