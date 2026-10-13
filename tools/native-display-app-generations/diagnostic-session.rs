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

// DIAGNOSTIC SOURCE COPY ONLY. No production hook or Source-frame alteration.
// Fixed112-byte nonblocking records, emitted only on rejection/terminal stop.
struct Diagnostic {
    fd: OwnedFd,
    attempted: u64,
    sent: u64,
    dropped: u64,
    summary: bool,
}
#[derive(Clone, Copy, Default)]
struct TraceFields {
    previous_sequence: u64,
    previous_sample: u64,
    sequence: u64,
    sample: u64,
    examined: u64,
    drain: u32,
    errno: i32,
    bytes: u32,
    rights_messages: u32,
    rights_handles: u32,
    recv_flags: u32,
    flags: u32,
}
impl Diagnostic {
    fn new() -> io::Result<Self> {
        let path = std::env::var_os("POKELINUX_SESSION_DIAGNOSTIC_TRACE")
            .ok_or(io::ErrorKind::InvalidInput)?;
        let fd = net::socket_with(
            net::AddressFamily::UNIX,
            net::SocketType::DGRAM,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
            None,
        )?;
        net::connect(&fd, &net::SocketAddrUnix::new(std::path::Path::new(&path))?)?;
        Ok(Self {
            fd,
            attempted: 0,
            sent: 0,
            dropped: 0,
            summary: false,
        })
    }
    fn record(&mut self, category: u16, f: TraceFields) {
        self.attempted = self.attempted.saturating_add(1);
        let mut bytes = [0u8; 112];
        bytes[..8].copy_from_slice(b"PKSDIAG1");
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        bytes[10..12].copy_from_slice(&category.to_le_bytes());
        bytes[12..16].copy_from_slice(&f.flags.to_le_bytes());
        let mut offset = 16;
        for value in [
            boottime_ns(),
            f.previous_sequence,
            f.previous_sample,
            f.sequence,
            f.sample,
            f.examined,
        ] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            offset += 8;
        }
        for value in [
            f.drain,
            f.errno as u32,
            f.bytes,
            f.rights_messages,
            f.rights_handles,
            f.recv_flags,
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }
        for value in [self.attempted, self.sent, self.dropped] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            offset += 8;
        }
        match net::send(
            &self.fd,
            &bytes,
            net::SendFlags::DONTWAIT | net::SendFlags::NOSIGNAL,
        ) {
            Ok(112) => self.sent = self.sent.saturating_add(1),
            _ => self.dropped = self.dropped.saturating_add(1),
        }
    }
    fn finish(&mut self) {
        if !self.summary {
            self.summary = true;
            self.record(0, TraceFields::default());
        }
    }
}

