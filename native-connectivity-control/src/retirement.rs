//! First local terminal/stop decision; no payloads, cleanup or retry authority.
use serde::{Serialize, Serializer};
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementReason {
    StartupFailure,
    StartupExpired,
    CapabilityReductionFailure,
    WorkerAdmissionLost,
    ActionExpired,
    CommandSendFailure,
    ReceiveFailure,
    RecordLength,
    RecordVersion,
    RecordType,
    RecordMalformed,
    RecordFuture,
    RecordExpired,
    RecordIdentity,
    RecordRegression,
    CommandMismatch,
    CompletionLate,
    CompletionMismatch,
    CompletionRejected,
    WorkerUncertain,
    UnexpectedOutput,
    OutputReadFailure,
    ReapFailure,
    WorkerExited,
    MainFailure,
    ExternalStop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Retirement {
    pub reason: RetirementReason,
    pub detected_at_ns: u64,
    pub operation: Option<&'static str>,
    pub original_until_ns: Option<u64>,
    pub observation_at_ns: Option<u64>,
    pub observation_age_ns: Option<u64>,
    pub record_length: Option<usize>,
    pub record_type: Option<u8>,
    #[serde(serialize_with = "serialize_kind")]
    pub io_kind: Option<io::ErrorKind>,
    pub errno: Option<i32>,
}
fn serialize_kind<S: Serializer>(
    kind: &Option<io::ErrorKind>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match kind {
        Some(kind) => serializer.collect_str(&format_args!("{kind:?}")),
        None => serializer.serialize_none(),
    }
}
impl Retirement {
    pub(super) fn new(reason: RetirementReason, detected_at_ns: u64) -> Self {
        Self {
            reason,
            detected_at_ns,
            operation: None,
            original_until_ns: None,
            observation_at_ns: None,
            observation_age_ns: None,
            record_length: None,
            record_type: None,
            io_kind: None,
            errno: None,
        }
    }
    pub(super) fn operation(mut self, operation: &'static str) -> Self {
        self.operation = Some(operation);
        self
    }
    pub(super) fn until(mut self, until: u64) -> Self {
        self.original_until_ns = Some(until);
        self
    }
    pub(super) fn observation(mut self, at: u64) -> Self {
        self.observation_at_ns = Some(at);
        self.observation_age_ns = self.detected_at_ns.checked_sub(at);
        self
    }
    pub(super) fn record(mut self, bytes: &[u8]) -> Self {
        self.record_length = Some(bytes.len());
        self.record_type = bytes.get(1).copied();
        self
    }
    pub(super) fn io(mut self, error: &io::Error) -> Self {
        self.io_kind = Some(error.kind());
        self.errno = error.raw_os_error();
        self
    }
}
