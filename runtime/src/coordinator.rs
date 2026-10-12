//! One admission authority. Network work never holds the journal mutex, and
//! submitted commands are owned here rather than by a local connection task.
use crate::{
    Identity,
    authorization::Session,
    dispatch::exchange_command,
    ledger::{Admission, Error, Ledger, Outcome, StoredReceipt},
    tls::TlsClient,
};
use std::{
    net::SocketAddr,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

struct AdmissionState {
    token: Identity,
    handle: Option<JoinHandle<Result<(), Error>>>,
}
struct PresentationState {
    dismissals: crate::notification_policy::Dismissals,
    attempts: crate::notification_policy::BannerAttempts,
    failed: bool,
}
pub struct Coordinator {
    ledger: Arc<Mutex<Ledger>>,
    worker: Mutex<AdmissionState>,
    client: Arc<TlsClient>,
    address: SocketAddr,
    session: Session,
    presentation: Mutex<PresentationState>,
}
fn poisoned() -> Error {
    Error::Refused("service owner failed; restart and recover")
}
/// The async notification path must never wait behind a presentation commit.
fn try_claim_presentation(
    presentation: &Mutex<PresentationState>,
    current: &crate::watch::Snapshot,
) -> Result<bool, Error> {
    let mut state = presentation.try_lock().map_err(|error| match error {
        std::sync::TryLockError::WouldBlock => Error::Io(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "presentation policy busy",
        )),
        std::sync::TryLockError::Poisoned(_) => poisoned(),
    })?;
    if state.failed {
        return Err(Error::Refused(
            "presentation journal failed; restart and recover",
        ));
    }
    let PresentationState {
        dismissals,
        attempts,
        ..
    } = &mut *state;
    Ok(attempts.claim(current, dismissals))
}
impl Coordinator {
    /// Ledger already holds the lifetime lock and has recovered pending work.
    pub fn new(ledger: Ledger, client: TlsClient, address: SocketAddr) -> Result<Self, Error> {
        let dismissals = ledger.load_dismissals()?;
        Ok(Self {
            ledger: Arc::new(Mutex::new(ledger)),
            worker: Mutex::new(AdmissionState {
                token: Identity(uuid::Uuid::new_v4().simple().to_string()),
                handle: None,
            }),
            client: Arc::new(client),
            address,
            session: Session::default(),
            presentation: Mutex::new(PresentationState {
                dismissals,
                attempts: crate::notification_policy::BannerAttempts::default(),
                failed: false,
            }),
        })
    }
    /// Service-owned observation input, not a client command. This supplies
    /// dismissal eligibility only; freshness, lock and bus ownership are checked
    /// by the presentation lifecycle before any actual display or activation.
    pub fn observe_presentation(&self, current: &crate::watch::Snapshot) -> Result<bool, Error> {
        self.update_presentation(current, |state| state.observe(current))
    }
    pub fn dismiss_presentation(
        &self,
        presented: &crate::watch::Snapshot,
        current: &crate::watch::Snapshot,
    ) -> Result<bool, Error> {
        self.update_presentation(current, |state| state.dismiss(presented, current))
    }
    /// Service-owned cosmetic attempt reservation, deliberately not an RPC.
    /// Call only after the notification owner checks its current lease, policy,
    /// bus owner and free request slot. Keep this coordinator across reconnects.
    /// No network or journal write occurs here; uncertainty does not refund it.
    /// Contention returns WouldBlock without waiting or consuming an attempt.
    pub fn claim_presentation(&self, current: &crate::watch::Snapshot) -> Result<bool, Error> {
        try_claim_presentation(&self.presentation, current)
    }
    fn update_presentation(
        &self,
        current: &crate::watch::Snapshot,
        update: impl FnOnce(&mut crate::notification_policy::Dismissals),
    ) -> Result<bool, Error> {
        // Lock order is presentation -> ledger. Command paths take only ledger;
        // no network operation occurs under either presentation/journal lock.
        let mut state = self.presentation.lock().map_err(|_| poisoned())?;
        if state.failed {
            return Err(Error::Refused(
                "presentation journal failed; restart and recover",
            ));
        }
        let mut next = state.dismissals.clone();
        update(&mut next);
        if next.stored() != state.dismissals.stored() {
            let committed = self
                .ledger
                .lock()
                .map_err(|_| poisoned())?
                .store_dismissals(&next);
            if let Err(error) = committed {
                state.failed = true;
                return Err(error);
            }
        }
        // No candidate policy is exposed before the successful commit.
        state.dismissals = next;
        state.attempts.observe(current);
        Ok(state.dismissals.has_undismissed(current))
    }
    pub fn hello(&self, socket: &UnixStream) -> Result<(Identity, Identity), Error> {
        let session = self.session.identity_for(socket)?;
        let epoch = self.ledger.lock().map_err(|_| poisoned())?.epoch().clone();
        Ok((epoch, session))
    }
    fn observe<T>(
        &self,
        socket: &UnixStream,
        id: &Identity,
        op: &'static str,
        decode: impl FnOnce(&[u8], &Identity) -> Result<T, &'static str>,
    ) -> Result<T, Error> {
        self.session.identity_for(socket)?;
        let body = serde_json::to_vec(&serde_json::json!({"v":1,"op":op,"id":id}))
            .map_err(|_| Error::Refused("invalid observation query"))?;
        self.client
            .exchange_decoded(self.address, &body, Duration::from_secs(1), |bytes| {
                decode(bytes, id)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            })
            .map_err(|_| Error::Refused("Android observation unavailable"))
    }
    pub fn status(
        &self,
        socket: &UnixStream,
        id: &Identity,
    ) -> Result<crate::android_protocol::PhoneStatus, Error> {
        self.observe(socket, id, "status", crate::android_protocol::decode_status)
    }
    pub fn controls(
        &self,
        socket: &UnixStream,
        id: &Identity,
    ) -> Result<crate::android_protocol::Controls, Error> {
        self.observe(
            socket,
            id,
            "controls",
            crate::android_protocol::decode_controls,
        )
    }
    pub fn dial_status(
        &self,
        socket: &UnixStream,
        id: &Identity,
    ) -> Result<crate::android_protocol::DialStatus, Error> {
        self.observe(
            socket,
            id,
            "dial_status",
            crate::android_protocol::decode_dial_status,
        )
    }
    pub fn submit(&self, socket: &mut UnixStream) -> Result<Outcome, Error> {
        // Slow framing does not hold the admission lock. A complete frame may
        // still be admitted after disconnect; closing a UI is not cancellation.
        self.session.identity_for(socket)?;
        let frame = crate::ipc::receive_frame(socket, Duration::from_secs(2))?;
        let crate::local_protocol::LocalRequest::Submit(request, admission) =
            crate::local_protocol::decode(&frame).map_err(Error::Refused)?
        else {
            return Err(Error::Refused("submit operation required"));
        };
        self.admit(socket, request, &admission)
    }
    pub(crate) fn admit(
        &self,
        socket: &UnixStream,
        request: crate::Request,
        admission: &Identity,
    ) -> Result<Outcome, Error> {
        if request.session() != &self.session.identity_for(socket)? {
            return Err(Error::Refused("stale service session"));
        }
        let mut worker = self.worker.lock().map_err(|_| poisoned())?;
        match self
            .ledger
            .lock()
            .map_err(|_| poisoned())?
            .reserve_fenced(&request, admission == &worker.token)?
        {
            Admission::Existing(outcome) => return Ok(outcome),
            Admission::New => (),
        }
        // A new admission is possible only after the prior worker persisted a
        // terminal result. Release the ledger guard before joining that worker.
        if let Some(previous) = worker.handle.take() {
            previous.join().map_err(|_| poisoned())??;
        }
        let ledger = self.ledger.clone();
        let client = self.client.clone();
        let address = self.address;
        let id = request.id().clone();
        match std::thread::Builder::new()
            .name("phone-command".into())
            .spawn(move || {
                let outcome = exchange_command(&client, address, &request, Duration::from_secs(5));
                ledger
                    .lock()
                    .map_err(|_| poisoned())?
                    .finish(request.id(), outcome)
            }) {
            Ok(handle) => {
                worker.handle = Some(handle);
                Ok(Outcome::Pending)
            }
            Err(error) => {
                // No worker was started, so this command cannot have been sent.
                self.ledger
                    .lock()
                    .map_err(|_| poisoned())?
                    .finish(&id, Outcome::NotSent)?;
                Err(Error::Io(error))
            }
        }
    }
    /// Exclude earlier unadmitted requests, without cancelling journaled work.
    pub fn fence(
        &self,
        socket: &UnixStream,
        epoch: &Identity,
        session: &Identity,
    ) -> Result<(Identity, Vec<crate::local_reply::RecoveryItem>), Error> {
        if session != &self.session.identity_for(socket)? {
            return Err(Error::Refused("stale service session"));
        }
        // Same lock order as admission. No token check outside this lock can
        // authorize a new reservation after the barrier.
        let mut admission = self.worker.lock().map_err(|_| poisoned())?;
        let ledger = self.ledger.lock().map_err(|_| poisoned())?;
        let items = Self::recovery_items(&ledger, epoch)?;
        admission.token = Identity(uuid::Uuid::new_v4().simple().to_string());
        Ok((admission.token.clone(), items))
    }
    pub fn recovery(
        &self,
        socket: &UnixStream,
        epoch: &Identity,
    ) -> Result<Vec<crate::local_reply::RecoveryItem>, Error> {
        self.session.identity_for(socket)?;
        let ledger = self.ledger.lock().map_err(|_| poisoned())?;
        Self::recovery_items(&ledger, epoch)
    }
    fn recovery_items(
        ledger: &Ledger,
        epoch: &Identity,
    ) -> Result<Vec<crate::local_reply::RecoveryItem>, Error> {
        if ledger.epoch() != epoch {
            return Err(Error::Refused("stale journal epoch"));
        }
        let ids = ledger.unresolved()?;
        if ids.len() > 1 {
            return Err(Error::Refused("journal admission invariant violated"));
        }
        ids.into_iter()
            .map(|id| {
                let receipt = ledger
                    .lookup(epoch, &id)?
                    .ok_or(Error::Refused("missing recovery receipt"))?;
                let outcome = match receipt.outcome() {
                    Outcome::Pending => crate::local_reply::Outcome::Pending,
                    Outcome::Unknown => crate::local_reply::Outcome::Unknown,
                    _ => return Err(Error::Refused("invalid recovery receipt")),
                };
                Ok(crate::local_reply::RecoveryItem { id, outcome })
            })
            .collect()
    }
    pub fn lookup(
        &self,
        socket: &UnixStream,
        epoch: &Identity,
        id: &Identity,
    ) -> Result<Option<StoredReceipt>, Error> {
        self.session.identity_for(socket)?;
        self.ledger
            .lock()
            .map_err(|_| poisoned())?
            .lookup(epoch, id)
    }
    pub fn acknowledge(
        &self,
        socket: &UnixStream,
        epoch: &Identity,
        id: &Identity,
    ) -> Result<(), Error> {
        self.session.identity_for(socket)?;
        self.ledger
            .lock()
            .map_err(|_| poisoned())?
            .acknowledge(epoch, id)
    }
}
impl Drop for Coordinator {
    fn drop(&mut self) {
        // No borrowed local socket or caller cancellation controls this join.
        // A failed worker leaves pending state for recovery; never erase it.
        let slot = self
            .worker
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(worker) = slot.handle.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod presentation_contention_tests {
    use super::*;
    #[test]
    fn busy_policy_does_not_wait_or_consume_the_banner_attempt() {
        let state = Arc::new(Mutex::new(PresentationState {
            dismissals: crate::notification_policy::Dismissals::default(),
            attempts: crate::notification_policy::BannerAttempts::default(),
            failed: false,
        }));
        let id = Identity::try_from("dddddddddddddddddddddddddddddddd".to_owned()).unwrap();
        let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,
            "generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,
            "source":"android.telecom.InCallService","source_state":"observed",
            "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]}))
        .unwrap();
        let current = crate::watch::decode_snapshot(&bytes, &id).unwrap();
        let held = state.lock().unwrap();
        let other_state = state.clone();
        let other_current = current.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = done_tx.send(try_claim_presentation(&other_state, &other_current));
        });
        let error = done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("claim must not wait behind the policy lock")
            .unwrap_err();
        assert!(
            matches!(error, Error::Io(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        drop(held);
        worker.join().unwrap();
        assert!(try_claim_presentation(&state, &current).unwrap());
        assert!(!try_claim_presentation(&state, &current).unwrap());
    }
}
