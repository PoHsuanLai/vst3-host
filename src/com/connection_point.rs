//! IConnectionPoint COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use crate::ffi::{
    IConnectionPointVtable, IMessageVtable, K_NOT_IMPLEMENTED, K_RESULT_OK,
    IID_ICONNECTION_POINT,
};


/// IConnectionPoint COM implementation.
///
/// Enables communication between processor and controller components.
#[repr(C)]
pub struct ConnectionPoint {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IConnectionPointVtable,
    ref_count: AtomicU32,
    connected: Mutex<Option<*mut c_void>>,
    /// Callback for receiving messages
    message_callback: Mutex<Option<Box<dyn Fn(&[u8]) + Send + Sync>>>,
}

// Safety: ConnectionPoint uses thread-safe types
unsafe impl Send for ConnectionPoint {}
unsafe impl Sync for ConnectionPoint {}

impl ConnectionPoint {
    /// Create a new connection point.
    pub fn new() -> Box<Self> {
        Box::new(ConnectionPoint {
            vtable: &CONNECTION_POINT_VTABLE,
            ref_count: AtomicU32::new(1),
            connected: Mutex::new(None),
            message_callback: Mutex::new(None),
        })
    }

    /// Set a callback for receiving messages.
    pub fn set_message_callback<F>(&self, callback: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        *self.message_callback.lock() = Some(Box::new(callback));
    }

    /// Get a raw pointer suitable for passing to VST3 APIs.
    pub fn as_ptr(&self) -> *mut c_void {
        self as *const ConnectionPoint as *mut c_void
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected.lock().is_some()
    }

    /// Send a message to the connected point.
    ///
    /// # Safety
    ///
    /// The message must be a valid IMessage pointer.
    pub unsafe fn send_message(&self, message: *mut c_void) -> i32 {
        let connected = *self.connected.lock();
        if let Some(point) = connected {
            // Call notify on the connected point
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
    let point = &*(this as *const ConnectionPoint);
    point.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn conn_release(this: *mut c_void) -> u32 {
    let point = &*(this as *const ConnectionPoint);
    let count = point.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut ConnectionPoint);
    }
    count
}

unsafe extern "system" fn conn_connect(this: *mut c_void, other: *mut c_void) -> i32 {
    let point = &*(this as *const ConnectionPoint);

    // Add ref to the other point
    if !other.is_null() {
        let vtable = *(other as *const *const IConnectionPointVtable);
        ((*vtable).add_ref)(other);
    }

    // Release previous connection if any
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
            // Release the connection
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

    // Try to get message data and invoke callback
    if !message.is_null() {
        let vtable = *(message as *const *const IMessageVtable);
        let _message_id = ((*vtable).get_message_id)(message);

        // Get attributes and extract binary data if callback is set
        if let Some(ref callback) = *point.message_callback.lock() {
            let attrs = ((*vtable).get_attributes)(message);
            if !attrs.is_null() {
                // Try to get "data" attribute
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
