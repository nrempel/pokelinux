#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use omarchy_phone_core::service_socket::ServiceSocket;
use std::{
    fs,
    os::unix::{fs::DirBuilderExt, net::UnixStream},
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
#[cfg(target_arch = "x86_64")]
#[test]
fn killed_owner_leaves_a_recoverable_socket_and_releases_its_lock() {
    use std::{
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };
    struct OwnedChild(Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "socket_owner_crash_helper", "--ignored"])
            .env("PHONE_SOCKET_CRASH_DIR", &f.0)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !f.0.join("ready").exists() {
        assert!(child.0.try_wait().unwrap().is_none(), "helper exited early");
        assert!(Instant::now() < deadline, "helper startup deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(ServiceSocket::bind(&f.0).is_err());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(f.0.join("bridge.sock").exists(), "SIGKILL bypasses Drop");
    let recovered = ServiceSocket::bind(&f.0).unwrap();
    assert!(UnixStream::connect(f.0.join("bridge.sock")).is_ok());
    drop(recovered);
}
#[cfg(target_arch = "x86_64")]
#[test]
#[ignore = "child process of the socket crash test"]
fn socket_owner_crash_helper() {
    let path = PathBuf::from(std::env::var_os("PHONE_SOCKET_CRASH_DIR").unwrap());
    let _owner = ServiceSocket::bind(&path).unwrap();
    fs::write(path.join("ready"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}
