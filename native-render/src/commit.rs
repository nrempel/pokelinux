//! One-shot authority ordering for an individual native display attempt.
//!
//! A sealed record is a pregranted attempt, never proof of submission, GPU
//! completion or presentation. An entered native call cannot be retroactively
//! revoked. The executor must reserve outcome custody before issuing a ticket.
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

const STOP: u64 = u64::MAX;
const LAST_SERIAL: u64 = (u64::MAX >> 1) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationPolicy {
    pub security_generation: u64,
    pub output_id: u64,
    pub output_generation: u64,
    pub scene_sequence: u64,
    pub session_generation: u64,
}
impl OperationPolicy {
    fn valid(self) -> bool {
        [
            self.security_generation,
            self.output_id,
            self.output_generation,
            self.scene_sequence,
            self.session_generation,
        ]
        .iter()
        .all(|v| *v != 0)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateError {
    Invalid,
    Stale,
    Stopping,
    Expired,
    Clock,
    Exhausted,
}

/// Control owns this object uniquely. Authentic generations come from the real
/// control authority; supplying arbitrary integers does not establish authority.
/// Publication is CPU-only; no native operation or wait runs under a gate lock.
pub struct CommitAuthority {
    word: Arc<AtomicU64>,
    security_generation: u64,
    serial: u64,
}
#[derive(Debug)]
pub struct StopHandle {
    word: Arc<AtomicU64>,
}
impl StopHandle {
    pub fn stop(&self) {
        self.word.store(STOP, Ordering::Release);
    }
}
#[derive(Debug)]
pub struct CommitTicket {
    word: Arc<AtomicU64>,
    serial: u64,
    policy: OperationPolicy,
    not_after_boottime_ns: u64,
}
/// Move-only old-attempt identity. Retain it through the actual native outcome,
/// even if a later intent or Stop supersedes current authority.
#[derive(Debug)]
pub struct SealedAttempt {
    serial: u64,
    policy: OperationPolicy,
    granted_at_boottime_ns: u64,
}
impl SealedAttempt {
    pub fn serial(&self) -> u64 {
        self.serial
    }
    pub fn policy(&self) -> OperationPolicy {
        self.policy
    }
    pub fn granted_at_boottime_ns(&self) -> u64 {
        self.granted_at_boottime_ns
    }
}
impl CommitAuthority {
    pub fn new(security_generation: u64) -> Result<Self, GateError> {
        if security_generation == 0 {
            return Err(GateError::Invalid);
        }
        Ok(Self {
            word: Arc::new(AtomicU64::new(0)),
            security_generation,
            serial: 0,
        })
    }
    pub fn stop_handle(&self) -> StopHandle {
        StopHandle {
            word: Arc::clone(&self.word),
        }
    }
    fn publish_word(&mut self) -> Result<u64, GateError> {
        let serial = match self.serial.checked_add(1).filter(|v| *v <= LAST_SERIAL) {
            Some(v) => v,
            None => {
                self.word.store(STOP, Ordering::Release);
                return Err(GateError::Exhausted);
            }
        };
        let mut previous = self.word.load(Ordering::Acquire);
        loop {
            if previous == STOP {
                return Err(GateError::Stopping);
            }
            match self.word.compare_exchange(
                previous,
                serial << 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.serial = serial;
                    return Ok(serial);
                }
                Err(actual) => previous = actual,
            }
        }
    }
    /// Revoke current unsealed work for session/device/output loss without
    /// inventing a security epoch or granting a replacement operation.
    pub fn invalidate_current(&mut self) -> Result<(), GateError> {
        self.publish_word().map(|_| ())
    }

    pub fn publish_security(&mut self, generation: u64) -> Result<(), GateError> {
        if self.word.load(Ordering::Acquire) == STOP {
            return Err(GateError::Stopping);
        }
        if generation <= self.security_generation {
            return Err(GateError::Invalid);
        }
        self.publish_word()?;
        self.security_generation = generation;
        Ok(())
    }
    /// Called only after the executor has reserved its bounded job, policy and
    /// outcome slots. Refusal before publication leaves earlier work unchanged.
    pub fn issue(
        &mut self,
        policy: OperationPolicy,
        until_ns: u64,
    ) -> Result<CommitTicket, GateError> {
        if self.word.load(Ordering::Acquire) == STOP {
            return Err(GateError::Stopping);
        }
        if !policy.valid()
            || policy.security_generation != self.security_generation
            || until_ns == 0
        {
            return Err(GateError::Invalid);
        }
        if boottime_ns()? >= until_ns {
            return Err(GateError::Expired);
        }
        let word = Arc::clone(&self.word);
        let serial = self.publish_word()?;
        Ok(CommitTicket {
            word,
            serial,
            policy,
            not_after_boottime_ns: until_ns,
        })
    }
}
impl Drop for CommitAuthority {
    fn drop(&mut self) {
        // Fence unused tickets before the unique control owner disappears.
        // Existing SealedAttempt values remain owned records of older grants.
        self.word.store(STOP, Ordering::Release);
    }
}
impl CommitTicket {
    /// Consume immediately at the final native-attempt boundary, after blocking
    /// preparation. An AQ hidden retry cannot obtain another use of this value.
    pub fn seal(self) -> Result<SealedAttempt, GateError> {
        if boottime_ns()? >= self.not_after_boottime_ns {
            return Err(GateError::Expired);
        }
        let expected = self.serial << 1;
        self.word
            .compare_exchange(expected, expected | 1, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|actual| {
                if actual == STOP {
                    GateError::Stopping
                } else {
                    GateError::Stale
                }
            })?;
        let granted_at_boottime_ns = boottime_ns()?;
        // A cutoff crossing the clock read/CAS interval consumes the ticket but
        // returns no native grant. This conservative refusal cannot renew time.
        if granted_at_boottime_ns >= self.not_after_boottime_ns {
            return Err(GateError::Expired);
        }
        Ok(SealedAttempt {
            serial: self.serial,
            policy: self.policy,
            granted_at_boottime_ns,
        })
    }
    #[cfg(test)]
    fn with_test_deadline(mut self, until: u64) -> Self {
        self.not_after_boottime_ns = until;
        self
    }
}

/// Linux CLOCK_BOOTTIME includes time spent suspended. Actual suspend/wake
/// behavior remains a separate integration/hardware gate.
pub fn boottime_ns() -> Result<u64, GateError> {
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        #[repr(C)]
        struct Timespec {
            seconds: std::ffi::c_long,
            nanoseconds: std::ffi::c_long,
        }
        unsafe extern "C" {
            fn clock_gettime(clock: std::ffi::c_int, result: *mut Timespec) -> std::ffi::c_int;
        }
        let mut value = Timespec {
            seconds: 0,
            nanoseconds: 0,
        };
        if unsafe { clock_gettime(7, &mut value) } != 0
            || value.seconds < 0
            || !(0..1_000_000_000).contains(&value.nanoseconds)
        {
            return Err(GateError::Clock);
        }
        (value.seconds as u64)
            .checked_mul(1_000_000_000)
            .and_then(|v| v.checked_add(value.nanoseconds as u64))
            .ok_or(GateError::Clock)
    }
    #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
    {
        Err(GateError::Clock)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommitAuthority, GateError, OperationPolicy, boottime_ns};
    use std::{sync::mpsc, thread, time::Duration};

