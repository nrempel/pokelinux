//! One notification bus generation. The service owns episode policy and event
//! delivery above this loop; no call command or application launch occurs here.
use crate::{
    notification::{Completion, Notifications},
    notification_bus::{NotificationBus, NotificationEvent},
    notification_inputs::{PresentationInputs, PresentationUpdate},
    notification_pending::wait_pending,
    notification_visible::{EndReason, VisibleEvent, VisibleNotification},
    watch::Snapshot,
};
use futures_lite::future;
use std::{io, time::Duration};

pub enum Notice {
    /// Navigation intent only. The receiver applies current activation policy.
    OpenPhone,
    /// Original banner cohort, never a newer call observed after dispatch.
    Dismissed(Snapshot),
}
fn now() -> Duration {
    let clock = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32)
}
enum Idle {
    Input(io::Result<PresentationUpdate>),
    Signal(zbus::Result<NotificationEvent>),
}

/// Run discovery, dispatch, visible handling and cleanup on one connection.
/// `claim` reserves the service-owned episode attempt immediately before send;
/// `emit` enqueues notices for that service. Both callbacks must be short and
/// nonblocking; disk writes and application activation belong outside this loop.
///
/// On any error, discard this connection and guard. Reconnect with the same
/// service episode policy and inputs, never with old notification handles.
/// Producer loss ends this generation after bounded cleanup of a known banner.
/// Cancellation while Notify/Close is in flight aborts their original socket.
/// The caller must let normal producer-loss cleanup finish during shutdown.
pub async fn run_generation(
    bus: &NotificationBus,
    inputs: &mut PresentationInputs,
    claim: &mut impl FnMut(&Snapshot) -> io::Result<bool>,
    emit: &mut impl FnMut(Notice) -> io::Result<()>,
) -> io::Result<()> {
    let mut signals = bus.subscribe().await.map_err(io::Error::other)?;
    let mut owner = bus.discover(&mut signals).await.map_err(io::Error::other)?;
    let mut guard = Notifications::default();
    let mut latest = inputs.receive(&mut guard).await?;
    loop {
        // Consume a queued revocation before deciding to submit new work.
        // The capacity-one channel preserves intermediate revocation epochs.
        if let Some(update) = future::poll_once(inputs.receive(&mut guard)).await {
            latest = update?;
        }
        if let (Some(destination), Some(lease)) = (owner.as_deref(), latest.lease())
            && let Some(request) = guard.begin(destination, lease, latest.allowed(), now())
        {
            if claim(lease.snapshot())? {
                let reply =
                    wait_pending(bus, &mut signals, inputs, &mut guard, request, latest).await?;
                match VisibleNotification::new(reply) {
                    Ok(mut visible) => {
                        let outcome = loop {
                            match visible.next(&mut signals, inputs).await {
                                Ok(VisibleEvent::OpenPhone) => {
                                    if let Err(error) = emit(Notice::OpenPhone) {
                                        break Err(error);
                                    }
                                }
                                Ok(VisibleEvent::End(reason)) => {
                                    if reason == EndReason::Dismissed
                                        && let Err(error) =
                                            emit(Notice::Dismissed(visible.presented().clone()))
                                    {
                                        break Err(error);
                                    }
                                    break Ok(reason);
                                }
                                Err(error) => break Err(error),
                            }
                        };
                        let server_closed = visible.server_closed();
                        let (delivery, current, current_owner) = visible.into_parts();
                        let cleanup = if server_closed {
                            Ok(())
                        } else {
                            delivery.close().await.map_err(io::Error::other)
                        };
                        outcome?;
                        cleanup?;
                        latest = current;
                        owner = current_owner;
                    }
                    Err(reply) => {
                        // A valid early action may precede server-side closure.
                        // Preserve navigation separately from display cleanup.
                        let notice_result = (|| {
                            if reply.open_requested {
                                emit(Notice::OpenPhone)?;
                            }
                            if reply.dismissed
                                && let Completion::Close(handle) = &reply.completion
                            {
                                emit(Notice::Dismissed(handle.presented().clone()))?;
                            }
                            Ok::<(), io::Error>(())
                        })();
                        let cleanup = if reply.closed {
                            Ok(())
                        } else {
                            reply.delivery.close().await.map_err(io::Error::other)
                        };
                        notice_result?;
                        cleanup?;
                        latest = reply.latest;
                        owner = reply.owner;
                    }
                }
                continue;
            }
            // The service declined this cosmetic attempt before any send.
            guard.not_sent(request);
        }
        match future::or(
            async { Idle::Input(inputs.receive(&mut guard).await) },
            async { Idle::Signal(signals.next(owner.as_deref()).await) },
        )
        .await
        {
            Idle::Input(update) => latest = update?,
            Idle::Signal(event) => {
                if let NotificationEvent::OwnerChanged { new_owner, .. } =
                    event.map_err(io::Error::other)?
                {
                    owner = (!new_owner.is_empty()).then_some(new_owner);
                }
                // Signals for retired or other applications' IDs have no
                // authority over a future notification or call episode.
            }
        }
        future::yield_now().await;
    }
}
