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

use super::bundle::find_library_path;

/// A loaded VST3 library with access to the plugin factory.
///
/// This struct manages the lifetime of a loaded VST3 shared library
/// and provides access to the plugin factory for creating instances.
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
    /// Load a VST3 library from a bundle path.
    ///
    /// # Arguments
    ///
    /// * `bundle_path` - Path to the `.vst3` bundle directory or library file
    ///
    /// # Example
    ///
    /// ```ignore
    /// let library = Vst3Library::load("/Library/Audio/Plug-Ins/VST3/MyPlugin.vst3")?;
    /// println!("Found {} plugin classes", library.count_classes());
    /// ```
    pub fn load(bundle_path: &Path) -> Result<Arc<Self>> {
        // Locate the actual library file within the bundle
        let lib_path = find_library_path(bundle_path)?;

        // Load the shared library
        let library = unsafe {
            Library::new(&lib_path).map_err(|e| Vst3Error::LoadFailed {
                path: lib_path.clone(),
                stage: LoadStage::Opening,
                reason: e.to_string(),
            })?
        };

        // Get the factory function
        let get_factory: libloading::Symbol<GetPluginFactoryFn> = unsafe {
            library
                .get(b"GetPluginFactory\0")
                .map_err(|e| Vst3Error::LoadFailed {
                    path: lib_path.clone(),
                    stage: LoadStage::Factory,
                    reason: format!("Missing GetPluginFactory symbol: {}", e),
                })?
        };

        // Call the factory function
        let factory = unsafe { get_factory() };
        if factory.is_null() {
            return Err(Vst3Error::LoadFailed {
                path: lib_path,
                stage: LoadStage::Factory,
                reason: "GetPluginFactory returned null".to_string(),
            });
        }

        // Get vtable from the factory object (first pointer is vtable)
        let vtable = unsafe { *(factory as *const *const IPluginFactoryVtable) };

        Ok(Arc::new(Self {
            _library: library,
            factory,
            vtable,
        }))
    }

    /// Get factory information (vendor, URL, email).
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

    /// Get the number of plugin classes in this library.
    pub fn count_classes(&self) -> i32 {
        unsafe { ((*self.vtable).count_classes)(self.factory) }
    }

    /// Get information about a plugin class at the given index.
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

    /// Create an instance of a plugin class.
    ///
    /// This is typically called internally by `Vst3Instance::load`.
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
        // Release the factory
        unsafe {
            ((*self.vtable).release)(self.factory);
        }
    }
}

/// Factory information from a VST3 library.
#[derive(Debug, Clone)]
pub struct FactoryInfo {
    /// Plugin vendor name.
    pub vendor: String,
    /// Vendor URL.
    pub url: String,
    /// Vendor email.
    pub email: String,
}

/// Information about a plugin class.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Class ID (GUID).
    pub cid: [u8; 16],
    /// Category (e.g., "Audio Module Class").
    pub category: String,
    /// Class name.
    pub name: String,
}
