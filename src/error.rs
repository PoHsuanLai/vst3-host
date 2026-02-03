//! Error types for vst3-host.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for vst3-host operations.
pub type Result<T> = std::result::Result<T, Vst3Error>;

/// Errors that can occur when loading or using VST3 plugins.
#[derive(Error, Debug)]
pub enum Vst3Error {
    /// Failed to load the plugin at a specific stage.
    #[error("Failed to load plugin at {path}: {stage} - {reason}")]
    LoadFailed {
        /// Path to the plugin bundle.
        path: PathBuf,
        /// Stage at which loading failed.
        stage: LoadStage,
        /// Human-readable error description.
        reason: String,
    },

    /// Plugin returned an error code.
    #[error("Plugin error at {stage}: code {code}")]
    PluginError {
        /// Stage at which the error occurred.
        stage: LoadStage,
        /// VST3 result code.
        code: i32,
    },

    /// Plugin is not in the expected state.
    #[error("Plugin is not active")]
    NotActive,

    /// The requested feature is not supported by the plugin.
    #[error("Feature not supported: {0}")]
    NotSupported(String),

    /// Invalid parameter index or ID.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// State serialization/deserialization error.
    #[error("State error: {0}")]
    StateError(String),
}

/// Stage of plugin loading/initialization where an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    /// Scanning for plugins.
    Scanning,
    /// Opening the shared library.
    Opening,
    /// Getting the plugin factory.
    Factory,
    /// Creating the plugin instance.
    Instantiation,
    /// Initializing the plugin.
    Initialization,
    /// Setting up audio processing.
    Setup,
    /// Activating audio buses.
    Activation,
}

impl std::fmt::Display for LoadStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadStage::Scanning => write!(f, "scanning"),
            LoadStage::Opening => write!(f, "opening library"),
            LoadStage::Factory => write!(f, "getting factory"),
            LoadStage::Instantiation => write!(f, "creating instance"),
            LoadStage::Initialization => write!(f, "initialization"),
            LoadStage::Setup => write!(f, "processing setup"),
            LoadStage::Activation => write!(f, "activation"),
        }
    }
}
