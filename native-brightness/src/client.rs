//! One verified root-owner connection. Only explicit readonly Inspect may
//! establish a connection. Poll never sends and an uncertain Set is not replayed.
use crate::protocol::{Attributes, LIMIT, Reply, Request, SECOND};
use rustix::{
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
    time::{ClockId, clock_gettime},
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::{Component, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static INCARNATION: AtomicU64 = AtomicU64::new(1);
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .ok()
        .and_then(|s| s.checked_mul(SECOND))
        .and_then(|s| {
            u64::try_from(t.tv_nsec)
                .ok()
                .filter(|n| *n < SECOND)
                .and_then(|n| s.checked_add(n))
        })
        .unwrap_or(u64::MAX)
}
fn valid_path(path: &std::path::Path) -> bool {
    path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir))
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub connected: bool,
    /// Known remote custody or a sent request whose retirement is unobserved.
    pub pending: bool,
    pub revision: u64,
    pub observation: Option<Attributes>,
    pub until_ns: u64,
    pub requested_percent: Option<u8>,
    pub last_set_uncertain: bool,
}
pub enum Event {
    Observed,
    Completed { id: u64, reply: Reply },
    Rejected { id: u64 },
    Unavailable,
}
#[derive(Clone, Copy)]
enum Operation {
    Inspect,
    Set {
        id: u64,
        percent: u8,
        expected_revision: u64,
        max: u32,
    },
}
#[derive(Clone, Copy)]
struct Pending {
    request: u64,
    until: u64,
    operation: Operation,
}
pub struct Client {
    fd: Option<OwnedFd>,
    peer: Option<UCred>,
    path: Option<PathBuf>,
    incarnation: u64,
    sequence: u64,
    pending: Option<Pending>,
    view: Snapshot,
}
impl Client {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        if !valid_path(&path) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self::empty(Some(path)))
    }
    fn empty(path: Option<PathBuf>) -> Self {
        Self {
            fd: None,
            peer: None,
            path,
            incarnation: 0,
            sequence: 0,
            pending: None,
            view: Snapshot::default(),
        }
    }
    /// Trusted setup may supply an actual owned socket for syscall tests. Root
    /// peer identity and packet type are still verified; no authority is injected.
    pub fn from_socket(fd: OwnedFd, path: Option<PathBuf>) -> io::Result<Self> {
        if path.as_ref().is_some_and(|p| !valid_path(p)) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut client = Self::empty(path);
        client.attach(fd)?;
        Ok(client)
    }
    fn attach(&mut self, fd: OwnedFd) -> io::Result<()> {
        if net::sockopt::socket_type(&fd)? != SocketType::SEQPACKET {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let peer = net::sockopt::socket_peercred(&fd)?;
        if peer.uid.as_raw() != 0 || peer.gid.as_raw() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        net::sockopt::set_socket_passcred(&fd, true)?;
        let incarnation = INCARNATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| io::Error::other("brightness incarnation exhausted"))?;
        self.fd = Some(fd);
        self.peer = Some(peer);
        self.incarnation = incarnation;
        Ok(())
    }
    pub fn incarnation(&self) -> u64 {
        self.incarnation
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut view = self.view.clone();
        view.connected = self.fd.is_some();
        view.pending |= self.pending.is_some();
        if !view.connected || now() >= view.until_ns {
            view.observation = None;
            view.until_ns = 0;
            view.requested_percent = None;
        }
        view
    }
    fn budget(until: u64) -> bool {
        let sample = now();
        until > sample && until - sample <= SECOND
    }
    fn idle(&mut self) -> bool {
        let Some(fd) = &self.fd else {
            return false;
        };
        let mut byte = [0];
        match net::recv(
            fd,
            &mut byte,
            RecvFlags::PEEK | RecvFlags::DONTWAIT | RecvFlags::TRUNC,
        ) {
            Err(rustix::io::Errno::AGAIN) => true,
            _ => {
                self.revoke();
                false
            }
        }
    }
    fn send(&mut self, until: u64, operation: Operation) -> io::Result<bool> {
        if self.pending.is_some() || !Self::budget(until) || !self.idle() {
            return Ok(false);
        }
        let Some(id) = self.sequence.checked_add(1) else {
            self.revoke();
            return Err(io::Error::other("brightness request exhausted"));
        };
        self.sequence = id;
        let request = match operation {
            Operation::Inspect => Request::Inspect {
                id,
                until_ns: until.to_string(),
            },
            Operation::Set {
                percent,
                expected_revision,
                ..
            } => Request::Set {
                id,
                percent,
                expected_revision,
                until_ns: until.to_string(),
            },
        };
        let bytes = serde_json::to_vec(&request)?;
        if bytes.len() > LIMIT || !Self::budget(until) {
            return Ok(false);
        }
        match net::send(
            self.fd.as_ref().expect("verified live socket"),
            &bytes,
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        ) {
            Ok(n) if n == bytes.len() => {
                self.pending = Some(Pending {
                    request: id,
                    until,
                    operation,
                });
                if matches!(operation, Operation::Set { .. }) {
                    self.view.observation = None;
                    self.view.until_ns = 0;
                    self.view.requested_percent = None;
                    self.view.last_set_uncertain = true;
                }
                if now() >= until {
                    self.revoke();
                    return Err(io::ErrorKind::TimedOut.into());
                }
                Ok(true)
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => Ok(false),
            result => {
                // No partial packet is retained or resent. A successful full
                // packet followed by loss is handled through pending/revoke.
                self.view.pending = true;
                if matches!(operation, Operation::Set { .. }) {
                    self.view.last_set_uncertain = true;
                }
                self.revoke();
                Err(match result {
                    Err(e) => e.into(),
                    _ => io::ErrorKind::WriteZero.into(),
                })
            }
        }
    }
    pub fn inspect(&mut self, until: u64) -> io::Result<bool> {
        if self.pending.is_some() || !Self::budget(until) {
            return Ok(false);
        }
        if self.fd.is_none() {
            let Some(path) = &self.path else {
                return Ok(false);
            };
            let fd = net::socket_with(
                AddressFamily::UNIX,
                SocketType::SEQPACKET,
                SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
                None,
            )?;
            net::connect(&fd, &SocketAddrUnix::new(path)?)?;
            self.attach(fd)?;
        }
        self.send(until, Operation::Inspect)
    }
    pub fn set(
        &mut self,
        id: u64,
        percent: u8,
        expected_revision: u64,
        until: u64,
    ) -> io::Result<bool> {
        let state = self.snapshot();
        if id == 0
            || percent > 100
            || state.pending
            || !state.connected
            || expected_revision == 0
            || state.revision != expected_revision
            || !Self::budget(until)
        {
            return Ok(false);
        }
        let Some(observation) = state.observation else {
            return Ok(false);
        };
        self.send(
            until,
            Operation::Set {
                id,
                percent,
                expected_revision,
                max: observation.max,
            },
        )
    }
    fn receive(&self) -> io::Result<Option<Reply>> {
        let fd = self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?;
        let peer = self.peer.as_ref().ok_or(io::ErrorKind::NotConnected)?;
        let mut bytes = [0; LIMIT];
        let mut space =
            [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
        let mut ancillary = RecvAncillaryBuffer::new(&mut space);
        let message = match net::recvmsg(
            fd,
            &mut [IoSliceMut::new(&mut bytes)],
            &mut ancillary,
            RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
        ) {
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
            result => result?,
        };
        let mut credential = None;
        let mut invalid = message.bytes == 0
            || message
                .flags
                .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
        for entry in ancillary.drain() {
            match entry {
                RecvAncillaryMessage::ScmCredentials(c) if credential.is_none() => {
                    credential = Some(c)
                }
                _ => invalid = true,
            }
        }
        if invalid {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let c = credential.ok_or(io::ErrorKind::PermissionDenied)?;
        if c.pid != peer.pid || c.uid != peer.uid || c.gid != peer.gid {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Some(serde_json::from_slice(&bytes[..message.bytes])?))
    }
    pub fn poll(&mut self) -> Option<Event> {
        let Some(pending) = self.pending else {
            return (self.fd.is_some() && !self.idle()).then_some(Event::Unavailable);
        };
        if now() >= pending.until {
            self.revoke();
            return Some(Event::Unavailable);
        }
        let reply = match self.receive() {
            Ok(Some(reply)) => reply,
            Ok(None) => return None,
            Err(_) => {
                self.revoke();
                return Some(Event::Unavailable);
            }
        };
        if now() >= pending.until || reply.id != pending.request || !reply.valid(now()) {
            self.revoke();
            return Some(Event::Unavailable);
        }
        let event = match pending.operation {
            Operation::Inspect => {
                if reply.requested_percent.is_some() {
                    self.revoke();
                    return Some(Event::Unavailable);
                }
                Event::Observed
            }
            Operation::Set {
                id,
                percent,
                expected_revision,
                max,
            } => {
                if reply.ok {
                    if reply.pending
                        || reply.revision <= expected_revision
                        || reply.requested_percent != Some(percent)
                        || !reply.observation.is_some_and(|v| {
                            v.max == max && v.requested == crate::model::scaled(max, percent)
                        })
                    {
                        self.revoke();
                        return Some(Event::Unavailable);
                    }
                    Event::Completed {
                        id,
                        reply: reply.clone(),
                    }
                } else {
                    Event::Rejected { id }
                }
            }
        };
        self.pending = None;
        self.view.pending = reply.pending;
        self.view.revision = reply.revision;
        self.view.observation = if reply.ok { reply.observation } else { None };
        self.view.until_ns = if reply.ok {
            crate::protocol::decimal(&reply.until_ns).expect("validated deadline")
        } else {
            0
        };
        self.view.requested_percent = reply.requested_percent;
        if matches!(event, Event::Completed { .. }) {
            self.view.last_set_uncertain = false;
        } else {
            self.view.last_set_uncertain |= reply.last_set_uncertain;
        }
        Some(event)
    }
    pub fn revoke(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.view.pending = true;
            if matches!(pending.operation, Operation::Set { .. }) {
                self.view.last_set_uncertain = true;
            }
        }
        self.fd = None;
        self.peer = None;
        self.view.observation = None;
        self.view.until_ns = 0;
        self.view.requested_percent = None;
    }
}
