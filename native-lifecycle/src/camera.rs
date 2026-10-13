//! One camera intent through the current lifecycle actor. Hardware stays in adapters.
use pokelinux_camera::{CaptureId, Generation, Receipt, Snapshot};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub ttl_ms: u64,
}

/// Internal domain data, never an application-supplied effect capability.
/// The actor retains its separate non-serializable current-authority stamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent {
    pub capture: CaptureId,
    pub generation: Generation,
    pub until_ns: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Submission {
    Pending,
    Accepted,
    Rejected,
    Uncertain,
    InvalidCount,
}
impl From<Receipt> for Submission {
    fn from(receipt: Receipt) -> Self {
        match receipt {
            Receipt::Pending => Self::Pending,
            Receipt::Accepted => Self::Accepted,
            Receipt::Rejected => Self::Rejected,
            Receipt::Uncertain => Self::Uncertain,
            Receipt::InvalidCount => Self::InvalidCount,
        }
    }
}

/// Bounded observations only; no image, publication, reuse or reset permission.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub receipt: Submission,
    pub returned: bool,
    pub capture_complete: bool,
    pub fence_ready: bool,
    pub quarantined: bool,
}
impl From<Snapshot> for State {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            receipt: snapshot.receipt.into(),
            returned: snapshot.returned,
            capture_complete: snapshot.capture_complete,
            fence_ready: snapshot.fence_ready,
            quarantined: snapshot.quarantined,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Complete { capture: CaptureId, state: State },
    Failed { capture: CaptureId },
}

pub trait Provider {
    fn incarnation(&self) -> &str;
    fn generation(&self) -> Generation;
    /// Remains occupied until the actual Child has been waited/reaped.
    fn occupied(&self) -> bool;
    fn busy(&self) -> bool;
    /// Admission capacity only; it need not remain true for current publication.
    fn available(&self) -> bool;
    /// Exact admitted/live/nonterminal capture, never inferred from occupancy.
    fn current(&self, capture: CaptureId) -> bool;
    /// The actor checks current authority immediately before every invocation.
    /// false means WouldBlock: no command was sent and no local queue is created.
    fn submit(&mut self, intent: &Intent) -> io::Result<bool>;
    /// Observe only. poll must never flush or initiate an effect command.
    fn poll(&mut self) -> io::Result<Option<Event>>;
    fn revoke(&mut self, capture: CaptureId);
    fn stop(&mut self);
    fn drain_until(&mut self, until_ns: u64) -> bool;
}
