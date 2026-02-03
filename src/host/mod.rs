//! Host-side VST3 plugin management.
//!
//! This module provides the main types for loading and using VST3 plugins:
//!
//! - [`Vst3Library`] - Loads a VST3 bundle and provides access to the plugin factory
//! - [`Vst3Instance`] - A loaded plugin instance ready for audio processing

mod bundle;
mod instance;
mod library;

pub use instance::Vst3Instance;
pub use library::Vst3Library;
