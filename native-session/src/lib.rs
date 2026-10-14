#![forbid(unsafe_code)]
//! Compositor facts only. No grants, app launch, unlock command or publication authority.
use pokelinux_kernel_identity::ProcessView;
use pokelinux_lifecycle::session::OwnedFocus;
use pokelinux_lifecycle::{authority::living, boottime_ns};
use rustix::net::{self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, UCred};
use rustix::{fd::OwnedFd, process::Pid};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
};
pub const SAMPLE_BUDGET_NS: u64 = pokelinux_lifecycle::session::SAMPLE_BUDGET_NS;
pub const MAX_FRAME: usize = 1024;
pub const MAX_DRAIN: usize = 16;
pub use pokelinux_lifecycle::session::{NativeFocus, ProtocolUnlock};
/// Mapped lock-surface peer facts. Neither mapping nor protocol completion is
/// proof of a secure frame, authentication or authority to unlock.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Locker {
    Unavailable,
    Mapped { pid: u32, uid: u32, gid: u32 },
}
impl Locker {
    fn metadata(&self) -> Option<NativeFocus> {
        match *self {
            Self::Unavailable => None,
            Self::Mapped { pid, uid, gid } => Some(NativeFocus { pid, uid, gid }),
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub source: String,
    pub sequence: u64,
    pub sampled_at_ns: u64,
    pub locked: bool,
    pub lock_completed: bool,
    pub focus: Option<NativeFocus>,
    pub locker: Locker,
    #[serde(deserialize_with = "required_option")]
    pub lock_generation: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub protocol_unlock: Option<ProtocolUnlock>,
}
// An explicit null is required. Serde's implicit missing-Option default must not
// silently accept earlier wire revisions.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
pub struct Observer {
    fd: Option<OwnedFd>,
    alive: OwnedFd,
    pid: Pid,
    uid: u32,
    source: String,
    current: Option<Frame>,
    focus: Option<OwnedFocus>,
    locker: Option<OwnedFocus>,
    view: ProcessView,
    sequence: u64,
    revision: u64,
    accepted_generation: u64,
    unlock_eligible: Option<u64>,
    failed: bool,
}
fn canonical(source: &str) -> bool {
    source.len() == 32
        && source
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Observer {
    /// The installer supplies the owned source pidfd and its matching launched identity.
    /// Source facts are not permission and must still match the actor's native app generation.
    pub fn new(
        fd: OwnedFd,
        alive: OwnedFd,
        pid: Pid,
        uid: u32,
        source: String,
    ) -> io::Result<Self> {
        if !canonical(&source)
            || rustix::net::sockopt::socket_type(&fd)? != rustix::net::SocketType::SEQPACKET
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        rustix::net::sockopt::set_socket_passcred(&fd, true)?;
        rustix::io::fcntl_setfd(&fd, rustix::io::FdFlags::CLOEXEC)?;
        rustix::io::fcntl_setfd(&alive, rustix::io::FdFlags::CLOEXEC)?;
        let view = ProcessView::open()?;
        if !view
            .matches(&alive, &alive, pid.as_raw_nonzero().get() as u32)
            .unwrap_or(false)
        {
            return Err(io::ErrorKind::NotConnected.into());
        }
        Ok(Self {
            fd: Some(fd),
            alive,
            pid,
            uid,
            source,
            current: None,
            focus: None,
            locker: None,
            view,
            sequence: 0,
            revision: 0,
            accepted_generation: 0,
            unlock_eligible: None,
            failed: false,
        })
    }
    fn fail(&mut self, kind: io::ErrorKind) -> io::Result<()> {
        self.failed = true;
        self.current = None;
        self.focus = None;
        self.locker = None;
        self.fd = None;
        Err(kind.into())
    }
    fn live_sample(frame: &Frame, now: u64) -> bool {
        frame.sampled_at_ns <= now
            && frame
                .sampled_at_ns
                .checked_add(SAMPLE_BUDGET_NS)
                .is_some_and(|until| now < until)
    }
    fn accept_protocol(&mut self, frame: &Frame) -> bool {
        if !pokelinux_lifecycle::session::valid_protocol_facts(
            frame.locked,
            frame.sequence,
            frame.sampled_at_ns,
            frame.lock_generation,
            frame.protocol_unlock.as_ref(),
        ) {
            return false;
        }
        let previous_unlock = self
            .current
            .as_ref()
            .and_then(|old| old.protocol_unlock.as_ref());
        if let Some(generation) = frame.lock_generation {
            let previous_generation = self.current.as_ref().and_then(|old| old.lock_generation);
            if previous_generation != Some(generation) {
                if self.accepted_generation.checked_add(1) != Some(generation) {
                    return false;
                }
                self.accepted_generation = generation;
            }
            self.unlock_eligible = Some(generation);
        } else if frame.locked {
            // A new unknown lock or destroyed association retires the earlier
            // operation even if its accepted successor has not mapped yet.
            self.unlock_eligible = None;
        }
        if let Some(event) = &frame.protocol_unlock {
            if let Some(previous) = previous_unlock {
                if previous != event {
                    return false;
                }
            } else {
                if self.unlock_eligible != Some(event.generation)
                    || event.generation != self.accepted_generation
                    || event.sequence != frame.sequence
                    || self
                        .current
                        .as_ref()
                        .is_none_or(|old| event.observed_at_ns < old.sampled_at_ns)
                {
                    return false;
                }
                self.unlock_eligible = None;
            }
        } else if previous_unlock.is_some() && !frame.locked {
            // Exact occurrences cannot disappear on ordinary unlocked heartbeats.
            return false;
        }
        true
    }
    pub fn poll(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(io::ErrorKind::NotConnected.into());
        }
        if !living(&self.alive).unwrap_or(false) {
            return self.fail(io::ErrorKind::NotConnected);
        }
        let mut count = 0;
        loop {
            let packet = match receive(self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?) {
                Ok(v) => v,
                Err(e) if e.raw_os_error() == Some(4) => {
                    return self.fail(io::ErrorKind::Interrupted);
                }
                Err(_) => return self.fail(io::ErrorKind::InvalidData),
            };
            let Some(SourcePacket { bytes, cred, peer }) = packet else {
                break;
            };
            count += 1;
            if count > MAX_DRAIN {
                return self.fail(io::ErrorKind::InvalidData);
            }
            if bytes.len() > MAX_FRAME
                || cred.pid != self.pid
                || cred.uid.as_raw() != self.uid
                || !living(&self.alive).unwrap_or(false)
            {
                return self.fail(io::ErrorKind::PermissionDenied);
            }
            let frame: Frame = match serde_json::from_slice(&bytes) {
                Ok(frame) => frame,
                Err(_) => return self.fail(io::ErrorKind::InvalidData),
            };
            let locker_metadata = frame.locker.metadata();
            if frame.source != self.source
                || self.sequence.checked_add(1) != Some(frame.sequence)
                || (frame.locked && frame.focus.is_some())
                || (!frame.locked && (frame.lock_completed || locker_metadata.is_some()))
                || frame
                    .focus
                    .as_ref()
                    .is_some_and(|f| f.pid == 0 || f.pid > i32::MAX as u32)
                // Timely authentic successors can already be queued when the
                // actor resumes. Require an uninterrupted, monotonic sample
                // chain before replacing its cached sample; never bridge a gap.
                || self.current.as_ref().is_some_and(|previous| {
                    frame.sampled_at_ns < previous.sampled_at_ns
                        || previous
                            .sampled_at_ns
                            .checked_add(SAMPLE_BUDGET_NS)
                            .is_none_or(|until| frame.sampled_at_ns > until)
                })
                || !Self::live_sample(&frame, boottime_ns())
            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            if (frame.focus.is_some() || locker_metadata.is_some()) != peer.is_some()
                || peer
                    .as_ref()
                    .is_some_and(|fd| !self.view.is_pidfd(fd).unwrap_or(false))
            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            // A mapped locker must name the actual live task behind the received
            // handle in this pinned procfs namespace. Focus keeps its existing
            // fact-only death/replacement semantics; actor grants revalidate it.
            if let Some(metadata) = &locker_metadata
                && !peer
                    .as_ref()
                    .is_some_and(|fd| self.view.matches(fd, fd, metadata.pid).unwrap_or(false))
            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            let (focus, locker) = match (frame.focus.clone(), locker_metadata, peer) {
                (Some(metadata), None, Some(fd)) => (Some(OwnedFocus::new(metadata, fd)), None),
                (None, Some(metadata), Some(fd)) => (None, Some(OwnedFocus::new(metadata, fd))),
                (None, None, None) => (None, None),
                _ => return self.fail(io::ErrorKind::InvalidData),
            };
            if !self.accept_protocol(&frame) {
                return self.fail(io::ErrorKind::InvalidData);
            }
            let same_focus = match (&self.focus, &focus) {
                (None, None) => true,
                (Some(left), Some(right)) => left.equivalent(right, &self.view),
                _ => false,
            };
            let same_locker = match (&self.locker, &locker) {
                (None, None) => true,
                (Some(left), Some(right)) => left.equivalent(right, &self.view),
                _ => false,
            };
            // Inspect every packet before coalescing. A lock/unlock or focus cycle
            // cannot disappear merely because the final facts equal old facts.
            if self.current.as_ref().is_none_or(|previous| {
                previous.locked != frame.locked
                    || previous.lock_completed != frame.lock_completed
                    || previous.lock_generation != frame.lock_generation
                    || previous.protocol_unlock != frame.protocol_unlock
                    || !same_focus
                    || !same_locker
            }) {
                let Some(revision) = self.revision.checked_add(1) else {
                    return self.fail(io::ErrorKind::InvalidData);
                };
                self.revision = revision;
            }
            self.sequence = frame.sequence;
            self.current = Some(frame);
            self.focus = focus;
            self.locker = locker;
        }
        if !living(&self.alive).unwrap_or(false) {
            return self.fail(io::ErrorKind::NotConnected);
        }
        if self
            .current
            .as_ref()
            .is_some_and(|f| !Self::live_sample(f, boottime_ns()))
        {
            return self.fail(io::ErrorKind::TimedOut);
        }
        Ok(())
    }
    /// Semantic change counter used by the single actor to fence old reads.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// A living source alone never extends the original sample's absolute deadline.
    /// Observed expiry/death is terminal, including before a later poll.
    pub fn snapshot(&mut self) -> Option<&Frame> {
        if self.failed {
            return None;
        }
        if !living(&self.alive).unwrap_or(false) {
            let _ = self.fail(io::ErrorKind::NotConnected);
            return None;
        }
        if self
            .current
            .as_ref()
            .is_some_and(|f| !Self::live_sample(f, boottime_ns()))
        {
            let _ = self.fail(io::ErrorKind::TimedOut);
            return None;
        }
        self.current.as_ref()
    }
}

/// Dedicated trusted source receiver. General app/policy IPC still rejects all
/// descriptors. Every ancillary handle is owned and dropped on rejection.
struct SourcePacket {
    bytes: Vec<u8>,
    cred: UCred,
    peer: Option<OwnedFd>,
}
fn receive(fd: &OwnedFd) -> io::Result<Option<SourcePacket>> {
    let mut bytes = [0; MAX_FRAME];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let result = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        result => result?,
    };
    let mut invalid = result.bytes == 0
        || result
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut credentials = None;
    let mut peer = None;
    let mut rights_messages = 0;
    for message in ancillary.drain() {
        match message {
            RecvAncillaryMessage::ScmCredentials(cred) if credentials.is_none() => {
                credentials = Some(cred)
            }
            RecvAncillaryMessage::ScmRights(handles) => {
                rights_messages += 1;
                for handle in handles {
                    if peer.is_none() {
                        peer = Some(handle);
                    } else {
                        invalid = true;
                    }
                }
            }
            _ => invalid = true,
        }
    }
    if invalid || rights_messages > 1 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(SourcePacket {
        bytes: bytes[..result.bytes].to_vec(),
        cred: credentials.ok_or(io::ErrorKind::PermissionDenied)?,
        peer,
    }))
}

