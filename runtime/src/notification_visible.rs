//! Continuous observation of one adopted desktop notification. OpenPhone is an
//! intent for the owner to recheck and activate; it grants no call authority.
use crate::{
    lease_timer::LeaseTimer,
    notification::{Completion, NotificationHandle},
    notification_bus::{CloseReason, Delivery, NotificationEvent, NotificationSignals},
    notification_inputs::{PresentationInputs, PresentationUpdate},
    notification_pending::PendingReply,
};
use futures_lite::future;
use std::{io, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    Expired,
    Revoked,
    OwnerChanged,
    Dismissed,
    ServerClosed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleEvent {
    OpenPhone,
    End(EndReason),
}
pub struct VisibleNotification {
    delivery: Delivery,
    handle: NotificationHandle,
    latest: PresentationUpdate,
    owner: Option<String>,
    finished: Option<EndReason>,
    open_pending: bool,
    opened: bool,
}
enum Event {
    Expiry(io::Result<()>),
    Input(io::Result<(PresentationUpdate, bool)>),
    Signal(zbus::Result<NotificationEvent>),
    Open,
}
fn now() -> Duration {
    let clock = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32)
}
impl VisibleNotification {
    /// Closure can be latched while returning an earlier OpenPhone intent.
    /// Cleanup must not infer remote state from event-delivery success.
    pub(crate) fn server_closed(&self) -> bool {
        matches!(
            self.finished,
            Some(EndReason::Dismissed | EndReason::ServerClosed)
        )
    }
    pub fn presented(&self) -> &crate::watch::Snapshot {
        self.handle.presented()
    }
    /// A rejected reply is returned intact for its required cleanup.
    pub fn new(reply: PendingReply) -> Result<Self, Box<PendingReply>> {
        if !matches!(reply.completion, Completion::Visible(_)) {
            return Err(Box::new(reply));
        }
        let Completion::Visible(handle) = reply.completion else {
            unreachable!()
        };
        Ok(Self {
            delivery: reply.delivery,
            handle,
            latest: reply.latest,
            owner: reply.owner,
            finished: None,
            open_pending: reply.open_requested,
            opened: false,
        })
    }
    fn end(&mut self, reason: EndReason) -> VisibleEvent {
        self.finished = Some(reason);
        VisibleEvent::End(reason)
    }
    /// Returns at most one OpenPhone intent per delivery. Terminal state is
    /// latched; renewed permission or a reused server ID cannot revive it.
    /// Errors withdraw authority; the caller still owns cleanup of the handle.
    pub async fn next(
        &mut self,
        signals: &mut NotificationSignals,
        inputs: &mut PresentationInputs,
    ) -> io::Result<VisibleEvent> {
        if !signals.belongs_to(&self.delivery) {
            self.finished = Some(EndReason::Revoked);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "notification signal stream belongs to another connection",
            ));
        }
        let result = self.wait(signals, inputs).await;
        if result.is_err() {
            self.finished = Some(EndReason::Revoked);
        }
        result
    }
    async fn wait(
        &mut self,
        signals: &mut NotificationSignals,
        inputs: &mut PresentationInputs,
    ) -> io::Result<VisibleEvent> {
        if let Some(reason) = self.finished {
            return Ok(VisibleEvent::End(reason));
        }
        let timer = LeaseTimer::new()?.into_async()?;
        loop {
            if !self.latest.allowed() || self.latest.lease().is_none() {
                return Ok(self.end(EndReason::Revoked));
            }
            let deadline = self
                .handle
                .expires()
                .min(self.latest.lease().unwrap().expires());
            if now() >= deadline {
                return Ok(self.end(EndReason::Expired));
            }
            timer.arm(Some(deadline))?;
            let event = future::or(
                async { Event::Expiry(timer.expired().await) },
                future::or(
                    async { Event::Input(inputs.receive_state().await) },
                    future::or(
                        async { Event::Signal(signals.next(Some(self.handle.owner())).await) },
                        async {
                            if self.open_pending && !self.opened {
                                Event::Open
                            } else {
                                std::future::pending().await
                            }
                        },
                    ),
                ),
            )
            .await;
            match event {
                Event::Expiry(result) => {
                    result?;
                }
                Event::Input(update) => {
                    let (latest, changed) = update?;
                    self.latest = latest;
                    if changed {
                        return Ok(self.end(EndReason::Revoked));
                    }
                }
                Event::Signal(signal) => match signal.map_err(io::Error::other)? {
                    NotificationEvent::OwnerChanged { new_owner, .. } => {
                        self.owner = (!new_owner.is_empty()).then_some(new_owner);
                        return Ok(self.end(EndReason::OwnerChanged));
                    }
                    NotificationEvent::Closed { id, reason } if id == self.handle.id() => {
                        let ended = self.end(if reason == CloseReason::Dismissed {
                            EndReason::Dismissed
                        } else {
                            EndReason::ServerClosed
                        });
                        // Servers may close a banner immediately after its
                        // default action. Preserve that earlier user intent;
                        // closure ends display, not an already received click.
                        // Expiry and policy inputs still win above, and the
                        // owner must recheck activation policy independently.
                        if self.open_pending && !self.opened && now() < deadline {
                            self.open_pending = false;
                            self.opened = true;
                            return Ok(VisibleEvent::OpenPhone);
                        }
                        return Ok(ended);
                    }
                    NotificationEvent::OpenPhone { id } if id == self.handle.id() => {
                        self.open_pending = true;
                    }
                    _ => (),
                },
                Event::Open => {
                    if now() >= deadline {
                        return Ok(self.end(EndReason::Expired));
                    }
                    self.open_pending = false;
                    self.opened = true;
                    return Ok(VisibleEvent::OpenPhone);
                }
            }
            future::yield_now().await;
        }
    }
    /// Preserve the current input and owner for the next lifecycle phase.
    /// A Dismissed/ServerClosed result needs no duplicate close request.
    pub fn into_parts(self) -> (Delivery, PresentationUpdate, Option<String>) {
        (self.delivery, self.latest, self.owner)
    }
}
