//! Cross-platform VST3 bundle resolution.

use std::path::{Path, PathBuf};

use crate::error::{LoadStage, Vst3Error};

/// Find the actual library file within a .vst3 bundle.
///
/// VST3 plugins are distributed as bundles with platform-specific paths:
/// - macOS: `Contents/MacOS/{name}`
/// - Linux: `Contents/x86_64-linux/{name}.so`
/// - Windows: `Contents/x86_64-win/{name}.vst3`
pub fn find_library_path(bundle_path: &Path) -> Result<PathBuf, Vst3Error> {
    #[cfg(target_os = "macos")]
    {
        let lib_name = bundle_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: bundle_path.to_path_buf(),
                stage: LoadStage::Opening,
                reason: "Failed to extract plugin name from bundle path".to_string(),
            })?;

        let lib_path = bundle_path.join("Contents").join("MacOS").join(lib_name);
        if lib_path.exists() {
            return Ok(lib_path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let lib_name = bundle_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: bundle_path.to_path_buf(),
                stage: LoadStage::Opening,
                reason: "Failed to extract plugin name from bundle path".to_string(),
            })?;

        let lib_path = bundle_path
            .join("Contents")
            .join("x86_64-linux")
            .join(format!("{}.so", lib_name));
        if lib_path.exists() {
            return Ok(lib_path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let lib_name = bundle_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Vst3Error::LoadFailed {
                path: bundle_path.to_path_buf(),
                stage: LoadStage::Opening,
                reason: "Failed to extract plugin name from bundle path".to_string(),
            })?;

        let lib_path = bundle_path
            .join("Contents")
            .join("x86_64-win")
            .join(format!("{}.vst3", lib_name));
        if lib_path.exists() {
            return Ok(lib_path);
        }
    }

    // If bundle is a direct library file, use it directly
    if bundle_path.is_file() {
        return Ok(bundle_path.to_path_buf());
    }

    Err(Vst3Error::LoadFailed {
        path: bundle_path.to_path_buf(),
        stage: LoadStage::Opening,
        reason: "Could not find library in VST3 bundle".to_string(),
    })
}
