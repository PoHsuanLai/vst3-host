//! VST3 library loading and factory access.

use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

use libloading::Library;
use vst3::{ComPtr, Interface};
use vst3::Steinberg::{
    kResultOk, FIDString, IPluginFactory, IPluginFactoryTrait, PClassInfo, PFactoryInfo, TUID,
};

use crate::error::{LoadStage, Result, Vst3Error};
use crate::helpers::c_str_to_string;

type GetPluginFactoryFn = unsafe extern "system" fn() -> *mut IPluginFactory;

/// Convert a 16-byte `Guid` (the type of `Interface::IID`) to a `TUID`
/// (`[int8; 16]`) — same bytes, different signed-ness.
fn guid_as_tuid(guid: &vst3::com_scrape_types::Guid) -> TUID {
    let mut out: TUID = [0; 16];
    for (i, b) in guid.iter().enumerate() {
        out[i] = *b as i8;
    }
    out
}

pub struct Vst3Library {
    _library: Library,
    factory: ComPtr<IPluginFactory>,
}

unsafe impl Send for Vst3Library {}
unsafe impl Sync for Vst3Library {}

impl Vst3Library {
    /// Load a VST3 library from a pre-resolved path to the actual binary.
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

        let factory_ptr = unsafe { get_factory() };
        let factory = unsafe { ComPtr::from_raw(factory_ptr) }.ok_or_else(|| {
            Vst3Error::LoadFailed {
                path: lib_path.to_path_buf(),
                stage: LoadStage::Factory,
                reason: "GetPluginFactory returned null".to_string(),
            }
        })?;

        Ok(Arc::new(Self {
            _library: library,
            factory,
        }))
    }

    pub fn get_factory_info(&self) -> Option<FactoryInfo> {
        let mut info: PFactoryInfo = unsafe { std::mem::zeroed() };
        let result = unsafe { self.factory.getFactoryInfo(&mut info) };
        if result == kResultOk {
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
        unsafe { self.factory.countClasses() }
    }

    pub fn get_class_info(&self, index: i32) -> Result<ClassInfo> {
        let mut info: PClassInfo = unsafe { std::mem::zeroed() };
        let result = unsafe { self.factory.getClassInfo(index, &mut info) };
        if result == kResultOk {
            let cid_bytes: [u8; 16] = unsafe { std::mem::transmute(info.cid) };
            Ok(ClassInfo {
                cid: info.cid,
                cid_bytes,
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

    /// Instantiate a class by CID and query for an interface. Returns a
    /// `+1` refcounted raw pointer to the requested interface.
    pub(crate) fn create_instance<I: Interface>(&self, cid: &TUID) -> Result<ComPtr<I>> {
        let iid_tuid = guid_as_tuid(&I::IID);
        let mut obj: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            self.factory.createInstance(
                cid.as_ptr() as FIDString,
                iid_tuid.as_ptr() as FIDString,
                &mut obj,
            )
        };
        if result != kResultOk || obj.is_null() {
            return Err(Vst3Error::PluginError {
                stage: LoadStage::Instantiation,
                code: result,
            });
        }
        unsafe { ComPtr::from_raw(obj as *mut I) }.ok_or(Vst3Error::PluginError {
            stage: LoadStage::Instantiation,
            code: result,
        })
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
    pub cid: TUID,
    /// Human-readable byte-order independent representation for formatting.
    pub cid_bytes: [u8; 16],
    pub category: String,
    pub name: String,
}
