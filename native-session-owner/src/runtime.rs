//! One production session composition, retaining Source custody on every return.
use crate::{Compositor, MAX_SOURCE_STARTUP_NS, SourceOwner, config::Config};
use pokelinux_lifecycle::messages::Provider as _;
use pokelinux_lifecycle::{boottime_ns, session::Provider};
use pokelinux_messages::read_service::Remote;
use std::{io, time::Duration};
pub const CLEANUP_BUDGET_NS: u64 = 2_000_000_000;
/// Every caller retains the same SourceOwner through successful actual reap.
/// The unchanged absolute cleanup cutoff is selected once by the caller.
pub fn drain_after<T>(
    source: &mut SourceOwner,
    outcome: io::Result<T>,
    until: u64,
) -> io::Result<T> {
    if !source.occupied() {
        return outcome;
    }
    source.stop();
    while source.occupied() {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Terminal facts still poll the actual Child wait; neither errors nor
        // signal delivery clear ownership. The original cutoff never changes.
        let _ = source.poll();
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if source.occupied() {
            std::thread::sleep(POLL_CADENCE);
        }
    }
    outcome
}

fn selected_brightness(
    selection: &crate::config::Brightness,
) -> io::Result<Option<pokelinux_lifecycle::brightness::Owner>> {
    match selection {
        crate::config::Brightness::Disabled => Ok(None),
        crate::config::Brightness::Owner { socket } => {
            pokelinux_lifecycle::brightness::Owner::new(socket).map(Some)
        }
    }
}

fn selected_connectivity(
    selection: &crate::config::Connectivity,
) -> io::Result<Option<pokelinux_lifecycle::connectivity::Owner>> {
    match selection {
        crate::config::Connectivity::Disabled {} => Ok(None),
        crate::config::Connectivity::Owner { socket } => {
            pokelinux_lifecycle::connectivity::Owner::new(socket).map(Some)
        }
    }
}

pub fn run(config: &Config) -> io::Result<()> {
    let helper = std::env::current_exe()?;
    pokelinux_lifecycle::service::trusted_executable(&helper)?;
    // Arm before compositor custody can exist; keep this process-owned guard
    // alive through every Source/history/actor drain and occupancy check.
    let _termination = pokelinux_kernel_identity::TerminationGuard::install()?;
    let until = boottime_ns()
        .checked_add(MAX_SOURCE_STARTUP_NS)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let mut source = SourceOwner::spawn(
        Compositor {
            helper,
            executable: config.compositor_executable.clone(),
            arguments: vec![
                "--config".into(),
                config
                    .compositor_config
                    .to_str()
                    .ok_or(io::ErrorKind::InvalidInput)?
                    .into(),
            ],
            environment: config.environment.clone(),
            source: pokelinux_lifecycle::fresh_id()?,
        },
        until,
    )?;
    // Retain the optional read connection outside subsequent fallible setup.
    let mut messages = None;
    let outcome = (|| {
        if let crate::config::Messages::Owner { socket } = &config.messages {
            messages = Some(Remote::connect(socket)?);
        }
        let mut display = crate::display::Factory::new(
            config.display_parent.clone(),
            config.wayland.clone(),
            source.duplicate_pidfd()?,
            pokelinux_display::SourceIdentity {
                pid: source.pid(),
                uid: 1000,
                gid: 1000,
            },
            std::env::current_exe()?,
        )?;
        let mut startup = std::env::var_os("NOTIFY_SOCKET")
            .map(|destination| {
                crate::readiness::Readiness::new(
                    config.wayland.clone(),
                    source.duplicate_pidfd()?,
                    pokelinux_display::SourceIdentity {
                        pid: source.pid(),
                        uid: 1000,
                        gid: 1000,
                    },
                    destination,
                    until,
                )
            })
            .transpose()?;
        let mut audio = pokelinux_lifecycle::audio::Owner::connect(&config.audio_socket, 11040)?;
        let mut brightness = selected_brightness(&config.brightness)?;
        let mut connectivity = selected_connectivity(&config.connectivity)?;
        pokelinux_lifecycle::service::run_with_connectivity_services(
            &config.runtime,
            &config.policy,
            &config.profiles,
            &config.app_entry,
            None,
            messages
                .as_mut()
                .map(|p| p as &mut dyn pokelinux_lifecycle::messages::Provider),
            Some(&mut source),
            Some(&mut display),
            None,
            Some(&mut audio),
            brightness
                .as_mut()
                .map(|p| p as &mut dyn pokelinux_lifecycle::brightness::Provider),
            connectivity
                .as_mut()
                .map(|p| p as &mut dyn pokelinux_lifecycle::connectivity::Provider),
            startup
                .as_mut()
                .map(|observer| observer as &mut dyn pokelinux_lifecycle::startup::Observer),
        )
    })();
    // The actor drains its Source internally on normal/loop-error completion.
    // This outer original budget also covers setup errors before that loop.
    let cleanup_until = boottime_ns().saturating_add(CLEANUP_BUDGET_NS);
    // Fence local reads and Source before the original shared cleanup cutoff.
    // Remote closure does not stop the daemon or certify its reader retirement.
    source.stop();
    if let Some(p) = &mut messages {
        p.stop();
    }
    let history_drained = messages
        .as_mut()
        .is_none_or(|p| p.drain_until(cleanup_until));
    let outcome = if history_drained {
        outcome
    } else {
        Err(io::ErrorKind::TimedOut.into())
    };
    let outcome = drain_after(&mut source, outcome, cleanup_until);
    if source.occupied() || messages.as_ref().is_some_and(|p| p.occupied()) {
        std::process::abort();
    }
    outcome
}
const POLL_CADENCE: Duration = Duration::from_millis(2);

#[cfg(test)]
#[path = "runtime_brightness_tests.rs"]
mod brightness_tests;
