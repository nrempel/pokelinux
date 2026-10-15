//! Restricted native PID1 probe, not a phone init or service manager.
//! Runs solely from the authored RAM image; mounts no persistent storage.
mod diagnostic;
mod pmsg;
use diagnostic::Reporter;
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
struct Failure {
    phase: &'static str,
    error: io::Error,
}
impl Failure {
    fn at(phase: &'static str, error: io::Error) -> Self {
        Self { phase, error }
    }
}
fn failed(report: Option<&Reporter>, failure: Failure) {
    let message = format!(
        "POKELINUX_PROBE_FAILURE phase={} kind={:?} errno={:?} reason={}",
        failure.phase,
        failure.error.kind(),
        failure.error.raw_os_error(),
        failure.error
    );
    eprintln!("{message}");
    if let Some(report) = report
        && let Err(error) = report.emit(format_args!("{message}"))
    {
        eprintln!(
            "POKELINUX_DIAGNOSTIC_FAILURE phase=failure-report kind={:?} errno={:?} reason={error}",
            error.kind(),
            error.raw_os_error()
        );
    }
}
fn child(report: &Reporter) -> Result<(), Failure> {
    let mut child: Child = Command::new("/probe-child")
        .env_clear()
        .spawn()
        .map_err(|error| Failure::at("child-spawn", error))?;
    let pid = child.id();
    let until = now().saturating_add(2_000_000_000);
    let failure = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                report
                    .emit(format_args!(
                        "POKELINUX_CHILD_WAIT pid={pid} actual_wait=true exit={:?}",
                        status.code()
                    ))
                    .map_err(|error| Failure::at("child-wait-report", error))?;
                return if status.code() == Some(37) {
                    Ok(())
                } else {
                    Err(Failure::at(
                        "child-status",
                        io::Error::new(io::ErrorKind::InvalidData, "unexpected-child-exit"),
                    ))
                };
            }
            Ok(None) if now() < until => std::thread::sleep(Duration::from_millis(2)),
            Ok(None) => break Failure::at("child-timeout", io::ErrorKind::TimedOut.into()),
            Err(error) => break Failure::at("child-wait", error),
        }
    };
    // Child ownership is retained until actual wait, including an error or
    // deadline. No detached worker or successful-cleanup claim follows a kill.
    let _ = child.kill();
    let cleanup_until = now().saturating_add(2_000_000_000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Err(failure),
            Ok(None) if now() < cleanup_until => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                // Abort keeps this failure visible as a kernel PID1 failure.
                eprintln!(
                    "POKELINUX_PROBE_FAILURE phase=child-cleanup kind=Other errno=None reason=actual-wait-not-observed"
                );
                std::process::abort();
            }
        }
    }
}
fn poweroff(report: Option<&Reporter>) -> ! {
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    if let Err(error) = reboot(RebootCommand::PowerOff) {
        failed(report, Failure::at("poweroff", error.into()));
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
    let report = match Reporter::open() {
        Ok(report) => report,
        Err(error) => {
            failed(None, Failure::at("diagnostic-open", error));
            poweroff(None);
        }
    };
    if let Err(error) = report.emit(format_args!(
        "POKELINUX_NATIVE_PID1 pid=1 uid=0 version={}",
        env!("CARGO_PKG_VERSION")
    )) {
        failed(None, Failure::at("diagnostic-admission", error));
        poweroff(Some(&report));
    }
    if let Err(error) = mounted() {
        failed(Some(&report), Failure::at("mount", error));
        poweroff(Some(&report));
    }
    report.attach_pmsg();
    if let Err(error) = report.emit(format_args!(
        "POKELINUX_PROBE_KERNEL release={}",
        uname().release().to_string_lossy()
    )) {
        failed(None, Failure::at("diagnostic-admission", error));
        poweroff(Some(&report));
    }
    if let Err(failure) = child(&report) {
        failed(Some(&report), failure);
        poweroff(Some(&report));
    }
    if let Err(error) = report.emit(format_args!(
        "POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false"
    )) {
        failed(None, Failure::at("diagnostic-admission", error));
    }
    poweroff(Some(&report));
}
