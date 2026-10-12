//! Service presentation orchestration. Session/lock/power policy comes from the
//! host adapter; this owner neither unlocks the compositor nor controls calls.
use crate::{
    activation_worker::ActivationSlot, coordinator::Coordinator, ledger,
    notification_events::NoticeReceiver, notification_generation::Notice,
    notification_worker::NotificationWorker, observer::Lease, watch::Snapshot,
};
use std::{io, os::unix::net::UnixStream, sync::Arc, time::Duration};

pub struct PresentationOwner {
    worker: NotificationWorker,
    activation: ActivationSlot,
    coordinator: Arc<Coordinator>,
    address: String,
    dismissal_context: Option<Snapshot>,
    allowed: bool,
}
fn policy_error(error: ledger::Error) -> io::Error {
    match error {
        ledger::Error::Io(error) => error,
        _ => io::Error::other("presentation policy unavailable"),
    }
}
fn now() -> Duration {
    let clock = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32)
}
impl PresentationOwner {
    pub fn start(address: String, coordinator: Arc<Coordinator>) -> io::Result<Self> {
        let claims = coordinator.clone();
        let worker = NotificationWorker::start(address.clone(), move |snapshot| {
            claims.claim_presentation(snapshot).map_err(policy_error)
        })?;
        Ok(Self {
            worker,
            activation: ActivationSlot::default(),
            coordinator,
            address,
            dismissal_context: None,
            allowed: false,
        })
    }
    /// Supply a complete current observation and current host eligibility. A
    /// journal failure withdraws presentation before returning to the service.
    /// Persistence is synchronous here, never in the notification worker.
    pub fn publish(&mut self, lease: Option<Lease>, allowed: bool) -> io::Result<()> {
        self.allowed = false;
        if !allowed {
            self.worker.publish(None, false)?;
        }
        if let Some(current) = &lease
            && current.expires() > now()
        {
            if let Err(error) = self.coordinator.observe_presentation(current.snapshot()) {
                let _ = self.worker.publish(None, false);
                return Err(policy_error(error));
            }
            // Keep only dismissal context across gaps. It never supplies a
            // display lease or command authority, and newer observations replace it.
            self.dismissal_context = Some(current.snapshot().clone());
        }
        self.worker.publish(lease, allowed)?;
        self.allowed = allowed;
        Ok(())
    }
    pub fn notices(&self) -> &NoticeReceiver {
        self.worker.events()
    }
    pub fn activation_completion(&self) -> Option<&UnixStream> {
        self.activation.completion()
    }
    pub fn finish_activation(&mut self) -> io::Result<Option<zbus::Result<()>>> {
        self.activation.finish()
    }
    /// Process one bounded batch. An OpenPhone notice is navigation only; its
    /// current host policy is checked here, not copied from the original banner.
    /// Explicit dismissal affects only the original/current episode intersection.
    pub fn drain_notices(&mut self) -> io::Result<()> {
        let batch = self.worker.events().drain()?;
        if batch.closed {
            self.allowed = false;
            self.worker.shutdown()?;
            return Err(io::Error::other("notification worker stopped"));
        }
        for notice in batch.notices {
            match notice {
                Notice::OpenPhone if self.allowed => {
                    match self.activation.start(self.address.clone()) {
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
                        result => result?,
                    }
                }
                Notice::Dismissed(presented) => {
                    if let Some(current) = &self.dismissal_context
                        && let Err(error) =
                            self.coordinator.dismiss_presentation(&presented, current)
                    {
                        self.allowed = false;
                        let _ = self.worker.publish(None, false);
                        return Err(policy_error(error));
                    }
                }
                Notice::OpenPhone => (),
            }
        }
        Ok(())
    }
    pub fn shutdown(&mut self) -> io::Result<()> {
        self.allowed = false;
        let notifications = self.worker.shutdown();
        // Join bounded activation before returning even if notification cleanup
        // failed. Drop never leaves a detached app-start request behind.
        drop(std::mem::take(&mut self.activation));
        notifications
    }
}
