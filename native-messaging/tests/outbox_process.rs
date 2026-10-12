#![cfg(target_os = "linux")]
//! Actual native process/SQLite boundary; ARM64 user-mode QEMU is excluded.
use pokelinux_messaging::outbox::{Outbox, PartState};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const ID: &str = "0123456789abcdef0123456789abcdef";
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires an actual ARM64 kernel; run the native-process system VM"
)]
fn crash_child() {
    let Some(path) = std::env::var_os("POKELINUX_OUTBOX_CRASH_TEST") else {
        return;
    };
    let path = PathBuf::from(path);
    let mut box_ = Outbox::open(&path).unwrap();
    box_.prepare(ID, "+15550100", &"A".repeat(161), false)
        .unwrap();
    let _held = box_.claim(ID, 0).unwrap();
    fs::write(path.join("test-claim-committed"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires an actual ARM64 kernel; run the native-process system VM"
)]
fn sigkill_releases_owner_but_does_not_release_uncertain_part() {
    let directory = Directory(
        std::env::temp_dir().join(format!("sms-crash-{}", uuid::Uuid::new_v4().simple())),
    );
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child", "--nocapture", "--include-ignored"])
            .env("POKELINUX_OUTBOX_CRASH_TEST", &directory.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !directory.0.join("test-claim-committed").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before durable claim"
        );
        assert!(Instant::now() < deadline, "child did not commit claim");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(Outbox::open(&directory.0).is_err());
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(9));
    let mut box_ = Outbox::open(&directory.0).unwrap();
    assert_eq!(
        box_.lookup(ID).unwrap().parts,
        vec![PartState::Uncertain, PartState::Ready]
    );
    assert!(box_.claim(ID, 0).is_err());
    assert!(box_.claim(ID, 1).is_err());
}
