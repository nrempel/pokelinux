//! Separately built synthetic service composition. Normal messages-service has no selector.
use pokelinux_lifecycle::{boottime_ns, messages::Provider};
use std::{io, path::Path};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--messages-worker") {
        return pokelinux_messages::worker::entry(&args[1..]);
    }
    if args
        .first()
        .is_some_and(|a| a == "--messages-incoming-worker")
    {
        if args.len() != 8 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let generation = args[6]
            .to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .to_owned();
        let fault = args[7]
            .to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .to_owned();
        let backend = native_messages_incoming_binder_fixture::receiver::Synthetic::new(
            generation.clone(),
            fault,
        )?;
        return pokelinux_messages::worker::entry_with_backend(
            &args[1..6],
            Some((generation, Box::new(backend))),
        );
    }
    if args.len() != 8 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut messages = pokelinux_messages::worker::Projector::spawn_receiving(
        Path::new(&args[0]),
        Path::new(&args[4]),
        Path::new(&args[5]),
        &std::env::current_exe()?,
        &args[6..],
    )?;
    let result = pokelinux_lifecycle::service::run_with_messages(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        None,
        Some(&mut messages),
    );
    if !messages.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
        std::process::abort();
    }
    result
}