    fn policy(generation: u64) -> OperationPolicy {
        OperationPolicy {
            security_generation: generation,
            output_id: 11,
            output_generation: 17,
            scene_sequence: 23,
            session_generation: 29,
        }
    }
    fn deadline() -> u64 {
        boottime_ns().unwrap() + 5_000_000_000
    }

    #[test]
    fn accepted_intent_before_seal_refuses_the_old_native_attempt() {
        let mut control = CommitAuthority::new(1).unwrap();
        let ticket = control.issue(policy(1), deadline()).unwrap();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (go_tx, go_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            ready_tx.send(()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            ticket.seal()
        });
        let ready = ready_rx.recv_timeout(Duration::from_secs(2));
        let invalidated = control.publish_security(2);
        let released = go_tx.send(());
        let result = worker.join();
        assert!(ready.is_ok() && invalidated.is_ok() && released.is_ok());
        assert_eq!(result.unwrap().unwrap_err(), GateError::Stale);
    }

    #[test]
    fn prior_irrevocable_grant_survives_later_intent_and_stop_as_an_old_record() {
        let mut control = CommitAuthority::new(1).unwrap();
        let ticket = control.issue(policy(1), deadline()).unwrap();
        let (sealed_tx, sealed_rx) = mpsc::channel();
        let (go_tx, go_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let granted = ticket.seal();
            sealed_tx.send(granted.is_ok()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            granted
        });
        let sealed = sealed_rx.recv_timeout(Duration::from_secs(2));
        let invalidated = control.publish_security(2);
        control.stop_handle().stop();
        let released = go_tx.send(());
        let joined = worker.join();
        assert!(sealed.unwrap());
        assert!(invalidated.is_ok() && released.is_ok());
        let grant = joined.unwrap().unwrap();
        assert_eq!(grant.policy(), policy(1));
        assert!(grant.serial() > 0);
        assert_eq!(
            control.issue(policy(2), deadline()).unwrap_err(),
            GateError::Stopping
        );
    }

