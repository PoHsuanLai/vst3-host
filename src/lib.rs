//! # vst3-host
//!
//! A Rust library for hosting VST3 audio plugins.
//!
//! This crate provides a safe and ergonomic API for loading, configuring, and
//! processing audio through VST3 plugins. It handles the low-level COM interface
//! details, allowing you to focus on your audio application.
//!
//! ## Features
//!
//! - Load VST3 plugins from `.vst3` bundles (macOS, Linux, Windows)
//! - Process audio in f32 or f64 format
//! - Send MIDI events to plugins
//! - Automation via parameter changes
//! - Transport/tempo synchronization
//! - Plugin state save/load
//! - Editor window support
//!
//! ## Example
//!
//! ```ignore
//! use vst3_host::{Vst3Instance, AudioBuffer, MidiEvent, TransportState};
//!
//! // Load a VST3 plugin (sample_rate=44100, block_size=512)
//! let mut plugin = Vst3Instance::load("/path/to/plugin.vst3", 44100.0, 512)?;
//!
//! // Check capabilities
//! println!("Name: {}", plugin.info().name);
//! println!("Supports f64: {}", plugin.supports_f64());
//!
//! // Process audio with MIDI
//! let midi = vec![MidiEvent::note_on(0, 0, 60, 0.8)];
//! let transport = TransportState::new().tempo(120.0).playing(true);
//! plugin.process(&mut buffer, &midi, &transport)?;
//! ```
//!
//! ## Custom MIDI Types
//!
//! If you have your own MIDI event type, implement the `Vst3MidiEvent` trait:
//!
//! ```ignore
//! use vst3_host::{Vst3MidiEvent, ffi::Vst3Event};
//!
//! impl Vst3MidiEvent for MyMidiEvent {
//!     fn sample_offset(&self) -> i32 { self.offset as i32 }
//!     fn to_vst3_event(&self) -> Option<Vst3Event> { /* ... */ }
//! }
//! ```

pub mod com;
pub mod error;
pub mod ffi;
pub mod host;
pub mod types;

pub use error::{LoadStage, Result, Vst3Error};
pub use ffi::{parameter_flags, BusInfo, Vst3ParameterInfo};
pub use host::{Vst3Instance, Vst3Library};
pub use types::{
    AudioBuffer, MidiData, MidiEvent, NoteExpressionType, NoteExpressionValue, ParameterChanges,
    ParameterPoint, ParameterQueue, PluginInfo, Sample, TransportState, Vst3MidiEvent,
};

pub use com::{BStream, ComponentHandler, EventList, ParameterChangesImpl, ParameterEditEvent};
