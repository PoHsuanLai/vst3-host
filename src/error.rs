//! Error types returned by plugin loading and processing.

use std::path::PathBuf;
use thiserror::Error;

/// Convenience alias for `Result<T, Vst3Error>`.
pub type Result<T> = std::result::Result<T, Vst3Error>;

/// Errors produced while loading, initializing, or driving a VST3 plugin.
#[derive(Error, Debug)]
pub enum Vst3Error {
    /// A file-system or factory-level failure that aborted loading before the
    /// plugin could be instantiated. `stage` pinpoints which step failed.
    #[error("Failed to load plugin at {path}: {stage} - {reason}")]
    LoadFailed {
        path: PathBuf,
        stage: LoadStage,
        reason: String,
    },

    /// A plugin call returned a non-OK `tresult`. `code` is the raw VST3 return
    /// code as defined in `pluginterfaces/base/funknown.h`.
    #[error("Plugin error at {stage}: code {code}")]
    PluginError { stage: LoadStage, code: i32 },

    /// Operation requires the plugin to be in the active (processing) state.
    #[error("Plugin is not active")]
    NotActive,

    /// Feature requested by the host is not supported by this plugin (e.g.
    /// 64-bit processing, an editor view).
    #[error("Feature not supported: {0}")]
    NotSupported(String),

    /// Parameter index, id, or value fell outside the plugin's allowed range.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Saving or restoring plugin state failed — truncated data, mismatched
    /// format, or the plugin rejected the stream.
    #[error("State error: {0}")]
    StateError(String),
}

/// Labels the phase of plugin loading in which a [`Vst3Error`] was produced.
///
/// Reported in [`Vst3Error::LoadFailed`] and [`Vst3Error::PluginError`] so
/// callers can distinguish "file not found" from "plugin rejected our sample
/// rate" without parsing free-form messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    /// Resolving the plugin bundle on disk.
    Scanning,
    /// Loading the dynamic library.
    Opening,
    /// Calling `GetPluginFactory` / walking `IPluginFactory` classes.
    Factory,
    /// `IPluginFactory::createInstance`.
    Instantiation,
    /// `IPluginBase::initialize` on the component or controller.
    Initialization,
    /// `IAudioProcessor::setupProcessing`.
    Setup,
    /// `IComponent::setActive(1)` / `IAudioProcessor::setProcessing(1)`.
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