    #[test]
    fn a_new_operation_supersedes_old_unsealed_work_without_reusing_a_grant() {
        let mut control = CommitAuthority::new(1).unwrap();
        let old = control.issue(policy(1), deadline()).unwrap();
        let mut current_policy = policy(1);
        current_policy.scene_sequence += 1;
        let current = control.issue(current_policy, deadline()).unwrap();
        assert_eq!(old.seal().unwrap_err(), GateError::Stale);
        let grant = current.seal().unwrap();
        let retry = control
            .issue(current_policy, deadline())
            .unwrap()
            .seal()
            .unwrap();
        assert!(retry.serial() > grant.serial());
        assert_eq!(retry.policy(), grant.policy());
    }

    #[test]
    fn invalid_admission_does_not_revoke_previously_admitted_work() {
        let mut control = CommitAuthority::new(u64::MAX - 1).unwrap();
        let ticket = control.issue(policy(u64::MAX - 1), deadline()).unwrap();
        assert_eq!(
            control.issue(policy(1), deadline()).unwrap_err(),
            GateError::Invalid
        );
        assert_eq!(control.publish_security(1).unwrap_err(), GateError::Invalid);
        assert_eq!(
            ticket.seal().unwrap().policy().security_generation,
            u64::MAX - 1
        );
        control.publish_security(u64::MAX).unwrap();
        let ticket = control.issue(policy(u64::MAX), deadline()).unwrap();
        assert_eq!(
            ticket.seal().unwrap().policy().security_generation,
            u64::MAX
        );
    }

    #[test]
    fn losing_the_control_owner_revokes_an_unsealed_native_attempt() {
        let ticket = {
            let mut control = CommitAuthority::new(1).unwrap();
            control.issue(policy(1), deadline()).unwrap()
        };
        let worker = thread::spawn(move || ticket.seal());
        assert_eq!(worker.join().unwrap().unwrap_err(), GateError::Stopping);
    }

    #[test]
    fn same_security_epoch_device_invalidation_revokes_without_forging_an_epoch() {
        let mut control = CommitAuthority::new(1).unwrap();
        let old = control.issue(policy(1), deadline()).unwrap();
        control.invalidate_current().unwrap();
        assert_eq!(old.seal().unwrap_err(), GateError::Stale);
        let next = control
            .issue(policy(1), deadline())
            .unwrap()
            .seal()
            .unwrap();
        assert_eq!(next.policy(), policy(1));
    }

    #[test]
    fn expired_original_boottime_deadline_cannot_seal() {
        let mut control = CommitAuthority::new(1).unwrap();
        let ticket = control.issue(policy(1), deadline()).unwrap();
        // A deadline is immutable in the admitted ticket. This private unit test
        // sets an already elapsed original bound; it does not move any clock.
        let expired = ticket.with_test_deadline(boottime_ns().unwrap() - 1);
        assert_eq!(expired.seal().unwrap_err(), GateError::Expired);
    }

    #[test]
    fn stop_racing_control_publication_is_terminal_and_cannot_be_overwritten() {
        let mut control = CommitAuthority::new(1).unwrap();
        let stopper = control.stop_handle();
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            stopper.stop();
            ready_tx.send(()).unwrap();
        });
        let mut entered = 0;
        for _ in 0..1000 {
            match control.issue(policy(1), deadline()) {
                Ok(_) => entered += 1,
                Err(GateError::Stopping) => break,
                Err(other) => panic!("unexpected {other:?}"),
            }
        }
        let stopped = ready_rx.recv_timeout(Duration::from_secs(2));
        let joined = worker.join();
        assert!(stopped.is_ok() && joined.is_ok());
        assert!(entered <= 1000);
        assert_eq!(
            control.issue(policy(1), deadline()).unwrap_err(),
            GateError::Stopping
        );
        assert_eq!(
            control.publish_security(2).unwrap_err(),
            GateError::Stopping
        );
    }
}