pub struct Observer {
    fd: Option<OwnedFd>,
    alive: OwnedFd,
    pid: Pid,
    uid: u32,
    source: String,
    current: Option<Frame>,
    focus: Option<OwnedFocus>,
    view: ProcessView,
    sequence: u64,
    revision: u64,
    failed: bool,
    diagnostic: Diagnostic,
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
            view,
            sequence: 0,
            revision: 0,
            failed: false,
            diagnostic: Diagnostic::new()?,
        })
    }
    fn fail(&mut self, kind: io::ErrorKind) -> io::Result<()> {
        self.failed = true;
        self.current = None;
        self.focus = None;
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
        let mut count = 0;
        loop {
            let context = TraceFields {
                previous_sequence: self.sequence,
                previous_sample: self.current.as_ref().map_or(0, |v| v.sampled_at_ns),
                drain: count as u32,
                ..TraceFields::default()
            };
            let packet = match receive(
                self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?,
                &mut self.diagnostic,
                context,
            ) {
                Ok(v) => v,
                Err(e) if e.raw_os_error() == Some(4) => {
                    return self.fail(io::ErrorKind::Interrupted);
                }
                Err(e) => {
                    self.diagnostic.record(
                        1,
                        TraceFields {
                            errno: e.raw_os_error().unwrap_or(0),
                            ..context
                        },
                    );
                    return self.fail(io::ErrorKind::InvalidData);
                }
            };
            let Some(SourcePacket { bytes, cred, peer }) = packet else {
                break;
            };
            count += 1;
            if count > MAX_DRAIN {
                self.diagnostic.record(
                    2,
                    TraceFields {
                        drain: count as u32,
                        bytes: bytes.len() as u32,
                        ..context
                    },
                );
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
                Err(_) => {
                    self.diagnostic.record(
                        3,
                        TraceFields {
                            drain: count as u32,
                            bytes: bytes.len() as u32,
                            ..context
                        },
                    );
                    return self.fail(io::ErrorKind::InvalidData);
                }
            };
            let mut examined = 0;
            if frame.source != self.source
                || self.sequence.checked_add(1) != Some(frame.sequence)
                || (frame.locked && frame.focus.is_some())
                || (!frame.locked && frame.lock_completed)
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
                || !Self::live_sample(&frame, { examined = boottime_ns(); examined })
            {
                // Classify only after the unchanged composite guard rejected.
                // Static fields and the exact original live_sample examination
                // time are reused; no second live clock/authority check occurs.
                let category = if frame.source != self.source {
                    4
                } else if self.sequence.checked_add(1) != Some(frame.sequence) {
                    5
                } else if frame.locked && frame.focus.is_some() {
                    6
                } else if !frame.locked && frame.lock_completed {
                    7
                } else if frame
                    .focus
                    .as_ref()
                    .is_some_and(|f| f.pid == 0 || f.pid > i32::MAX as u32)
                {
                    8
                } else if self
                    .current
                    .as_ref()
                    .is_some_and(|p| frame.sampled_at_ns < p.sampled_at_ns)
                {
                    9
                } else if self.current.as_ref().is_some_and(|p| {
                    p.sampled_at_ns
                        .checked_add(SAMPLE_BUDGET_NS)
                        .is_none_or(|until| frame.sampled_at_ns > until)
                }) {
                    10
                } else if frame.sampled_at_ns > examined {
                    11
                } else {
                    12
                };
                self.diagnostic.record(
                    category,
                    TraceFields {
                        sequence: frame.sequence,
                        sample: frame.sampled_at_ns,
                        examined,
                        drain: count as u32,
                        bytes: bytes.len() as u32,
                        ..context
                    },
                );
                return self.fail(io::ErrorKind::InvalidData);
            }
            let mut type_examined = false;
            let mut typed = false;
            if frame.focus.is_some() != peer.is_some()
                || peer.as_ref().is_some_and(|fd| {
                    type_examined = true;
                    typed = self.view.is_pidfd(fd).unwrap_or(false);
                    !typed
                })
            {
                self.diagnostic.record(
                    if type_examined { 14 } else { 13 },
                    TraceFields {
                        sequence: frame.sequence,
                        sample: frame.sampled_at_ns,
                        drain: count as u32,
                        flags: u32::from(frame.focus.is_some())
                            | (u32::from(peer.is_some()) << 1)
                            | (u32::from(type_examined) << 2)
                            | (u32::from(typed) << 3),
                        ..context
                    },
                );
                return self.fail(io::ErrorKind::InvalidData);
            }
            let focus = frame
                .focus
                .clone()
                .zip(peer)
                .map(|(metadata, fd)| OwnedFocus::new(metadata, fd));
            let same_focus = match (&self.focus, &focus) {
                (None, None) => true,
                (Some(left), Some(right)) => left.equivalent(right, &self.view),
                _ => false,
            };
            // Inspect every packet before coalescing. A lock/unlock or focus cycle
            // cannot disappear merely because the final facts equal old facts.
            if self.current.as_ref().is_none_or(|previous| {
                previous.locked != frame.locked
                    || previous.lock_completed != frame.lock_completed
                    || !same_focus
            }) {
                let Some(revision) = self.revision.checked_add(1) else {
                    self.diagnostic.record(
                        15,
                        TraceFields {
                            sequence: frame.sequence,
                            sample: frame.sampled_at_ns,
                            drain: count as u32,
                            ..context
                        },
                    );
                    return self.fail(io::ErrorKind::InvalidData);
                };
                self.revision = revision;
            }
            self.sequence = frame.sequence;
            self.current = Some(frame);
            self.focus = focus;
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
fn receive(
    fd: &OwnedFd,
    diagnostic: &mut Diagnostic,
    context: TraceFields,
) -> io::Result<Option<SourcePacket>> {
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
    let mut shape = u32::from(result.bytes == 0)
        | (u32::from(result.flags.contains(ReturnFlags::TRUNC)) << 1)
        | (u32::from(result.flags.contains(ReturnFlags::CTRUNC)) << 2);
    let mut handles_count = 0;
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
                    handles_count += 1;
                    if peer.is_none() {
                        peer = Some(handle);
                    } else {
                        invalid = true;
                        shape |= 1 << 3;
                    }
                }
            }
            _ => {
                invalid = true;
                shape |= 1 << 4;
            }
        }
    }
    if invalid || rights_messages > 1 {
        diagnostic.record(
            16,
            TraceFields {
                bytes: result.bytes as u32,
                rights_messages,
                rights_handles: handles_count,
                recv_flags: result.flags.bits() as u32,
                flags: shape | (u32::from(rights_messages > 1) << 5),
                ..context
            },
        );
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(SourcePacket {
        bytes: bytes[..result.bytes].to_vec(),
        cred: credentials.ok_or_else(|| {
            diagnostic.record(
                17,
                TraceFields {
                    bytes: result.bytes as u32,
                    rights_messages,
                    rights_handles: handles_count,
                    recv_flags: result.flags.bits() as u32,
                    ..context
                },
            );
            io::ErrorKind::PermissionDenied
        })?,
        peer,
    }))
}

impl pokelinux_lifecycle::session::Provider for Observer {
    fn stop(&mut self) {
        self.diagnostic.finish();
        self.failed = true;
        self.current = None;
        self.focus = None;
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
        }))
    }
}
