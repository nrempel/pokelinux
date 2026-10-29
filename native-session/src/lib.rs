#![forbid(unsafe_code)]
//! Compositor facts only. No grants, app launch, unlock command or publication authority.
use pokelinux_kernel_identity::ProcessView;
use pokelinux_lifecycle::session::{
    AcceptedSourceSample, OwnedFocus, SourceReceiveObservation, SourceRejection,
    SourceRejectionReason as Reason, SourceRejectionStage as Stage, TimingLoss, TimingLossReason,
    Update,
};
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
    // Wire/protocol history remains in current even when startup timing expires.
    candidate_live: bool,
    pair_first_sampled_at: Option<u64>,
    loss_boundary: Option<u64>,
    pending_loss: Option<TimingLoss>,
    first_export_sequence: Option<u64>,
    exported: Option<Frame>,
    focus: Option<OwnedFocus>,
    locker: Option<OwnedFocus>,
    view: ProcessView,
    sequence: u64,
    revision: u64,
    accepted_generation: u64,
    unlock_eligible: Option<u64>,
    failed: bool,
    first_rejection: Option<SourceRejection>,
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
            candidate_live: false,
            pair_first_sampled_at: None,
            loss_boundary: None,
            pending_loss: None,
            first_export_sequence: None,
            exported: None,
            focus: None,
            locker: None,
            view,
            sequence: 0,
            revision: 0,
            accepted_generation: 0,
            unlock_eligible: None,
            failed: false,
            first_rejection: None,
        })
    }
    fn fail(
        &mut self,
        kind: io::ErrorKind,
        stage: Stage,
        reason: Reason,
        context: RejectionContext,
    ) -> io::Result<()> {
        if self.first_rejection.is_none() {
            self.first_rejection = Some(SourceRejection {
                stage,
                reason,
                drain_ordinal: context.drain_ordinal,
                sequence: context.sample.as_ref().map(|sample| sample.sequence),
                sampled_at_ns: context.sample.as_ref().map(|sample| sample.sampled_at_ns),
                previous_accepted: self.current.as_ref().map(sample_metadata),
                validated_at_boottime_ns: context.validated_at.unwrap_or_else(boottime_ns),
                receive: context.receive,
            });
        }
        self.failed = true;
        self.candidate_live = false;
        self.pair_first_sampled_at = None;
        self.first_export_sequence = None;
        self.pending_loss = None;
        self.current = None;
        self.exported = None;
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
    fn reset_candidate(&mut self) {
        self.candidate_live = false;
        self.pair_first_sampled_at = None;
    }
    fn lose_authority(&mut self, now: u64, reason: TimingLossReason) {
        if self.first_export_sequence.take().is_some() {
            self.loss_boundary = Some(now);
            self.pending_loss = Some(TimingLoss {
                source: self.source.clone(),
                observed_at_ns: now,
                reason,
            });
        }
        self.exported = None;
        self.reset_candidate();
        // Raw sequence, generation and unlock history remain intact. Only a
        // later export can start a new authentication-eligible sequence range.
    }
    fn refresh_timing(&mut self, now: u64) {
        // Inspect the actual exported lease before draining newer packets.
        // Queued frames cannot conceal a lapse in already exported authority.
        if self.first_export_sequence.is_some()
            && self
                .exported
                .as_ref()
                .is_none_or(|frame| !Self::live_sample(frame, now))
        {
            self.lose_authority(now, TimingLossReason::SampleExpired);
        }
        if self.first_export_sequence.is_none() {
            if self
                .current
                .as_ref()
                .is_none_or(|frame| !Self::live_sample(frame, now))
            {
                self.reset_candidate();
            } else if self.pair_first_sampled_at.is_some_and(|sampled| {
                sampled
                    .checked_add(SAMPLE_BUDGET_NS)
                    .is_none_or(|until| now >= until)
            }) {
                self.pair_first_sampled_at = None;
            }
        }
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
            return self.fail(
                io::ErrorKind::NotConnected,
                Stage::Liveness,
                Reason::SourceNotLiving,
                RejectionContext::default(),
            );
        }
        self.refresh_timing(boottime_ns());
        let mut count = 0;
        loop {
            let mut context = RejectionContext {
                drain_ordinal: count + 1,
                ..Default::default()
            };
            let packet = match receive(self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?) {
                Ok(v) => v,
                Err(error) => {
                    let kind = if error.observation.errno == Some(4) {
                        io::ErrorKind::Interrupted
                    } else {
                        io::ErrorKind::InvalidData
                    };
                    context.receive = Some(error.observation);
                    return self.fail(kind, Stage::Receive, error.reason, context);
                }
            };
            let Some(SourcePacket {
                bytes,
                cred,
                peer,
                observation,
            }) = packet
            else {
                break;
            };
            context.receive = Some(observation);
            count += 1;
            if count > MAX_DRAIN as u32 {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Drain,
                    Reason::DrainLimit,
                    context,
                );
            }
            if bytes.len() > MAX_FRAME {
                return self.fail(
                    io::ErrorKind::PermissionDenied,
                    Stage::Validation,
                    Reason::FrameSize,
                    context,
                );
            }
            if cred.pid != self.pid || cred.uid.as_raw() != self.uid {
                return self.fail(
                    io::ErrorKind::PermissionDenied,
                    Stage::Validation,
                    Reason::SenderCredentials,
                    context,
                );
            }
            if !living(&self.alive).unwrap_or(false) {
                return self.fail(
                    io::ErrorKind::PermissionDenied,
                    Stage::Liveness,
                    Reason::SourceNotLiving,
                    context,
                );
            }
            let frame: Frame = match serde_json::from_slice(&bytes) {
                Ok(frame) => frame,
                Err(_) => {
                    return self.fail(
                        io::ErrorKind::InvalidData,
                        Stage::Decode,
                        Reason::MalformedFrame,
                        context,
                    );
                }
            };
            context.sample = Some(sample_metadata(&frame));
            let locker_metadata = frame.locker.metadata();
            if frame.source != self.source {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Validation,
                    Reason::SourceMismatch,
                    context,
                );
            }
            if self.sequence.checked_add(1) != Some(frame.sequence) {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Validation,
                    Reason::Sequence,
                    context,
                );
            }
            if (frame.locked && frame.focus.is_some())
                || (!frame.locked && (frame.lock_completed || locker_metadata.is_some()))
                || frame
                    .focus
                    .as_ref()
                    .is_some_and(|f| f.pid == 0 || f.pid > i32::MAX as u32)
            {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Validation,
                    Reason::FrameShape,
                    context,
                );
            }
            // Sequence, timestamp and protocol history survive every timing loss.
            let mut continuous = false;
            let mut gap = false;
            if let Some(previous) = &self.current {
                if frame.sampled_at_ns < previous.sampled_at_ns {
                    return self.fail(
                        io::ErrorKind::InvalidData,
                        Stage::Validation,
                        Reason::SampleBackwards,
                        context,
                    );
                }
                gap = previous
                    .sampled_at_ns
                    .checked_add(SAMPLE_BUDGET_NS)
                    .is_none_or(|until| frame.sampled_at_ns > until);
                continuous = !gap && frame.sampled_at_ns > previous.sampled_at_ns;
            }
            let now = boottime_ns();
            context.validated_at = Some(now);
            if frame.sampled_at_ns > now {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Validation,
                    Reason::SampleFuture,
                    context,
                );
            }
            if (frame.focus.is_some() || locker_metadata.is_some()) != peer.is_some()
                || peer
                    .as_ref()
                    .is_some_and(|fd| !self.view.is_pidfd(fd).unwrap_or(false))
            {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Peer,
                    Reason::PeerHandle,
                    context,
                );
            }
            if let Some(metadata) = &locker_metadata
                && !peer
                    .as_ref()
                    .is_some_and(|fd| self.view.matches(fd, fd, metadata.pid).unwrap_or(false))
            {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Peer,
                    Reason::LockerIdentity,
                    context,
                );
            }
            let (focus, locker) = match (frame.focus.clone(), locker_metadata, peer) {
                (Some(metadata), None, Some(fd)) => (Some(OwnedFocus::new(metadata, fd)), None),
                (None, Some(metadata), Some(fd)) => (None, Some(OwnedFocus::new(metadata, fd))),
                (None, None, None) => (None, None),
                _ => {
                    return self.fail(
                        io::ErrorKind::InvalidData,
                        Stage::Peer,
                        Reason::PeerHandle,
                        context,
                    );
                }
            };
            if !self.accept_protocol(&frame) {
                return self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Protocol,
                    Reason::ProtocolFacts,
                    context,
                );
            }
            let now = boottime_ns();
            self.refresh_timing(now);
            let fresh = Self::live_sample(&frame, now);
            if gap || !fresh {
                self.lose_authority(
                    now,
                    if gap {
                        TimingLossReason::SampleGap
                    } else {
                        TimingLossReason::SampleExpired
                    },
                );
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
            // Inspect every packet before coalescing; preserve the existing semantic fence.
            if self.current.as_ref().is_none_or(|previous| {
                previous.locked != frame.locked
                    || previous.lock_completed != frame.lock_completed
                    || previous.lock_generation != frame.lock_generation
                    || previous.protocol_unlock != frame.protocol_unlock
                    || !same_focus
                    || !same_locker
            }) {
                let Some(revision) = self.revision.checked_add(1) else {
                    return self.fail(
                        io::ErrorKind::InvalidData,
                        Stage::Revision,
                        Reason::RevisionOverflow,
                        context,
                    );
                };
                self.revision = revision;
            }
            self.sequence = frame.sequence;
            if self.first_export_sequence.is_none() {
                let eligible = fresh
                    && self
                        .loss_boundary
                        .is_none_or(|floor| frame.sampled_at_ns > floor);
                self.pair_first_sampled_at = self.current.as_ref().and_then(|previous| {
                    (self.candidate_live
                        && continuous
                        && eligible
                        && Self::live_sample(previous, now)
                        && self
                            .loss_boundary
                            .is_none_or(|floor| previous.sampled_at_ns > floor))
                    .then_some(previous.sampled_at_ns)
                });
                self.candidate_live = eligible;
            }
            self.current = Some(frame);
            self.focus = focus;
            self.locker = locker;
        }
        if !living(&self.alive).unwrap_or(false) {
            return self.fail(
                io::ErrorKind::NotConnected,
                Stage::Liveness,
                Reason::SourceNotLiving,
                RejectionContext::default(),
            );
        }
        self.refresh_timing(boottime_ns());
        Ok(())
    }
    /// Semantic change counter used by the single actor to fence old reads.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// A living source alone never extends the original sample's absolute deadline.
    /// A fresh advancing pair establishes or reestablishes authority. Timing
    /// loss must be consumed through Provider before any replacement export.
    /// Source death and malformed wire/protocol history remain terminal.
    pub fn snapshot(&mut self) -> Option<&Frame> {
        if self.failed {
            return None;
        }
        if !living(&self.alive).unwrap_or(false) {
            let _ = self.fail(
                io::ErrorKind::NotConnected,
                Stage::Liveness,
                Reason::SourceNotLiving,
                RejectionContext::default(),
            );
            return None;
        }
        self.refresh_timing(boottime_ns());
        if self.pending_loss.is_some() {
            return None;
        }
        let frame = self.current.as_ref()?;
        if self.first_export_sequence.is_none() && self.pair_first_sampled_at.is_none() {
            return None;
        }
        // Recovery changes exported eligibility even when raw facts are identical.
        // Commit this semantic fence exactly once with the replacement export.
        if self.first_export_sequence.is_none() && self.loss_boundary.is_some() {
            let Some(revision) = self.revision.checked_add(1) else {
                let context = RejectionContext {
                    sample: Some(sample_metadata(frame)),
                    validated_at: Some(boottime_ns()),
                    ..Default::default()
                };
                let _ = self.fail(
                    io::ErrorKind::InvalidData,
                    Stage::Revision,
                    Reason::RevisionOverflow,
                    context,
                );
                return None;
            };
            self.revision = revision;
        }
        // Commit only at export, never partway through draining queued frames.
        let first = *self.first_export_sequence.get_or_insert(frame.sequence);
        let mut exported = frame.clone();
        // Validate old occurrences against raw wire history, but never turn an
        // occurrence from before authority existed into authentication evidence.
        if exported
            .protocol_unlock
            .as_ref()
            .is_some_and(|event| event.sequence <= first)
        {
            exported.protocol_unlock = None;
        }
        self.exported = Some(exported);
        self.exported.as_ref()
    }
}
#[derive(Default)]
struct RejectionContext {
    drain_ordinal: u32,
    sample: Option<AcceptedSourceSample>,
    receive: Option<SourceReceiveObservation>,
    validated_at: Option<u64>,
}
fn sample_metadata(frame: &Frame) -> AcceptedSourceSample {
    AcceptedSourceSample {
        sequence: frame.sequence,
        sampled_at_ns: frame.sampled_at_ns,
    }
}

