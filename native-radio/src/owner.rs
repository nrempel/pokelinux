//! Session publication authority. Raw response observations are not call state.
use crate::{
    deadline::{BootClock, Clock, Deadline},
    receiver::{
        self, CallbackControl, CallbackInbox, CallbackQueue, RadioEndpoints, RadioEvent,
        ReceiverFailure,
    },
    response::{RadioResponse, ResponseKind},
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex, MutexGuard, Weak},
    time::Instant,
};

/// Opaque in-process capability. Clones preserve identity; other owners and
/// attempts cannot recreate it. This is not a cross-process wire identifier.
#[derive(Clone, Debug)]
pub struct SessionKey(Arc<()>);
impl SessionKey {
    pub(crate) fn new() -> Self {
        Self(Arc::new(()))
    }
}
impl PartialEq for SessionKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SessionKey {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Registering,
    Active { client_id: i32 },
    Failed(ReceiverFailure),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerError {
    StaleSession,
    NotActive,
    NotRegistering,
    Preparing,
    RetainedLimit,
    Exhausted,
    Unsupported,
    Failed(ReceiverFailure),
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub session: Option<SessionKey>,
    pub revision: u64,
    pub phase: Phase,
    /// Latest committed raw vendor response, not decoded call/IMS state.
    pub response: Option<Arc<RadioResponse>>,
}
pub(crate) struct Gate {
    pub(crate) current: Option<SessionKey>,
    pub(crate) phase: Phase,
    pub(crate) queue: Weak<CallbackQueue>,
    revision: u64,
    response: Option<Arc<RadioResponse>>,
    preparing: Option<Arc<()>>,
    deadline: Option<Deadline>,
}
impl Default for Gate {
    fn default() -> Self {
        Self {
            current: None,
            phase: Phase::Idle,
            queue: Weak::new(),
            revision: 0,
            response: None,
            preparing: None,
            deadline: None,
        }
    }
}
impl Gate {
    pub(crate) fn is_current(&self, key: &SessionKey) -> bool {
        self.current.as_ref() == Some(key)
    }
    pub(crate) fn invalidate(&mut self, key: &SessionKey, reason: ReceiverFailure) {
        if self.is_current(key) && !matches!(self.phase, Phase::Failed(_)) {
            self.phase = Phase::Failed(reason);
            self.response = None;
            self.preparing = None;
            self.revision = self.revision.saturating_add(1);
        }
    }
    fn bump(&mut self) -> Result<(), OwnerError> {
        self.revision = self.revision.checked_add(1).ok_or(OwnerError::Exhausted)?;
        Ok(())
    }
    fn active(&self) -> Result<(), OwnerError> {
        match self.phase {
            Phase::Active { .. } => Ok(()),
            Phase::Failed(reason) => Err(OwnerError::Failed(reason)),
            _ => Err(OwnerError::NotActive),
        }
    }
}
pub(crate) struct Shared(pub(crate) Mutex<Gate>, Arc<dyn Clock>);
impl Default for Shared {
    fn default() -> Self {
        Self(Mutex::new(Gate::default()), Arc::new(BootClock))
    }
}
impl Shared {
    pub(crate) fn lock(&self) -> MutexGuard<'_, Gate> {
        let mut state = self.0.lock().unwrap_or_else(|poison| {
            let mut state = poison.into_inner();
            if let Some(queue) = state.queue.upgrade() {
                queue.fail_locked(&mut state, ReceiverFailure::InternalFailure);
            } else if let Some(key) = state.current.clone() {
                state.invalidate(&key, ReceiverFailure::InternalFailure);
            }
            state
        });
        // Every admission/activation/commit/snapshot checks absolute time, even
        // if the executor has not delivered a timer after suspend.
        if matches!(state.phase, Phase::Registering | Phase::Active { .. })
            && state.deadline.is_some_and(|end| end.elapsed(self.1.now()))
        {
            if let Some(queue) = state.queue.upgrade() {
                queue.fail_locked(&mut state, ReceiverFailure::Expired);
            } else if let Some(key) = state.current.clone() {
                state.invalidate(&key, ReceiverFailure::Expired);
            }
        }
        state
    }
}

