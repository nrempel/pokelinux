//! One initial-token-zero SIT attempt; no I/O, process custody, or APN authority.
//! Completion records one report observation while dispatch remains uncertain.
#![forbid(unsafe_code)]

use pokelinux_process_owner::{
    deadline::{Clock, Deadline},
    supervisor::ProcessIdentity,
};
use pokelinux_sit_codec::{
    Error as CodecError, INITIAL_LTE_SETUP_V3_BYTES, InitialLteSetupV3, Protocol,
    modern_report::{ModernSetupReport22, ReportError},
    response::{BASE_SETUP_RESPONSE_BYTES, BaseSetupResponse, DecodeError},
};
use std::{
    num::NonZeroU8,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSetup(CodecError),
    InvalidPresentationIndex,
    Deadline,
    Cancelled,
    SourceLost,
    WrongSource,
    NotDispatched,
    AlreadyDispatched,
    DuplicateResponse,
    WrongAttempt,
    Decode(DecodeError),
    ContextMismatch,
    Projection(ReportError),
    Fault,
    Unsupported,
}

/// A validated, bounded initial request. There is no token override or reset.
/// The explicit index is presentation data, not a source/channel/interface binding.
pub struct Setup {
    context: NonZeroU8,
    presentation_index: u8,
    packet: [u8; INITIAL_LTE_SETUP_V3_BYTES],
}
impl Setup {
    pub fn new(
        context: NonZeroU8,
        apn: &str,
        protocol: Protocol,
        presentation_index: u8,
    ) -> Result<Self, Error> {
        if presentation_index > 99 {
            return Err(Error::InvalidPresentationIndex);
        }
        let packet = InitialLteSetupV3::new(context, apn, protocol)
            .map_err(Error::InvalidSetup)?
            .encode()
            .map_err(Error::InvalidSetup)?;
        Ok(Self {
            context,
            presentation_index,
            packet,
        })
    }
}

/// Taking this non-Clone packet has already recorded dispatch uncertainty.
/// Its byte view cannot enforce the number of writes performed by outside code.
pub struct DispatchPacket {
    bytes: [u8; INITIAL_LTE_SETUP_V3_BYTES],
}
impl DispatchPacket {
    pub fn bytes(&self) -> &[u8; INITIAL_LTE_SETUP_V3_BYTES] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    NotSent,
    OutcomeUnknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Withdrawal {
    Cancelled,
    Deadline,
    SourceLost,
}
impl Withdrawal {
    fn error(self) -> Error {
        match self {
            Self::Cancelled => Error::Cancelled,
            Self::Deadline => Error::Deadline,
            Self::SourceLost => Error::SourceLost,
        }
    }
}

/// An immutable historical observation with an opaque constructor.
/// A valid response/report does not settle remote dispatch outcome or APN state.
#[derive(Debug, PartialEq, Eq)]
pub struct Observation {
    source: ProcessIdentity,
    record: [u8; BASE_SETUP_RESPONSE_BYTES],
    report: ModernSetupReport22,
}
impl Observation {
    pub fn source(&self) -> ProcessIdentity {
        self.source
    }
    pub fn record(&self) -> &[u8; BASE_SETUP_RESPONSE_BYTES] {
        &self.record
    }
    pub fn report(&self) -> &ModernSetupReport22 {
        &self.report
    }
}

/// Opaque, non-Clone candidate. Only one can be issued by a given attempt.
pub struct ResponseCandidate {
    owner: Arc<Shared>,
    observation: Arc<Observation>,
}
impl ResponseCandidate {
    pub fn report(&self) -> &ModernSetupReport22 {
        self.observation.report()
    }
    pub fn record(&self) -> &[u8; BASE_SETUP_RESPONSE_BYTES] {
        self.observation.record()
    }
    pub fn source(&self) -> ProcessIdentity {
        self.observation.source()
    }
}

/// `current` is a live local report observation, not a provider-acceptance grant.
/// Returned snapshots are immutable historical samples; query again after loss.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub disposition: Disposition,
    pub current: Option<Arc<Observation>>,
    pub history: Option<Arc<Observation>>,
    pub withdrawal: Option<Withdrawal>,
}

struct Gate {
    packet: Option<[u8; INITIAL_LTE_SETUP_V3_BYTES]>,
    disposition: Disposition,
    candidate_issued: bool,
    current: Option<Arc<Observation>>,
    history: Option<Arc<Observation>>,
    withdrawal: Option<Withdrawal>,
}
impl Gate {
    fn live(&self) -> Result<(), Error> {
        match self.withdrawal {
            Some(reason) => Err(reason.error()),
            None => Ok(()),
        }
    }
    fn withdraw(&mut self, reason: Withdrawal) {
        if self.withdrawal.is_none() {
            self.withdrawal = Some(reason);
        }
        self.current = None;
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            disposition: self.disposition,
            current: self.current.clone(),
            history: self.history.clone(),
            withdrawal: self.withdrawal,
        }
    }
}
struct Shared {
    gate: Mutex<Gate>,
    clock: Arc<dyn Clock>,
    until: Deadline,
    context: NonZeroU8,
    presentation_index: u8,
    source: ProcessIdentity,
}

