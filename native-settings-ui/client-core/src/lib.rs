//! Settings brightness client: one owned bounded worker and a narrow C ABI.
mod clock;
pub mod connectivity;
pub mod connectivity_ffi;
mod engine;
pub mod ffi;
mod protocol;
