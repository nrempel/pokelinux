//! Fixed IPv4 connectivity on exclusively administered virtual or existing links.
//! Application authorization and physical backends remain unqualified here.
pub mod client;
pub mod fixed_address;
pub mod kernel;
pub mod main_service;
pub mod owner;
mod private_channel;
pub mod protocol;
pub mod service;
