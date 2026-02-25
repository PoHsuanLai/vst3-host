//! Host-side COM interface implementations for VST3 hosting.

mod component_handler;
mod connection_point;
mod data_exchange;
mod event_list;
mod host_application;
mod param_changes;
mod param_queue;
mod progress;
mod stream;
mod unit_handler;

#[cfg(test)]
mod tests;

// Types used by instance.rs (production code)
pub use component_handler::{ComponentHandler, ParameterEditEvent, ProgressEvent, UnitEvent};
pub use event_list::EventList;
pub use host_application::HostApplication;
pub use param_changes::ParameterChangesImpl;
pub use stream::BStream;

// Types used only by COM tests
#[cfg(test)]
pub use connection_point::ConnectionPoint;
#[cfg(test)]
pub use data_exchange::DataExchangeHandler;
#[cfg(test)]
#[allow(unused_imports)]
pub use host_application::{AttributeList, Message};
#[cfg(test)]
pub use param_queue::ParamValueQueueImpl;
#[cfg(test)]
pub use progress::ProgressHandler;
#[cfg(test)]
pub use unit_handler::UnitHandler;

// ---------------------------------------------------------------------------
// Shared COM helpers
// ---------------------------------------------------------------------------

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

/// Recover a pointer to `$Parent` from a COM vtable field pointer.
///
/// In multi-interface COM objects, secondary interfaces point to inner vtable
/// fields rather than the struct base. This macro performs the reverse
/// `offset_of` arithmetic to recover the parent pointer.
macro_rules! container_of {
    ($ptr:expr, $Parent:ty, $field:ident) => {{
        ($ptr as *const u8).sub(std::mem::offset_of!($Parent, $field)) as *mut $Parent
    }};
}

pub(crate) use container_of;

/// Increment a COM reference count on a `#[repr(C)]` struct whose
/// `ref_count: AtomicU32` field lives at the same offset in every COM object.
///
/// # Safety
///
/// `this` must point to a valid `T` whose `ref_count` field is an `AtomicU32`.
pub(crate) unsafe fn com_add_ref<T: HasRefCount>(this: *mut c_void) -> u32 {
    let obj = &*(this as *const T);
    obj.ref_count().fetch_add(1, Ordering::SeqCst) + 1
}

/// Decrement a COM reference count, dropping the `Box<T>` when it hits zero.
///
/// # Safety
///
/// `this` must point to a valid, heap-allocated `T`.
pub(crate) unsafe fn com_release<T: HasRefCount>(this: *mut c_void) -> u32 {
    let obj = &*(this as *const T);
    let count = obj.ref_count().fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut T);
    }
    count
}

/// Implemented by every `#[repr(C)]` COM struct so `com_add_ref` / `com_release`
/// can access the refcount without knowing the concrete type's field layout.
pub(crate) trait HasRefCount {
    fn ref_count(&self) -> &AtomicU32;
}
