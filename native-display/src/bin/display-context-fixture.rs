//! Isolated root setup runner. No installed service or application FD selector.
use pokelinux_display::{
    Context, MAX_INITIALIZATION_NS, Metadata, Phase, SourceIdentity, boottime_ns,
};
use rustix::{
    fd::{FromRawFd, OwnedFd},
    net,
    process::{Gid, Pid, Uid},
};
use std::{
    io,
    os::fd::AsFd,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::Path,
};

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 8
        || rustix::process::getuid().as_raw() != 0
        || std::env::var("NATIVE_DISPLAY_ISOLATED").as_deref() != Ok("1")
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let integer = |index: usize| -> io::Result<u32> {
        args[index]
            .to_str()
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| io::ErrorKind::InvalidInput.into())
    };
    let gid = integer(4)?;
    if gid < 10000 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let pid = i32::try_from(integer(5)?)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = integer(6)?;
    let raw = i32::try_from(integer(7)?)
        .ok()
        .filter(|v| *v >= 3)
        .ok_or(io::ErrorKind::InvalidInput)?;
    // SAFETY: only this separately built root fixture is called by the test
    // launcher, which exclusively transfers the listed retained source pidfd.
    let expected = unsafe { OwnedFd::from_raw_fd(raw) };
    rustix::io::fcntl_setfd(&expected, rustix::io::FdFlags::CLOEXEC)?;
    let path = Path::new(&args[1]);
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let directory = std::fs::symlink_metadata(parent)?;
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
    let mut context = Context::new(
        UnixStream::from(socket),
        &expected,
        SourceIdentity { pid, uid, gid: uid },
        listener.into(),
        metadata,
        until,
    )?;
    let input = io::stdin();
    let flags = rustix::fs::fcntl_getfl(input.as_fd())?;
    rustix::fs::fcntl_setfl(input.as_fd(), flags | rustix::fs::OFlags::NONBLOCK)?;
    let mut notified = false;
    let mut pending = Vec::new();
    loop {
        match context.poll() {
            Ok(Phase::Ready) if !notified => {
                println!(
                    "{{\"event\":\"ready\",\"state\":\"ready\",\"protocol_creator_disconnected\":true}}"
                );
                notified = true;
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("context failed: {error}");
                return Err(error);
            }
        }
        let mut bytes = [0; 64];
        match rustix::io::read(input.as_fd(), &mut bytes) {
            Ok(0) => {
                context.stop();
                return Ok(());
            }
            Ok(n) => {
                pending.extend_from_slice(&bytes[..n]);
                if pending.len() > 64 {
                    context.stop();
                    return Err(io::ErrorKind::InvalidData.into());
                }
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    let command: Vec<_> = pending.drain(..=end).collect();
                    match command.as_slice() {
                        b"status\n" => println!(
                            "{{\"event\":\"status\",\"state\":\"{}\"}}",
                            match context.snapshot() {
                                Phase::Ready => "ready",
                                Phase::Initializing => "initializing",
                                Phase::Failed => "failed",
                                Phase::Stopped => "stopped",
                            }
                        ),
                        b"stop\n" => {
                            context.stop();
                            println!("{{\"event\":\"stopped\",\"state\":\"stopped\"}}");
                            return Ok(());
                        }
                        _ => {
                            context.stop();
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                    }
                }
            }
            Err(rustix::io::Errno::AGAIN) => {}
            Err(error) => {
                context.stop();
                return Err(error.into());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
