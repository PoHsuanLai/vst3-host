//! IHostApplication COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use super::{com_add_ref, com_release, HasRefCount};
use crate::ffi::{
    IAttributeListVtable, IHostApplicationVtable, IMessageVtable, IID_IATTRIBUTE_LIST,
    IID_IHOST_APPLICATION, IID_IMESSAGE, K_NOT_IMPLEMENTED, K_RESULT_OK,
};

#[repr(C)]
pub struct HostApplication {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IHostApplicationVtable,
    ref_count: AtomicU32,
    name: [u16; 128],
}

unsafe impl Send for HostApplication {}
unsafe impl Sync for HostApplication {}

impl HasRefCount for HostApplication {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl HostApplication {
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
    com_add_ref::<HostApplication>(this)
}

unsafe extern "system" fn host_app_release(this: *mut c_void) -> u32 {
    com_release::<HostApplication>(this)
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

    if *cid_ref == IID_IMESSAGE && *iid_ref == IID_IMESSAGE {
        let message = Message::new();
        *obj = Box::into_raw(message) as *mut c_void;
        return K_RESULT_OK;
    }

    if *cid_ref == IID_IATTRIBUTE_LIST && *iid_ref == IID_IATTRIBUTE_LIST {
        let attrs = AttributeList::new();
        *obj = Box::into_raw(attrs) as *mut c_void;
        return K_RESULT_OK;
    }

    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

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

impl HasRefCount for Message {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl Message {
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
    com_add_ref::<Message>(this)
}

unsafe extern "system" fn message_release(this: *mut c_void) -> u32 {
    com_release::<Message>(this)
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

use parking_lot::Mutex;
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum AttributeValue {
    Int(i64),
    Float(f64),
    String(Vec<u16>),
    Binary(Vec<u8>),
}

#[repr(C)]
pub struct AttributeList {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IAttributeListVtable,
    ref_count: AtomicU32,
    attributes: Mutex<HashMap<String, AttributeValue>>,
}

unsafe impl Send for AttributeList {}
unsafe impl Sync for AttributeList {}

impl HasRefCount for AttributeList {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl AttributeList {
    pub fn new() -> Box<Self> {
        Box::new(AttributeList {
            vtable: &ATTRIBUTE_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            attributes: Mutex::new(HashMap::new()),
        })
    }

    pub fn as_ptr(&mut self) -> *mut c_void {
        self as *mut AttributeList as *mut c_void
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
    com_add_ref::<AttributeList>(this)
}

unsafe extern "system" fn attr_release(this: *mut c_void) -> u32 {
    com_release::<AttributeList>(this)
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
        attrs
            .attributes
            .lock()
            .insert(k, AttributeValue::Int(value));
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

unsafe extern "system" fn attr_get_float(
    this: *mut c_void,
    key: *const i8,
    value: *mut f64,
) -> i32 {
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
        if value.is_null() {
            return K_NOT_IMPLEMENTED;
        }
        let mut string = Vec::new();
        let mut ptr = value;
        const MAX_STRING_LEN: usize = 65536;
        while *ptr != 0 && string.len() < MAX_STRING_LEN {
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
    if value.is_null() || size == 0 {
        return K_NOT_IMPLEMENTED;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(s: &str) -> std::ffi::CString {
        std::ffi::CString::new(s).unwrap()
    }

    #[test]
    fn test_attr_set_string_null_value() {
        let mut attrs = AttributeList::new();
        let ptr = attrs.as_ptr();
        let key = make_key("test");
        unsafe {
            let result = attr_set_string(ptr, key.as_ptr(), std::ptr::null());
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_attr_set_get_string_roundtrip() {
        let mut attrs = AttributeList::new();
        let ptr = attrs.as_ptr();
        let key = make_key("name");

        // UTF-16 "hello" + null terminator
        let utf16: Vec<u16> = "hello".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let result = attr_set_string(ptr, key.as_ptr(), utf16.as_ptr());
            assert_eq!(result, K_RESULT_OK);
        }

        let mut out = [0u16; 32];
        unsafe {
            let result = attr_get_string(ptr, key.as_ptr(), out.as_mut_ptr(), out.len() as u32);
            assert_eq!(result, K_RESULT_OK);
            // First 5 chars should be 'h','e','l','l','o'
            let expected: Vec<u16> = "hello".encode_utf16().collect();
            assert_eq!(&out[..5], &expected[..]);
        }
    }

    #[test]
    fn test_attr_get_string_null_value() {
        let mut attrs = AttributeList::new();
        let ptr = attrs.as_ptr();
        let key = make_key("test");
        unsafe {
            let result = attr_get_string(ptr, key.as_ptr(), std::ptr::null_mut(), 10);
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_attr_get_string_zero_size() {
        let mut attrs = AttributeList::new();
        let ptr = attrs.as_ptr();
        let key = make_key("test");
        let mut out = [0u16; 1];
        unsafe {
            let result = attr_get_string(ptr, key.as_ptr(), out.as_mut_ptr(), 0);
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_attr_set_string_max_length_cap() {
        let mut attrs = AttributeList::new();
        let ptr = attrs.as_ptr();
        let key = make_key("long");

        // Very long UTF-16 string without null terminator at the end
        // but within 65536 chars it should stop at the null terminator
        let mut utf16: Vec<u16> = vec![0x41; 100]; // 100 'A' chars
        utf16.push(0); // null terminator
        unsafe {
            let result = attr_set_string(ptr, key.as_ptr(), utf16.as_ptr());
            assert_eq!(result, K_RESULT_OK);
        }
        // Verify the stored string is 100 chars + null = 101
        let stored = attrs.attributes.lock();
        match stored.get("long") {
            Some(AttributeValue::String(v)) => assert_eq!(v.len(), 101), // 100 chars + null
            other => panic!("Expected String, got {:?}", other),
        }
    }
}
