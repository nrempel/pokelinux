use crate::{MAX_PACKET, boottime_ns};
use rustix::fd::{AsFd, OwnedFd};
use rustix::net::{
    self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
    SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::Path,
};

pub fn listener(path: &Path, mode: u32) -> io::Result<OwnedFd> {
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    net::bind(&fd, &SocketAddrUnix::new(path)?)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    net::listen(&fd, 16)?;
    Ok(fd)
}
pub fn connect(path: &Path) -> io::Result<OwnedFd> {
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    net::connect(&fd, &SocketAddrUnix::new(path)?)?;
    Ok(fd)
}
pub fn receive(fd: impl AsFd) -> io::Result<Option<(Vec<u8>, UCred)>> {
    receive_limit(fd, MAX_PACKET)
}
pub(crate) fn receive_storage(fd: impl AsFd) -> io::Result<Option<(Vec<u8>, UCred)>> {
    receive_limit(fd, 65536)
}
fn receive_limit(fd: impl AsFd, limit: usize) -> io::Result<Option<(Vec<u8>, UCred)>> {
    let mut data = vec![0u8; limit];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let msg = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut data)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        result => result?,
    };
    if msg.bytes == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    let mut cred = None;
    let mut invalid = msg
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if cred.is_none() => cred = Some(c),
            _ => invalid = true,
        }
    }
    if invalid {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some((
        data[..msg.bytes].to_vec(),
        cred.ok_or(io::ErrorKind::PermissionDenied)?,
    )))
}
pub fn send(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    send_limit(fd, bytes, MAX_PACKET)
}
pub(crate) fn send_storage(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    send_limit(fd, bytes, 65536)
}
fn send_limit(fd: impl AsFd, bytes: &[u8], limit: usize) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > limit {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let count = net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)?;
    if count != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
pub fn call<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    fd: impl AsFd,
    request: &T,
) -> io::Result<R> {
    let bytes = serde_json::to_vec(request)?;
    let deadline = boottime_ns()
        .checked_add(2_000_000_000)
        .ok_or(io::ErrorKind::InvalidInput)?;
    loop {
        if boottime_ns() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match send(&fd, &bytes) {
            Ok(()) => break,
            Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                wait(&fd, rustix::event::PollFlags::OUT, deadline)?
            }
            Err(e) => return Err(e),
        }
    }
    if boottime_ns() >= deadline {
        return Err(io::ErrorKind::TimedOut.into());
    }
    loop {
        if boottime_ns() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match receive(&fd) {
            Ok(Some((data, cred))) => {
                if boottime_ns() >= deadline {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                if cred.uid.as_raw() != 0 {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                return Ok(serde_json::from_slice(&data)?);
            }
            Ok(None) => wait(&fd, rustix::event::PollFlags::IN, deadline)?,
            Err(e) if e.raw_os_error() == Some(4) => {}
            Err(e) => return Err(e),
        }
    }
}
fn wait(fd: impl AsFd, events: rustix::event::PollFlags, deadline: u64) -> io::Result<()> {
    let now = boottime_ns();
    if now >= deadline {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let duration = (deadline - now).min(50_000_000);
    let ts = rustix::event::Timespec {
        tv_sec: 0,
        tv_nsec: duration as i64,
    };
    match rustix::event::poll(&mut [rustix::event::PollFd::new(&fd, events)], Some(&ts)) {
        Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
