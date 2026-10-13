fn main() -> std::io::Result<()> {
    use std::path::Path;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--messages-worker") {
        return pokelinux_messages::worker::entry(&args[1..]);
    }
    if args.len() != 6 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let mut messages = pokelinux_messages::worker::Projector::spawn(
        Path::new(&args[0]),
        Path::new(&args[4]),
        Path::new(&args[5]),
        &std::env::current_exe()?,
    )?;
    let result = pokelinux_lifecycle::service::run_with_messages(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        None,
        Some(&mut messages),
    );
    use pokelinux_lifecycle::messages::Provider;
    if !messages.drain_until(pokelinux_lifecycle::boottime_ns().saturating_add(2_000_000_000)) {
        std::process::abort();
    }
    result
}
