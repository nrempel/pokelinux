//! Event pump for one unresolved Notify. No retry, launch or call commands.
use crate::{
    lease_timer::LeaseTimer,
    notification::{Completion, Notifications, NotifyRequest},
    notification_bus::{
        CloseReason, Delivery, NotificationBus, NotificationEvent, NotificationSignals,
    },
    notification_inputs::{PresentationInputs, PresentationUpdate},
};
use futures_lite::future;
use std::{collections::HashMap, io, time::Duration};

// Keep a short late-ID cleanup opportunity after the original banner lease.
// This is an absolute BOOTTIME bound, not a per-message or renewal timeout.
const LATE_REPLY_GRACE: Duration = Duration::from_millis(1500);

pub struct PendingReply {
    pub delivery: Delivery,
    pub completion: Completion,
    pub latest: PresentationUpdate,
    pub owner: Option<String>,
    /// A matching default action was received while pending. The outer loop
    /// must still check current policy before activating the fixed Phone app.
    /// This can be true with Completion::Close when the server closed the banner
    /// after the action; closing display must not erase that earlier intent.
    pub open_requested: bool,
    /// Matching user dismissal, for the outer loop's ringing-episode latch.
    /// Expiry and programmatic closure must not be treated as user dismissal.
    pub dismissed: bool,
    /// The matching ID was already closed by the server. Do not close it again.
    pub closed: bool,
}
#[derive(Clone, Copy, Default)]
struct EarlySignals {
    closed: bool,
    dismissed: bool,
    action: bool,
}
fn apply_signal(
    signal: NotificationEvent,
    owner: &mut Option<String>,
    early: &mut HashMap<u32, EarlySignals>,
    guard: &mut Notifications,
) -> io::Result<()> {
    match signal {
        NotificationEvent::OwnerChanged { new_owner, .. } => {
            guard.invalidate();
            *owner = (!new_owner.is_empty()).then_some(new_owner);
        }
        event => {
            let (id, reason) = match event {
                NotificationEvent::Closed { id, reason } => (id, Some(reason)),
                NotificationEvent::OpenPhone { id } => (id, None),
                NotificationEvent::OwnerChanged { .. } => unreachable!(),
            };
            if early.len() == 32 && !early.contains_key(&id) {
                return Err(io::Error::other("too many early notification IDs"));
            }
            let state = early.entry(id).or_default();
            if let Some(reason) = reason {
                state.closed = true;
                state.dismissed |= reason == CloseReason::Dismissed;
            } else if !state.closed {
                state.action = true;
            }
        }
    }
    Ok(())
}
fn now() -> Duration {
    let clock = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32)
}
enum Event {
    Input(io::Result<PresentationUpdate>),
    Expiry(io::Result<()>),
    Signal(zbus::Result<NotificationEvent>),
    Reply(zbus::Result<Delivery>),
}

/// The subscription must precede owner discovery and Notify. On ANY error,
/// discard this bus generation and its signal stream; a cancelled partial send
/// cannot safely share a connection with subsequent messages. Unknown delivery
/// is not proof of remote removal. The caller owns bounded cleanup and shutdown.
/// An unresolved operation ends 1.5 seconds after its original lease expires:
/// cancellation aborts the original socket, and the caller must start a new bus
/// generation while retaining service-owned attempted-episode state.
///
/// Before completion, drain signals ordered before the reply on this connection.
/// The outer owner loop must still check newer policy/events before activation
/// and continue watching an adopted delivery.
pub async fn wait_pending(
    bus: &NotificationBus,
    signals: &mut NotificationSignals,
    inputs: &mut PresentationInputs,
    guard: &mut Notifications,
    request: NotifyRequest,
    mut latest: PresentationUpdate,
) -> io::Result<PendingReply> {
    let result = async {
        let timer = LeaseTimer::new()?.into_async()?;
        let retire_at = request
            .expires()
            .checked_add(LATE_REPLY_GRACE)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "notification deadline overflow",
                )
            })?;
        let mut owner = Some(request.owner().to_owned());
        // At most 32 early IDs. They are only candidates until the reply gives
        // us our own ID; an unrelated notification must not dismiss this call.
        let mut early = HashMap::<u32, EarlySignals>::new();
        let mut expired = false;
        let delivery = {
            let mut pending = Box::pin(bus.notify(&request));
            loop {
                let current_time = now();
                if current_time >= retire_at {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "notification reply deadline reached; delivery unknown",
                    ));
                }
                let deadline = latest.lease().map_or(request.expires(), |lease| {
                    request.expires().min(lease.expires())
                });
                if !latest.allowed() || latest.lease().is_none() || current_time >= deadline {
                    guard.invalidate();
                }
                if current_time >= deadline {
                    expired = true;
                }
                timer.arm(Some(if expired { retire_at } else { deadline }))?;
                let event = future::or(
                    async { Event::Expiry(timer.expired().await) },
                    future::or(
                        async { Event::Input(inputs.receive(guard).await) },
                        future::or(
                            async { Event::Signal(signals.next(Some(request.owner())).await) },
                            async { Event::Reply(pending.as_mut().await) },
                        ),
                    ),
                )
                .await;
                match event {
                    Event::Expiry(result) => {
                        result?;
                        // Deadline is checked again on every iteration, even
                        // when several sources become ready at the same time.
                    }
                    Event::Input(update) => latest = update?,
                    Event::Signal(signal) => apply_signal(
                        signal.map_err(io::Error::other)?,
                        &mut owner,
                        &mut early,
                        guard,
                    )?,
                    Event::Reply(result) => break result.map_err(io::Error::other)?,
                }
                // Cooperative progress for the bus executor and state producer
                // even when one event source remains continuously ready.
                future::yield_now().await;
            }
        };
        while let Some(signal) = signals
            .before_reply(&delivery)
            .await
            .map_err(io::Error::other)?
        {
            apply_signal(signal, &mut owner, &mut early, guard)?;
        }
        let early = early.get(&delivery.id()).copied().unwrap_or_default();
        let completion = guard.complete(
            request,
            delivery.id(),
            owner.as_deref(),
            latest.lease(),
            latest.allowed(),
            now(),
        );
        let open_requested = early.action && matches!(completion, Completion::Visible(_));
        // Separate display eligibility from a valid action received before
        // closure. External revocation/expiry still fences both above.
        let completion = match completion {
            Completion::Visible(handle) if early.closed => Completion::Close(handle),
            other => other,
        };
        Ok(PendingReply {
            delivery,
            completion,
            latest,
            owner,
            open_requested,
            dismissed: early.dismissed,
            closed: early.closed,
        })
    }
    .await;
    if result.is_err() {
        guard.invalidate();
    }
    result
}
