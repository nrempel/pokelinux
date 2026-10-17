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
pub mod registration;

pub mod worker;

pub mod request;

pub mod observation;

pub mod status;

mod registration_state;

#[cfg(test)]
mod add_pdn_transport_tests;

#[cfg(test)]
mod add_pdn_response_binder_tests;

/// One Root-selected operator attempt; observations never grant network readiness.
pub mod operator;
