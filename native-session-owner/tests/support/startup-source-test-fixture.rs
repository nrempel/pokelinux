//! Authored UID1000 Source packets; never a compositor or production sender.
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_session::{Frame, Locker};
use rustix::fd::{AsFd, FromRawFd, OwnedFd};
use std::{io, path::PathBuf, time::Duration};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [mode, root] = args.as_slice() else {
        return Err(io::ErrorKind::InvalidInput.into());
    };
    if !matches!(
        mode.as_str(),
        "gated_locked" | "gated_malformed" | "gated_stale"
    ) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fd = if let Ok(raw) = std::env::var("POKELINUX_SESSION_FD") {
        // Adopt only the actual SourceOwner-owned inherited Source endpoint.
        unsafe { OwnedFd::from_raw_fd(raw.parse().map_err(|_| io::ErrorKind::InvalidInput)?) }
    } else {
        rustix::io::fcntl_setfd(io::stdin().as_fd(), rustix::io::FdFlags::CLOEXEC)?;
        rustix::io::fcntl_dupfd_cloexec(io::stdin().as_fd(), 3)?
    };
    if rustix::process::getuid().as_raw() == 0 {
        rustix::thread::set_thread_groups(&[])?;
        let gid = rustix::process::Gid::from_raw(1000);
        rustix::thread::set_thread_res_gid(gid, gid, gid)?;
        let uid = rustix::process::Uid::from_raw(1000);
        rustix::thread::set_thread_res_uid(uid, uid, uid)?;
    }
    assert_eq!(rustix::process::getuid().as_raw(), 1000);
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    let root = PathBuf::from(root);
    while !root.join("allow-source").exists() {
        std::thread::sleep(Duration::from_millis(2));
    }
    if mode == "gated_malformed" {
        ipc::send(&fd, b"authored-startup-invalid-json")?;
    } else {
        let source = std::env::var("POKELINUX_SESSION_SOURCE")
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        for sequence in 1..=10000 {
            let frame = Frame {
                source: source.clone(),
                sequence,
                sampled_at_ns: if mode == "gated_stale" {
                    boottime_ns()
                        .checked_sub(200_000_000)
                        .ok_or(io::ErrorKind::InvalidInput)?
                } else {
                    boottime_ns()
                },
                locked: true,
                lock_completed: false,
                focus: None,
                locker: Locker::Unavailable,
                lock_generation: None,
                protocol_unlock: None,
            };
            if let Err(error) = ipc::send(&fd, &serde_json::to_vec(&frame)?) {
                if matches!(error.raw_os_error(), Some(32 | 104)) {
                    return Ok(());
                }
                return Err(error);
            }
            if mode == "gated_stale" {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    // A rejected sender stays alive until the real SourceOwner stops/reaps it.
    loop {
        std::thread::sleep(Duration::from_millis(10));
    }
}
