//! VST3 library loading and factory access.

use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

use libloading::Library;

use crate::error::{LoadStage, Result, Vst3Error};
use crate::ffi::{
    c_str_to_string, GetPluginFactoryFn, IPluginFactoryVtable, PClassInfo, PFactoryInfo,
    K_RESULT_OK,
};

pub struct Vst3Library {
    _library: Library,
    factory: *mut c_void,
    vtable: *const IPluginFactoryVtable,
}

// Safety: The library handle and factory pointer are thread-safe
// as long as we don't call factory methods from multiple threads
// without synchronization (which we don't do).
unsafe impl Send for Vst3Library {}
unsafe impl Sync for Vst3Library {}

impl Vst3Library {
    /// Load a VST3 library from a pre-resolved path to the actual binary.
    ///
    /// Bundle resolution (finding the binary inside a `.vst3` bundle) should
    /// be done by the caller before invoking this method.
    pub fn load(lib_path: &Path) -> Result<Arc<Self>> {
        let library = unsafe {
            Library::new(lib_path).map_err(|e| Vst3Error::LoadFailed {
                path: lib_path.to_path_buf(),
                stage: LoadStage::Opening,
                reason: e.to_string(),
            })?
        };

        let get_factory: libloading::Symbol<GetPluginFactoryFn> = unsafe {
            library
                .get(b"GetPluginFactory\0")
                .map_err(|e| Vst3Error::LoadFailed {
                    path: lib_path.to_path_buf(),
                    stage: LoadStage::Factory,
                    reason: format!("Missing GetPluginFactory symbol: {}", e),
                })?
        };

        let factory = unsafe { get_factory() };
        if factory.is_null() {
            return Err(Vst3Error::LoadFailed {
                path: lib_path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "GetPluginFactory returned null".to_string(),
            });
        }

        let vtable = unsafe { *(factory as *const *const IPluginFactoryVtable) };

        Ok(Arc::new(Self {
            _library: library,
            factory,
            vtable,
        }))
    }

    pub fn get_factory_info(&self) -> Option<FactoryInfo> {
        let mut info = PFactoryInfo::default();
        let result = unsafe { ((*self.vtable).get_factory_info)(self.factory, &mut info) };
        if result == K_RESULT_OK {
            Some(FactoryInfo {
                vendor: c_str_to_string(&info.vendor),
                url: c_str_to_string(&info.url),
                email: c_str_to_string(&info.email),
            })
        } else {
            None
        }
    }

    pub fn count_classes(&self) -> i32 {
        unsafe { ((*self.vtable).count_classes)(self.factory) }
    }

    pub fn get_class_info(&self, index: i32) -> Result<ClassInfo> {
        let mut info = PClassInfo::default();
        let result = unsafe { ((*self.vtable).get_class_info)(self.factory, index, &mut info) };
        if result == K_RESULT_OK {
            Ok(ClassInfo {
                cid: info.cid,
                category: c_str_to_string(&info.category),
                name: c_str_to_string(&info.name),
            })
        } else {
            Err(Vst3Error::PluginError {
                stage: LoadStage::Factory,
                code: result,
            })
        }
    }

    pub(crate) fn create_instance(&self, cid: &[u8; 16], iid: &[u8; 16]) -> Result<*mut c_void> {
        let mut obj: *mut c_void = std::ptr::null_mut();
        let result = unsafe { ((*self.vtable).create_instance)(self.factory, cid, iid, &mut obj) };
        if result == K_RESULT_OK && !obj.is_null() {
            Ok(obj)
        } else {
            Err(Vst3Error::PluginError {
                stage: LoadStage::Instantiation,
                code: result,
            })
        }
    }
}

impl Drop for Vst3Library {
    fn drop(&mut self) {
        unsafe {
            ((*self.vtable).release)(self.factory);
        }
    }
}

#[derive(Debug, Clone)]
pub struct FactoryInfo {
    pub vendor: String,
    pub url: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub cid: [u8; 16],
    pub category: String,
    pub name: String,
}
