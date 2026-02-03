//! COM interface implementations for VST3 hosting.
//!
//! This module provides host-side implementations of VST3 COM interfaces
//! that are needed to interact with plugins:
//!
//! - [`EventList`] - Provides MIDI events to plugins
//! - [`ParamValueQueueImpl`] - Provides parameter automation points
//! - [`ParameterChangesImpl`] - Collection of parameter automation queues
//! - [`HostApplication`] - Provides host information to plugins
//! - [`ComponentHandler`] - Receives parameter edit notifications from GUI
//! - [`BStream`] - Binary stream for state serialization
//! - [`ConnectionPoint`] - Processor/controller communication
//! - [`UnitHandler`] - Unit/program change notifications
//! - [`ProgressHandler`] - Progress reporting
//! - [`DataExchangeHandler`] - Waveform/visualization data exchange

mod component_handler;
mod connection_point;
mod data_exchange;
mod event_list;
mod host_application;
mod param_changes;
mod param_queue;
mod progress;
mod stream;
mod unit_handler;

pub use component_handler::{ComponentHandler, ParameterEditEvent};
pub use connection_point::ConnectionPoint;
pub use data_exchange::{DataBlock, DataExchangeHandler};
pub use event_list::EventList;
pub use host_application::{AttributeList, HostApplication, Message};
pub use param_changes::ParameterChangesImpl;
pub use param_queue::ParamValueQueueImpl;
pub use progress::{ProgressEvent, ProgressHandler};
pub use stream::BStream;
pub use unit_handler::{UnitEvent, UnitHandler};
