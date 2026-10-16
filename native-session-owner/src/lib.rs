#![forbid(unsafe_code)]
//! Owned compositor child and its fact-only Source observer. No policy authority.
mod bootstrap;
mod source;
pub use bootstrap::bootstrap_entry;
pub use source::{Compositor, MAX_SOURCE_STARTUP_NS, SourceOwner};
pub mod config;
pub mod display;
pub mod runtime;
pub mod stop;
