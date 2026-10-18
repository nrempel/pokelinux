//! Typed display brightness intent/readback. No paths, UIDs or app capabilities.
use serde::{Deserialize, Serialize};
use std::io;
pub const APP: &str = "org.omarchy.Settings";
pub const UID: u32 = 11050;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub percent: u8,
    pub expected_revision: u64,
    pub ttl_ms: u64,
}
/// Current owner state. Attribute readback is not physical luminance.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub connected: bool,
    pub available: bool,
    pub revision: u64,
    /// Actual backend operation custody, separate from actor reply publication.
    pub pending: bool,
    #[serde(deserialize_with = "required_option")]
    pub requested_percent: Option<u8>,
    /// Normalized Linux brightness attribute readback, not the requested value.
    #[serde(deserialize_with = "required_option")]
    pub observed_percent: Option<u8>,
    /// Optional normalized actual_brightness attribute, not measured luminance.
    #[serde(deserialize_with = "required_option")]
    pub actual_percent: Option<u8>,
    /// Canonical decimal CLOCK_BOOTTIME; zero only while unavailable.
    pub until_ns: String,
}
fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
impl Default for State {
    fn default() -> Self {
        Self {
            connected: false,
            available: false,
            revision: 0,
            pending: false,
            requested_percent: None,
            observed_percent: None,
            actual_percent: None,
            until_ns: "0".into(),
        }
    }
}
impl State {
    pub fn is_current(&self, now: u64) -> bool {
        let Ok(until) = self.until_ns.parse::<u64>() else {
            return false;
        };
        self.connected
            && self.available
            && self.revision > 0
            && now < until
            && until.to_string() == self.until_ns
            && self.observed_percent.is_some_and(|v| v <= 100)
            && self.actual_percent.is_none_or(|v| v <= 100)
            && self.requested_percent.is_none_or(|v| v <= 100)
    }
    pub(crate) fn current(mut self, now: u64) -> Self {
        if !self.is_current(now) {
            self.available = false;
            self.requested_percent = None;
            self.observed_percent = None;
            self.actual_percent = None;
            self.until_ns = "0".into();
        }
        self
    }
}
/// Actor-only, never deserialized from application input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: u64,
    pub percent: u8,
    pub expected_revision: u64,
    pub until_ns: u64,
}
pub enum Event {
    Observed,
    Completed { id: u64, state: State },
    Rejected { id: u64 },
    Unavailable,
}
/// Methods are bounded/nonblocking. This connection never owns the device writer.
pub trait Provider {
    fn incarnation(&self) -> &str;
    fn state(&self) -> State;
    /// Only this connection's outstanding Inspect with a fresh retained baseline
    /// and no reported remote custody. This never grants permission to write.
    fn readonly_in_flight(&self) -> bool;
    /// false proves no bytes sent and no queued command. It is not retried here.
    fn submit(&mut self, intent: &Intent) -> io::Result<bool>;
    /// Fresh readonly connection/status is permitted; no flush, replay or write.
    fn inspect(&mut self, until_ns: u64) -> io::Result<bool>;
    /// Readonly; never dispatch, reconnect for Set or replay a prior intent.
    fn poll(&mut self) -> Option<Event>;
    /// Retire intent/connection. No hardware rollback or remote cleanup proof.
    fn revoke(&mut self);
}

#[path = "brightness_owner.rs"]
mod owner;
pub use owner::Owner;
