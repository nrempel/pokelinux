//! Fixed IPv4 connectivity in an exclusively administered disposable virtual backend.
//! This component does not authorize applications or configure physical interfaces.
pub mod client;
pub mod kernel;
pub mod main_service;
pub mod owner;
mod private_channel;
pub mod protocol;
pub mod service;
