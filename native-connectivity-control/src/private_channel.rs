//! Same root-private credential boundary as the brightness owner.
use crate::protocol::LIMIT;
use rustix::{
    fd::OwnedFd,
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, SendFlags, UCred,
    },
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
};
pub(crate) fn peer(fd: &OwnedFd) -> io::Result<UCred> {
    net::sockopt::set_socket_passcred(fd, true)?;
    let c = net::sockopt::socket_peercred(fd)?;
    if c.uid.as_raw() != 0 || c.gid.as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(c)
}
pub(crate) fn receive(fd: &OwnedFd, peer: &UCred) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = [0; LIMIT];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let msg = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        v => v?,
    };
    let mut cred = None;
    let mut invalid = msg.bytes == 0
        || msg
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if cred.is_none() => cred = Some(c),
            _ => invalid = true,
        }
    }
    let c = cred.ok_or(io::ErrorKind::PermissionDenied)?;
    if invalid || c.pid != peer.pid || c.uid != peer.uid || c.gid != peer.gid {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(Some(bytes[..msg.bytes].to_vec()))
}
pub(crate) fn send(fd: &OwnedFd, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > LIMIT {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    if net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)? != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