/// Dedicated trusted source receiver. General app/policy IPC still rejects all
/// descriptors. Every ancillary handle is owned and dropped on rejection.
struct SourcePacket {
    bytes: Vec<u8>,
    cred: UCred,
    peer: Option<OwnedFd>,
    observation: SourceReceiveObservation,
}
struct ReceiveRejection {
    reason: Reason,
    observation: SourceReceiveObservation,
}
fn receive(fd: &OwnedFd) -> Result<Option<SourcePacket>, ReceiveRejection> {
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
        Err(error) => {
            return Err(ReceiveRejection {
                reason: Reason::ReceiveError,
                observation: SourceReceiveObservation {
                    received_bytes: None,
                    payload_truncated: None,
                    control_truncated: None,
                    errno: Some(error.raw_os_error()),
                },
            });
        }
        Ok(result) => result,
    };
    let observation = SourceReceiveObservation {
        received_bytes: Some(result.bytes as u64),
        payload_truncated: Some(result.flags.contains(ReturnFlags::TRUNC)),
        control_truncated: Some(result.flags.contains(ReturnFlags::CTRUNC)),
        errno: None,
    };
    let mut invalid_ancillary = false;
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
                        invalid_ancillary = true;
                    }
                }
            }
            _ => invalid_ancillary = true,
        }
    }
    let reason = if result.bytes == 0 {
        Some(Reason::ReceiveZeroBytes)
    } else if observation.payload_truncated == Some(true) {
        Some(Reason::PayloadTruncated)
    } else if observation.control_truncated == Some(true) {
        Some(Reason::ControlTruncated)
    } else if invalid_ancillary || rights_messages > 1 {
        Some(Reason::InvalidAncillary)
    } else if credentials.is_none() {
        Some(Reason::MissingCredentials)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(ReceiveRejection {
            reason,
            observation,
        });
    }
    Ok(Some(SourcePacket {
        bytes: bytes[..result.bytes].to_vec(),
        cred: credentials.expect("validated Source credentials"),
        peer,
        observation,
    }))
}

