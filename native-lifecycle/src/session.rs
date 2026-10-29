//! Internal compositor facts supplied by the owned native observer. No app IPC,
//! grant, launch or unlock capability is represented by this boundary.
use pokelinux_kernel_identity::ProcessView;
use serde::{Deserialize, Serialize};
use std::{io, os::fd::OwnedFd, sync::Arc};
pub const SAMPLE_BUDGET_NS: u64 = 100_000_000;
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFocus {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
}
/// Metadata is serializable; the retained kernel handle is internal authority input.
#[derive(Debug, Clone)]
pub struct OwnedFocus {
    pub metadata: NativeFocus,
    pidfd: Arc<OwnedFd>,
}
impl OwnedFocus {
    pub fn new(metadata: NativeFocus, pidfd: OwnedFd) -> Self {
        Self {
            metadata,
            pidfd: Arc::new(pidfd),
        }
    }
    pub fn pidfd(&self) -> &OwnedFd {
        &self.pidfd
    }
    /// Compare live kernel tasks in one pinned procfs namespace. Metadata alone
    /// and descriptor numbers never establish equivalence.
    pub fn equivalent(&self, other: &Self, view: &ProcessView) -> bool {
        self.metadata == other.metadata
            && view
                .matches(self.pidfd(), other.pidfd(), self.metadata.pid)
                .unwrap_or(false)
    }
    /// Exact retained-object sharing permits repeated observation of a sample,
    /// including after its focused app died. It does not establish live authority.
    pub(crate) fn same_reference(&self, other: &Self) -> bool {
        self.metadata == other.metadata && Arc::ptr_eq(&self.pidfd, &other.pidfd)
    }
}
/// Exact accepted-object protocol request, scoped to this Source incarnation.
/// This is historical fact, not authentication or permission to restore reads.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolUnlock {
    pub generation: u64,
    pub sequence: u64,
    pub observed_at_ns: u64,
}
pub fn valid_protocol_facts(
    locked: bool,
    sequence: u64,
    sampled_at_ns: u64,
    generation: Option<u64>,
    unlock: Option<&ProtocolUnlock>,
) -> bool {
    !generation.is_some_and(|value| value == 0 || !locked)
        && !unlock.is_some_and(|event| {
            locked
                || generation.is_some()
                || event.generation == 0
                || event.sequence == 0
                || event.sequence > sequence
                || event.observed_at_ns == 0
                || event.observed_at_ns > sampled_at_ns
        })
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub source: String,
    pub revision: u64,
    pub sequence: u64,
    pub sampled_at_ns: u64,
    pub until: u64,
    pub locked: bool,
    pub focus: Option<OwnedFocus>,
    /// Retained mapped-locker peer from the same sample. This is not an unlock
    /// capability. Consumers must revalidate live owned identity when using it.
    pub locker: Option<OwnedFocus>,
    /// Unknown for a lock already present when Source first attaches.
    pub lock_generation: Option<u64>,
    pub protocol_unlock: Option<ProtocolUnlock>,
}
/// Timing loss withdraws authority without changing the Source's wire identity.
/// This is internal provider state, not an app-supplied recovery request.
#[derive(Debug, Clone)]
pub struct TimingLoss {
    pub source: String,
    pub observed_at_ns: u64,
    pub reason: TimingLossReason,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimingLossReason {
    SampleGap,
    SampleExpired,
}
/// Loss must be delivered before a replacement Current, including when both
/// are discovered in one bounded drain. Recovery confers no grants or effects.
#[derive(Debug, Clone)]
pub enum Update {
    AwaitingFirst,
    AuthorityLost(TimingLoss),
    Recovering,
    Current(Snapshot),
}
/// Poll is bounded and nonblocking. Only Current supplies candidate facts;
/// the actor retains policy and must match its exact launched app pidfd.
pub trait Provider {
    fn poll(&mut self) -> io::Result<Update>;
    /// Bounded first Observer rejection, when this Provider owns an Observer.
    /// Diagnostic history only; stop and later errors must not replace it.
    fn first_rejection(&self) -> Option<&SourceRejection> {
        None
    }
    /// Fence this source's local admission and signal any owned helper.
    fn stop(&mut self);
    /// True until owned helpers have actually been reaped, even after failure.
    fn occupied(&self) -> bool;
}

/// Bounded actor-observed history for root diagnostics, never current authority.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub state: SourceState,
    pub last_accepted: Option<AcceptedSourceSample>,
    pub first_failure: Option<SourceFailure>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    NotConfigured,
    AwaitingFirst,
    Observed,
    Recovering,
    Failed,
    Stopping,
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedSourceSample {
    pub sequence: u64,
    pub sampled_at_ns: u64,
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFailure {
    pub category: SourceFailureCategory,
    pub error_kind: Option<String>,
    /// First actor observation, not producer fault time.
    pub observed_at_boottime_ns: u64,
    pub observer_rejection: Option<Box<SourceRejection>>,
}
/// First terminal Observer decision. No Source token, packet bytes or peer identity.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRejection {
    pub stage: SourceRejectionStage,
    pub reason: SourceRejectionReason,
    /// One-based packet/receive attempt within this poll; zero outside the drain.
    pub drain_ordinal: u32,
    pub sequence: Option<u64>,
    pub sampled_at_ns: Option<u64>,
    /// Observer acceptance, which can precede Authority acceptance during a drain.
    pub previous_accepted: Option<AcceptedSourceSample>,
    /// Exact freshness-validation BOOTTIME if evaluated, otherwise failure observation.
    pub validated_at_boottime_ns: u64,
    pub receive: Option<SourceReceiveObservation>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRejectionStage {
    Liveness,
    Receive,
    Drain,
    Decode,
    Validation,
    Peer,
    Protocol,
    Revision,
    CachedSample,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRejectionReason {
    SourceNotLiving,
    ReceiveError,
    ReceiveZeroBytes,
    PayloadTruncated,
    ControlTruncated,
    InvalidAncillary,
    MissingCredentials,
    DrainLimit,
    FrameSize,
    SenderCredentials,
    MalformedFrame,
    SourceMismatch,
    Sequence,
    FrameShape,
    SampleBackwards,
    SampleGap,
    SampleFuture,
    SampleExpired,
    PeerHandle,
    LockerIdentity,
    ProtocolFacts,
    RevisionOverflow,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReceiveObservation {
    pub received_bytes: Option<u64>,
    pub payload_truncated: Option<bool>,
    pub control_truncated: Option<bool>,
    pub errno: Option<i32>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFailureCategory {
    ProviderError,
    AuthorityRejected,
    MissingAfterEstablished,
}
impl Default for SourceObservation {
    fn default() -> Self {
        Self {
            state: SourceState::NotConfigured,
            last_accepted: None,
            first_failure: None,
        }
    }
}
impl SourceObservation {
    pub(crate) fn configure(&mut self) {
        if self.state == SourceState::NotConfigured {
            self.state = SourceState::AwaitingFirst;
        }
    }
    pub(crate) fn accept(&mut self, sequence: u64, sampled_at_ns: u64) {
        if matches!(self.state, SourceState::Failed | SourceState::Stopping) {
            return;
        }
        self.state = SourceState::Observed;
        self.last_accepted = Some(AcceptedSourceSample {
            sequence,
            sampled_at_ns,
        });
    }
    pub(crate) fn recovering(&mut self) {
        if matches!(
            self.state,
            SourceState::AwaitingFirst | SourceState::Observed | SourceState::Recovering
        ) {
            self.state = SourceState::Recovering;
        }
    }
    pub(crate) fn fail(
        &mut self,
        category: SourceFailureCategory,
        kind: Option<io::ErrorKind>,
        observer_rejection: Option<SourceRejection>,
    ) {
        if matches!(
            self.state,
            SourceState::NotConfigured | SourceState::Stopping
        ) {
            return;
        }
        if self.first_failure.is_none() {
            self.first_failure = Some(SourceFailure {
                category,
                error_kind: kind.map(|k| error_kind_label(k).into()),
                observed_at_boottime_ns: crate::boottime_ns(),
                observer_rejection: observer_rejection.map(Box::new),
            });
        }
        self.state = SourceState::Failed;
    }
    pub(crate) fn stopping(&mut self) {
        if !matches!(self.state, SourceState::NotConfigured | SourceState::Failed) {
            self.state = SourceState::Stopping;
        }
    }
}
// Enum-only static labels: no arbitrary Error message or raw OS text is exported.
fn error_kind_label(kind: io::ErrorKind) -> &'static str {
    use io::ErrorKind::*;
    match kind {
        NotFound => "NotFound",
        PermissionDenied => "PermissionDenied",
        ConnectionRefused => "ConnectionRefused",
        ConnectionReset => "ConnectionReset",
        ConnectionAborted => "ConnectionAborted",
        NotConnected => "NotConnected",
        AddrInUse => "AddrInUse",
        AddrNotAvailable => "AddrNotAvailable",
        BrokenPipe => "BrokenPipe",
        AlreadyExists => "AlreadyExists",
        WouldBlock => "WouldBlock",
        InvalidInput => "InvalidInput",
        InvalidData => "InvalidData",
        TimedOut => "TimedOut",
        WriteZero => "WriteZero",
        Interrupted => "Interrupted",
        Unsupported => "Unsupported",
        UnexpectedEof => "UnexpectedEof",
        OutOfMemory => "OutOfMemory",
        Other => "Other",
        _ => "Unclassified",
    }
}
