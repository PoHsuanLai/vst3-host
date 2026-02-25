//! IBStream COM implementation.

use std::ffi::c_void;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::sync::atomic::AtomicU32;

use parking_lot::Mutex;

use super::{com_add_ref, com_release, HasRefCount};
use crate::ffi::{
    IBStreamVtable, IID_IBSTREAM, K_IB_SEEK_CUR, K_IB_SEEK_END, K_IB_SEEK_SET, K_NOT_IMPLEMENTED,
    K_RESULT_OK,
};

#[repr(C)]
pub struct BStream {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IBStreamVtable,
    ref_count: AtomicU32,
    cursor: Mutex<Cursor<Vec<u8>>>,
}

unsafe impl Send for BStream {}
unsafe impl Sync for BStream {}

impl HasRefCount for BStream {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl BStream {
    pub fn new() -> Box<Self> {
        Box::new(BStream {
            vtable: &BSTREAM_VTABLE,
            ref_count: AtomicU32::new(1),
            cursor: Mutex::new(Cursor::new(Vec::new())),
        })
    }

    pub fn from_data(data: Vec<u8>) -> Box<Self> {
        Box::new(BStream {
            vtable: &BSTREAM_VTABLE,
            ref_count: AtomicU32::new(1),
            cursor: Mutex::new(Cursor::new(data)),
        })
    }

    pub fn into_data(self) -> Vec<u8> {
        self.cursor.into_inner().into_inner()
    }

    pub fn data(&self) -> Vec<u8> {
        self.cursor.lock().get_ref().clone()
    }

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
    com_add_ref::<BStream>(this)
}

unsafe extern "system" fn stream_release(this: *mut c_void) -> u32 {
    com_release::<BStream>(this)
}

unsafe extern "system" fn stream_read(
    this: *mut c_void,
    buffer: *mut c_void,
    num_bytes: i32,
    num_bytes_read: *mut i32,
) -> i32 {
    let stream = &*(this as *const BStream);
    let mut cursor = stream.cursor.lock();

    if num_bytes <= 0 || buffer.is_null() {
        if !num_bytes_read.is_null() {
            *num_bytes_read = 0;
        }
        return if num_bytes == 0 {
            K_RESULT_OK
        } else {
            K_NOT_IMPLEMENTED
        };
    }
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

    if num_bytes <= 0 || buffer.is_null() {
        if !num_bytes_written.is_null() {
            *num_bytes_written = 0;
        }
        return if num_bytes == 0 {
            K_RESULT_OK
        } else {
            K_NOT_IMPLEMENTED
        };
    }
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

        unsafe {
            let mut new_pos = 0i64;
            let result = stream_seek(ptr, 0, K_IB_SEEK_SET, &mut new_pos);
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(new_pos, 0);
        }

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

    #[test]
    fn test_stream_read_negative_num_bytes() {
        let mut stream = BStream::from_data(vec![1, 2, 3]);
        let ptr = stream.as_ptr();
        let mut buf = [0u8; 8];
        let mut bytes_read = 99i32;
        unsafe {
            let result = stream_read(ptr, buf.as_mut_ptr() as *mut c_void, -1, &mut bytes_read);
            assert_ne!(result, K_RESULT_OK);
            assert_eq!(bytes_read, 0);
        }
    }

    #[test]
    fn test_stream_read_zero_num_bytes() {
        let mut stream = BStream::from_data(vec![1, 2, 3]);
        let ptr = stream.as_ptr();
        let mut buf = [0u8; 8];
        let mut bytes_read = 99i32;
        unsafe {
            let result = stream_read(ptr, buf.as_mut_ptr() as *mut c_void, 0, &mut bytes_read);
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(bytes_read, 0);
        }
    }

    #[test]
    fn test_stream_read_null_buffer() {
        let mut stream = BStream::from_data(vec![1, 2, 3]);
        let ptr = stream.as_ptr();
        let mut bytes_read = 99i32;
        unsafe {
            let result = stream_read(ptr, std::ptr::null_mut(), 10, &mut bytes_read);
            assert_ne!(result, K_RESULT_OK);
            assert_eq!(bytes_read, 0);
        }
    }

    #[test]
    fn test_stream_write_negative_num_bytes() {
        let mut stream = BStream::new();
        let ptr = stream.as_ptr();
        let data = [1u8, 2, 3];
        let mut bytes_written = 99i32;
        unsafe {
            let result = stream_write(ptr, data.as_ptr() as *const c_void, -5, &mut bytes_written);
            assert_ne!(result, K_RESULT_OK);
            assert_eq!(bytes_written, 0);
        }
        // Stream should still be empty
        assert!(stream.data().is_empty());
    }

    #[test]
    fn test_stream_write_zero_num_bytes() {
        let mut stream = BStream::new();
        let ptr = stream.as_ptr();
        let data = [1u8, 2, 3];
        let mut bytes_written = 99i32;
        unsafe {
            let result = stream_write(ptr, data.as_ptr() as *const c_void, 0, &mut bytes_written);
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(bytes_written, 0);
        }
    }

    #[test]
    fn test_stream_write_null_buffer() {
        let mut stream = BStream::new();
        let ptr = stream.as_ptr();
        let mut bytes_written = 99i32;
        unsafe {
            let result = stream_write(ptr, std::ptr::null(), 10, &mut bytes_written);
            assert_ne!(result, K_RESULT_OK);
            assert_eq!(bytes_written, 0);
        }
    }

    #[test]
    fn test_stream_read_null_bytes_read_pointer() {
        let mut stream = BStream::from_data(vec![1, 2, 3]);
        let ptr = stream.as_ptr();
        let mut buf = [0u8; 8];
        unsafe {
            // null num_bytes_read pointer should not crash
            let result = stream_read(
                ptr,
                buf.as_mut_ptr() as *mut c_void,
                3,
                std::ptr::null_mut(),
            );
            assert_eq!(result, K_RESULT_OK);
            assert_eq!(&buf[..3], &[1, 2, 3]);
        }
    }
}
