//! One nonblocking request at a time; no worker, background Set retry or UI policy.
pub mod ffi;
use rustix::{
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::{Path, PathBuf},
};
const BUDGET: u64 = 900_000_000;
const REFRESH: u64 = 250_000_000;
const LIMIT: usize = 4096;
fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (t.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(t.tv_nsec as u64)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    pub allowed: Option<bool>,
    pub pending: bool,
    pub can_set: bool,
    pub unconfirmed: bool,
}
#[derive(Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    PermissionState {
        target: String,
        permission: String,
        allowed: bool,
        owner: String,
        revision: u64,
    },
    Denied,
    Unavailable,
}
#[derive(Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request<'a> {
    PermissionStatus {
        deadline_ns: u64,
    },
    PermissionSet {
        allowed: bool,
        expected_owner: &'a str,
        expected_revision: u64,
        deadline_ns: u64,
    },
}
struct Confirmed {
    allowed: bool,
    owner: String,
    revision: u64,
    until: u64,
}
enum Kind {
    Status,
    Set {
        allowed: bool,
        owner: String,
        revision: u64,
    },
}
struct Pending {
    fd: OwnedFd,
    peer: UCred,
    wire: Vec<u8>,
    sent: bool,
    kind: Kind,
    until: u64,
    dispatch_until: u64,
}
impl Pending {
    fn setting(&self) -> bool {
        matches!(self.kind, Kind::Set { .. })
    }
}
pub struct Client {
    path: PathBuf,
    active: bool,
    confirmed: Option<Confirmed>,
    pending: Option<Pending>,
    next_refresh: u64,
    unconfirmed: bool,
}
impl Client {
    pub fn new(path: &Path) -> io::Result<Self> {
        if rustix::process::getuid().as_raw() != 11045
            || rustix::process::geteuid().as_raw() != 11045
            || rustix::process::getgid().as_raw() != 11045
            || rustix::process::getegid().as_raw() != 11045
            || !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        SocketAddrUnix::new(path)?;
        Ok(Self {
            path: path.into(),
            active: false,
            confirmed: None,
            pending: None,
            next_refresh: 0,
            unconfirmed: false,
        })
    }
    pub fn active(&mut self, value: bool) {
        if self.active == value {
            return;
        }
        self.active = value;
        if !value {
            self.fail(now())
        } else {
            self.next_refresh = 0;
        }
    }
    fn expire(&mut self, t: u64) {
        if self.confirmed.as_ref().is_some_and(|s| t >= s.until) {
            self.confirmed = None;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| t >= p.until || !p.sent && t >= p.dispatch_until)
        {
            self.fail(t);
        }
    }
    fn fail(&mut self, t: u64) {
        if self.pending.as_ref().is_some_and(|p| p.sent && p.setting()) {
            self.unconfirmed = true;
        }
        self.pending = None;
        self.confirmed = None;
        self.next_refresh = t.saturating_add(REFRESH);
    }
    fn open(&self, kind: Kind, until: u64, dispatch_until: u64) -> io::Result<Pending> {
        let wire = serde_json::to_vec(&match &kind {
            Kind::Status => Request::PermissionStatus { deadline_ns: until },
            Kind::Set {
                allowed,
                owner,
                revision,
            } => Request::PermissionSet {
                allowed: *allowed,
                expected_owner: owner,
                expected_revision: *revision,
                deadline_ns: until,
            },
        })?;
        let fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        net::sockopt::set_socket_passcred(&fd, true)?;
        // A failed/in-progress connect is never retried for an explicit choice.
        net::connect(&fd, &SocketAddrUnix::new(&self.path)?)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        if peer.uid.as_raw() != 0 || peer.gid.as_raw() != 0 || now() >= dispatch_until {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Pending {
            fd,
            peer,
            wire,
            sent: false,
            kind,
            until,
            dispatch_until,
        })
    }
    pub fn choose(&mut self, allowed: bool) -> bool {
        let t = now();
        self.expire(t);
        if !self.active || self.pending.as_ref().is_some_and(Pending::setting) {
            return false;
        }
        let Some(s) = self.confirmed.as_ref() else {
            return false;
        };
        if s.revision == u64::MAX {
            return false;
        }
        let until = t.saturating_add(BUDGET);
        let kind = Kind::Set {
            allowed,
            owner: s.owner.clone(),
            revision: s.revision,
        };
        let dispatch_until = until.min(s.until);
        // Retire only the read-only refresh. Its socket and any late response
        // cannot overwrite this explicit choice; an outstanding Set is never replaced.
        self.pending = None;
        let pending = self.open(kind, until, dispatch_until);
        self.confirmed = None;
        match pending {
            Ok(p) => {
                self.pending = Some(p);
                self.unconfirmed = false;
                true
            }
            Err(_) => {
                self.fail(now());
                false
            }
        }
    }
    pub fn view(&mut self) -> View {
        self.expire(now());
        View {
            allowed: self.confirmed.as_ref().map(|s| s.allowed),
            pending: self.pending.is_some(),
            can_set: self.active
                && self.pending.as_ref().is_none_or(|p| !p.setting())
                && self
                    .confirmed
                    .as_ref()
                    .is_some_and(|s| s.revision < u64::MAX),
            unconfirmed: self.unconfirmed,
        }
    }
    pub fn tick(&mut self) {
        let t = now();
        self.expire(t);
        if !self.active {
            return;
        }
        if self.pending.is_none() && t >= self.next_refresh {
            let until = t.saturating_add(BUDGET);
            match self.open(Kind::Status, until, until) {
                Ok(p) => self.pending = Some(p),
                Err(_) => {
                    self.fail(now());
                    return;
                }
            }
        }
        let Some(p) = self.pending.as_mut() else {
            return;
        };
        if !p.sent {
            if now() >= p.dispatch_until {
                self.fail(now());
                return;
            }
            match net::send(&p.fd, &p.wire, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
                Ok(n) if n == p.wire.len() => p.sent = true,
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return,
                _ => {
                    self.fail(now());
                    return;
                }
            }
        }
        let response = receive(p);
        match response {
            Ok(None) => {}
            Ok(Some(reply)) => {
                let t = now();
                if t >= p.until {
                    self.fail(t);
                    return;
                }
                let Reply::PermissionState {
                    target,
                    permission,
                    allowed,
                    owner,
                    revision,
                } = reply
                else {
                    self.fail(t);
                    return;
                };
                let valid = target == "org.omarchy.Sounds"
                    && permission == "media_route"
                    && owner.len() == 32
                    && owner
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    && match &p.kind {
                        Kind::Status => true,
                        Kind::Set {
                            allowed: a,
                            owner: o,
                            revision: r,
                        } => allowed == *a && owner == *o && r.checked_add(1) == Some(revision),
                    };
                if !valid || now() >= p.until {
                    self.fail(now());
                    return;
                }
                let until = p.until;
                if p.setting() {
                    self.unconfirmed = false;
                }
                self.pending = None;
                self.confirmed = Some(Confirmed {
                    allowed,
                    owner,
                    revision,
                    until,
                });
                self.next_refresh = t.saturating_add(REFRESH);
            }
            Err(_) => self.fail(now()),
        }
    }
}
fn receive(p: &Pending) -> io::Result<Option<Reply>> {
    let mut bytes = [0; LIMIT];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let m = match net::recvmsg(
        &p.fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        r => r?,
    };
    let mut cred = None;
    let mut invalid = m.bytes == 0 || m.flags.intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for a in ancillary.drain() {
        match a {
            RecvAncillaryMessage::ScmCredentials(c) if cred.is_none() => cred = Some(c),
            _ => invalid = true,
        }
    }
    if invalid || cred != Some(p.peer) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(Some(serde_json::from_slice(&bytes[..m.bytes])?))
}
