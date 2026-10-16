use crate::source::{Compositor, MAX_BOOTSTRAP};
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use pokelinux_lifecycle::{authority::living, boottime_ns, ipc};
use rustix::{
    fd::{AsFd, AsRawFd},
    process::{self, Signal},
};
use serde::{Deserialize, Serialize};
use std::{io, os::unix::process::CommandExt, process::Command, time::Duration};

const MAX_BOOTSTRAP_RECEIVE_NS: u64 = 5_000_000_000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bootstrap {
    compositor: Compositor,
    original_until_ns: u64,
}
pub(crate) fn encode(spec: &Compositor, original_until_ns: u64) -> io::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(&Bootstrap {
        compositor: spec.clone(),
        original_until_ns,
    })?;
    if bytes.len() > MAX_BOOTSTRAP {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(bytes)
}
/// Fixed internal selector. No pathname listener and no raw-FD command argument.
/// The single-threaded worker becomes the trusted compositor in the same PID.
pub fn bootstrap_entry() -> io::Result<()> {
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    rustix::thread::set_no_new_privs(true)?;
    if process::getuid().as_raw() != 0 || process::getgid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let input = io::stdin();
    rustix::io::fcntl_setfd(input.as_fd(), rustix::io::FdFlags::CLOEXEC)?;
    let channel = rustix::io::fcntl_dupfd_cloexec(input.as_fd(), 3)?;
    if rustix::net::sockopt::socket_domain(&channel)? != rustix::net::AddressFamily::UNIX
        || rustix::net::sockopt::socket_type(&channel)? != rustix::net::SocketType::SEQPACKET
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let credentials = rustix::net::sockopt::socket_peercred(&channel)?;
    if credentials.uid.as_raw() != 0
        || credentials.gid.as_raw() != 0
        || process::getppid() != Some(credentials.pid)
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent = socket_peer_pidfd(&channel)?;
    let view = ProcessView::open()?;
    if !view.matches(
        &parent,
        &parent,
        credentials.pid.as_raw_nonzero().get() as u32,
    )? {
        return Err(io::ErrorKind::NotConnected.into());
    }
    let receive_until = boottime_ns()
        .checked_add(MAX_BOOTSTRAP_RECEIVE_NS)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let bytes = loop {
        if !living(&parent)?
            || process::getppid() != Some(credentials.pid)
            || boottime_ns() >= receive_until
        {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some((bytes, actual)) = ipc::receive(&channel)? {
            if actual != credentials {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            break bytes;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    let start: Bootstrap = serde_json::from_slice(&bytes)?;
    start.compositor.validate(start.original_until_ns)?;
    let gid = process::Gid::from_raw(1000);
    let uid = process::Uid::from_raw(1000);
    rustix::thread::set_thread_groups(&[])?;
    rustix::thread::set_thread_res_gid(gid, gid, gid)?;
    rustix::thread::set_thread_res_uid(uid, uid, uid)?;
    // Linux clears PDEATHSIG on credential changes; rearm then close parent race.
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    if !living(&parent)? || process::getppid() != Some(credentials.pid) {
        return Err(io::ErrorKind::NotConnected.into());
    }
    let mut command = Command::new(&start.compositor.executable);
    command
        .args(&start.compositor.arguments)
        .env_clear()
        .envs(&start.compositor.environment)
        .env("POKELINUX_SESSION_FD", channel.as_raw_fd().to_string())
        .env("POKELINUX_SESSION_SOURCE", &start.compositor.source)
        .current_dir("/");
    if boottime_ns() >= start.original_until_ns {
        return Err(io::ErrorKind::TimedOut.into());
    }
    // Only this selected >=3 endpoint crosses final exec. stdin and parent pidfd
    // remain CLOEXEC. The existing adapter sets its inherited copy CLOEXEC.
    rustix::io::fcntl_setfd(&channel, rustix::io::FdFlags::empty())?;
    Err(command.exec())
}
