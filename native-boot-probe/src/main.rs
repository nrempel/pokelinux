//! Restricted native PID1 probe, not a phone init or service manager.
//! Runs solely from the authored RAM image; mounts no persistent storage.
use rustix::{
    fs::{CWD, FileType, Mode, makedev, mknodat},
    mount::{MountFlags, mount},
    process::{getpid, getuid},
    system::{RebootCommand, reboot, uname},
    time::{ClockId, clock_gettime},
};
use std::{
    io::{self, Write},
    process::{Child, Command},
    time::Duration,
};

fn now() -> u64 {
    let value = clock_gettime(ClockId::Boottime);
    (value.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(value.tv_nsec as u64)
}
fn mounted() -> io::Result<()> {
    // Google's selected kernel has no DEVTMPFS. Create our required nodes
    // explicitly; their numbers are standard Linux, independent of a phone.
    mount(
        "tmpfs",
        "/dev",
        "tmpfs",
        MountFlags::NOSUID,
        c"size=1m,mode=0755",
    )?;
    mknodat(
        CWD,
        "/dev/console",
        FileType::CharacterDevice,
        Mode::RUSR | Mode::WUSR,
        makedev(5, 1),
    )?;
    mknodat(
        CWD,
        "/dev/null",
        FileType::CharacterDevice,
        Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::WGRP | Mode::ROTH | Mode::WOTH,
        makedev(1, 3),
    )?;
    let restricted =
        MountFlags::NOSUID | MountFlags::NODEV | MountFlags::NOEXEC | MountFlags::RDONLY;
    mount("proc", "/proc", "proc", restricted, None)?;
    mount("sysfs", "/sys", "sysfs", restricted, None)?;
    Ok(())
}
fn child() -> io::Result<()> {
    let mut child: Child = Command::new("/probe-child").env_clear().spawn()?;
    let pid = child.id();
    let until = now().saturating_add(2_000_000_000);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!(
                    "POKELINUX_CHILD_WAIT pid={pid} actual_wait=true exit={:?}",
                    status.code()
                );
                return if status.code() == Some(37) {
                    Ok(())
                } else {
                    Err(io::ErrorKind::InvalidData.into())
                };
            }
            Ok(None) if now() < until => std::thread::sleep(Duration::from_millis(2)),
            _ => break,
        }
    }
    // Child ownership is retained until actual wait, including an error or
    // deadline. No detached worker or successful-cleanup claim follows a kill.
    let _ = child.kill();
    let cleanup_until = now().saturating_add(2_000_000_000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Err(io::ErrorKind::TimedOut.into()),
            Ok(None) if now() < cleanup_until => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                // Abort keeps this failure visible as a kernel PID1 failure.
                std::process::abort();
            }
        }
    }
}
fn poweroff() -> ! {
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    if let Err(error) = reboot(RebootCommand::PowerOff) {
        eprintln!(
            "POKELINUX_PROBE_FAILURE phase=poweroff errno={:?}",
            error.raw_os_error()
        );
    }
    // An unexpected return from shutdown is failure, never a successful exit
    // from init. No process may continue into a different execution role.
    std::process::abort();
}
fn main() {
    if getpid().as_raw_nonzero().get() != 1 || getuid().as_raw() != 0 {
        eprintln!("POKELINUX_PROBE_REFUSED requires=isolated-root-pid1");
        std::process::exit(2);
    }
    println!(
        "POKELINUX_NATIVE_PID1 pid=1 uid=0 version={}",
        env!("CARGO_PKG_VERSION")
    );
    if let Err(error) = mounted() {
        eprintln!(
            "POKELINUX_PROBE_FAILURE phase=mount errno={:?}",
            error.raw_os_error()
        );
        poweroff();
    }
    println!(
        "POKELINUX_PROBE_KERNEL release={}",
        uname().release().to_string_lossy()
    );
    if let Err(error) = child() {
        eprintln!(
            "POKELINUX_PROBE_FAILURE phase=child-spawn errno={:?}",
            error.raw_os_error()
        );
        poweroff();
    }
    println!("POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false");
    poweroff();
}
