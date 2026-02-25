//! Error types for vst3-host.

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Vst3Error>;

#[derive(Error, Debug)]
pub enum Vst3Error {
    #[error("Failed to load plugin at {path}: {stage} - {reason}")]
    LoadFailed {
        path: PathBuf,
        stage: LoadStage,
        reason: String,
    },

    #[error("Plugin error at {stage}: code {code}")]
    PluginError { stage: LoadStage, code: i32 },

    #[error("Plugin is not active")]
    NotActive,

    #[error("Feature not supported: {0}")]
    NotSupported(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("State error: {0}")]
    StateError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    Scanning,
    Opening,
    Factory,
    Instantiation,
    Initialization,
    Setup,
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
