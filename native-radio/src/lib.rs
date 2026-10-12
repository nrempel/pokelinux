//! Generated native vendor metadata bindings, checked against original Android bytes.
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
