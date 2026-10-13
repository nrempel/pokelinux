//! Packet credentials are authoritative even for inherited socketpairs.
use crate::MAX_PACKET;
use rustix::fd::{AsFd, OwnedFd};
use rustix::net::{
    self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, SendFlags, UCred,
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::Path,
};

pub fn listener(path: &Path, mode: u32) -> io::Result<OwnedFd> {
    let fd = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::SEQPACKET,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    net::bind(&fd, &net::SocketAddrUnix::new(path)?)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    net::listen(&fd, 16)?;
    Ok(fd)
}
pub fn send(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    if net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)? != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
pub fn receive(fd: impl AsFd) -> io::Result<Option<(Vec<u8>, UCred)>> {
    let mut bytes = [0; MAX_PACKET];
    let mut control =
        [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(16))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut control);
    let msg = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        r => r?,
    };
    if msg.bytes == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    let mut credentials = None;
    let mut invalid = msg
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if credentials.is_none() => {
                credentials = Some(c)
            }
            _ => invalid = true,
        }
    }
    if invalid {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some((
        bytes[..msg.bytes].to_vec(),
        credentials.ok_or(io::ErrorKind::PermissionDenied)?,
    )))
}
pub fn living(fd: impl AsFd) -> io::Result<bool> {
    let mut polls = [rustix::event::PollFd::new(
        &fd,
        rustix::event::PollFlags::IN,
    )];
    rustix::event::poll(
        &mut polls,
        Some(&rustix::event::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    Ok(polls[0].revents().is_empty())
}
