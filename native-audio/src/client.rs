//! Explicit media-only requests on one verified connection. Poll never sends.
use crate::{PreviewState, Role};
use pokelinux_power::clock::{BootClock, Clock};
use rustix::net::{RecvFlags, SendFlags, recv, send, sockopt};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::io;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};

const FRAME_LIMIT: usize = 65_536;
static INCARNATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaRoute {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub lease: u64,
    pub route: String,
    pub intent_revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub epoch: u64,
    pub revision: u64,
    pub available: bool,
    pub intent_revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub incarnation: u64,
    pub connected: bool,
    pub media_routes: Vec<MediaRoute>,
    pub admitted: Option<Admission>,
    pub applied: Option<Admission>,
    pub observation: Option<Observation>,
    pub in_flight: bool,
    /// Selection policy is settled; this does not assert an observed audio link.
    pub policy_settled: bool,
    pub preview: PreviewState,
    /// Owner uncertainty or a locally unknown outcome; never a cleanup claim.
    pub uncertain: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    WouldBlock,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Write,
    ShortWrite,
    Read,
    PeerClosed,
    Deadline,
    Protocol,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Busy,
    Closed,
    Expired,
    NoCatalog,
    InvalidRoute,
    Uncertain(Failure),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Updated(Snapshot),
    Rejected { reason: String, snapshot: Snapshot },
    Unavailable { reason: Failure, snapshot: Snapshot },
}

