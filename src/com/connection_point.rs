//! IConnectionPoint COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use parking_lot::Mutex;

use super::{com_add_ref, com_release, HasRefCount};

type MessageCallback = Mutex<Option<Box<dyn Fn(&[u8]) + Send + Sync>>>;

use crate::ffi::{
    IConnectionPointVtable, IMessageVtable, IID_ICONNECTION_POINT, K_NOT_IMPLEMENTED, K_RESULT_OK,
};

#[repr(C)]
pub struct ConnectionPoint {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IConnectionPointVtable,
    ref_count: AtomicU32,
    connected: Mutex<Option<*mut c_void>>,
    message_callback: MessageCallback,
}

unsafe impl Send for ConnectionPoint {}
unsafe impl Sync for ConnectionPoint {}

impl HasRefCount for ConnectionPoint {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl ConnectionPoint {
    pub fn new() -> Box<Self> {
        Box::new(ConnectionPoint {
            vtable: &CONNECTION_POINT_VTABLE,
            ref_count: AtomicU32::new(1),
            connected: Mutex::new(None),
            message_callback: Mutex::new(None),
        })
    }

    pub fn set_message_callback<F>(&self, callback: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        *self.message_callback.lock() = Some(Box::new(callback));
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const ConnectionPoint as *mut c_void
    }

    pub fn is_connected(&self) -> bool {
        self.connected.lock().is_some()
    }

    /// # Safety
    ///
    /// `message` must be a valid IMessage pointer.
    pub unsafe fn send_message(&self, message: *mut c_void) -> i32 {
        let connected = *self.connected.lock();
        if let Some(point) = connected {
            let vtable = *(point as *const *const IConnectionPointVtable);
            ((*vtable).notify)(point, message)
        } else {
            K_NOT_IMPLEMENTED
        }
    }
}

impl Default for ConnectionPoint {
    fn default() -> Self {
        ConnectionPoint {
            vtable: &CONNECTION_POINT_VTABLE,
            ref_count: AtomicU32::new(1),
            connected: Mutex::new(None),
            message_callback: Mutex::new(None),
        }
    }
}

static CONNECTION_POINT_VTABLE: IConnectionPointVtable = IConnectionPointVtable {
    query_interface: conn_query_interface,
    add_ref: conn_add_ref,
    release: conn_release,
    connect: conn_connect,
    disconnect: conn_disconnect,
    notify: conn_notify,
};

unsafe extern "system" fn conn_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_ICONNECTION_POINT {
        *obj = this;
        conn_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn conn_add_ref(this: *mut c_void) -> u32 {
    com_add_ref::<ConnectionPoint>(this)
}

unsafe extern "system" fn conn_release(this: *mut c_void) -> u32 {
    com_release::<ConnectionPoint>(this)
}

unsafe extern "system" fn conn_connect(this: *mut c_void, other: *mut c_void) -> i32 {
    let point = &*(this as *const ConnectionPoint);

    if !other.is_null() {
        let vtable = *(other as *const *const IConnectionPointVtable);
        ((*vtable).add_ref)(other);
    }

    {
        let mut connected = point.connected.lock();
        if let Some(prev) = *connected {
            let vtable = *(prev as *const *const IConnectionPointVtable);
            ((*vtable).release)(prev);
        }
        *connected = if other.is_null() { None } else { Some(other) };
    }

    K_RESULT_OK
}

unsafe extern "system" fn conn_disconnect(this: *mut c_void, other: *mut c_void) -> i32 {
    let point = &*(this as *const ConnectionPoint);

    let mut connected = point.connected.lock();
    if let Some(current) = *connected {
        if current == other || other.is_null() {
            let vtable = *(current as *const *const IConnectionPointVtable);
            ((*vtable).release)(current);
            *connected = None;
            return K_RESULT_OK;
        }
    }

    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn conn_notify(this: *mut c_void, message: *mut c_void) -> i32 {
    let point = &*(this as *const ConnectionPoint);

    if !message.is_null() {
        let vtable = *(message as *const *const IMessageVtable);
        let _message_id = ((*vtable).get_message_id)(message);

        if let Some(ref callback) = *point.message_callback.lock() {
            let attrs = ((*vtable).get_attributes)(message);
            if !attrs.is_null() {
                let data_key = c"data".as_ptr();
                let mut data_ptr: *const c_void = std::ptr::null();
                let mut data_size: u32 = 0;

                use crate::ffi::IAttributeListVtable;
                let attr_vtable = *(attrs as *const *const IAttributeListVtable);
                let result =
                    ((*attr_vtable).get_binary)(attrs, data_key, &mut data_ptr, &mut data_size);

                if result == K_RESULT_OK && !data_ptr.is_null() && data_size > 0 {
                    let data_slice =
                        std::slice::from_raw_parts(data_ptr as *const u8, data_size as usize);
                    callback(data_slice);
                }
            }
        }
    }

    K_RESULT_OK
}
