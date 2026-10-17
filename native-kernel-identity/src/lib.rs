//! Fixed Linux socket-origin, owned-pidfd and process signal boundaries.
#[cfg(target_os = "linux")]
mod termination;
#[cfg(target_os = "linux")]
pub use termination::{TerminationGuard, termination_requested};
#[cfg(target_os = "linux")]
mod pidfd;
#[cfg(target_os = "linux")]
pub use pidfd::ProcessView;
use std::{
    io,
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};

/// Obtain an owned CLOEXEC pidfd for the exact process recorded by a connected
/// socket. An unsupported platform/kernel or malformed result is an error.
#[cfg(target_os = "linux")]
pub fn socket_peer_pidfd(socket: impl AsFd) -> io::Result<OwnedFd> {
    let mut raw: libc::c_int = -1;
    let mut length = size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: socket remains borrowed for the call; both output pointers refer
    // to initialized, aligned, writable objects of precisely the declared size.
    // The option is fixed, never supplied by a caller. Linux SO_PEERPIDFD creates
    // and installs a new descriptor for the socket's retained peer_pid reference.
    let status = unsafe {
        libc::getsockopt(
            socket.as_fd().as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERPIDFD,
            (&mut raw as *mut libc::c_int).cast(),
            &mut length,
        )
    };
    if status != 0 {
        // Linux may copy a prospective number and then fail copyout, releasing
        // the uninstalled slot. On syscall failure no descriptor is transferred;
        // never close that number or permit it to retarget a later reused slot.
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful SO_PEERPIDFD transfers a newly installed descriptor,
    // never a caller's borrowed descriptor. Own it before validating shape so
    // all rejected successful replies release the actual transferred resource.
    let owned = if raw >= 0 {
        Some(unsafe { OwnedFd::from_raw_fd(raw) })
    } else {
        None
    };
    validate(status, length, owned, None)
}
#[cfg(not(target_os = "linux"))]
pub fn socket_peer_pidfd(_socket: impl AsFd) -> io::Result<OwnedFd> {
    Err(io::ErrorKind::Unsupported.into())
}
#[cfg(target_os = "linux")]
fn validate(
    status: libc::c_int,
    length: libc::socklen_t,
    owned: Option<OwnedFd>,
    error: Option<io::Error>,
) -> io::Result<OwnedFd> {
    if let Some(error) = error {
        return Err(error);
    }
    if status != 0 || length != size_of::<libc::c_int>() as libc::socklen_t {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let fd = owned.ok_or(io::ErrorKind::InvalidData)?;
    // Linux installs pidfds with CLOEXEC atomically. Refuse an unexpected flag
    // state rather than opening an exec-inheritance interval with a later repair.
    if !rustix::io::fcntl_getfd(&fd)?.contains(rustix::io::FdFlags::CLOEXEC) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(fd)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn rejected_shape_and_error_release_the_actual_owned_descriptor() {
        for (status, length, error) in [
            (0, 0, None),
            (-1, 4, Some(io::Error::from_raw_os_error(libc::EINVAL))),
        ] {
            let file = std::fs::File::open("/dev/null").unwrap();
            let owned = rustix::io::fcntl_dupfd_cloexec(&file, 3).unwrap();
            let raw = owned.as_raw_fd();
            assert!(validate(status, length, Some(owned), error).is_err());
            assert!(!std::path::Path::new(&format!("/proc/self/fd/{raw}")).exists());
            assert!(rustix::io::fcntl_getfd(&file).is_ok());
        }
        assert!(validate(0, 4, None, None).is_err());
    }
}
