//! Private same-service-account diagnostics. No app or APN authority.
//! Domain-specific adaptation of the existing checked SEQPACKET transport.
use pokelinux_kernel_identity::socket_peer_pidfd;
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
    process,
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::unix::fs::PermissionsExt,
    path::Path,
};
pub const MAX_PACKET: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Status,
    Stop,
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}
fn command(bytes: &[u8]) -> io::Result<Request> {
    match bytes {
        [1] => Ok(Request::Status),
        [2] => Ok(Request::Stop),
        _ => Err(invalid()),
    }
}
fn socket() -> io::Result<OwnedFd> {
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    Ok(fd)
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Principal {
    uid: u32,
    gid: u32,
}
impl Principal {
    fn current() -> Self {
        Self {
            uid: process::geteuid().as_raw(),
            gid: process::getegid().as_raw(),
        }
    }
    fn accepts(self, peer: UCred) -> bool {
        peer.uid.as_raw() == self.uid && peer.gid.as_raw() == self.gid
    }
}
pub struct Listener {
    fd: OwnedFd,
    principal: Principal,
}
impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
impl Listener {
    /// Refuses any pre-existing socket path. Does not unlink on errors or Drop.
    pub fn bind(path: &Path) -> io::Result<Self> {
        let principal = Principal::current();
        let fd = socket()?;
        net::bind(&fd, &SocketAddrUnix::new(path)?)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        net::listen(&fd, 8)?;
        Ok(Self { fd, principal })
    }
    pub fn accept(&self) -> io::Result<Option<Connection>> {
        if self.principal != Principal::current() {
            return Err(denied());
        }
        let fd = match net::accept_with(&self.fd, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
            Err(rustix::io::Errno::AGAIN) => return Ok(None),
            value => value?,
        };
        net::sockopt::set_socket_passcred(&fd, true)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        if !self.principal.accepts(peer) {
            return Err(denied());
        }
        Ok(Some(Connection { fd, peer }))
    }
}
pub struct Connection {
    fd: OwnedFd,
    peer: UCred,
}
impl AsFd for Connection {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
impl Connection {
    pub fn peer(&self) -> UCred {
        self.peer
    }
    pub fn receive(&self) -> io::Result<Option<Request>> {
        receive(&self.fd, self.peer)?
            .map(|bytes| command(&bytes))
            .transpose()
    }
    /// Recompute a Status from current local state on each caller retry.
    /// Never queue an encoded historical response for later admission.
    pub fn send(&self, bytes: &[u8]) -> io::Result<()> {
        send(&self.fd, bytes)
    }
}
fn receive(fd: impl AsFd, expected: UCred) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = [0; MAX_PACKET];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        value => value?,
    };
    let mut invalid_message = message
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut cred = None;
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if cred.is_none() => cred = Some(value),
            RecvAncillaryMessage::ScmRights(handles) => {
                for handle in handles {
                    drop(handle);
                }
                invalid_message = true;
            }
            _ => invalid_message = true,
        }
    }
    if invalid_message {
        return Err(invalid());
    }
    if cred != Some(expected) {
        return Err(denied());
    }
    if message.bytes == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(Some(bytes[..message.bytes].to_vec()))
}
fn send(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET {
        return Err(invalid());
    }
    let count = net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)?;
    if count != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
/// One-shot diagnostics client; retains the exact server socket-origin pidfd.
pub struct Client {
    connection: Connection,
    server: OwnedFd,
}
impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.connection.as_fd()
    }
}
impl Client {
    /// Nonblocking attempt. Caller may retry EAGAIN/INTR under its original budget.
    pub fn connect(path: &Path) -> io::Result<Self> {
        let fd = socket()?;
        net::connect(&fd, &SocketAddrUnix::new(path)?)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        if !Principal::current().accepts(peer) {
            return Err(denied());
        }
        let server = socket_peer_pidfd(&fd)?;
        Ok(Self {
            connection: Connection { fd, peer },
            server,
        })
    }
    pub fn server_pid(&self) -> u32 {
        self.connection.peer.pid.as_raw_pid() as u32
    }
    pub fn send(&self, command: Request) -> io::Result<()> {
        send(
            &self.connection.fd,
            &[match command {
                Request::Status => 1,
                Request::Stop => 2,
            }],
        )
    }
    pub fn receive(&self) -> io::Result<Option<Vec<u8>>> {
        receive(&self.connection.fd, self.connection.peer)
    }
    /// Local process exit only: this client is neither the parent nor a reaper.
    pub fn server_exited(&self) -> io::Result<bool> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        let mut fds = [PollFd::new(&self.server, PollFlags::IN)];
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )?;
        let flags = fds[0].revents();
        if flags.intersects(PollFlags::ERR | PollFlags::NVAL) {
            return Err(invalid());
        }
        Ok(flags.contains(PollFlags::IN))
    }
    pub fn server_pidfd(&self) -> BorrowedFd<'_> {
        self.server.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_private_seqpacket_preserves_request_and_complete_reply() {
        use std::os::unix::fs::DirBuilderExt;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pkl-data-ipc-{}-{nonce}", std::process::id()));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let path = dir.join("status.sock");
        let listener = Listener::bind(&path).unwrap();
        let client = Client::connect(&path).unwrap();
        assert_eq!(client.server_pid(), std::process::id());
        let server = listener.accept().unwrap().unwrap();
        client.send(Request::Status).unwrap();
        assert_eq!(server.receive().unwrap(), Some(Request::Status));
        assert_eq!(server.peer().uid.as_raw(), process::geteuid().as_raw());
        let bytes = b"{\"schema\":1}\n";
        server.send(bytes).unwrap();
        assert_eq!(client.receive().unwrap(), Some(bytes.to_vec()));
        assert!(Listener::bind(&path).is_err());
        drop(server);
        drop(client);
        drop(listener);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn actual_seqpacket_refuses_oversized_incoming_reply() {
        use rustix::net::socketpair;
        let (left, right) = socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&right, true).unwrap();
        let peer = net::sockopt::socket_peercred(&right).unwrap();
        assert_eq!(
            net::send(&left, &vec![b'x'; MAX_PACKET + 1], SendFlags::DONTWAIT).unwrap(),
            MAX_PACKET + 1
        );
        assert_eq!(
            receive(&right, peer).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
