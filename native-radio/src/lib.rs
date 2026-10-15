//! Samsung radioExternal backend, checked against original Android bytes.
//! These vendor events and wire types are not the device-neutral phone model.
//! Vendor discovery and session recovery remain outside these transport primitives.
// AIDL package names are wire identifiers, including radioExternal.
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/radio.rs"));
}
pub use bindings::vendor::samsung_slsi::telephony::hardware::radioExternal::RadioExternalResponseInfo::RadioExternalResponseInfo;

pub mod receiver;
pub mod response;

pub mod transport;

pub mod assembly;
pub mod indication;

pub mod owner;

pub mod deadline;

pub mod worker;

pub mod supervisor;

pub mod request;

pub mod bootstrap;
