//! Isolated actual compositor harness consumer, absent from production entrypoints.
use pokelinux_lifecycle::boottime_ns;
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
    process::Pid,
};
use serde_json::json;
use std::{
    io::{self, Write},
    time::Duration,
};
fn adopt(value: &str) -> io::Result<OwnedFd> {
    let raw = value
        .parse::<i32>()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    if raw < 3 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    // SAFETY: the isolated parent transfers these two inherited descriptors
    // exclusively to this fixture child. It keeps actual compositor Child
    // ownership; these are not externally supplied application descriptors.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
fn emit(value: serde_json::Value) -> io::Result<()> {
    println!("{value}");
    io::stdout().flush()
}
fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 5 || a[0] == a[1] {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let pid = a[2]
        .parse::<i32>()
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = a[3].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    let mut source = Observer::new(adopt(&a[0])?, adopt(&a[1])?, pid, uid, a[4].clone())?;
    let until = boottime_ns()
        .checked_add(60_000_000_000)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let mut sequence = 0;
    let view = pokelinux_kernel_identity::ProcessView::open()?;
    emit(
        json!({"event":"ready","expected_source_pid":pid.as_raw_nonzero().get(),"expected_source_uid":uid}),
    )?;
    while boottime_ns() < until {
        let snapshot = match pokelinux_lifecycle::session::Provider::poll(&mut source) {
            Ok(pokelinux_lifecycle::session::Update::Current(snapshot)) => Some(snapshot),
            Ok(pokelinux_lifecycle::session::Update::AuthorityLost(loss)) => {
                emit(
                    json!({"event":"authority_lost", "observed_at_ns":loss.observed_at_ns,
                    "reason":format!("{:?}", loss.reason)}),
                )?;
                None
            }
            Ok(
                pokelinux_lifecycle::session::Update::AwaitingFirst
                | pokelinux_lifecycle::session::Update::Recovering,
            ) => None,
            Err(error) => {
                emit(
                    json!({"event":"failed","error":error.to_string(),"positive_state":source.snapshot().is_some()}),
                )?;
                return Ok(());
            }
        };
        if snapshot.is_some()
            && let Some(frame) = source.snapshot()
            && frame.sequence != sequence
        {
            sequence = frame.sequence;
            let focused = snapshot.as_ref().and_then(|s| s.focus.as_ref());
            let locker = snapshot.as_ref().and_then(|s| s.locker.as_ref());
            emit(json!({"event":"sample","frame":frame,
                "revision":snapshot.as_ref().map(|s| s.revision),
                "snapshot_lock_generation":snapshot.as_ref().and_then(|s| s.lock_generation),
                "snapshot_protocol_unlock":snapshot.as_ref().and_then(|s| s.protocol_unlock.as_ref()),
                "has_owned_focus":focused.is_some(),
                "has_owned_locker":locker.is_some(),
                "locker_peer_matches_metadata":locker.map(|f| view.matches(f.pidfd(), f.pidfd(), f.metadata.pid).unwrap_or(false)),
                "locker_peer_cloexec":locker.map(|f| rustix::io::fcntl_getfd(f.pidfd()).is_ok_and(|flags| flags.contains(rustix::io::FdFlags::CLOEXEC))),
                "focused_peer_matches_metadata":focused.map(|f| view.matches(f.pidfd(), f.pidfd(), f.metadata.pid).unwrap_or(false)),
                "focused_peer_cloexec":focused.map(|f| rustix::io::fcntl_getfd(f.pidfd()).is_ok_and(|flags| flags.contains(rustix::io::FdFlags::CLOEXEC)))}))?;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Err(io::ErrorKind::TimedOut.into())
}
