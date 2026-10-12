//! Capacity-one presentation input. Replacement retains the history needed to
//! revoke old work; a fast loss-and-return cannot disappear through coalescing.
use crate::{Identity, notification::Notifications, observer::Lease};
use std::io;

pub struct PresentationUpdate {
    lease: Option<Lease>,
    allowed: bool,
    epoch: Identity,
}
impl PresentationUpdate {
    pub fn lease(&self) -> Option<&Lease> {
        self.lease.as_ref()
    }
    pub fn allowed(&self) -> bool {
        self.allowed
    }
}
/// One producer owns publication order. Deliberately not Clone.
pub struct PresentationPublisher {
    sender: async_channel::Sender<PresentationUpdate>,
    continuity: Option<Identity>,
    epoch: Identity,
}
pub struct PresentationInputs {
    receiver: async_channel::Receiver<PresentationUpdate>,
    epoch: Option<Identity>,
}
fn epoch() -> Identity {
    Identity(uuid::Uuid::new_v4().simple().to_string())
}
fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "presentation input closed")
}
pub fn presentation_channel() -> (PresentationPublisher, PresentationInputs) {
    let (sender, receiver) = async_channel::bounded(1);
    (
        PresentationPublisher {
            sender,
            continuity: None,
            epoch: epoch(),
        },
        PresentationInputs {
            receiver,
            epoch: None,
        },
    )
}
impl PresentationPublisher {
    /// Never waits for the notification loop. Keep only the latest complete
    /// state, but change epoch on every eligibility/continuity transition.
    /// Consumer decisions must still check BOOTTIME deadlines independently.
    pub fn publish(&mut self, lease: Option<Lease>, allowed: bool) -> io::Result<()> {
        let continuity = lease
            .as_ref()
            .filter(|_| allowed)
            .map(|lease| lease.continuity().clone());
        if continuity != self.continuity {
            self.epoch = epoch();
            self.continuity = continuity;
        }
        self.sender
            .force_send(PresentationUpdate {
                lease,
                allowed,
                epoch: self.epoch.clone(),
            })
            .map(|_| ())
            .map_err(|_| closed())
    }
}
impl PresentationInputs {
    /// Producer loss is the notification worker's shutdown request.
    pub(crate) fn is_closed(&self) -> bool {
        self.receiver.is_closed()
    }
    /// Apply revocation before exposing the next desired state. Keep polling
    /// this concurrently with pending Notify; do not wait for Notify first.
    pub async fn receive(&mut self, guard: &mut Notifications) -> io::Result<PresentationUpdate> {
        match self.receive_state().await {
            Ok((update, changed)) => {
                if changed {
                    guard.invalidate();
                }
                Ok(update)
            }
            Err(error) => {
                guard.invalidate();
                Err(error)
            }
        }
    }
    /// Shared by pending and visible lifecycle phases. The epoch comparison
    /// advances exactly once when a complete latest state is consumed.
    pub(crate) async fn receive_state(&mut self) -> io::Result<(PresentationUpdate, bool)> {
        let received = self.receiver.recv().await;
        if self.receiver.is_closed() {
            return Err(closed());
        }
        let update = received.map_err(|_| closed())?;
        let changed = self
            .epoch
            .as_ref()
            .is_some_and(|epoch| epoch != &update.epoch);
        self.epoch = Some(update.epoch.clone());
        Ok((update, changed))
    }
}
