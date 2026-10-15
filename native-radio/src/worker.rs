//! Worker ownership primitives for a supervised native radio process.
use std::thread::{self, JoinHandle};

struct OwnedWorker<T>(Option<JoinHandle<T>>);
impl<T: Send + 'static> OwnedWorker<T> {
    fn spawn(job: impl FnOnce() -> T + Send + 'static) -> std::io::Result<Self> {
        thread::Builder::new()
            .name("radio-transport".into())
            .spawn(job)
            .map(|join| Self(Some(join)))
    }
    fn try_join(&mut self) -> Option<thread::Result<T>> {
        if !self.0.as_ref()?.is_finished() {
            return None;
        }
        Some(self.0.take().expect("owned worker").join())
    }
    fn occupied(&self) -> bool {
        self.0.is_some()
    }
}
// A live worker is never detached or joined indefinitely by Drop. Misuse ends
// this supervised process; its parent must fence output, terminate/reap it and
// preserve any uncertain remote registration. Normal shutdown polls try_join.
impl<T> Drop for OwnedWorker<T> {
    fn drop(&mut self) {
        if self.0.as_ref().is_some_and(|join| !join.is_finished()) {
            std::process::abort();
        }
    }
}

use crate::{
    deadline::{BootClock, Clock, Deadline},
    owner::{OwnerError, RadioOwner, SessionKey, Snapshot},
    receiver::ReceiverFailure,
    transport::{RadioDeathWatch, RadioTransport},
};
use rsbinder::{Interface, Status, StatusCode};
use std::{num::NonZeroUsize, sync::Arc};

