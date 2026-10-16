//! Fixed root-only Stop request; independent of session configuration preflight.
use pokelinux_lifecycle::{AdminRequest, Reply, ipc};
use std::{io, path::Path};

pub fn request(runtime: &Path) -> io::Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    if !runtime.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fd = ipc::connect(&runtime.join("admin.sock"))?;
    // Existing bounded call verifies the actual reply sender has UID0.
    let reply: Reply = ipc::call(fd, &AdminRequest::Stop {})?;
    if reply != Reply::Ok {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