impl pokelinux_lifecycle::session::Provider for Observer {
    fn first_rejection(&self) -> Option<&SourceRejection> {
        self.first_rejection.as_ref()
    }
    fn stop(&mut self) {
        self.failed = true;
        self.reset_candidate();
        self.first_export_sequence = None;
        self.pending_loss = None;
        self.current = None;
        self.exported = None;
        self.focus = None;
        self.locker = None;
        self.fd = None;
    }
    fn occupied(&self) -> bool {
        // The compositor is externally owned. Only retained local handles exist.
        false
    }
    fn poll(&mut self) -> io::Result<Update> {
        Observer::poll(self)?;
        if let Some(loss) = self.pending_loss.take() {
            return Ok(Update::AuthorityLost(loss));
        }
        let frame = self.snapshot().cloned();
        if self.failed {
            return Err(io::ErrorKind::NotConnected.into());
        }
        // A final freshness check may itself discover loss after the drain.
        if let Some(loss) = self.pending_loss.take() {
            return Ok(Update::AuthorityLost(loss));
        }
        let Some(frame) = frame else {
            return Ok(if self.loss_boundary.is_some() {
                Update::Recovering
            } else {
                Update::AwaitingFirst
            });
        };
        Ok(Update::Current(pokelinux_lifecycle::session::Snapshot {
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
