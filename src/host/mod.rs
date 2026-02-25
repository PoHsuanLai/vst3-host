//! Host-side VST3 plugin management.

mod bundle;
mod instance;
mod library;

pub use instance::Vst3Instance;
pub use library::Vst3Library;
