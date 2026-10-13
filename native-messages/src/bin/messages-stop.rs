//! Authenticated stop ACK precedes bounded observation of this exact service exit.
use pokelinux_lifecycle::{Reply, boottime_ns, ipc};
use rustix::{fd::AsFd, net::sockopt::socket_peercred};
use std::io;
fn wait(fd: impl AsFd, events: rustix::event::PollFlags, until: u64) -> io::Result<bool> {
    let now = boottime_ns();
    let remaining = until
        .checked_sub(now)
        .filter(|n| *n > 0)
        .ok_or(io::ErrorKind::TimedOut)?;
    let timeout = rustix::event::Timespec {
        tv_sec: 0,
        tv_nsec: remaining.min(50_000_000) as i64,
    };
    let mut fds = [rustix::event::PollFd::new(&fd, events)];
    match rustix::event::poll(&mut fds, Some(&timeout)) {
        Ok(_) if fds[0].revents().contains(rustix::event::PollFlags::NVAL) => {
            Err(io::ErrorKind::InvalidData.into())
        }
        Ok(_) => Ok(fds[0].revents().intersects(events)),
        Err(rustix::io::Errno::INTR) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn main() -> io::Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    // This original budget covers connect, Stop admission, ACK and actual exit.
    let until = boottime_ns()
        .checked_add(3_000_000_000)
        .ok_or(io::ErrorKind::InvalidData)?;
    let fd = ipc::connect(std::path::Path::new(&args[0]))?;
    let peer = socket_peercred(&fd)?;
    if peer.uid.as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // Pin the socket's retained process reference, never a racy numeric PID.
    // Unsupported platform/kernel is an explicit failure; no legacy fallback.
    let alive = pokelinux_kernel_identity::socket_peer_pidfd(&fd)?;
    loop {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match ipc::send(&fd, b"{\"op\":\"stop\"}") {
            Ok(()) => break,
            Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                wait(&fd, rustix::event::PollFlags::OUT, until)?;
            }
            Err(e) => return Err(e),
        }
    }
    loop {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match ipc::receive(&fd) {
            Ok(Some((bytes, cred))) => {
                if cred.uid.as_raw() != 0 || cred.pid != peer.pid || boottime_ns() >= until {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                if serde_json::from_slice::<Reply>(&bytes)? != Reply::Ok {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                break;
            }
            Ok(None) => {
                wait(&fd, rustix::event::PollFlags::IN, until)?;
            }
            Err(e) if e.raw_os_error() == Some(4) => {}
            Err(e) => return Err(e),
        }
    }
    while !wait(&alive, rustix::event::PollFlags::IN, until)? {}
    if boottime_ns() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    // Only local process exit is established. systemd Main status and private
    // policy validation separately determine whether restart is healthy.
    Ok(())
}
