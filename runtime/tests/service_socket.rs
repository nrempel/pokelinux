#![cfg(target_os = "linux")]
use omarchy_phone_core::service_socket::ServiceSocket;
use std::{
    fs,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("phone-socket-{}", uuid::Uuid::new_v4().simple()));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn exclusive_owner_does_not_unlink_the_running_service() {
    let f = Fixture::new();
    let first = ServiceSocket::bind(&f.0).unwrap();
    let path = f.0.join("bridge.sock");
    let before = fs::symlink_metadata(&path).unwrap();
    assert_eq!(before.mode() & 0o777, 0o600);
    assert!(ServiceSocket::bind(&f.0).is_err());
    assert_eq!(fs::symlink_metadata(&path).unwrap().ino(), before.ino());
    let client = UnixStream::connect(&path).unwrap();
    let _peer = first.listener().accept().unwrap();
    drop(client);
    drop(first);
    assert!(!path.exists());
    let restarted = ServiceSocket::bind(&f.0).unwrap();
    drop(restarted);
}
#[test]
fn stale_socket_is_recovered_but_an_active_listener_is_preserved() {
    let f = Fixture::new();
    let path = f.0.join("bridge.sock");
    let stale = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(ServiceSocket::bind(&f.0).is_err());
    drop(stale);
    let owner = ServiceSocket::bind(&f.0).unwrap();
    assert!(UnixStream::connect(&path).is_ok());
    drop(owner);
}
#[test]
fn rejects_unsafe_directory_and_unrelated_files_without_mutating_them() {
    let f = Fixture::new();
    let path = f.0.join("bridge.sock");
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(ServiceSocket::bind(&f.0).is_err());
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&path, b"keep").unwrap();
    assert!(ServiceSocket::bind(&f.0).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"keep");
    fs::remove_file(&path).unwrap();
    fs::write(f.0.join("target"), b"keep").unwrap();
    std::os::unix::fs::symlink("target", &path).unwrap();
    assert!(ServiceSocket::bind(&f.0).is_err());
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
#[test]
fn cleanup_preserves_a_replacement_at_the_socket_path() {
    let f = Fixture::new();
    let owner = ServiceSocket::bind(&f.0).unwrap();
    let path = f.0.join("bridge.sock");
    fs::rename(&path, f.0.join("moved.sock")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    drop(owner);
    assert_eq!(fs::read(&path).unwrap(), b"replacement");
}
