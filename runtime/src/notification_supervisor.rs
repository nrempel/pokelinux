//! Serial connection supervision. Service episode state lives in the caller's
//! claim callback and is never reconstructed when a bus generation fails.
use crate::{
    notification_bus::NotificationBus,
    notification_generation::{Notice, run_generation},
    notification_inputs::PresentationInputs,
    watch::Snapshot,
};
use std::{io, time::Duration};
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Reconnect on transport failure or transient claim contention with a bounded
/// retry rate. A permanent local policy/event error stops the worker. Keep the
/// same claim owner and latest-state input channel for this entire lifetime.
/// The callbacks have the nonblocking contract of run_generation.
///
/// Dropping the input producer requests shutdown. Normal generation cleanup is
/// awaited, and the old socket is then aborted before any replacement starts.
/// Setup/close bounds remain those of the bus adapter; shutdown during backoff
/// waits at most one retry interval while the process is scheduled.
pub async fn run_notifications(
    address: &str,
    inputs: &mut PresentationInputs,
    claim: &mut impl FnMut(&Snapshot) -> io::Result<bool>,
    emit: &mut impl FnMut(Notice) -> io::Result<()>,
) -> io::Result<()> {
    loop {
        if inputs.is_closed() {
            return Ok(());
        }
        match NotificationBus::connect(address).await {
            Ok(bus) => {
                let mut fatal_claim = false;
                let mut failed_emit = false;
                let result = {
                    let mut checked_claim = |snapshot: &Snapshot| {
                        claim(snapshot).inspect_err(|error| {
                            fatal_claim = error.kind() != io::ErrorKind::WouldBlock;
                        })
                    };
                    let mut checked_emit =
                        |notice| emit(notice).inspect_err(|_| failed_emit = true);
                    run_generation(&bus, inputs, &mut checked_claim, &mut checked_emit).await
                };
                // No method/stream clone from this generation may remain usable
                // after reconnect. Aborting does not prove remote banner removal.
                bus.abort();
                if fatal_claim || failed_emit {
                    return result;
                }
                if inputs.is_closed() {
                    return Ok(());
                }
                // Other errors are transport failure or transient claim contention.
            }
            Err(zbus::Error::Address(error)) => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, error));
            }
            Err(_) => (),
        }
        if inputs.is_closed() {
            return Ok(());
        }
        // Do not consume updates during backoff: the bounded publisher retains
        // its newest full state and revocation epoch for the next generation.
        async_io::Timer::after(RECONNECT_DELAY).await;
    }
}
