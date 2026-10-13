//! Session-user lock intent. Acknowledgement grants no presentation or unlock authority.
use crate::{Reply, boottime_ns, ipc};
use rustix::net;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

pub const SESSION_UID: u32 = 1000;
pub const SOCKET: &str = "/run/pokelinux/messages/lock.sock";
pub const ORIGINAL_ACTION: &str = "/usr/libexec/pokelinux/omarchy-system-lock-original";
pub const ENTRY_BUDGET_NS: u64 = 2_000_000_000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Lock {},
}

/// One absolute deadline covers connect, send and the actual actor acknowledgement.
pub fn admit(path: &Path, until: u64) -> io::Result<u64> {
    if boottime_ns() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let fd = loop {
        match ipc::connect(path) {
            Ok(fd) => break fd,
            Err(e) if e.raw_os_error() == Some(4) && boottime_ns() < until => {}
            Err(e) => return Err(e),
        }
    };
    let peer = net::sockopt::socket_peercred(&fd)?;
    if peer.uid.as_raw() != 0 || peer.gid.as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let reply: Reply = ipc::call_until_from(&fd, &Request::Lock {}, until, Some(peer))?;
    let acknowledged = boottime_ns();
    if acknowledged >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    if reply != Reply::Ok {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(acknowledged)
}