#[derive(Debug)]
pub enum RegistrationFailure {
    Monitoring(StatusCode),
    NotDispatched(OwnerError),
    Binder(Status),
    Panicked,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LifecycleError {
    Owner(OwnerError),
    WorkerOccupied,
    AlreadyAttempted,
    InvalidClient,
    SpawnFailed,
}
struct Completion {
    key: SessionKey,
    dispatched: bool,
    result: Result<i32, RegistrationFailure>,
    watch: Option<RadioDeathWatch>,
}
/// A local result is separate from vendor cleanup. Dispatched registration may
/// exist remotely even after cancellation, timeout, error, panic or process death.
#[derive(Debug)]
pub struct RegistrationReport {
    pub dispatched: bool,
    pub result: Result<i32, RegistrationFailure>,
    pub activation: Result<(), OwnerError>,
}
/// One registration attempt per supervised process. This intentionally exposes
/// no automatic retry/clear: the real vendor cleanup/reset contract is unresolved.
/// Binder endpoints, publication authority and death watch stay in this process.
/// Dropping a blocked worker aborts the process. Use stop plus poll; the parent
/// must handle permanent hangs by revoking process identity and terminating/reaping.
pub struct TransportSession {
    owner: RadioOwner,
    worker: Option<OwnedWorker<Completion>>,
    // Drop an unfinished request worker before a potentially blocking watch unlink.
    request_worker: Option<OwnedWorker<crate::request::RequestReport>>,
    watch: Option<RadioDeathWatch>,
    attempted: bool,
    transport: Option<RadioTransport>,
}
impl TransportSession {
    pub fn new(retained_limit: NonZeroUsize, selected_slot: i32) -> Self {
        Self::with_clock(retained_limit, selected_slot, Arc::new(BootClock))
    }
    pub fn with_clock(
        retained_limit: NonZeroUsize,
        selected_slot: i32,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            owner: RadioOwner::with_clock(retained_limit, selected_slot, clock),
            worker: None,
            watch: None,
            attempted: false,
            transport: None,
            request_worker: None,
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.owner.snapshot()
    }
    pub fn selected_slot(&self) -> i32 {
        self.owner.selected_slot()
    }
    pub fn worker_occupied(&self) -> bool {
        self.worker.as_ref().is_some_and(OwnedWorker::occupied)
            || self
                .request_worker
                .as_ref()
                .is_some_and(OwnedWorker::occupied)
    }
    pub fn start(
        &mut self,
        transport: RadioTransport,
        client: String,
        capacity: NonZeroUsize,
        deadline: Deadline,
    ) -> Result<SessionKey, LifecycleError> {
        if self.worker_occupied() {
            return Err(LifecycleError::WorkerOccupied);
        }
        if self.attempted {
            return Err(LifecycleError::AlreadyAttempted);
        }
        if client.is_empty() || client.len() > 64 {
            return Err(LifecycleError::InvalidClient);
        }
        let previous = self.owner.snapshot().session;
        let registration = self
            .owner
            .begin_until(previous.as_ref(), capacity, deadline)
            .map_err(LifecycleError::Owner)?;
        let key = registration.key.clone();
        let request_transport = transport.clone();
        let spawned = OwnedWorker::spawn(move || {
            let key = registration.key.clone();
            let watch = match transport.watch_death(registration.endpoints.control.clone()) {
                Ok(watch) => watch,
                Err(error) => {
                    return Completion {
                        key,
                        dispatched: false,
                        result: Err(RegistrationFailure::Monitoring(error)),
                        watch: None,
                    };
                }
            };
            if let Err(error) = registration.admit_dispatch() {
                return Completion {
                    key,
                    dispatched: false,
                    result: Err(RegistrationFailure::NotDispatched(error)),
                    watch: Some(watch),
                };
            }
            let result = transport
                .register(
                    &registration.endpoints.response.as_binder(),
                    &registration.endpoints.indication.as_binder(),
                    &client,
                )
                .map_err(RegistrationFailure::Binder);
            Completion {
                key,
                dispatched: true,
                result,
                watch: Some(watch),
            }
        });
        match spawned {
            Ok(worker) => {
                self.worker = Some(worker);
                self.attempted = true;
                self.transport = Some(request_transport);
                Ok(key)
            }
            Err(_) => {
                self.owner
                    .fail_current(&key, ReceiverFailure::InternalFailure);
                Err(LifecycleError::SpawnFailed)
            }
        }
    }
    pub fn prepare_request(
        &mut self,
        key: &SessionKey,
        request: i32,
        slot: i32,
        payload: Vec<u8>,
        deadline: Deadline,
    ) -> Result<crate::request::PreparedRequest, LifecycleError> {
        if self.worker_occupied() {
            return Err(LifecycleError::WorkerOccupied);
        }
        self.owner
            .prepare_request(key, request, slot, payload, deadline)
            .map_err(LifecycleError::Owner)
    }
    pub fn submit_request(
        &mut self,
        request: crate::request::PreparedRequest,
    ) -> Result<(), LifecycleError> {
        if self.worker_occupied() {
            return Err(LifecycleError::WorkerOccupied);
        }
        if !self.owner.owns_request(&request) {
            return Err(LifecycleError::Owner(OwnerError::StaleSession));
        }
        let transport = self
            .transport
            .as_ref()
            .ok_or(LifecycleError::Owner(OwnerError::NotActive))?
            .clone();
        self.request_worker = Some(
            OwnedWorker::spawn(move || request.run(&transport))
                .map_err(|_| LifecycleError::SpawnFailed)?,
        );
        Ok(())
    }
    pub fn prepare_callback_until(
        &mut self,
        deadline: std::time::Instant,
    ) -> Result<Option<crate::owner::PreparedEvent>, OwnerError> {
        self.owner.prepare_until(deadline)
    }
    pub fn publish_callback(
        &mut self,
        event: crate::owner::PreparedEvent,
    ) -> Result<u64, OwnerError> {
        self.owner.publish(event)
    }
    pub fn cancel_request(&mut self, key: &crate::request::RequestKey) -> Result<(), OwnerError> {
        self.owner.cancel_request(key)
    }
    pub fn poll_request(&mut self) -> Option<std::thread::Result<crate::request::RequestReport>> {
        let report = self.request_worker.as_mut()?.try_join()?;
        self.request_worker = None;
        Some(report)
    }
    /// Revokes local publication immediately without joining, dropping the watch
    /// or pretending that the blocked synchronous Binder transaction was cancelled.
    pub fn stop(&mut self, key: &SessionKey) -> Result<(), OwnerError> {
        self.owner.stop(key)
    }
    /// Completion lives in the owned JoinHandle, independently of callback loss.
    /// Join only after is_finished; occupancy survives local timeout/cancellation.
    pub fn poll_registration(&mut self) -> Option<RegistrationReport> {
        let result = self.worker.as_mut()?.try_join()?;
        self.worker = None;
        match result {
            Ok(completion) => {
                let activation = match &completion.result {
                    Ok(id) => self.owner.activate(&completion.key, *id),
                    Err(error) => {
                        let reason = match error {
                            RegistrationFailure::Binder(status)
                                if status.transaction_error() == StatusCode::DeadObject =>
                            {
                                ReceiverFailure::ServiceDied
                            }
                            RegistrationFailure::NotDispatched(OwnerError::Failed(reason)) => {
                                *reason
                            }
                            _ => ReceiverFailure::InternalFailure,
                        };
                        self.owner.fail_current(&completion.key, reason);
                        match self.owner.snapshot().phase {
                            crate::owner::Phase::Failed(reason) => Err(OwnerError::Failed(reason)),
                            _ => Err(OwnerError::NotRegistering),
                        }
                    }
                };
                self.watch = completion.watch;
                Some(RegistrationReport {
                    dispatched: completion.dispatched,
                    result: completion.result,
                    activation,
                })
            }
            Err(_) => {
                if let Some(key) = self.owner.snapshot().session {
                    self.owner
                        .fail_current(&key, ReceiverFailure::InternalFailure);
                }
                Some(RegistrationReport {
                    dispatched: true,
                    result: Err(RegistrationFailure::Panicked),
                    activation: Err(OwnerError::Failed(ReceiverFailure::InternalFailure)),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    #[test]
    fn dead_object_completion_preserves_service_death_before_watch_delivery() {
        let mut session = TransportSession::new(NonZeroUsize::new(2).unwrap(), 0);
        let registration = session
            .owner
            .begin_until(
                None,
                NonZeroUsize::new(4).unwrap(),
                Deadline::at(BootClock.now() + std::time::Duration::from_secs(60)),
            )
            .unwrap();
        session.worker = Some(
            OwnedWorker::spawn(move || Completion {
                key: registration.key,
                dispatched: true,
                result: Err(RegistrationFailure::Binder(StatusCode::DeadObject.into())),
                watch: None,
            })
            .unwrap(),
        );
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let report = loop {
            if let Some(report) = session.poll_registration() {
                break report;
            }
            assert!(std::time::Instant::now() < end);
            thread::yield_now();
        };
        assert_eq!(
            report.activation,
            Err(OwnerError::Failed(ReceiverFailure::ServiceDied))
        );
        assert_eq!(
            session.snapshot().phase,
            crate::owner::Phase::Failed(ReceiverFailure::ServiceDied)
        );
    }
    #[test]
    fn completion_does_not_release_capacity_before_join() {
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let mut worker = OwnedWorker::spawn(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            42
        })
        .unwrap();
        entered_rx.recv().unwrap();
        assert!(worker.occupied());
        assert!(worker.try_join().is_none());
        release_tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let result = loop {
            if let Some(value) = worker.try_join() {
                break value;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        };
        assert_eq!(result.unwrap(), 42);
        assert!(!worker.occupied());
    }
    #[test]
    #[cfg_attr(
        target_arch = "aarch64",
        ignore = "requires native ARM64 kernel process execution; run tools/radio-process-vm.py"
    )]
    fn blocked_worker_drop_aborts_process_instead_of_detaching_or_joining() {
        const MARKER: &str = "POKELINUX_BLOCKED_WORKER_CHILD";
        if std::env::var_os(MARKER).is_some() {
            let (tx, rx) = mpsc::sync_channel(1);
            let worker = OwnedWorker::spawn(move || {
                tx.send(()).unwrap();
                loop {
                    thread::park();
                }
            })
            .unwrap();
            rx.recv().unwrap();
            drop(worker);
            return;
        }
        use std::os::unix::process::{CommandExt, ExitStatusExt};
        let executable = std::env::current_exe().unwrap();
        let mut child = std::process::Command::new(executable)
            .args([
                "--exact",
                "worker::tests::blocked_worker_drop_aborts_process_instead_of_detaching_or_joining",
                "--include-ignored",
            ])
            .env(MARKER, "1")
            .process_group(0)
            .spawn()
            .unwrap();
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= end {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("blocked worker drop hung the process");
            }
            thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(
            status.signal(),
            Some(6),
            "blocked worker was detached instead of aborting"
        );
    }
}
