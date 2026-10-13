//! Explicit inherited-FD transfer in an isolated binary. No production selector.
use pokelinux_lifecycle::{boottime_ns, messages::Provider};
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
    process::Pid,
};
use std::{io, path::Path};
fn adopt(value: &std::ffi::OsStr) -> io::Result<OwnedFd> {
    let raw = value
        .to_str()
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|fd| *fd >= 3)
        .ok_or(io::ErrorKind::InvalidInput)?;
    // SAFETY: the isolated root runner passes exclusively transferred source
    // socket/pidfd descriptors. This binary accepts no application FD selector.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.first().is_some_and(|s| s == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&a[1..]);
    }
    if a.first().is_some_and(|s| s == "--messages-worker") {
        return pokelinux_messages::worker::entry(&a[1..]);
    }
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || a.len() != 11
        || a[6] == a[7]
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pid = a[8]
        .to_str()
        .and_then(|s| s.parse::<i32>().ok())
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = a[9]
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let source = a[10]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .to_owned();
    let mut observer = Observer::new(adopt(&a[6])?, adopt(&a[7])?, pid, uid, source)?;
    let mut messages = pokelinux_messages::worker::Projector::spawn(
        Path::new(&a[0]),
        Path::new(&a[4]),
        Path::new(&a[5]),
        &std::env::current_exe()?,
    )?;
    // The red selection uses exact f733 production service, which has no facts
    // ingress. It still constructs the real kernel-bound Observer and source.
    #[cfg(feature = "baseline-ignored")]
    let result = {
        let _ = &mut observer;
        pokelinux_lifecycle::service::run_with_messages(
            Path::new(&a[0]),
            Path::new(&a[1]),
            Path::new(&a[2]),
            Path::new(&a[3]),
            None,
            Some(&mut messages),
        )
    };
    #[cfg(not(feature = "baseline-ignored"))]
    let result = pokelinux_lifecycle::service::run_with_messages_and_session(
        Path::new(&a[0]),
        Path::new(&a[1]),
        Path::new(&a[2]),
        Path::new(&a[3]),
        None,
        Some(&mut messages),
        Some(&mut observer),
    );
    if !messages.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
        std::process::abort();
    }
    result
}
