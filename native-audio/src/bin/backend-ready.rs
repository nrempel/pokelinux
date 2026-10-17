//! Connect-only startup readiness for the selected backend Main; not audio health.
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use rustix::io::Errno;
use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType};
use std::io;
use std::os::fd::AsFd;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

fn before(until: BootTime) -> io::Result<()> {
    if BootClock.now() >= until {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "socket deadline expired",
        ));
    }
    Ok(())
}

fn pause(until: BootTime) -> io::Result<()> {
    before(until)?;
    let remaining = until.elapsed().saturating_sub(BootClock.now().elapsed());
    sleep(remaining.min(Duration::from_millis(10)));
    Ok(())
}

fn retryable(error: Errno) -> bool {
    matches!(error, Errno::NOENT | Errno::CONNREFUSED | Errno::AGAIN)
}

fn finish_connect(socket: &impl AsFd, until: BootTime) -> io::Result<bool> {
    loop {
        before(until)?;
        match net::sockopt::socket_error(socket)? {
            Err(error) if retryable(error) => return Ok(false),
            Err(Errno::INPROGRESS | Errno::ALREADY) => {}
            Err(error) => return Err(error.into()),
            Ok(()) => match net::getpeername(socket) {
                Ok(Some(_)) => return Ok(true),
                Ok(None) | Err(Errno::NOTCONN) => {}
                Err(error) => return Err(error.into()),
            },
        }
        pause(until)?;
    }
}

fn run() -> io::Result<()> {
    let until = BootClock
        .now()
        .checked_after(Duration::from_secs(5))
        .ok_or_else(|| io::Error::other("socket deadline overflow"))?;
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: backend-ready SOCKET EXPECTED_MAIN_PID EXPECTED_UID",
        ));
    }
    let parse = |index: usize| -> io::Result<u32> {
        args[index]
            .to_str()
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid numeric peer identity")
            })
    };
    let pid = parse(2)?;
    let uid = parse(3)?;
    if pid <= 1 || pid > i32::MAX as u32 || !Path::new(&args[1]).is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute socket and valid Main PID required",
        ));
    }
    let address = SocketAddrUnix::new(Path::new(&args[1]))?;
    loop {
        before(until)?;
        let socket = net::socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )?;
        let connected = match net::connect(&socket, &address) {
            Ok(()) => true,
            Err(Errno::INPROGRESS | Errno::ALREADY) => finish_connect(&socket, until)?,
            Err(error) if retryable(error) => false,
            Err(error) => return Err(error.into()),
        };
        if connected {
            before(until)?;
            let peer = net::sockopt::socket_peercred(&socket)?;
            before(until)?;
            if peer.pid.as_raw_pid() as u32 != pid
                || peer.uid.as_raw() != uid
                || peer.gid.as_raw() != uid
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "peer mismatch: expected pid={pid} uid={uid}; actual pid={} uid={} gid={}",
                        peer.pid.as_raw_pid(),
                        peer.uid.as_raw(),
                        peer.gid.as_raw()
                    ),
                ));
            }
            return Ok(());
        }
        drop(socket);
        pause(until)?;
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("backend-ready: {error}");
        std::process::exit(1);
    }
}
