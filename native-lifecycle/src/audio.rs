//! Current lifecycle authorization and a single existing audio owner's Media lease.
//! Presentation receives logical routes and observations; no PipeWire identity.
pub use crate::audio_owner::Owner;
use serde::{Deserialize, Serialize};
use std::io;

pub const APP: &str = "org.omarchy.Sounds";
pub const ROUTES_PER_PAGE: usize = 8;
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub route: String,
    pub ttl_ms: u64,
}
/// Trusted actor-only data, never an application-supplied capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: u64,
    pub route: String,
    pub until_ns: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub connected: bool,
    pub available: bool,
    pub pending: bool,
    pub requested: Option<String>,
    pub applied: Option<String>,
}
pub enum Event {
    Observed,
    Selected { id: u64, state: State },
    Rejected { id: u64 },
    Unavailable,
}
/// One selected app generation's trusted connection, never a second link manager.
/// Explicit effect submission is separate from read-only inspection and polling.
pub trait Provider {
    fn incarnation(&self) -> &str;
    fn state(&self) -> State;
    fn routes(&self) -> Vec<Route>;
    /// false means no command sent and no local command queue.
    fn submit(&mut self, intent: &Intent) -> io::Result<bool>;
    /// Read-only, same single-in-flight transport and original deadline.
    fn inspect(&mut self, until_ns: u64) -> io::Result<bool>;
    /// Read only: never submit, flush, reconcile or reconnect.
    fn poll(&mut self) -> Option<Event>;
    /// Close the actual connection; graph cleanup is a separate observation.
    fn revoke(&mut self);
    /// Explicit fresh connection only. Never inherit or replay prior intent.
    fn reconnect(&mut self) -> io::Result<()>;
}
