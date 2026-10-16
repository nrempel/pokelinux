//! Private helper for typed IPC control and real nonroot reply credentials.
use pokelinux_lifecycle::{AdminRequest, Reply, ipc};
use std::{io, os::fd::AsFd, path::Path};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, runtime] if mode == "--control" && rustix::process::getuid().as_raw() == 0 => {
            let fd = ipc::connect(&Path::new(runtime).join("admin.sock"))?;
            let reply: Reply = ipc::call(fd, &AdminRequest::Stop {})?;
            if reply != Reply::Ok {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(())
        }
        [mode] if mode == "--reply-ok" => {
            ipc::send(io::stdin().as_fd(), &serde_json::to_vec(&Reply::Ok)?)
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
