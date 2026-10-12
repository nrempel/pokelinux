//! Private local socket ownership. Acquire the persistent journal owner before
//! publishing this socket, and keep that owner until command workers finish.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixListener,
    },
    path::{Path, PathBuf},
};

pub struct ServiceSocket {
    listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
    _owner: File,
}
fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "unsafe or occupied service socket path",
    )
}
fn owned(metadata: &fs::Metadata) -> bool {
    metadata.uid() == rustix::process::geteuid().as_raw()
}
impl ServiceSocket {
    pub fn bind(directory: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(directory)?;
        if !directory.is_absolute()
            || !metadata.is_dir()
            || !owned(&metadata)
            || metadata.mode() & 0o777 != 0o700
            || fs::canonicalize(directory)? != directory
        {
            return Err(refused());
        }
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(directory.join("socket-owner.lock"))?;
        let metadata = owner.metadata()?;
        if !metadata.is_file()
            || !owned(&metadata)
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
        {
            return Err(refused());
        }
        owner.try_lock().map_err(|_| {
            io::Error::new(io::ErrorKind::AddrInUse, "service socket already owned")
        })?;
        let path = directory.join("bridge.sock");
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.file_type().is_socket()
                    || !owned(&metadata)
                    || metadata.mode() & 0o777 != 0o600
                    || metadata.nlink() != 1
                {
                    return Err(refused());
                }
                // A nonblocking probe must not hang on another listener's full
                // backlog. Only ECONNREFUSED establishes a stale socket here.
                let probe = rustix::net::socket_with(
                    rustix::net::AddressFamily::UNIX,
                    rustix::net::SocketType::STREAM,
                    rustix::net::SocketFlags::NONBLOCK | rustix::net::SocketFlags::CLOEXEC,
                    None,
                )?;
                let address = rustix::net::SocketAddrUnix::new(&path)?;
                if rustix::net::connect(&probe, &address) != Err(rustix::io::Errno::CONNREFUSED) {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "existing socket is not stale",
                    ));
                }
                let current = fs::symlink_metadata(&path)?;
                if current.dev() != metadata.dev() || current.ino() != metadata.ino() {
                    return Err(refused());
                }
                fs::remove_file(&path)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&path)?;
        let metadata = fs::symlink_metadata(&path)?;
        let result = Self {
            listener,
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
            _owner: owner,
        };
        fs::set_permissions(&result.path, fs::Permissions::from_mode(0o600))?;
        Ok(result)
    }
    pub fn listener(&self) -> &UnixListener {
        &self.listener
    }
}
impl Drop for ServiceSocket {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = fs::remove_file(&self.path);
        }
        // Keep the lock inode: deleting it could create two independent locks.
    }
}
