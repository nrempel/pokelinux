//! Service-owned notification thread. Drop closes inputs and joins; no detached
//! notification loop may retain a connection after its service owner is gone.
use crate::{
    notification_events::{NoticeReceiver, notice_channel},
    notification_inputs::{PresentationPublisher, presentation_channel},
    notification_supervisor::run_notifications,
    observer::Lease,
    watch::Snapshot,
};
use std::{io, thread};

pub struct NotificationWorker {
    publisher: Option<PresentationPublisher>,
    events: NoticeReceiver,
    thread: Option<thread::JoinHandle<io::Result<()>>>,
}
impl NotificationWorker {
    /// Claim must be short and nonblocking, as required by run_notifications.
    /// Keep its policy owner alive across all reconnections of this worker.
    pub fn start(
        address: String,
        mut claim: impl FnMut(&Snapshot) -> io::Result<bool> + Send + 'static,
    ) -> io::Result<Self> {
        let (publisher, mut inputs) = presentation_channel();
        let (sender, events) = notice_channel()?;
        let thread = thread::Builder::new()
            .name("phone-notifications".into())
            .spawn(move || {
                futures_lite::future::block_on(run_notifications(
                    &address,
                    &mut inputs,
                    &mut claim,
                    &mut |notice| sender.send(notice),
                ))
            })?;
        Ok(Self {
            publisher: Some(publisher),
            events,
            thread: Some(thread),
        })
    }
    pub fn publish(&mut self, lease: Option<Lease>, allowed: bool) -> io::Result<()> {
        self.publisher
            .as_mut()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "notification worker stopped")
            })?
            .publish(lease, allowed)
    }
    pub fn events(&self) -> &NoticeReceiver {
        &self.events
    }
    pub fn is_finished(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(thread::JoinHandle::is_finished)
    }
    /// Close the producer before joining so idle waits and reconnects can exit.
    /// Reports the worker's terminal error once. Event receiver remains drainable.
    pub fn shutdown(&mut self) -> io::Result<()> {
        drop(self.publisher.take());
        match self.thread.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| io::Error::other("notification worker failed"))?,
            None => Ok(()),
        }
    }
}
impl Drop for NotificationWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