impl pokelinux_lifecycle::session::Provider for Observer {
    fn stop(&mut self) {
        self.failed = true;
        self.current = None;
        self.focus = None;
        self.locker = None;
        self.fd = None;
    }
    fn occupied(&self) -> bool {
        // The compositor is externally owned. Only retained local handles exist.
        false
    }
    fn poll(&mut self) -> io::Result<Option<pokelinux_lifecycle::session::Snapshot>> {
        Observer::poll(self)?;
        let frame = self.snapshot().cloned();
        if self.failed {
            // Expiry between the bounded drain and this final observation is
            // terminal for resource-owning providers as well as data admission.
            return Err(io::ErrorKind::NotConnected.into());
        }
        Ok(frame.map(|frame| pokelinux_lifecycle::session::Snapshot {
            source: frame.source.clone(),
            revision: self.revision,
            sequence: frame.sequence,
            sampled_at_ns: frame.sampled_at_ns,
            // The original sample is checked before it reaches this conversion.
            until: frame.sampled_at_ns + SAMPLE_BUDGET_NS,
            locked: frame.locked,
            focus: self.focus.clone(),
            locker: self.locker.clone(),
            lock_generation: frame.lock_generation,
            protocol_unlock: frame.protocol_unlock.clone(),
        }))
    }
}
