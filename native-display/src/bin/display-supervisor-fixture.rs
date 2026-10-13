//! Isolated supervisor entry. No installed actor, grants or production launch.
use pokelinux_display::{
    MAX_INITIALIZATION_NS, Metadata, SourceIdentity, boottime_ns,
    supervisor::{State, Supervisor, WORKER_FD_LIMIT, WORKER_MEMORY_BYTES, worker_entry},
};
use rustix::{
    fd::{AsFd, FromRawFd, OwnedFd},
    net,
    process::{Gid, Pid, Uid},
};
use std::{
    io,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::Path,
};

fn state_name(state: State) -> &'static str {
    match state {
        State::Initializing => "initializing",
        State::Ready => "ready",
        State::Failed => "failed",
        State::Stopping => "stopping",
        State::Stopped => "stopped",
    }
}
fn report(event: &str, owner: &Supervisor) {
    println!(
        "{{\"event\":\"{event}\",\"state\":\"{}\",\"worker_pid\":{},\"occupied\":{},\"admission\":{},\"until_ns\":{},\"memory_bytes\":{},\"fd_limit\":{}}}",
        state_name(owner.snapshot()),
        owner.worker_pid(),
        owner.occupied(),
        owner.admission(),
        owner.until_ns(),
        WORKER_MEMORY_BYTES,
        WORKER_FD_LIMIT
    );
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--display-context-worker" {
        return worker_entry();
    }
    if args.len() != 8
        || rustix::process::getuid().as_raw() != 0
        || std::env::var("NATIVE_DISPLAY_ISOLATED").as_deref() != Ok("1")
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let integer = |index: usize| -> io::Result<u32> {
        args[index]
            .to_str()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| io::ErrorKind::InvalidInput.into())
    };
    let gid = integer(4)?;
    let pid = i32::try_from(integer(5)?)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = integer(6)?;
    let raw = i32::try_from(integer(7)?)
        .ok()
        .filter(|v| *v >= 3)
        .ok_or(io::ErrorKind::InvalidInput)?;
    if gid < 10000 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    // SAFETY: only this separately built root fixture accepts an FD argument;
    // its test launcher exclusively transfers one retained expected source watch.
    let expected = unsafe { OwnedFd::from_raw_fd(raw) };
    rustix::io::fcntl_setfd(&expected, rustix::io::FdFlags::CLOEXEC)?;
    let path = Path::new(&args[1]);
    let directory = std::fs::symlink_metadata(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    if !directory.is_dir() || directory.uid() != 0 || directory.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let listener = UnixListener::bind(path)?;
    rustix::fs::chown(path, Some(Uid::ROOT), Some(Gid::from_raw(gid)))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
    let until = boottime_ns()
        .checked_add(MAX_INITIALIZATION_NS)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let socket = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::STREAM,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )?;
    net::connect(&socket, &net::SocketAddrUnix::new(Path::new(&args[0]))?)?;
    let metadata = Metadata {
        app: args[2].to_str().ok_or(io::ErrorKind::InvalidInput)?.into(),
        instance: args[3].to_str().ok_or(io::ErrorKind::InvalidInput)?.into(),
    };
    let mut owner = Supervisor::spawn(
        &std::env::current_exe()?,
        UnixStream::from(socket),
        expected,
        SourceIdentity { pid, uid, gid: uid },
        listener.into(),
        metadata,
        until,
    )?;
    report("spawned", &owner);
    let input = io::stdin();
    let flags = rustix::fs::fcntl_getfl(input.as_fd())?;
    rustix::fs::fcntl_setfl(input.as_fd(), flags | rustix::fs::OFlags::NONBLOCK)?;
    let mut pending = Vec::new();
    let mut previous = State::Initializing;
    let mut stopping = false;
    loop {
        let _ = owner.poll();
        let state = owner.snapshot();
        if state != previous {
            if state == State::Ready {
                report("ready", &owner);
            }
            if state == State::Failed {
                report("failed", &owner);
            }
            previous = state;
        }
        if stopping && !owner.occupied() {
            report("stopped", &owner);
            return Ok(());
        }
        let mut bytes = [0; 64];
        match rustix::io::read(input.as_fd(), &mut bytes) {
            Ok(0) => {
                owner.stop();
                stopping = true;
            }
            Ok(n) => {
                pending.extend_from_slice(&bytes[..n]);
                if pending.len() > 64 {
                    owner.stop();
                    stopping = true;
                    pending.clear();
                }
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    let command: Vec<_> = pending.drain(..=end).collect();
                    match command.as_slice() {
                        b"status\n" => report("status", &owner),
                        b"stop\n" => {
                            owner.stop();
                            stopping = true;
                            report("stop_requested", &owner);
                        }
                        _ => {
                            owner.stop();
                            stopping = true;
                        }
                    }
                }
            }
            Err(rustix::io::Errno::AGAIN) => {}
            Err(_) => {
                owner.stop();
                stopping = true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