enum Operation {
    Inspect,
    Acquire(String),
    Route(String),
    PreviewPlay { route: String, lease: Option<u64> },
    PreviewStop,
}
struct Pending {
    operation: Operation,
    until_ns: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObservation {
    epoch: u64,
    revision: u64,
    available: bool,
    links: Vec<WireLink>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLink {
    source: String,
    target: String,
    state: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Application {
    Pending,
    Observed,
    Uncertain,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Running,
    Stopping,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Cleanup {
    None,
    Pending,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    ok: bool,
    lease: Option<u64>,
    peer_uid: u32,
    desired: Vec<(u64, Role, String)>,
    observed: WireObservation,
    intent_revision: u64,
    application: Application,
    phase: Phase,
    cleanup: Cleanup,
    policy_settled: bool,
    preview: PreviewState,
    media_routes: Option<Vec<MediaRoute>>,
}

fn now_ns() -> u64 {
    BootClock
        .now()
        .elapsed()
        .as_nanos()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}
fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

pub struct Client {
    stream: Option<UnixStream>,
    state: Snapshot,
    inspected: bool,
    pending: Option<Pending>,
    incoming: Vec<u8>,
    own_uid: u32,
}
impl Client {
    /// Takes connection ownership. UID0 trusts root deployment, not a unique actor.
    /// A successor must construct a new stream/client; this client never reconnects.
    pub fn from_stream(stream: UnixStream, expected_owner_uid: u32) -> io::Result<Self> {
        if sockopt::socket_type(&stream)? != rustix::net::SocketType::STREAM
            || sockopt::socket_peercred(&stream)?.uid.as_raw() != expected_owner_uid
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "audio owner",
            ));
        }
        stream.peer_addr()?;
        stream.set_nonblocking(true)?;
        let incarnation = INCARNATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| io::Error::other("audio incarnation exhausted"))?;
        Ok(Self {
            stream: Some(stream),
            state: Snapshot {
                incarnation,
                connected: true,
                media_routes: Vec::new(),
                admitted: None,
                applied: None,
                observation: None,
                in_flight: false,
                policy_settled: false,
                preview: PreviewState::Idle,
                uncertain: false,
            },
            inspected: false,
            pending: None,
            incoming: Vec::new(),
            own_uid: rustix::process::getuid().as_raw(),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    pub fn inspect(&mut self, until_ns: u64) -> Result<SendOutcome, SubmitError> {
        self.submit(Operation::Inspect, json!({"op":"status"}), until_ns)
    }
    pub fn select(&mut self, route: &str, until_ns: u64) -> Result<SendOutcome, SubmitError> {
        if self.stream.is_none() {
            return Err(SubmitError::Closed);
        }
        if self.pending.is_some() {
            return Err(SubmitError::Busy);
        }
        if !self.inspected {
            return Err(SubmitError::NoCatalog);
        }
        if !id(route) || !self.state.media_routes.iter().any(|r| r.id == route) {
            return Err(SubmitError::InvalidRoute);
        }
        if let Some(admitted) = &self.state.admitted {
            self.submit(
                Operation::Route(route.into()),
                json!({"op":"route","lease":admitted.lease,"route":route}),
                until_ns,
            )
        } else {
            self.submit(
                Operation::Acquire(route.into()),
                json!({"op":"acquire","role":"media","route":route}),
                until_ns,
            )
        }
    }
    /// One explicit finite preview. The actor supplies the already clamped
    /// CLOCK_BOOTTIME deadline; neither this transport nor the service renews it.
    pub fn preview_play(&mut self, route: &str, until_ns: u64) -> Result<SendOutcome, SubmitError> {
        if self.stream.is_none() {
            return Err(SubmitError::Closed);
        }
        if self.pending.is_some() {
            return Err(SubmitError::Busy);
        }
        if !self.inspected {
            return Err(SubmitError::NoCatalog);
        }
        if !id(route) || !self.state.media_routes.iter().any(|r| r.id == route) {
            return Err(SubmitError::InvalidRoute);
        }
        let lease = self.state.admitted.as_ref().map(|a| a.lease);
        self.submit(
            Operation::PreviewPlay {
                route: route.into(),
                lease,
            },
            json!({"op":"preview_play","lease":lease,"route":route,"until_boottime_ns":until_ns}),
            until_ns,
        )
    }
    /// Connection-owned cleanup only; no new route or playback input is supplied.
    pub fn preview_stop(&mut self, until_ns: u64) -> Result<SendOutcome, SubmitError> {
        let lease = self
            .state
            .admitted
            .as_ref()
            .ok_or(SubmitError::InvalidRoute)?
            .lease;
        self.submit(
            Operation::PreviewStop,
            json!({"op":"preview_stop","lease":lease}),
            until_ns,
        )
    }
    fn submit(
        &mut self,
        operation: Operation,
        value: serde_json::Value,
        until_ns: u64,
    ) -> Result<SendOutcome, SubmitError> {
        let Some(stream) = &self.stream else {
            return Err(SubmitError::Closed);
        };
        if self.pending.is_some() {
            return Err(SubmitError::Busy);
        }
        let mut bytes = serde_json::to_vec(&value).expect("bounded audio request");
        bytes.push(b'\n');
        if now_ns() >= until_ns {
            return Err(SubmitError::Expired);
        }
        match send(stream, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
            Ok(n) if n == bytes.len() => {
                let selecting = !matches!(operation, Operation::Inspect);
                self.pending = Some(Pending {
                    operation,
                    until_ns,
                });
                self.state.in_flight = true;
                if selecting {
                    self.state.applied = None;
                }
                if now_ns() >= until_ns {
                    self.terminal(true);
                    return Err(SubmitError::Uncertain(Failure::Deadline));
                }
                Ok(SendOutcome::Sent)
            }
            Err(rustix::io::Errno::AGAIN) => Ok(SendOutcome::WouldBlock),
            result => {
                let reason = if result.is_ok() {
                    Failure::ShortWrite
                } else {
                    Failure::Write
                };
                self.terminal(true);
                Err(SubmitError::Uncertain(reason))
            }
        }
    }
    fn expired(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| now_ns() >= pending.until_ns)
    }
    /// One nonblocking read only. Never schedules a status, mutation or write retry.
    pub fn poll(&mut self) -> Option<Event> {
        let stream = self.stream.as_ref()?;
        if self.expired() {
            return Some(self.failed(Failure::Deadline));
        }
        let mut bytes = [0; 4096];
        match recv(stream, &mut bytes[..], RecvFlags::DONTWAIT) {
            Ok((_, 0)) => return Some(self.failed(Failure::PeerClosed)),
            Ok((_, n)) => {
                if self.expired() {
                    return Some(self.failed(Failure::Deadline));
                }
                if self.pending.is_none() || self.incoming.len() + n > FRAME_LIMIT {
                    return Some(self.failed(Failure::Protocol));
                }
                self.incoming.extend_from_slice(&bytes[..n]);
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return None,
            Err(_) => return Some(self.failed(Failure::Read)),
        }
        let end = self.incoming.iter().position(|b| *b == b'\n')?;
        if end + 1 != self.incoming.len() {
            return Some(self.failed(Failure::Protocol));
        }
        let value = serde_json::from_slice(&self.incoming[..end]);
        self.incoming.clear();
        let event = match value.and_then(|value| self.accept(value)) {
            Ok(event) => event,
            Err(_) => return Some(self.failed(Failure::Protocol)),
        };
        if self.expired() {
            return Some(self.failed(Failure::Deadline));
        }
        self.pending = None;
        self.state.in_flight = false;
        match event {
            Some(reason) => Some(Event::Rejected {
                reason,
                snapshot: self.snapshot(),
            }),
            None => Some(Event::Updated(self.snapshot())),
        }
    }
    fn accept(&mut self, value: serde_json::Value) -> Result<Option<String>, serde_json::Error> {
        use serde::de::Error;
        let invalid = || serde_json::Error::custom("invalid audio response");
        let object = value.as_object().ok_or_else(invalid)?;
        if value["ok"] == false {
            let reason = value["error"]
                .as_str()
                .filter(|s| text(s, 64))
                .ok_or_else(invalid)?;
            if object.len() != 2 {
                return Err(invalid());
            }
            return Ok(Some(reason.into()));
        }
        if !object.contains_key("lease") {
            return Err(invalid());
        }
        let reply: Reply = serde_json::from_value(value)?;
        if !reply.ok
            || reply.peer_uid != self.own_uid
            || reply.desired.len() > 64
            || reply.observed.links.len() > 64
            || reply
                .observed
                .links
                .iter()
                .any(|l| !text(&l.source, 256) || !text(&l.target, 256) || !text(&l.state, 32))
            || self.state.observation.as_ref().is_some_and(|old| {
                reply.intent_revision < old.intent_revision
                    || reply.observed.epoch < old.epoch
                    || reply.observed.revision < old.revision
            })
        {
            return Err(invalid());
        }
        let mut leases = BTreeSet::new();
        if reply
            .desired
            .iter()
            .any(|(lease, _, route)| *lease == 0 || !leases.insert(*lease) || !id(route))
        {
            return Err(invalid());
        }
        let pending = self.pending.as_ref().ok_or_else(invalid)?;
        match &pending.operation {
            Operation::Inspect => {
                if reply.lease.is_some() {
                    return Err(invalid());
                }
                let routes = reply.media_routes.ok_or_else(invalid)?;
                let mut ids = BTreeSet::new();
                if routes.len() > 32
                    || routes
                        .iter()
                        .any(|r| !id(&r.id) || !text(&r.label, 96) || !ids.insert(r.id.clone()))
                {
                    return Err(invalid());
                }
                let mut sorted = routes;
                sorted.sort_by(|a, b| a.id.cmp(&b.id));
                self.state.media_routes = sorted;
                self.inspected = true;
            }
            Operation::Acquire(route) => {
                if reply.media_routes.is_some() {
                    return Err(invalid());
                }
                let lease = reply.lease.filter(|n| *n > 0).ok_or_else(invalid)?;
                self.state.admitted = Some(Admission {
                    lease,
                    route: route.clone(),
                    intent_revision: reply.intent_revision,
                });
            }
            Operation::PreviewPlay {
                route,
                lease: previous,
            } => {
                if reply.media_routes.is_some() {
                    return Err(invalid());
                }
                let lease = reply.lease.filter(|n| *n > 0).ok_or_else(invalid)?;
                if previous.is_some_and(|old| old != lease) {
                    return Err(invalid());
                }
                self.state.admitted = Some(Admission {
                    lease,
                    route: route.clone(),
                    intent_revision: reply.intent_revision,
                });
            }
            Operation::PreviewStop => {
                if reply.media_routes.is_some() || reply.lease.is_some() {
                    return Err(invalid());
                }
            }
            Operation::Route(route) => {
                if reply.media_routes.is_some() || reply.lease.is_some() {
                    return Err(invalid());
                }
                let old = self.state.admitted.as_mut().ok_or_else(invalid)?;
                old.route = route.clone();
                old.intent_revision = reply.intent_revision;
            }
        }
        if let Some(own) = &self.state.admitted
            && reply.desired.iter().any(|(lease, role, route)| {
                *lease == own.lease && (*role != Role::Media || *route != own.route)
            })
        {
            return Err(invalid());
        }
        self.state.preview = reply.preview;
        self.state.policy_settled = reply.policy_settled
            && self.state.admitted.as_ref().is_some_and(|own| {
                reply.desired.iter().any(|(lease, role, route)| {
                    *lease == own.lease && *role == Role::Media && *route == own.route
                })
            });
        self.state.uncertain = matches!(reply.application, Application::Uncertain);
        self.state.observation = Some(Observation {
            epoch: reply.observed.epoch,
            revision: reply.observed.revision,
            available: reply.observed.available,
            intent_revision: reply.intent_revision,
        });
        self.state.applied = self
            .state
            .admitted
            .as_ref()
            .filter(|own| {
                matches!(reply.application, Application::Observed)
                    && matches!(reply.phase, Phase::Running)
                    && matches!(reply.cleanup, Cleanup::None)
                    && reply.observed.available
                    && reply.desired.iter().any(|(lease, role, route)| {
                        *lease == own.lease && *role == Role::Media && *route == own.route
                    })
            })
            .cloned();
        Ok(None)
    }
    fn failed(&mut self, reason: Failure) -> Event {
        self.terminal(true);
        Event::Unavailable {
            reason,
            snapshot: self.snapshot(),
        }
    }
    fn terminal(&mut self, uncertain: bool) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        self.pending = None;
        self.incoming.clear();
        self.inspected = false;
        self.state.connected = false;
        self.state.in_flight = false;
        self.state.policy_settled = false;
        self.state.preview = PreviewState::Idle;
        self.state.uncertain = uncertain;
        self.state.admitted = None;
        self.state.applied = None;
        self.state.media_routes.clear();
        self.state.observation = None;
    }
    /// Revokes this connection, not a remote cleanup acknowledgment.
    pub fn revoke(&mut self) {
        self.terminal(false);
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.terminal(false);
    }
}
