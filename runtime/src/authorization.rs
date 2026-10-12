//! Same-user Unix socket authorization and service-incarnation fencing.
//! The session is public to trusted same-UID clients, not an app isolation key.
use crate::{Identity, Request, ipc::receive_request};
use std::{io, os::unix::net::UnixStream, time::Duration};

pub struct Session {
    identity: Identity,
    owner: rustix::process::Uid,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            identity: Identity(uuid::Uuid::new_v4().simple().to_string()),
            owner: rustix::process::geteuid(),
        }
    }
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "unauthorized local connection or stale service session",
    )
}
impl Session {
    fn check_peer(&self, socket: &UnixStream) -> io::Result<()> {
        // Credentials come from the kernel at connect/socketpair time, never
        // from a request field or PID lookup. No cache shared across sockets.
        let peer = rustix::net::sockopt::socket_peercred(socket)?;
        if peer.uid != self.owner || rustix::process::geteuid() != self.owner {
            return Err(denied());
        }
        Ok(())
    }
    pub fn identity_for(&self, socket: &UnixStream) -> io::Result<Identity> {
        self.check_peer(socket)?;
        Ok(self.identity.clone())
    }
    /// Check the peer before reading any payload, then fence stale UI work.
    /// The caller must keep admission and dispatch on the service-owned path.
    pub fn receive(&self, socket: &mut UnixStream, budget: Duration) -> io::Result<Request> {
        self.check_peer(socket)?;
        let request = receive_request(socket, budget)?;
        if request.session() != &self.identity {
            return Err(denied());
        }
        Ok(request)
    }
}
