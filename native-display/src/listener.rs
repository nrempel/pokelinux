//! Root-owned, distinct per-instance UNIX listener pathname.
//! Withdrawal fences new pathname connections; accepted clients remain independent.
use rustix::{
    fd::OwnedFd,
    fs::chown,
    process::{Gid, Uid},
};
use std::{
    fs, io,
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        net::UnixListener,
    },
    path::{Path, PathBuf},
};

fn protected_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for parent in path.ancestors() {
        let metadata = fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}

/// Caller creates one instance, transfers its fd to the owned SDK supervisor,
/// and retains this pathname owner until both app and SDK children are reaped.
/// No existing pathname is overwritten or reused.
pub struct Listener {
    parent: PathBuf,
    parent_identity: (u64, u64),
    directory: PathBuf,
    directory_identity: (u64, u64),
    path: PathBuf,
    identity: Option<(u64, u64)>,
    fd: Option<OwnedFd>,
    withdrawn: bool,
}
impl Listener {
    pub fn bind(parent: &Path, instance: &str, app_uid: u32) -> io::Result<Self> {
        if rustix::process::getuid().as_raw() != 0
            || rustix::process::geteuid().as_raw() != 0
            || !(10000..=60000).contains(&app_uid)
            || instance.len() != 32
            || !instance
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        protected_directory(parent)?;
        let metadata = fs::symlink_metadata(parent)?;
        let directory = parent.join(instance);
        // Nobody outside Root can reach the socket during its ACL setup,
        // regardless of the process umask. A collision is refused, never retried.
        fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let directory_metadata = fs::symlink_metadata(&directory)?;
        let mut owner = Self {
            parent: parent.to_owned(),
            parent_identity: (metadata.dev(), metadata.ino()),
            directory_identity: (directory_metadata.dev(), directory_metadata.ino()),
            path: directory.join("wayland.sock"),
            directory,
            identity: None,
            fd: None,
            withdrawn: false,
        };
        let listener = UnixListener::bind(&owner.path)?;
        let socket_metadata = fs::symlink_metadata(&owner.path)?;
        owner.identity = Some((socket_metadata.dev(), socket_metadata.ino()));
        owner.fd = Some(listener.into());
        rustix::io::fcntl_setfd(owner.fd.as_ref().unwrap(), rustix::io::FdFlags::CLOEXEC)?;
        let flags = rustix::fs::fcntl_getfl(owner.fd.as_ref().unwrap())?;
        rustix::fs::fcntl_setfl(
            owner.fd.as_ref().unwrap(),
            flags | rustix::fs::OFlags::NONBLOCK,
        )?;
        chown(&owner.path, Some(Uid::ROOT), Some(Gid::from_raw(app_uid)))?;
        fs::set_permissions(&owner.path, fs::Permissions::from_mode(0o660))?;
        chown(
            &owner.directory,
            Some(Uid::ROOT),
            Some(Gid::from_raw(app_uid)),
        )?;
        fs::set_permissions(&owner.directory, fs::Permissions::from_mode(0o710))?;
        owner.check_parent()?;
        Ok(owner)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn take_fd(&mut self) -> io::Result<OwnedFd> {
        if self.withdrawn {
            return Err(io::ErrorKind::NotConnected.into());
        }
        self.fd.take().ok_or(io::ErrorKind::NotConnected.into())
    }
    fn check_parent(&self) -> io::Result<()> {
        protected_directory(&self.parent)?;
        let metadata = fs::symlink_metadata(&self.parent)?;
        if (metadata.dev(), metadata.ino()) != self.parent_identity {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }
    fn check_directory(&self) -> io::Result<()> {
        let metadata = fs::symlink_metadata(&self.directory)?;
        if !metadata.is_dir()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || (metadata.dev(), metadata.ino()) != self.directory_identity
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }
    /// Synchronously remove only this protected socket node, before SDK Stop.
    /// An inode/parent mismatch is an error; it never deletes a replacement.
    /// Source watches and accepted Wayland clients are not changed here.
    pub fn withdraw(&mut self) -> io::Result<()> {
        self.check_parent()?;
        match self.check_directory() {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.withdrawn = true;
                self.fd = None;
                return Ok(());
            }
            Err(e) => return Err(e),
            Ok(()) => {}
        }
        if !self.withdrawn {
            match fs::symlink_metadata(&self.path) {
                Ok(metadata) => {
                    if !metadata.file_type().is_socket()
                        || metadata.uid() != 0
                        || metadata.nlink() != 1
                        || self.identity != Some((metadata.dev(), metadata.ino()))
                    {
                        return Err(io::ErrorKind::PermissionDenied.into());
                    }
                    fs::remove_file(&self.path)?;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            self.withdrawn = true;
            self.fd = None;
        }
        fs::remove_dir(&self.directory)
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        // Setup errors retain no helper. A successful caller separately retains
        // the SDK/app process owners through actual reap before dropping this.
        let _ = self.withdraw();
    }
}
