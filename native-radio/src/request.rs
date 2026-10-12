//! One bounded request attempt. Binder submission never proves modem acceptance.
use crate::{
    deadline::Deadline,
    owner::{OwnerError, SessionKey, Shared},
    transport::RadioTransport,
};
use omarchy_ims::requests::RequestParts;
use rsbinder::StatusCode;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RequestKey(Arc<()>);
impl PartialEq for RequestKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for RequestKey {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Queued,
    NotSent,
    OutcomeUnknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestReceipt {
    pub key: RequestKey,
    pub serial: i32,
    pub outcome: Outcome,
    pub revision: u64,
}
pub(crate) struct RequestState {
    pub(crate) receipt: RequestReceipt,
    pub(crate) deadline: Deadline,
    pub(crate) cancelled: bool,
    pub(crate) reason: Option<crate::receiver::ReceiverFailure>,
}
impl RequestState {
    pub(crate) fn cancel(&mut self, reason: crate::receiver::ReceiverFailure) {
        if !self.cancelled {
            self.receipt.revision += 1;
        }
        self.cancelled = true;
        self.reason.get_or_insert(reason);
        if self.receipt.outcome == Outcome::Queued {
            self.receipt.outcome = Outcome::NotSent;
        }
    }
}
#[derive(Debug)]
pub enum RequestFailure {
    Admission(OwnerError),
    Binder(StatusCode),
}
#[derive(Debug)]
pub struct RequestReport {
    pub key: RequestKey,
    pub submitted_parts: usize,
    pub submission: Result<(), RequestFailure>,
    pub outcome: Outcome,
}
/// Non-clone admission. A future installed service must persist uncertainty
/// before exposing dispatch; this local boundary has no crash-safe request ledger.
pub struct PreparedRequest {
    shared: Arc<Shared>,
    session: SessionKey,
    key: RequestKey,
    arguments: [i32; 4],
    payload: Vec<u8>,
}
impl PreparedRequest {
    pub(crate) fn new(
        shared: Arc<Shared>,
        session: SessionKey,
        arguments: [i32; 4],
        payload: Vec<u8>,
        deadline: Deadline,
    ) -> Result<Self, OwnerError> {
        let key = RequestKey(Arc::new(()));
        {
            let mut gate = shared.lock();
            if !gate.is_current(&session) {
                return Err(OwnerError::StaleSession);
            }
            gate.active()?;
            if gate.request.is_some() {
                return Err(OwnerError::OperationPending);
            }
            if payload.len() > 4096 {
                return Err(OwnerError::Unsupported);
            }
            if deadline.elapsed(shared.1.now()) {
                return Err(OwnerError::Failed(
                    crate::receiver::ReceiverFailure::Expired,
                ));
            }
            gate.request = Some(RequestState {
                receipt: RequestReceipt {
                    key: key.clone(),
                    serial: arguments[0],
                    outcome: Outcome::Queued,
                    revision: 0,
                },
                deadline,
                cancelled: false,
                reason: None,
            });
        }
        Ok(Self {
            shared,
            session,
            key,
            arguments,
            payload,
        })
    }
    pub fn key(&self) -> RequestKey {
        self.key.clone()
    }
    pub(crate) fn belongs_to(&self, shared: &Arc<Shared>) -> bool {
        Arc::ptr_eq(&self.shared, shared)
    }
    fn admit_part(&self) -> Result<(), OwnerError> {
        let mut gate = self.shared.lock();
        if !gate.is_current(&self.session) {
            return Err(OwnerError::StaleSession);
        }
        gate.active()?;
        let request = gate.request.as_mut().ok_or(OwnerError::StaleSession)?;
        if request.receipt.key != self.key {
            return Err(OwnerError::StaleSession);
        }
        // This serialized permission is the irreversible local uncertainty boundary.
        // A later cancel cannot prove Binder or modem execution never occurred.
        if request.deadline.elapsed(self.shared.1.now()) {
            request.cancel(crate::receiver::ReceiverFailure::Expired);
        }
        if request.cancelled {
            return Err(OwnerError::Failed(request.reason.expect("cancel reason")));
        }
        if request.receipt.outcome == Outcome::Queued {
            request.receipt.outcome = Outcome::OutcomeUnknown;
            request.receipt.revision += 1;
        }
        Ok(())
    }
    pub(crate) fn run(self, transport: &RadioTransport) -> RequestReport {
        let mut submitted_parts = 0;
        let submission = (|| {
            for part in RequestParts::new(&self.payload).expect("bounded wire length") {
                self.admit_part().map_err(RequestFailure::Admission)?;
                transport
                    .send_part(self.arguments, part)
                    .map_err(RequestFailure::Binder)?;
                submitted_parts += 1;
            }
            Ok(())
        })();
        let outcome = self
            .shared
            .lock()
            .request
            .as_ref()
            .filter(|r| r.receipt.key == self.key)
            .map(|r| r.receipt.outcome)
            .unwrap_or(Outcome::OutcomeUnknown);
        RequestReport {
            key: self.key.clone(),
            submitted_parts,
            submission,
            outcome,
        }
    }
}
impl Drop for PreparedRequest {
    fn drop(&mut self) {
        let mut gate = self.shared.lock();
        if let Some(request) = gate
            .request
            .as_mut()
            .filter(|r| r.receipt.key == self.key && r.receipt.outcome == Outcome::Queued)
        {
            request.cancel(crate::receiver::ReceiverFailure::Cancelled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{deadline::Clock, owner::RadioOwner, receiver::ReceiverFailure};
    use std::{
        num::NonZeroUsize,
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };
    struct Controlled(AtomicU64);
    impl Clock for Controlled {
        fn now(&self) -> Duration {
            Duration::from_nanos(self.0.load(Ordering::Acquire))
        }
    }
    fn prepared() -> (RadioOwner, PreparedRequest, Arc<Controlled>) {
        let clock = Arc::new(Controlled(AtomicU64::new(10)));
        let mut owner = RadioOwner::with_clock(NonZeroUsize::new(2).unwrap(), clock.clone());
        let reg = owner
            .begin_until(
                None,
                NonZeroUsize::new(8).unwrap(),
                Deadline::at(Duration::from_nanos(100)),
            )
            .unwrap();
        owner.activate(&reg.key, 42).unwrap();
        let request = owner
            .prepare_request(
                &reg.key,
                137,
                0,
                vec![42; 4096],
                Deadline::at(Duration::from_nanos(20)),
            )
            .unwrap();
        (owner, request, clock)
    }
    #[test]
    fn expiry_between_part_permissions_retains_unknown_without_timer() {
        let (owner, request, clock) = prepared();
        request.admit_part().unwrap();
        clock.0.store(20, Ordering::Release);
        assert_eq!(
            request.admit_part(),
            Err(OwnerError::Failed(ReceiverFailure::Expired))
        );
        assert_eq!(
            owner.snapshot().request.unwrap().outcome,
            Outcome::OutcomeUnknown
        );
    }
    #[test]
    fn cancellation_before_permission_and_drop_preserve_not_sent() {
        let (mut owner, request, _) = prepared();
        owner.cancel_request(&request.key()).unwrap();
        assert_eq!(
            request.admit_part(),
            Err(OwnerError::Failed(ReceiverFailure::Cancelled))
        );
        drop(request);
        assert_eq!(owner.snapshot().request.unwrap().outcome, Outcome::NotSent);
    }
    #[test]
    fn a_dropped_queued_permit_never_creates_uncertainty() {
        let (owner, request, _) = prepared();
        drop(request);
        assert_eq!(owner.snapshot().request.unwrap().outcome, Outcome::NotSent);
    }
    #[test]
    fn permissions_and_receipts_are_bound_to_exact_owner_allocation() {
        let (owner, request, _) = prepared();
        let foreign = RadioOwner::new(NonZeroUsize::new(2).unwrap());
        assert!(owner.owns_request(&request));
        assert!(!foreign.owns_request(&request));
    }
    #[test]
    fn first_terminal_reason_and_receipt_revision_do_not_change_on_repeat_cancel() {
        let (mut owner, request, clock) = prepared();
        clock.0.store(20, Ordering::Release);
        let before = owner.snapshot().request.unwrap();
        owner.cancel_request(&request.key()).unwrap();
        let after = owner.snapshot().request.unwrap();
        assert_eq!(before, after);
        assert_eq!(
            request.admit_part(),
            Err(OwnerError::Failed(ReceiverFailure::Expired))
        );
    }
}
