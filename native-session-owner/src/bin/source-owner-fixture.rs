//! Authored host Source sender only, never installed as the compositor.
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_session::{Frame, Locker};
use rustix::fd::{AsFd, FromRawFd, OwnedFd};
use std::{io, os::unix::process::CommandExt, process::Command, time::Duration};
fn main() -> io::Result<()> {
    let mode = std::env::args().nth(1).ok_or(io::ErrorKind::InvalidInput)?;
    if mode == "--compositor-bootstrap" {
        return pokelinux_session_owner::bootstrap_entry();
    }
    let fd = if let Ok(raw) = std::env::var("POKELINUX_SESSION_FD") {
        // This isolated fixture adopts only the endpoint inherited by its owned parent.
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
    if mode == "quiet" {
        loop {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let source = std::env::var("POKELINUX_SESSION_SOURCE")
        .unwrap_or_else(|_| "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    let diagnostic_root = std::env::args().nth(2).map(std::path::PathBuf::from);
    for sequence in 1..=10000 {
        if mode == "diagnostic" {
            let root = diagnostic_root
                .as_ref()
                .ok_or(io::ErrorKind::InvalidInput)?;
            if let Ok(command) = std::fs::read_to_string(root.join("reject")) {
                if command == "eof" {
                    drop(fd);
                } else {
                    let mut frame = Frame {
                        source: source.clone(),
                        sequence,
                        sampled_at_ns: boottime_ns(),
                        locked: false,
                        lock_completed: false,
                        focus: None,
                        locker: Locker::Unavailable,
                        lock_generation: None,
                        protocol_unlock: None,
                    };
                    match command.as_str() {
                        "queued_expired" => {
                            ipc::send(&fd, &serde_json::to_vec(&frame)?)?;
                            std::thread::sleep(Duration::from_millis(130));
                            frame.sequence += 1;
                            frame.sampled_at_ns = boottime_ns();
                            ipc::send(&fd, &serde_json::to_vec(&frame)?)?;
                        }
                        "sample_gap" => {
                            std::thread::sleep(Duration::from_millis(130));
                            frame.sampled_at_ns = boottime_ns();
                            ipc::send(&fd, &serde_json::to_vec(&frame)?)?;
                        }
                        "sequence" => {
                            frame.sequence += 1;
                            ipc::send(&fd, &serde_json::to_vec(&frame)?)?;
                        }
                        "zero_bytes" => {
                            assert_eq!(
                                rustix::net::send(
                                    &fd,
                                    b"",
                                    rustix::net::SendFlags::DONTWAIT
                                        | rustix::net::SendFlags::NOSIGNAL
                                )?,
                                0
                            );
                        }
                        "truncated" => {
                            ipc::send(&fd, &vec![b'x'; pokelinux_session::MAX_FRAME + 1])?
                        }
                        "malformed" => ipc::send(&fd, b"private-frame-marker-not-json")?,
                        "drain_limit" => {
                            for offset in 0..17 {
                                frame.sequence = sequence + offset;
                                frame.sampled_at_ns = boottime_ns();
                                ipc::send(&fd, &serde_json::to_vec(&frame)?)?;
                            }
                        }
                        _ => return Err(io::ErrorKind::InvalidInput.into()),
                    }
                }
                std::fs::write(root.join("rejected-input-ready"), b"ready")?;
                loop {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        let frame = Frame {
            source: source.clone(),
            sequence,
            sampled_at_ns: boottime_ns(),
            locked: false,
            lock_completed: false,
            focus: None,
            locker: Locker::Unavailable,
            lock_generation: None,
            protocol_unlock: None,
        };
        if let Err(e) = ipc::send(&fd, &serde_json::to_vec(&frame)?) {
            if matches!(e.raw_os_error(), Some(32 | 104)) {
                break;
            }
            return Err(e);
        }
        if mode == "unload" && sequence == 3 {
            rustix::io::fcntl_setfd(&fd, rustix::io::FdFlags::CLOEXEC)?;
            return Err(Command::new("/usr/bin/sleep").arg("30").exec());
        }
        if mode == "diagnostic" {
            let root = diagnostic_root
                .as_ref()
                .ok_or(io::ErrorKind::InvalidInput)?;
            while !root.join("reject").exists() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
