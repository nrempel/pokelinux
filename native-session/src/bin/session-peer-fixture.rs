//! Separately built synthetic sender; production library contains no fixture entrypoint.
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_session::{Frame, NativeFocus};
use rustix::fd::{FromRawFd, OwnedFd};
use std::{io, process::Command};
fn wire(source: &str, mode: &str, sequence: u64) -> Vec<u8> {
    let mut f = Frame {
        source: source.into(),
        sequence,
        sampled_at_ns: boottime_ns(),
        locked: false,
        lock_completed: false,
        focus: Some(NativeFocus {
            pid: 42,
            uid: 11010,
            gid: 11010,
            pidfs_device: 6,
            pidfs_inode: 12345,
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
fn packet(fd: &OwnedFd, bytes: &[u8]) -> io::Result<()> {
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
    while boottime_ns() < until {
        match ipc::receive(&command)? {
            Some((bytes, _)) => {
                let mode = std::str::from_utf8(&bytes).unwrap();
                if mode == "quit" {
                    return Ok(());
                }
                if mode == "delegated" {
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
