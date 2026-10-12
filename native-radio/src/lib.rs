//! Generated native vendor metadata bindings, checked against original Android bytes.
//! This does not implement vendor service discovery or radio transactions.
// AIDL package names are wire identifiers, including radioExternal.
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/radio.rs"));
}
pub use bindings::vendor::samsung_slsi::telephony::hardware::radioExternal::RadioExternalResponseInfo::RadioExternalResponseInfo;
