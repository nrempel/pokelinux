//! Fixed desired connectivity through the existing actor and root-private owner.
//! Explicit effects retain current actor authority; the separate Main owns transferred work.
use crate::{boottime_ns, fresh_id};
use pokelinux_connectivity_control::{client::Client, protocol};
use serde::{Deserialize, Serialize};
use std::{io, path::Path};
pub const APP: &str = "org.omarchy.Settings";
pub const UID: u32 = 11050;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub connected: bool,
    /// Original canonical absolute BOOTTIME deadline, fixed at user action.
    pub until_ns: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub connected: bool,
    pub available: bool,
    #[serde(deserialize_with = "required_option")]
    pub configured: Option<bool>,
    pub pending: bool,
    pub observed_at_ns: String,
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
            configured: None,
            pending: false,
            observed_at_ns: "0".into(),
            until_ns: "0".into(),
        }
    }
}
impl State {
    pub fn is_current(&self, now: u64) -> bool {
        let Some(at) = protocol::decimal(&self.observed_at_ns) else {
            return false;
        };
        let Some(until) = protocol::decimal(&self.until_ns) else {
            return false;
        };
        self.connected
            && self.available
            && self.configured.is_some()
            && at > 0
            && at <= now
            && now < until
            && at.checked_add(protocol::LEASE) == Some(until)
    }
    pub(crate) fn current(mut self, now: u64) -> Self {
        if !self.is_current(now) {
            self.available = false;
            self.configured = None;
            self.observed_at_ns = "0".into();
            self.until_ns = "0".into();
        }
        self
    }
}
/// Actor-only intent; never deserialized from an application packet.
pub struct Intent {
    pub id: u64,
    pub connected: bool,
    pub until_ns: u64,
}
pub enum Event {
    Observed,
    Completed { id: u64, state: State },
    Rejected { id: u64 },
    Unavailable,
}
/// Trusted configuration. Every method is bounded and never owns the worker.
pub trait Provider {
    fn incarnation(&self) -> &str;
    fn state(&self) -> State;
    /// Own readonly request with a fresh baseline and no reported remote effect.
    fn readonly_in_flight(&self) -> bool;
    /// A full send transfers work to Main; no retry or reconnect for an intent.
    fn submit(&mut self, intent: &Intent) -> io::Result<bool>;
    fn inspect(&mut self, original_until: u64) -> io::Result<bool>;
    fn poll(&mut self) -> Option<Event>;
    /// Withdraw publication; never Disconnect, Stop or claim remote cleanup.
    fn revoke(&mut self);
}
enum Pending {
    Inspect,
    Apply { actor: u64, incarnation: String },
}
/// Private transport IDs, actor intent IDs and backend incarnation are distinct.
pub struct Owner {
    client: Client,
    base: String,
    generation: u64,
    incarnation: String,
    sequence: u64,
    pending: Option<Pending>,
    state: State,
}
impl Owner {
    pub fn new(path: &Path) -> io::Result<Self> {
        let base = fresh_id()?;
        Ok(Self {
            client: Client::new(path.into())?,
            incarnation: format!("{base}:0"),
            base,
            generation: 0,
            sequence: 0,
            pending: None,
            state: State::default(),
        })
    }
    fn next_id(&mut self) -> io::Result<u64> {
        if self.incarnation.is_empty() {
            return Err(io::ErrorKind::Other.into());
        }
        let Some(next) = self.sequence.checked_add(1) else {
            self.revoke();
            self.incarnation.clear();
            return Err(io::ErrorKind::Other.into());
        };
        self.sequence = next;
        Ok(next)
    }
}
impl Provider for Owner {
    fn incarnation(&self) -> &str {
        &self.incarnation
    }
    fn state(&self) -> State {
        let mut state = self.state.clone().current(boottime_ns());
        state.pending |= self.pending.is_some();
        state
    }
    fn readonly_in_flight(&self) -> bool {
        matches!(self.pending, Some(Pending::Inspect))
            && self.state.is_current(boottime_ns())
            && !self.state.pending
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        let now = boottime_ns();
        // Only Inspect may establish a new socket/worker epoch. A withdrawn
        // client has no current state and cannot reconnect through Apply.
        if self.pending.is_some()
            || !self.state.is_current(now)
            || self.state.pending
            || intent.id == 0
            || intent.until_ns <= now
            || intent.until_ns - now > protocol::SECOND
        {
            return Ok(false);
        }
        let wire = self.next_id()?;
        let choice = if intent.connected {
            protocol::Choice::Connect
        } else {
            protocol::Choice::Disconnect
        };
        if let Err(e) = self.client.apply(wire, choice, intent.until_ns) {
            self.revoke();
            return Err(e);
        }
        self.pending = Some(Pending::Apply {
            actor: intent.id,
            incarnation: self.incarnation.clone(),
        });
        Ok(true)
    }
    fn inspect(&mut self, original_until: u64) -> io::Result<bool> {
        if self.pending.is_some() {
            return Ok(false);
        }
        let wire = self.next_id()?;
        if let Err(e) = self.client.inspect(wire, original_until) {
            self.revoke();
            return Err(e);
        }
        self.pending = Some(Pending::Inspect);
        Ok(true)
    }
    fn poll(&mut self) -> Option<Event> {
        let pending = self.pending.take()?;
        let reply = match self.client.poll() {
            Ok(None) => {
                self.pending = Some(pending);
                return None;
            }
            Ok(Some(reply)) => reply,
            Err(_) => {
                self.revoke();
                return Some(Event::Unavailable);
            }
        };
        let incarnation = format!("{}:{}:{}", self.base, self.generation, reply.state.worker);
        if matches!(&pending, Pending::Apply { incarnation: expected, .. } if expected != &incarnation)
            || reply.status == protocol::Status::Unavailable
        {
            self.revoke();
            return Some(Event::Unavailable);
        }
        self.incarnation = incarnation;
        self.state = State {
            connected: true,
            available: reply.state.available,
            configured: reply.state.configured,
            pending: reply.state.pending,
            observed_at_ns: reply.state.observed_at_ns,
            until_ns: reply.state.until_ns,
        };
        match (pending, reply.status) {
            (Pending::Inspect, protocol::Status::Observed) => Some(Event::Observed),
            (Pending::Apply { actor, .. }, protocol::Status::Completed) => Some(Event::Completed {
                id: actor,
                state: self.state(),
            }),
            (Pending::Apply { actor, .. }, protocol::Status::Refused) => {
                Some(Event::Rejected { id: actor })
            }
            _ => {
                self.revoke();
                Some(Event::Unavailable)
            }
        }
    }
    fn revoke(&mut self) {
        self.client.withdraw();
        self.pending = None;
        self.state = State::default();
        if let Some(next) = self.generation.checked_add(1) {
            self.generation = next;
            self.incarnation = format!("{}:{}", self.base, next);
        } else {
            self.incarnation.clear();
        }
    }
}
