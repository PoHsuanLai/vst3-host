//! Host-side COM interface implementations for VST3 hosting.
//!
//! Every type here is `pub(crate)` from the consumer's perspective: callers
//! reach the COM objects through `ComWrapper<T>` and `ComPtr<IFoo>` from
//! the `vst3` crate.

mod attr_list;
mod component_handler;
mod connection_point;
mod data_exchange;
mod event_list;
mod host_application;
mod message;
mod param_changes;
mod param_queue;
mod progress;
mod stream;
mod unit_handler;

#[cfg(test)]
mod tests;

pub use component_handler::{ComponentHandler, ParameterEditEvent, ProgressEvent, UnitEvent};
pub use event_list::EventList;
pub use host_application::HostApplication;
pub use param_changes::ParameterChangesImpl;
pub use stream::BStream;

#[cfg(test)]
#[allow(unused_imports)]
pub use attr_list::AttributeList;
#[cfg(test)]
pub use connection_point::ConnectionPoint;
#[cfg(test)]
pub use data_exchange::DataExchangeHandler;
#[cfg(test)]
#[allow(unused_imports)]
pub use message::Message;
#[cfg(test)]
pub use param_queue::ParamValueQueueImpl;
#[cfg(test)]
pub use progress::ProgressHandler;
#[cfg(test)]
pub use unit_handler::UnitHandler;
