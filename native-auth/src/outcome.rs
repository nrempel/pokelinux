//! Sealed local closure classifications. These do not authorize actor unlock.
use crate::{boottime_ns, owner::Candidate};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};

struct Ownership {
    until: u64,
    revoked: AtomicBool,
}
impl Ownership {
    fn current(&self) -> bool {
        !self.revoked.load(Ordering::Acquire) && boottime_ns() < self.until
    }
}

pub(crate) struct Issuer {
    owner: Arc<Ownership>,
}
impl Issuer {
    pub(crate) fn new(until: u64) -> Self {
        Self {
            owner: Arc::new(Ownership {
                until,
                revoked: AtomicBool::new(false),
            }),
        }
    }
    pub(crate) fn deadline(&self) -> u64 {
        self.owner.until
    }
    pub(crate) fn revoke(&self) {
        self.owner.revoked.store(true, Ordering::Release);
    }
    pub(crate) fn current(&self) -> bool {
        self.owner.current()
    }
    fn ticket(&self, value: Candidate) -> Option<Ticket> {
        self.current().then(|| Ticket {
            owner: Arc::downgrade(&self.owner),
            until: self.deadline(),
            value,
        })
    }
    pub(crate) fn worker(&self, worker_pid: u32, value: Candidate) -> Option<WorkerOutcome> {
        if !(2..=i32::MAX as u32).contains(&worker_pid) {
            return None;
        }
        self.ticket(value)
            .map(|ticket| WorkerOutcome { ticket, worker_pid })
    }
    pub(crate) fn parent(&self, value: Candidate) -> Option<ClosedOutcome> {
        self.ticket(value).map(|ticket| ClosedOutcome { ticket })
    }
    pub(crate) fn consume(&self, outcome: ClosedOutcome) -> Option<Candidate> {
        let owner = outcome.ticket.live_owner()?;
        if !Arc::ptr_eq(&self.owner, &owner) || !self.current() {
            return None;
        }
        Some(outcome.ticket.value)
    }
}

// No Debug/Clone/public constructors. Only an actual owner may issue a ticket
// after its own closure predicates; raw records and snapshots cannot do so.
struct Ticket {
    owner: Weak<Ownership>,
    until: u64,
    value: Candidate,
}
impl Ticket {
    fn live_owner(&self) -> Option<Arc<Ownership>> {
        let owner = self.owner.upgrade()?;
        (owner.until == self.until && owner.current()).then_some(owner)
    }
}

pub struct WorkerOutcome {
    ticket: Ticket,
    worker_pid: u32,
}
impl WorkerOutcome {
    pub(crate) fn into_parts(self) -> Option<(u32, Candidate, u64)> {
        self.ticket.live_owner()?;
        Some((self.worker_pid, self.ticket.value, self.ticket.until))
    }
}

pub struct ClosedOutcome {
    ticket: Ticket,
}
impl ClosedOutcome {
    /// An immutable observation, not a promise that this token remains current.
    pub fn deadline(&self) -> u64 {
        self.ticket.until
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_token_requires_exact_live_issuer_even_with_equal_deadline() {
        let until = boottime_ns() + crate::owner::MAX_ATTEMPT_NS;
        let first = Issuer::new(until);
        let second = Issuer::new(until);
        let token = first.parent(Candidate::Authenticated).unwrap();
        assert!(second.consume(token).is_none());
        let token = first.parent(Candidate::Denied).unwrap();
        assert!(first.consume(token) == Some(Candidate::Denied));
        // One-time issuance belongs to ScopeChild/Attempt. Issuer is private
        // plumbing; this model does not establish any process closure.
    }

    #[test]
    fn revoked_or_dropped_owner_invalidates_previously_issued_tokens() {
        let until = boottime_ns() + crate::owner::MAX_ATTEMPT_NS;
        let issuer = Issuer::new(until);
        let worker = issuer.worker(100, Candidate::Authenticated).unwrap();
        let parent = issuer.parent(Candidate::Authenticated).unwrap();
        issuer.revoke();
        assert!(worker.into_parts().is_none());
        assert!(issuer.consume(parent).is_none());
        assert!(issuer.worker(100, Candidate::Denied).is_none());
        assert!(issuer.parent(Candidate::Denied).is_none());
        let issuer = Issuer::new(until);
        let worker = issuer.worker(100, Candidate::Authenticated).unwrap();
        let parent = issuer.parent(Candidate::Authenticated).unwrap();
        drop(issuer);
        assert!(worker.into_parts().is_none());
        assert!(parent.ticket.live_owner().is_none());
    }

    #[test]
    fn token_consumption_uses_original_clock_after_issuance() {
        let until = boottime_ns() + 100_000_000;
        let issuer = Issuer::new(until);
        let worker = issuer.worker(100, Candidate::Denied).unwrap();
        let parent = issuer.parent(Candidate::Denied).unwrap();
        let remaining = until.saturating_sub(boottime_ns());
        std::thread::sleep(
            std::time::Duration::from_nanos(remaining) + std::time::Duration::from_millis(2),
        );
        assert!(worker.into_parts().is_none());
        assert!(issuer.consume(parent).is_none());
        assert!(issuer.deadline() == until && !issuer.current());
    }
}