/// One prepared event at a time. Dropping it releases the preparation slot.
/// Its provenance cannot be changed by the caller.
pub struct PreparedEvent {
    // Retained-generation accounting includes work consumed from old queues.
    _queue: Arc<CallbackQueue>,
    shared: Weak<Shared>,
    key: SessionKey,
    token: Arc<()>,
    base_revision: u64,
    event: Option<RadioEvent>,
}
impl PreparedEvent {
    pub fn event(&self) -> &RadioEvent {
        self.event.as_ref().expect("prepared event has data")
    }
}
impl Drop for PreparedEvent {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            let mut gate = shared.lock();
            if gate.is_current(&self.key)
                && gate
                    .preparing
                    .as_ref()
                    .is_some_and(|token| Arc::ptr_eq(token, &self.token))
            {
                gate.preparing = None;
            }
        }
    }
}
pub struct Registration {
    shared: Arc<Shared>,
    pub key: SessionKey,
    pub endpoints: RadioEndpoints,
}
impl Registration {
    pub(crate) fn admit_dispatch(&self) -> Result<(), OwnerError> {
        let gate = self.shared.lock();
        if !gate.is_current(&self.key) {
            return Err(OwnerError::StaleSession);
        }
        match gate.phase {
            Phase::Registering => Ok(()),
            Phase::Failed(reason) => Err(OwnerError::Failed(reason)),
            _ => Err(OwnerError::NotRegistering),
        }
    }
}
/// Owns the receive path and the authoritative raw-response observation. The
/// transport worker, death watch and absolute lifetime timers are not yet owned
/// here; callers must retain/drive them and must not infer full session recovery.
pub struct RadioOwner {
    shared: Arc<Shared>,
    inbox: Option<CallbackInbox>,
    control: Option<CallbackControl>,
    retained: Vec<Weak<CallbackQueue>>,
    retained_limit: NonZeroUsize,
}
impl RadioOwner {
    pub fn new(retained_limit: NonZeroUsize) -> Self {
        Self::with_clock(retained_limit, Arc::new(BootClock))
    }
    pub fn with_clock(retained_limit: NonZeroUsize, clock: Arc<dyn Clock>) -> Self {
        Self {
            shared: Arc::new(Shared(Mutex::new(Gate::default()), clock)),
            inbox: None,
            control: None,
            retained: Vec::new(),
            retained_limit,
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let gate = self.shared.lock();
        Snapshot {
            session: gate.current.clone(),
            revision: gate.revision,
            phase: gate.phase,
            response: gate.response.clone(),
        }
    }
    fn begin_queue(
        &mut self,
        expected: Option<&SessionKey>,
        capacity: NonZeroUsize,
        deadline: Deadline,
    ) -> Result<(SessionKey, Arc<CallbackQueue>), OwnerError> {
        self.retained.retain(|queue| queue.strong_count() != 0);
        if self.retained.len() >= self.retained_limit.get() {
            return Err(OwnerError::RetainedLimit);
        }
        let key = SessionKey(Arc::new(()));
        let (queue, inbox) = receiver::stream(self.shared.clone(), key.clone(), capacity);
        {
            let mut gate = self.shared.lock();
            if gate.current.as_ref() != expected {
                return Err(OwnerError::StaleSession);
            }
            if deadline.elapsed(self.shared.1.now()) {
                return Err(OwnerError::Failed(ReceiverFailure::Expired));
            }
            if self.shared.0.is_poisoned() {
                return Err(OwnerError::Failed(ReceiverFailure::InternalFailure));
            }
            // Reserve old invalidation, begin, activation and new invalidation.
            // A failed reservation must leave the current session unchanged.
            let required = if matches!(gate.phase, Phase::Registering | Phase::Active { .. }) {
                4
            } else {
                3
            };
            if gate.revision.checked_add(required).is_none() {
                return Err(OwnerError::Exhausted);
            }
            if let Some(control) = &self.control {
                control.fail_locked(&mut gate, ReceiverFailure::Cancelled);
            }
            gate.bump()?;
            gate.current = Some(key.clone());
            gate.queue = Arc::downgrade(&queue);
            gate.phase = Phase::Registering;
            gate.deadline = Some(deadline);
            gate.response = None;
            gate.preparing = None;
        }
        // Inbox teardown reacquires the gate; keep it outside the critical section.
        self.inbox = Some(inbox);
        self.control = Some(CallbackControl::from_queue(queue.clone()));
        self.retained.push(Arc::downgrade(&queue));
        Ok((key, queue))
    }
    pub fn begin_until(
        &mut self,
        expected: Option<&SessionKey>,
        capacity: NonZeroUsize,
        deadline: Deadline,
    ) -> Result<Registration, OwnerError> {
        let (key, queue) = self.begin_queue(expected, capacity, deadline)?;
        Ok(Registration {
            shared: self.shared.clone(),
            key,
            endpoints: receiver::endpoints(queue),
        })
    }
    pub fn expire(&mut self, key: &SessionKey) -> Result<(), OwnerError> {
        if !self.shared.lock().is_current(key) {
            return Err(OwnerError::StaleSession);
        }
        Ok(())
    }
    /// Call only with the result of registration on these exact endpoints.
    /// This bounds local activation; it does not cancel the remote call.
    pub fn activate(&mut self, key: &SessionKey, client_id: i32) -> Result<(), OwnerError> {
        let mut gate = self.shared.lock();
        if !gate.is_current(key) {
            return Err(OwnerError::StaleSession);
        }
        match gate.phase {
            Phase::Registering => (),
            Phase::Failed(reason) => return Err(OwnerError::Failed(reason)),
            _ => return Err(OwnerError::NotRegistering),
        }
        if gate.revision >= u64::MAX - 1 {
            return Err(OwnerError::Exhausted);
        }
        gate.bump()?;
        gate.phase = Phase::Active { client_id };
        Ok(())
    }
    pub(crate) fn fail_current(&self, key: &SessionKey, reason: ReceiverFailure) {
        let mut gate = self.shared.lock();
        if gate.is_current(key)
            && let Some(control) = &self.control
        {
            control.fail_locked(&mut gate, reason);
        }
    }
    pub fn stop(&mut self, expected: &SessionKey) -> Result<(), OwnerError> {
        let mut gate = self.shared.lock();
        if !gate.is_current(expected) {
            return Err(OwnerError::StaleSession);
        }
        if let Some(control) = &self.control {
            control.fail_locked(&mut gate, ReceiverFailure::Cancelled);
        }
        Ok(())
    }
    pub fn prepare_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<PreparedEvent>, OwnerError> {
        let queue = self.control.as_ref().ok_or(OwnerError::NotActive)?.queue();
        let mut prepared = {
            let mut gate = self.shared.lock();
            gate.active()?;
            if gate.preparing.is_some() {
                return Err(OwnerError::Preparing);
            }
            let token = Arc::new(());
            gate.preparing = Some(token.clone());
            PreparedEvent {
                _queue: queue,
                shared: Arc::downgrade(&self.shared),
                key: gate.current.clone().ok_or(OwnerError::StaleSession)?,
                token,
                base_revision: gate.revision,
                event: None,
            }
        };
        let event = self
            .inbox
            .as_ref()
            .ok_or(OwnerError::NotActive)?
            .recv_until(deadline)
            .map_err(OwnerError::Failed)?;
        if let Some(event) = event {
            prepared.event = Some(event);
            Ok(Some(prepared))
        } else {
            Ok(None)
        }
    }
    /// Commits a transport observation, not call state or operation success.
    /// No arbitrary caller code or deferred state-mutating outbox runs here.
    pub fn publish(&mut self, mut prepared: PreparedEvent) -> Result<u64, OwnerError> {
        let response = match prepared.event.take() {
            Some(RadioEvent::Response(response)) if response.kind == ResponseKind::Raw => {
                Arc::new(response)
            }
            _ => return Err(OwnerError::Unsupported),
        };
        let mut gate = self.shared.lock();
        if !prepared.shared.ptr_eq(&Arc::downgrade(&self.shared)) || !gate.is_current(&prepared.key)
        {
            return Err(OwnerError::StaleSession);
        }
        gate.active()?;
        if gate.revision != prepared.base_revision
            || !gate
                .preparing
                .as_ref()
                .is_some_and(|token| Arc::ptr_eq(token, &prepared.token))
        {
            return Err(OwnerError::StaleSession);
        }
        if gate.revision >= u64::MAX - 1 {
            return Err(OwnerError::Exhausted);
        }
        gate.bump()?;
        let retired = gate.response.replace(response);
        gate.preparing = None;
        let revision = gate.revision;
        drop(gate);
        drop(retired);
        Ok(revision)
    }
}
impl Drop for RadioOwner {
    fn drop(&mut self) {
        if let Some(control) = &self.control {
            control.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RadioExternalResponseInfo,
        receiver::RadioEvent,
        response::{RadioResponse, ResponseKind},
    };
    impl RadioOwner {
        fn test_begin_queue(
            &mut self,
            expected: Option<&SessionKey>,
            capacity: NonZeroUsize,
        ) -> Result<(SessionKey, Arc<CallbackQueue>), OwnerError> {
            self.begin_queue(
                expected,
                capacity,
                Deadline::at(self.shared.1.now() + std::time::Duration::from_secs(60)),
            )
        }
    }
    fn response(value: u8) -> RadioEvent {
        RadioEvent::Response(RadioResponse {
            info: RadioExternalResponseInfo::default(),
            payload: vec![value],
            kind: ResponseKind::Raw,
        })
    }
    struct TestClock(std::sync::atomic::AtomicU64);
    impl crate::deadline::Clock for TestClock {
        fn now(&self) -> std::time::Duration {
            std::time::Duration::from_nanos(self.0.load(std::sync::atomic::Ordering::SeqCst))
        }
    }
    fn timed_owner() -> (RadioOwner, Arc<TestClock>) {
        let clock = Arc::new(TestClock(std::sync::atomic::AtomicU64::new(10)));
        (
            RadioOwner::with_clock(NonZeroUsize::new(4).unwrap(), clock.clone()),
            clock,
        )
    }
    #[test]
    fn expired_registration_rejects_late_success_without_timer_delivery() {
        let (mut owner, clock) = timed_owner();
        let registration = owner
            .begin_until(
                None,
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(20)),
            )
            .unwrap();
        clock.0.store(20, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            owner.activate(&registration.key, 42),
            Err(OwnerError::Failed(ReceiverFailure::Expired))
        );
        assert_eq!(
            owner.snapshot().phase,
            Phase::Failed(ReceiverFailure::Expired)
        );
        assert_eq!(
            registration.endpoints.control.queue().failure(),
            Some(ReceiverFailure::Expired)
        );
    }
    #[test]
    fn expired_publication_withdraws_response_without_timer_delivery() {
        let (mut owner, clock) = timed_owner();
        let registration = owner
            .begin_until(
                None,
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(20)),
            )
            .unwrap();
        owner.activate(&registration.key, 42).unwrap();
        let queue = registration.endpoints.control.queue();
        queue.deliver(response(7)).unwrap();
        let first = owner.prepare_until(Instant::now()).unwrap().unwrap();
        owner.publish(first).unwrap();
        queue.deliver(response(8)).unwrap();
        let late = owner.prepare_until(Instant::now()).unwrap().unwrap();
        clock.0.store(21, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            owner.publish(late),
            Err(OwnerError::Failed(ReceiverFailure::Expired))
        );
        assert!(owner.snapshot().response.is_none());
    }
    #[test]
    fn snapshot_expires_live_state_and_old_timer_cannot_fail_replacement() {
        let (mut owner, clock) = timed_owner();
        let old = owner
            .begin_until(
                None,
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(20)),
            )
            .unwrap();
        owner.activate(&old.key, 42).unwrap();
        clock.0.store(20, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            owner.snapshot().phase,
            Phase::Failed(ReceiverFailure::Expired)
        );
        let fresh = owner
            .begin_until(
                Some(&old.key),
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(30)),
            )
            .unwrap();
        owner.activate(&fresh.key, 42).unwrap();
        assert_eq!(owner.expire(&old.key), Err(OwnerError::StaleSession));
        assert_eq!(owner.snapshot().phase, Phase::Active { client_id: 42 });
    }
    #[test]
    fn already_expired_begin_preserves_current_session() {
        let (mut owner, _clock) = timed_owner();
        let current = owner
            .begin_until(
                None,
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(20)),
            )
            .unwrap();
        owner.activate(&current.key, 42).unwrap();
        assert!(matches!(
            owner.begin_until(
                Some(&current.key),
                NonZeroUsize::new(2).unwrap(),
                crate::deadline::Deadline::at(std::time::Duration::from_nanos(10))
            ),
            Err(OwnerError::Failed(ReceiverFailure::Expired))
        ));
        assert_eq!(owner.snapshot().phase, Phase::Active { client_id: 42 });
    }
    #[test]
    fn consumed_event_cannot_publish_after_cancellation() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        queue.deliver(response(7)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        std::thread::spawn(move || queue.fail(ReceiverFailure::Cancelled))
            .join()
            .unwrap();
        assert_eq!(
            owner.publish(prepared),
            Err(OwnerError::Failed(ReceiverFailure::Cancelled))
        );
        assert!(owner.snapshot().response.is_none());
    }
    #[test]
    fn replacement_rejects_old_work_and_old_endpoint_failure() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (old, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&old, 42).unwrap();
        queue.deliver(response(99)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        let (new, fresh) = owner
            .test_begin_queue(Some(&old), NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&new, 42).unwrap();
        let revision = owner.snapshot().revision;
        assert_eq!(owner.publish(prepared), Err(OwnerError::StaleSession));
        queue.fail(ReceiverFailure::ServiceDied);
        assert_eq!(owner.snapshot().revision, revision);
        assert_eq!(owner.snapshot().phase, Phase::Active { client_id: 42 });
        fresh.deliver(response(7)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        owner.publish(prepared).unwrap();
        assert_eq!(owner.snapshot().response.unwrap().payload, [7]);
    }
    #[test]
    fn registration_cannot_publish_or_activate_after_failure() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        queue.deliver(response(7)).unwrap();
        assert!(matches!(
            owner.prepare_until(Instant::now()),
            Err(OwnerError::NotActive)
        ));
        queue.fail(ReceiverFailure::QueueFull);
        assert_eq!(
            owner.activate(&key, 42),
            Err(OwnerError::Failed(ReceiverFailure::QueueFull))
        );
    }
    #[test]
    fn commit_before_failure_is_visible_then_withdrawn() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        queue.deliver(response(7)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        let committed = owner.publish(prepared).unwrap();
        assert!(owner.snapshot().response.is_some());
        queue.fail(ReceiverFailure::ServiceDied);
        assert!(owner.snapshot().revision > committed);
        assert!(owner.snapshot().response.is_none());
    }
    #[test]
    fn exhausted_begin_does_not_revoke_current_session() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        queue.deliver(response(7)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        owner.publish(prepared).unwrap();
        owner.shared.lock().revision = u64::MAX - 2;
        assert!(matches!(
            owner.test_begin_queue(Some(&key), NonZeroUsize::new(2).unwrap()),
            Err(OwnerError::Exhausted)
        ));
        assert_eq!(owner.snapshot().phase, Phase::Active { client_id: 42 });
        assert_eq!(queue.failure(), None);
        assert_eq!(owner.snapshot().response.unwrap().payload, [7]);
    }
    #[test]
    fn old_preparation_drop_cannot_release_current_preparation() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (old, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&old, 42).unwrap();
        queue.deliver(response(99)).unwrap();
        let stale = owner.prepare_until(Instant::now()).unwrap().unwrap();
        let (new, fresh) = owner
            .test_begin_queue(Some(&old), NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&new, 42).unwrap();
        fresh.deliver(response(7)).unwrap();
        let current = owner.prepare_until(Instant::now()).unwrap().unwrap();
        drop(stale);
        assert!(matches!(
            owner.prepare_until(Instant::now()),
            Err(OwnerError::Preparing)
        ));
        owner.publish(current).unwrap();
        assert!(owner.prepare_until(Instant::now()).unwrap().is_none());
    }
    #[test]
    fn foreign_preparation_cannot_publish_and_releases_original_slot() {
        let mut first = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let mut other = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = first
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        first.activate(&key, 42).unwrap();
        let (other_key, other_queue) = other
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        other.activate(&other_key, 42).unwrap();
        other_queue.deliver(response(8)).unwrap();
        let other_prepared = other.prepare_until(Instant::now()).unwrap().unwrap();
        queue.deliver(response(7)).unwrap();
        let prepared = first.prepare_until(Instant::now()).unwrap().unwrap();
        assert_eq!(other.publish(prepared), Err(OwnerError::StaleSession));
        assert!(other.snapshot().response.is_none());
        assert!(matches!(
            other.prepare_until(Instant::now()),
            Err(OwnerError::Preparing)
        ));
        other.publish(other_prepared).unwrap();
        assert_eq!(other.snapshot().response.unwrap().payload, [8]);
        assert!(first.prepare_until(Instant::now()).unwrap().is_none());
    }
    #[test]
    fn retained_limit_and_stale_begin_preserve_current_state() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(2).unwrap());
        let (old, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&old, 42).unwrap();
        assert!(matches!(
            owner.test_begin_queue(None, NonZeroUsize::new(2).unwrap()),
            Err(OwnerError::StaleSession)
        ));
        assert_eq!(queue.failure(), None);
        let (current, fresh) = owner
            .test_begin_queue(Some(&old), NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&current, 42).unwrap();
        assert!(matches!(
            owner.test_begin_queue(Some(&current), NonZeroUsize::new(2).unwrap()),
            Err(OwnerError::RetainedLimit)
        ));
        assert_eq!(fresh.failure(), None);
        drop(queue);
        assert!(
            owner
                .test_begin_queue(Some(&current), NonZeroUsize::new(2).unwrap())
                .is_ok()
        );
    }

    #[test]
    fn poisoned_gate_invalidates_the_current_callback_stream() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(2).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        let shared = owner.shared.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = shared.0.lock().unwrap();
                panic!("injected owner failure");
            })
            .join()
            .is_err()
        );
        assert_eq!(
            owner.snapshot().phase,
            Phase::Failed(ReceiverFailure::InternalFailure)
        );
        assert_eq!(queue.failure(), Some(ReceiverFailure::InternalFailure));
        assert!(matches!(
            owner.test_begin_queue(Some(&key), NonZeroUsize::new(2).unwrap()),
            Err(OwnerError::Failed(ReceiverFailure::InternalFailure))
        ));
        assert!(matches!(
            owner.inbox.as_ref().unwrap().recv_until(Instant::now()),
            Err(ReceiverFailure::InternalFailure)
        ));
    }
    #[test]
    fn overflow_withdraws_published_state_and_rejects_preparing_work() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(1).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        queue.deliver(response(1)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        owner.publish(prepared).unwrap();
        queue.deliver(response(2)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        queue.deliver(response(3)).unwrap();
        assert!(queue.deliver(response(4)).is_err());
        assert!(owner.snapshot().response.is_none());
        assert_eq!(
            owner.publish(prepared),
            Err(OwnerError::Failed(ReceiverFailure::QueueFull))
        );
    }
    #[test]
    fn held_preparation_counts_against_retained_limit() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(2).unwrap());
        let (old, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(1).unwrap())
            .unwrap();
        owner.activate(&old, 42).unwrap();
        queue.deliver(response(7)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        drop(queue);
        let (current, _fresh) = owner
            .test_begin_queue(Some(&old), NonZeroUsize::new(1).unwrap())
            .unwrap();
        assert!(matches!(
            owner.test_begin_queue(Some(&current), NonZeroUsize::new(1).unwrap()),
            Err(OwnerError::RetainedLimit)
        ));
        drop(prepared);
        assert!(
            owner
                .test_begin_queue(Some(&current), NonZeroUsize::new(1).unwrap())
                .is_ok()
        );
    }
    #[test]
    fn unsupported_preparation_is_rejected_and_releases_slot() {
        let mut owner = RadioOwner::new(NonZeroUsize::new(2).unwrap());
        let (key, queue) = owner
            .test_begin_queue(None, NonZeroUsize::new(1).unwrap())
            .unwrap();
        owner.activate(&key, 42).unwrap();
        let mut segmented = response(7);
        if let RadioEvent::Response(response) = &mut segmented {
            response.kind = ResponseKind::Segmented { parameters: [0, 1] };
        }
        queue.deliver(segmented).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        assert_eq!(owner.publish(prepared), Err(OwnerError::Unsupported));
        assert!(owner.snapshot().response.is_none());
        queue.deliver(response(8)).unwrap();
        let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
        owner.publish(prepared).unwrap();
        assert_eq!(owner.snapshot().response.unwrap().payload, [8]);
    }
}
