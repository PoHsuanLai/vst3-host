//! IBStream COM implementation.

use std::ffi::c_void;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use crate::ffi::{
    IBStreamVtable, K_IB_SEEK_CUR, K_IB_SEEK_END, K_IB_SEEK_SET, K_NOT_IMPLEMENTED, K_RESULT_OK,
    IID_IBSTREAM,
};


/// IBStream COM implementation for state serialization.
///
/// This wraps a byte buffer and implements the VST3 stream interface
/// for reading/writing plugin state.
#[repr(C)]
pub struct BStream {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IBStreamVtable,
    ref_count: AtomicU32,
    cursor: Mutex<Cursor<Vec<u8>>>,
}

// Safety: BStream only contains thread-safe types
unsafe impl Send for BStream {}
unsafe impl Sync for BStream {}

impl BStream {
    /// Create a new empty stream for writing.
    pub fn new() -> Box<Self> {
        Box::new(BStream {
            vtable: &BSTREAM_VTABLE,
            ref_count: AtomicU32::new(1),
            cursor: Mutex::new(Cursor::new(Vec::new())),
        })
    }

    /// Create a stream from existing data for reading.
    pub fn from_data(data: Vec<u8>) -> Box<Self> {
        Box::new(BStream {
            vtable: &BSTREAM_VTABLE,
            ref_count: AtomicU32::new(1),
            cursor: Mutex::new(Cursor::new(data)),
        })
    }

    /// Get the underlying data.
    pub fn into_data(self) -> Vec<u8> {
        self.cursor.into_inner().into_inner()
    }

    /// Get a copy of the data.
    pub fn data(&self) -> Vec<u8> {
        self.cursor.lock().get_ref().clone()
    }

    /// Get a raw pointer suitable for passing to VST3 APIs.
    pub fn as_ptr(&mut self) -> *mut c_void {
        self as *mut BStream as *mut c_void
    }
}

impl Default for BStream {
    fn default() -> Self {
        BStream {
            vtable: &BSTREAM_VTABLE,
            ref_count: AtomicU32::new(1),
            cursor: Mutex::new(Cursor::new(Vec::new())),
        }
    }
}


static BSTREAM_VTABLE: IBStreamVtable = IBStreamVtable {
    query_interface: stream_query_interface,
    add_ref: stream_add_ref,
    release: stream_release,
    read: stream_read,
    write: stream_write,
    seek: stream_seek,
    tell: stream_tell,
};

unsafe extern "system" fn stream_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    let iid_ref = &*iid;
    if *iid_ref == IID_IBSTREAM {
        *obj = this;
        stream_add_ref(this);
        return K_RESULT_OK;
    }
    *obj = std::ptr::null_mut();
    K_NOT_IMPLEMENTED
}

unsafe extern "system" fn stream_add_ref(this: *mut c_void) -> u32 {
    let stream = &*(this as *const BStream);
    stream.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn stream_release(this: *mut c_void) -> u32 {
    let stream = &*(this as *const BStream);
    let count = stream.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        let _ = Box::from_raw(this as *mut BStream);
    }
    count
}

unsafe extern "system" fn stream_read(
    this: *mut c_void,
    buffer: *mut c_void,
    num_bytes: i32,
    num_bytes_read: *mut i32,
) -> i32 {
    let stream = &*(this as *const BStream);
    let mut cursor = stream.cursor.lock();

    let buf_slice = std::slice::from_raw_parts_mut(buffer as *mut u8, num_bytes as usize);
    match cursor.read(buf_slice) {
        Ok(n) => {
            if !num_bytes_read.is_null() {
                *num_bytes_read = n as i32;
            }
            K_RESULT_OK
        }
        Err(_) => K_NOT_IMPLEMENTED,
    }
}

unsafe extern "system" fn stream_write(
    this: *mut c_void,
    buffer: *const c_void,
    num_bytes: i32,
    num_bytes_written: *mut i32,
) -> i32 {
    let stream = &*(this as *const BStream);
    let mut cursor = stream.cursor.lock();

    let buf_slice = std::slice::from_raw_parts(buffer as *const u8, num_bytes as usize);
    match cursor.write(buf_slice) {
        Ok(n) => {
            if !num_bytes_written.is_null() {
                *num_bytes_written = n as i32;
            }
            K_RESULT_OK
        }
        Err(_) => K_NOT_IMPLEMENTED,
    }
}

unsafe extern "system" fn stream_seek(
    this: *mut c_void,
    pos: i64,
    mode: i32,
    result: *mut i64,
) -> i32 {
    let stream = &*(this as *const BStream);
    let mut cursor = stream.cursor.lock();

    let seek_from = match mode {
        K_IB_SEEK_SET => SeekFrom::Start(pos as u64),
        K_IB_SEEK_CUR => SeekFrom::Current(pos),
        K_IB_SEEK_END => SeekFrom::End(pos),
        _ => return K_NOT_IMPLEMENTED,
    };

    match cursor.seek(seek_from) {
        Ok(new_pos) => {
            if !result.is_null() {
                *result = new_pos as i64;
            }
            K_RESULT_OK
        }
        Err(_) => K_NOT_IMPLEMENTED,
    }
}

unsafe extern "system" fn stream_tell(this: *mut c_void, pos: *mut i64) -> i32 {
    let stream = &*(this as *const BStream);
    let cursor = stream.cursor.lock();

    if !pos.is_null() {
        *pos = cursor.position() as i64;
    }
    K_RESULT_OK
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_write_read() {
        let mut stream = BStream::new();

        // Write some data
        let data = b"Hello, VST3!";
        let ptr = stream.as_ptr();
        unsafe {
            let mut written = 0i32;
            let result = stream_write(
                ptr,
                data.as_ptr() as *const c_void,
                data.len() as i32,
                &mut written,
            );
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(written, data.len() as i32);
        }

        // Seek back to start
        unsafe {
            let mut new_pos = 0i64;
            let result = stream_seek(ptr, 0, K_IB_SEEK_SET, &mut new_pos);
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(new_pos, 0);
        }

        // Read it back
        let mut buffer = [0u8; 32];
        unsafe {
            let mut bytes_read = 0i32;
            let result = stream_read(
                ptr,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as i32,
                &mut bytes_read,
            );
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(bytes_read, data.len() as i32);
            assert_eq!(&buffer[..data.len()], data);
        }
    }

    #[test]
    fn test_stream_from_data() {
        let data = vec![1, 2, 3, 4, 5];
        let stream = BStream::from_data(data.clone());
        assert_eq!(stream.data(), data);
    }
}
