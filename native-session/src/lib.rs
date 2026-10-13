#![forbid(unsafe_code)]
//! Compositor facts only. No grants, app launch, unlock command or publication authority.
use pokelinux_lifecycle::{authority::living, boottime_ns, ipc};
use rustix::{fd::OwnedFd, process::Pid};
use serde::{Deserialize, Serialize};
use std::io;
pub const SAMPLE_BUDGET_NS: u64 = pokelinux_lifecycle::session::SAMPLE_BUDGET_NS;
pub const MAX_FRAME: usize = 1024;
pub const MAX_DRAIN: usize = 16;
pub use pokelinux_lifecycle::session::NativeFocus;
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub source: String,
    pub sequence: u64,
    pub sampled_at_ns: u64,
    pub locked: bool,
    pub lock_completed: bool,
    pub focus: Option<NativeFocus>,
}
pub struct Observer {
    fd: Option<OwnedFd>,
    alive: OwnedFd,
    pid: Pid,
    uid: u32,
    source: String,
    current: Option<Frame>,
    sequence: u64,
    revision: u64,
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
        if !living(&alive)? {
            return Err(io::ErrorKind::NotConnected.into());
        }
        Ok(Self {
            fd: Some(fd),
            alive,
            pid,
            uid,
            source,
            current: None,
            sequence: 0,
            revision: 0,
            failed: false,
        })
    }
    fn fail(&mut self, kind: io::ErrorKind) -> io::Result<()> {
        self.failed = true;
        self.current = None;
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
    pub fn poll(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(io::ErrorKind::NotConnected.into());
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
        let mut count = 0;
        loop {
            let packet = match ipc::receive(self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?) {
                Ok(v) => v,
                Err(e) if e.raw_os_error() == Some(4) => {
                    return self.fail(io::ErrorKind::Interrupted);
                }
                Err(_) => return self.fail(io::ErrorKind::InvalidData),
            };
            let Some((bytes, cred)) = packet else {
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
            if frame.source != self.source
                || self.sequence.checked_add(1) != Some(frame.sequence)
                || (frame.locked && frame.focus.is_some())
                || (!frame.locked && frame.lock_completed)
                || frame
                    .focus
                    .as_ref()
                    .is_some_and(|f| f.pid == 0 || f.pid > i32::MAX as u32 || f.pidfs_inode == 0)
                || !Self::live_sample(&frame, boottime_ns())
            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            // Inspect every packet before coalescing. A lock/unlock or focus cycle
            // cannot disappear merely because the final facts equal old facts.
            if self.current.as_ref().is_none_or(|previous| {
                previous.locked != frame.locked
                    || previous.lock_completed != frame.lock_completed
                    || previous.focus != frame.focus
            }) {
                let Some(revision) = self.revision.checked_add(1) else {
                    return self.fail(io::ErrorKind::InvalidData);
                };
                self.revision = revision;
            }
            self.sequence = frame.sequence;
            self.current = Some(frame);
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
    pub fn snapshot(&self) -> Option<&Frame> {
        if self.failed || !living(&self.alive).unwrap_or(false) {
            return None;
        }
        self.current
            .as_ref()
            .filter(|f| Self::live_sample(f, boottime_ns()))
    }
}

impl pokelinux_lifecycle::session::Provider for Observer {
    fn poll(&mut self) -> io::Result<Option<pokelinux_lifecycle::session::Snapshot>> {
        Observer::poll(self)?;
        Ok(self
            .snapshot()
            .map(|frame| pokelinux_lifecycle::session::Snapshot {
                source: frame.source.clone(),
                revision: self.revision,
                sequence: frame.sequence,
                sampled_at_ns: frame.sampled_at_ns,
                // The original sample is checked before it reaches this conversion.
                until: frame.sampled_at_ns + SAMPLE_BUDGET_NS,
                locked: frame.locked,
                focus: frame.focus.clone(),
            }))
    }
}
