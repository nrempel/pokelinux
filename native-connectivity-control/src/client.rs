//! Explicit root-private requests. Reconnect never replays an effect.
use crate::{
    private_channel,
    protocol::{Choice, Reply, Request},
};
use pokelinux_network_state::boot_time_ns;
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType, UCred},
};
use std::{io, path::PathBuf};
pub struct Client {
    path: PathBuf,
    fd: Option<OwnedFd>,
    peer: Option<UCred>,
    pending: Option<Request>,
    last_id: u64,
}
impl Client {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            path,
            fd: None,
            peer: None,
            pending: None,
            last_id: 0,
        })
    }
    fn connect(&mut self) -> io::Result<()> {
        if self.fd.is_some() {
            return Ok(());
        }
        let fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )?;
        net::sockopt::set_socket_passcred(&fd, true)?;
        net::connect(&fd, &SocketAddrUnix::new(&self.path)?)?;
        let peer = private_channel::peer(&fd)?;
        self.peer = Some(peer);
        self.fd = Some(fd);
        Ok(())
    }
    fn request(&mut self, request: Request) -> io::Result<()> {
        if self.pending.is_some()
            || request.id() <= self.last_id
            || request.until(boot_time_ns()).is_none()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.connect()?;
        if request.until(boot_time_ns()).is_none() {
            self.withdraw();
            return Err(io::ErrorKind::TimedOut.into());
        }
        let bytes = serde_json::to_vec(&request)?;
        // Any send failure retires this socket. No retry is performed here.
        if let Err(e) = private_channel::send(self.fd.as_ref().unwrap(), &bytes) {
            self.withdraw();
            return Err(e);
        }
        self.last_id = request.id();
        self.pending = Some(request);
        Ok(())
    }
    pub fn inspect(&mut self, id: u64, until: u64) -> io::Result<()> {
        self.request(Request::Inspect {
            id,
            until_ns: until.to_string(),
        })
    }
    pub fn apply(&mut self, id: u64, choice: Choice, until: u64) -> io::Result<()> {
        self.request(Request::Apply {
            id,
            choice,
            until_ns: until.to_string(),
        })
    }
    pub fn poll(&mut self) -> io::Result<Option<Reply>> {
        let Some(request) = self.pending.as_ref() else {
            return Ok(None);
        };
        if request.until(boot_time_ns()).is_none() {
            self.withdraw();
            return Err(io::ErrorKind::TimedOut.into());
        }
        let received =
            private_channel::receive(self.fd.as_ref().unwrap(), self.peer.as_ref().unwrap());
        let bytes = match received {
            Ok(Some(b)) => b,
            Ok(None) => return Ok(None),
            Err(e) => {
                self.withdraw();
                return Err(e);
            }
        };
        let reply = serde_json::from_slice::<Reply>(&bytes)
            .ok()
            .filter(|r| r.valid_for(request, boot_time_ns()));
        if let Some(reply) = reply {
            self.pending = None;
            Ok(Some(reply))
        } else {
            self.withdraw();
            Err(io::ErrorKind::InvalidData.into())
        }
    }
    /// Fence requester publication only; the server owns any transferred work.
    pub fn withdraw(&mut self) {
        self.pending = None;
        self.peer = None;
        self.fd = None;
    }
}
