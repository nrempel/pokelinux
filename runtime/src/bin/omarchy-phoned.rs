#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use omarchy_phone_core::{
        background::BackgroundObserver, coordinator::Coordinator, enrollment::Enrollment,
        ledger::Ledger, rpc, service_socket::ServiceSocket,
    };
    use rustix::event::{PollFd, PollFlags, poll};
    use std::{io, os::unix::net::UnixStream, path::PathBuf, thread};
    let mut enrollment = None;
    let mut state = None;
    let mut runtime = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(option) = args.next() {
        let slot = match option.to_str() {
            Some("--enrollment") => &mut enrollment,
            Some("--state") => &mut state,
            Some("--runtime") => &mut runtime,
            _ => return Err("expected --enrollment, --state and --runtime directories".into()),
        };
        if slot.is_some() {
            return Err("duplicate option".into());
        }
        let path = PathBuf::from(args.next().ok_or("missing directory")?);
        if !path.is_absolute() {
            return Err("absolute directories required".into());
        }
        *slot = Some(path);
    }
    let enrollment = Enrollment::load(&enrollment.ok_or("missing enrollment")?)?;
    let state = state.ok_or("missing state")?;
    let runtime = runtime.ok_or("missing runtime")?;
    if state.starts_with(&runtime) {
        return Err("persistent state must be outside the runtime directory".into());
    }
    let ledger = Ledger::open(&state)?;
    let (address, client) = enrollment.into_parts();
    let observation_client = client.clone();
    let owner = Coordinator::new(ledger, client, address)?;
    let socket = ServiceSocket::bind(&runtime)?;
    socket.listener().set_nonblocking(true)?;
    let (signal_read, signal_write) = UnixStream::pair()?;
    signal_write.set_nonblocking(true)?;
    let term = signal_hook::low_level::pipe::register(
        signal_hook::consts::SIGTERM,
        signal_write.try_clone()?,
    )?;
    let interrupt =
        signal_hook::low_level::pipe::register(signal_hook::consts::SIGINT, signal_write)?;
    let mut observer = BackgroundObserver::start(observation_client, address)?;
    let mut presentation_failed = false;
    // Poll sleeps until a connection or shutdown signal; no periodic idle wake.
    let result = thread::scope(|scope| -> io::Result<()> {
        let mut connections: Vec<thread::ScopedJoinHandle<'_, ()>> = Vec::new();
        loop {
            let changed = observer.changes();
            let mut fds = [
                PollFd::new(socket.listener(), PollFlags::IN),
                PollFd::new(&signal_read, PollFlags::IN),
                PollFd::new(&changed, PollFlags::IN),
                PollFd::new(observer.timer(), PollFlags::IN),
            ];
            match poll(&mut fds, None) {
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
                Ok(_) => (),
            }
            if !fds[1].revents().is_empty() {
                break;
            }
            if fds[0]
                .revents()
                .intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL)
            {
                return Err(io::Error::other("service listener failed"));
            }
            if !fds[2].revents().is_empty() || !fds[3].revents().is_empty() {
                if fds[2..].iter().any(|fd| {
                    fd.revents()
                        .intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL)
                }) {
                    return Err(io::Error::other("background observer signal failed"));
                }
                observer.expire()?;
                observer.drain_changes()?;
                if !presentation_failed
                    && let Some(lease) = observer.lease()?
                    && owner.observe_presentation(lease.snapshot()).is_err()
                {
                    // A presentation-storage fault must not shut down command
                    // service. Report once; the coordinator latches the failure.
                    presentation_failed = true;
                    eprintln!("phone presentation journal unavailable");
                }
                // Display eligibility remains disabled until lock/suspend
                // authority and the notification owner loop are connected.
            }
            let mut index = 0;
            while index < connections.len() {
                if connections[index].is_finished() {
                    connections
                        .swap_remove(index)
                        .join()
                        .map_err(|_| io::Error::other("local worker failed"))?;
                } else {
                    index += 1;
                }
            }
            match socket.listener().accept() {
                Ok((mut stream, _)) => {
                    // Close excess connections before reading or admitting any
                    // request. Never queue an action behind a long-running call.
                    if connections.len() >= 16 {
                        continue;
                    }
                    stream.set_nonblocking(false)?;
                    let owner = &owner;
                    connections.push(
                        thread::Builder::new()
                            .name("phone-local".into())
                            .spawn_scoped(scope, move || {
                                let _ = rpc::serve(owner, &mut stream);
                            })?,
                    );
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(error) => return Err(error),
            }
        }
        for connection in connections {
            connection
                .join()
                .map_err(|_| io::Error::other("local worker failed"))?;
        }
        Ok(())
    });
    signal_hook::low_level::unregister(term);
    signal_hook::low_level::unregister(interrupt);
    let stopped = observer.shutdown();
    // Stop publishing the socket, then finish service-owned command work.
    drop(socket);
    drop(owner);
    result?;
    stopped?;
    Ok(())
}
#[cfg(target_os = "linux")]
fn main() {
    if let Err(error) = run() {
        eprintln!("phone service failed: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("omarchy-phoned requires Linux");
    std::process::exit(1);
}