/// Data-level source equality is not authentication or process custody. The
/// caller must bind the identity to its retained supervisor and actual channel.
pub struct Attempt {
    shared: Arc<Shared>,
}
impl Attempt {
    pub fn new(
        setup: Setup,
        source: ProcessIdentity,
        clock: Arc<dyn Clock>,
        until: Deadline,
    ) -> Result<Self, Error> {
        let owner = Self {
            shared: Arc::new(Shared {
                gate: Mutex::new(Gate {
                    packet: Some(setup.packet),
                    disposition: Disposition::NotSent,
                    candidate_issued: false,
                    current: None,
                    history: None,
                    withdrawal: None,
                }),
                clock,
                until,
                context: setup.context,
                presentation_index: setup.presentation_index,
                source,
            }),
        };
        owner.checked_gate()?.live()?;
        Ok(owner)
    }

    /// No timer delivery is required. Every gate operation samples the original
    /// absolute deadline. Clock implementations follow the shared bounded,
    /// monotonic, non-reentrant trait contract and are called under this mutex.
    fn checked_gate(&self) -> Result<MutexGuard<'_, Gate>, Error> {
        let mut gate = self.shared.gate.lock().map_err(|_| Error::Fault)?;
        if self.shared.until.elapsed(self.shared.clock.now()) {
            gate.withdraw(Withdrawal::Deadline);
        }
        Ok(gate)
    }

    pub fn take_dispatch(&self) -> Result<DispatchPacket, Error> {
        let mut gate = self.checked_gate()?;
        gate.live()?;
        let bytes = gate.packet.take().ok_or(Error::AlreadyDispatched)?;
        // This precedes exposing bytes and the first possible outside write.
        // Dropping the permit or reporting a failed write cannot restore NotSent.
        gate.disposition = Disposition::OutcomeUnknown;
        Ok(DispatchPacket { bytes })
    }

    fn response_gate(&self, source: ProcessIdentity) -> Result<MutexGuard<'_, Gate>, Error> {
        let gate = self.checked_gate()?;
        gate.live()?;
        if source != self.shared.source {
            return Err(Error::WrongSource);
        }
        if gate.disposition != Disposition::OutcomeUnknown {
            return Err(Error::NotDispatched);
        }
        if gate.candidate_issued {
            return Err(Error::DuplicateResponse);
        }
        Ok(gate)
    }

    /// Decode and owned projection occur outside the gate. Both surrounding
    /// gate checks sample the original deadline/source-liveness state. Invalid
    /// records do not consume the single valid-candidate slot.
    pub fn prepare_response(
        &self,
        source: ProcessIdentity,
        complete_record: &[u8],
    ) -> Result<ResponseCandidate, Error> {
        drop(self.response_gate(source)?);
        let parsed = BaseSetupResponse::parse(complete_record, 0).map_err(Error::Decode)?;
        if parsed.context != self.shared.context {
            return Err(Error::ContextMismatch);
        }
        let report = ModernSetupReport22::project(&parsed, self.shared.presentation_index)
            .map_err(Error::Projection)?;
        let record = complete_record
            .try_into()
            .map_err(|_| Error::Decode(DecodeError::UnsupportedLength))?;
        let observation = Arc::new(Observation {
            source,
            record,
            report,
        });
        let mut gate = self.response_gate(source)?;
        gate.candidate_issued = true;
        Ok(ResponseCandidate {
            owner: self.shared.clone(),
            observation,
        })
    }

    /// Consume the exact candidate and recheck custody-derived source loss and
    /// the original deadline immediately before recording the current/history view.
    /// Recording a report observation never settles the remote dispatch outcome.
    pub fn commit(&self, candidate: ResponseCandidate) -> Result<Snapshot, Error> {
        if !Arc::ptr_eq(&self.shared, &candidate.owner) {
            return Err(Error::WrongAttempt);
        }
        let mut gate = self.checked_gate()?;
        gate.live()?;
        if candidate.source() != self.shared.source {
            return Err(Error::WrongSource);
        }
        if gate.disposition != Disposition::OutcomeUnknown {
            return Err(Error::NotDispatched);
        }
        if !gate.candidate_issued || gate.history.is_some() {
            return Err(Error::DuplicateResponse);
        }
        gate.current = Some(candidate.observation.clone());
        gate.history = Some(candidate.observation);
        Ok(gate.snapshot())
    }

    pub fn cancel(&self) -> Result<(), Error> {
        self.checked_gate()?.withdraw(Withdrawal::Cancelled);
        Ok(())
    }

    /// Notification from the caller's retained source owner; no remote cleanup
    /// or actual reaping is performed by this data gate.
    pub fn source_lost(&self, source: ProcessIdentity) -> Result<(), Error> {
        let mut gate = self.checked_gate()?;
        if source != self.shared.source {
            return Err(Error::WrongSource);
        }
        gate.withdraw(Withdrawal::SourceLost);
        Ok(())
    }

    /// Terminal withdrawal is observable as an ordinary readable snapshot.
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(self.checked_gate()?.snapshot())
    }
}
