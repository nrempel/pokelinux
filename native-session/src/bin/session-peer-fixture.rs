//! Separately built synthetic sender; production library contains no fixture entrypoint.
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_session::{Frame, Locker, NativeFocus};
use rustix::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};
use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage, SendFlags, sendmsg};
use std::{io, process::Command};
use std::{io::IoSlice, mem::MaybeUninit};
fn wire(source: &str, mode: &str, sequence: u64) -> Vec<u8> {
    let mut f = Frame {
        source: source.into(),
        sequence,
        sampled_at_ns: boottime_ns(),
        locked: false,
        lock_completed: false,
        locker: Locker::Unavailable,
        focus: Some(NativeFocus {
            pid: std::process::id(),
            uid: 11010,
            gid: 11010,
        }),
    };
    if mode == "wrong-source" {
        f.source = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    }
    if mode == "stale" {
        f.sampled_at_ns = f.sampled_at_ns.saturating_sub(200_000_000);
    }
    if mode == "gap" {
        f.sequence = 2;
    }
    if mode == "locked-focus" {
        f.locked = true;
    }
    let mut bytes = serde_json::to_vec(&f).unwrap();
    if mode == "oversize" {
        bytes.extend(std::iter::repeat_n(b' ', 1200));
    }
    bytes
}
fn adopt(raw: &str) -> OwnedFd {
    // Only own isolated test process adopts descriptors supplied by its spawning parent.
    unsafe { OwnedFd::from_raw_fd(raw.parse().unwrap()) }
}
fn plain_packet(fd: &OwnedFd, bytes: &[u8]) -> io::Result<()> {
    match ipc::send(fd, bytes) {
        Err(e) if matches!(e.raw_os_error(), Some(32 | 104)) => {
            eprintln!(
                "ACTUAL_LATE_SEND_REFUSED: errno={}",
                e.raw_os_error().unwrap()
            );
            Ok(())
        }
        result => result,
    }
}
fn packet(fd: &OwnedFd, bytes: &[u8]) -> io::Result<()> {
    let focused = serde_json::from_slice::<Frame>(bytes)
        .map(|f| f.focus.is_some())
        .unwrap_or(true);
    if focused {
        let owned = rustix::process::pidfd_open(
            rustix::process::getpid(),
            rustix::process::PidfdFlags::empty(),
        )?;
        packet_with_rights(fd, bytes, &[owned.as_fd()])
    } else {
        plain_packet(fd, bytes)
    }
}
fn packet_with_rights(fd: &OwnedFd, bytes: &[u8], rights: &[BorrowedFd<'_>]) -> io::Result<()> {
    let mut storage = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(2))];
    let mut ancillary = SendAncillaryBuffer::new(&mut storage);
    assert!(ancillary.push(SendAncillaryMessage::ScmRights(rights)));
    match sendmsg(
        fd,
        &[IoSlice::new(bytes)],
        &mut ancillary,
        SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
    ) {
        Ok(n) if n == bytes.len() => Ok(()),
        Ok(_) => Err(io::ErrorKind::WriteZero.into()),
        Err(e) if matches!(e.raw_os_error(), 32 | 104) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn owned_packet(fd: &OwnedFd, source: &str, mode: &str, sequence: u64) -> io::Result<()> {
    let mut frame: Frame = serde_json::from_slice(&wire(source, "valid", sequence)).unwrap();
    let owned = rustix::process::pidfd_open(
        rustix::process::getpid(),
        rustix::process::PidfdFlags::empty(),
    )?;
    match mode {
        "owned-focus" | "owned-focus-heartbeat" => {
            packet_with_rights(fd, &serde_json::to_vec(&frame).unwrap(), &[owned.as_fd()])
        }
        "owned-focus-missing" => plain_packet(fd, &serde_json::to_vec(&frame).unwrap()),
        "owned-focus-unexpected" => {
            frame.focus = None;
            packet_with_rights(fd, &serde_json::to_vec(&frame).unwrap(), &[owned.as_fd()])
        }
        "owned-focus-two" => packet_with_rights(
            fd,
            &serde_json::to_vec(&frame).unwrap(),
            &[owned.as_fd(), owned.as_fd()],
        ),
        "owned-focus-old-wire" => {
            let mut value = serde_json::to_value(&frame).unwrap();
            value["focus"]["pidfs_device"] = 6.into();
            value["focus"]["pidfs_inode"] = 12345.into();
            packet_with_rights(fd, &serde_json::to_vec(&value).unwrap(), &[owned.as_fd()])
        }
        "owned-focus-truncated-rights" => {
            let mut storage = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(64))];
            let rights = vec![owned.as_fd(); 64];
            let mut ancillary = SendAncillaryBuffer::new(&mut storage);
            assert!(ancillary.push(SendAncillaryMessage::ScmRights(&rights)));
            let bytes = serde_json::to_vec(&frame).unwrap();
            assert_eq!(
                sendmsg(
                    fd,
                    &[IoSlice::new(&bytes)],
                    &mut ancillary,
                    SendFlags::DONTWAIT | SendFlags::NOSIGNAL
                )?,
                bytes.len()
            );
            Ok(())
        }
        "owned-focus-file" => {
            let file = std::fs::File::open("/dev/null")?;
            packet_with_rights(fd, &serde_json::to_vec(&frame).unwrap(), &[file.as_fd()])
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
fn locker_packet(fd: &OwnedFd, source: &str, mode: &str, sequence: u64) -> io::Result<()> {
    let mut frame: serde_json::Value =
        serde_json::from_slice(&wire(source, "valid", sequence)).unwrap();
    frame["locked"] = true.into();
    frame["lock_completed"] = true.into();
    frame["focus"] = serde_json::Value::Null;
    frame["locker"] = serde_json::json!({"state":"mapped","pid":std::process::id(),
        "uid":rustix::process::getuid().as_raw(),"gid":rustix::process::getgid().as_raw()});
    let owned = rustix::process::pidfd_open(
        rustix::process::getpid(),
        rustix::process::PidfdFlags::empty(),
    )?;
    match mode {
        "owned-locker" | "owned-locker-heartbeat" => {}
        "owned-locker-missing" => return plain_packet(fd, &serde_json::to_vec(&frame).unwrap()),
        "owned-locker-two" => {
            return packet_with_rights(
                fd,
                &serde_json::to_vec(&frame).unwrap(),
                &[owned.as_fd(), owned.as_fd()],
            );
        }
        "owned-locker-file" => {
            let file = std::fs::File::open("/dev/null")?;
            return packet_with_rights(fd, &serde_json::to_vec(&frame).unwrap(), &[file.as_fd()]);
        }
        "owned-locker-wrong-pid" => {
            frame["locker"]["pid"] = std::process::id().saturating_add(1).into()
        }
        "owned-locker-zero-pid" => frame["locker"]["pid"] = 0.into(),
        "owned-locker-unlocked" => {
            frame["locked"] = false.into();
            frame["lock_completed"] = false.into();
        }
        "owned-locker-and-focus" => {
            frame["focus"] = serde_json::json!({"pid":std::process::id(),"uid":11010,"gid":11010})
        }
        "owned-locker-unknown-field" => frame["locker"]["trusted"] = true.into(),
        "owned-locker-null" => frame["locker"] = serde_json::Value::Null,
        "owned-locker-unavailable" => {
            frame["locker"] = serde_json::json!({"state":"unavailable"});
            return plain_packet(fd, &serde_json::to_vec(&frame).unwrap());
        }
        "owned-locker-unavailable-rights" => {
            frame["locker"] = serde_json::json!({"state":"unavailable"})
        }
        "owned-locker-legacy-wire" => {
            frame.as_object_mut().unwrap().remove("locker");
        }
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    }
    packet_with_rights(fd, &serde_json::to_vec(&frame).unwrap(), &[owned.as_fd()])
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--expect-source-fd-closed" {
        let fd: i32 = args[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        if let Ok(info) = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}"))
            && info.contains(&format!("Pid:\t{}", args[3]))
        {
            eprintln!(
                "ACTUAL_EXEC_INHERITED_SOURCE_PIDFD: fd={fd} pid={}",
                args[3]
            );
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        return Ok(());
    }
    if args.len() == 5 && args[1] == "--delegated" {
        let fd = adopt(&args[2]);
        packet(&fd, &wire(&args[3], &args[4], 1))?;
        return Ok(());
    }
    if args.len() != 4 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fd = adopt(&args[1]);
    let command = adopt(&args[2]);
    let source = &args[3];
    let until = boottime_ns() + 10_000_000_000;
    let mut own_sequence = 0;
    let mut focus_child = None::<(std::process::Child, OwnedFd, u32)>;
    while boottime_ns() < until {
        match ipc::receive(&command)? {
            Some((bytes, _)) => {
                let mode = std::str::from_utf8(&bytes).unwrap();
                if mode == "quit" {
                    if let Some((mut child, _, _)) = focus_child.take() {
                        child.kill()?;
                        child.wait()?;
                    }
                    return Ok(());
                }
                if mode.starts_with("focus-child-") || mode.starts_with("locker-child-") {
                    let locker = mode.starts_with("locker-child-");
                    let action = mode
                        .strip_prefix("locker-child-")
                        .or_else(|| mode.strip_prefix("focus-child-"))
                        .unwrap();
                    own_sequence += 1;
                    if action == "live" {
                        let child = Command::new("/bin/sleep").arg("30").spawn()?;
                        let pid = child.id();
                        let peer = rustix::process::pidfd_open(
                            rustix::process::Pid::from_raw(pid as i32).unwrap(),
                            rustix::process::PidfdFlags::empty(),
                        )?;
                        focus_child = Some((child, peer, pid));
                    } else if action == "dead" {
                        let (child, _, _) = focus_child.as_mut().unwrap();
                        child.kill()?;
                        child.wait()?;
                    } else if action == "new-same-metadata" {
                        let (_, _, old_pid) = focus_child.take().unwrap();
                        let child = Command::new("/bin/sleep").arg("30").spawn()?;
                        let peer = rustix::process::pidfd_open(
                            rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
                            rustix::process::PidfdFlags::empty(),
                        )?;
                        assert_ne!(
                            child.id(),
                            old_pid,
                            "fixture does not force numeric PID reuse"
                        );
                        focus_child = Some((child, peer, old_pid));
                    } else {
                        return Err(io::ErrorKind::InvalidInput.into());
                    }
                    let (_, peer, metadata_pid) = focus_child.as_ref().unwrap();
                    let mut frame: Frame =
                        serde_json::from_slice(&wire(source, "valid", own_sequence)).unwrap();
                    if locker {
                        frame.focus = None;
                        frame.locked = true;
                        frame.lock_completed = true;
                        frame.locker = Locker::Mapped {
                            pid: *metadata_pid,
                            uid: rustix::process::getuid().as_raw(),
                            gid: rustix::process::getgid().as_raw(),
                        };
                    } else {
                        frame.focus.as_mut().unwrap().pid = *metadata_pid;
                    }
                    packet_with_rights(&fd, &serde_json::to_vec(&frame).unwrap(), &[peer.as_fd()])?;
                } else if mode.starts_with("owned-locker") {
                    own_sequence += 1;
                    locker_packet(&fd, source, mode, own_sequence)?;
                } else if mode.starts_with("owned-focus") {
                    own_sequence += 1;
                    owned_packet(&fd, source, mode, own_sequence)?;
                } else if mode == "delegated" {
                    let mut c = Command::new(std::env::current_exe()?)
                        .args(["--delegated", &args[1], source, "valid"])
                        .spawn()?;
                    let deadline = boottime_ns() + 2_000_000_000;
                    loop {
                        match c.try_wait()? {
                            Some(s) => {
                                if !s.success() {
                                    return Err(io::ErrorKind::Other.into());
                                }
                                break;
                            }
                            None if boottime_ns() < deadline => {
                                std::thread::sleep(std::time::Duration::from_millis(1))
                            }
                            _ => std::process::abort(),
                        }
                    }
                } else if let Some(sample) = mode.strip_prefix("timestamp-heartbeat:") {
                    let mut frame: Frame =
                        serde_json::from_slice(&wire(source, "valid", 2)).unwrap();
                    frame.sampled_at_ns =
                        sample.parse().map_err(|_| io::ErrorKind::InvalidInput)?;
                    packet(&fd, &serde_json::to_vec(&frame).unwrap())?;
                } else if let Some(sample) = mode.strip_prefix("backward-cycle:") {
                    let mut frame: Frame =
                        serde_json::from_slice(&wire(source, "valid", 2)).unwrap();
                    frame.sampled_at_ns =
                        sample.parse().map_err(|_| io::ErrorKind::InvalidInput)?;
                    frame.locked = true;
                    frame.focus = None;
                    packet(&fd, &serde_json::to_vec(&frame).unwrap())?;
                    packet(&fd, &wire(source, "valid", 3))?;
                } else if mode == "expired-then-fresh" {
                    packet(&fd, &wire(source, "stale", 1))?;
                    packet(&fd, &wire(source, "valid", 2))?;
                } else if mode == "heartbeat" {
                    packet(&fd, &wire(source, "valid", 2))?;
                } else if mode == "cycle" {
                    let mut locked: Frame =
                        serde_json::from_slice(&wire(source, "valid", 2)).unwrap();
                    locked.locked = true;
                    locked.focus = None;
                    packet(&fd, &serde_json::to_vec(&locked).unwrap())?;
                    packet(&fd, &wire(source, "valid", 3))?;
                } else if mode == "burst" {
                    for sequence in 1..=18 {
                        packet(&fd, &wire(source, "valid", sequence))?;
                    }
                } else {
                    packet(&fd, &wire(source, mode, 1))?;
                }
                ipc::send(&command, b"ready")?;
                if mode == "die" {
                    return Ok(());
                }
            }
            None => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
    Err(io::ErrorKind::TimedOut.into())
}
