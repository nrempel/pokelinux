//! Real Unix listener/child boundaries. No PipeWire protocol or audio health proof.
use pokelinux_power::clock::{BootClock, Clock};
use std::fs;
use std::io::Read;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::sleep;
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "audio-backend-ready-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn now() -> Duration {
    BootClock.now().elapsed()
}
fn run(socket: &std::path::Path, pid: u32, role: &str) -> (ExitStatus, String, u32, Duration) {
    let start = now();
    let until = start.checked_add(Duration::from_secs(8)).unwrap();
    let mut child = Owned(
        Command::new(env!("CARGO_BIN_EXE_backend-ready"))
            .arg(socket)
            .arg(pid.to_string())
            .arg(rustix::process::getuid().as_raw().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let id = child.0.id();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(now() < until, "original eight-second child cutoff: {role}");
        sleep(Duration::from_millis(2));
    };
    let status_wait = child.0.wait().unwrap();
    assert_eq!(status, status_wait);
    let mut stdout = Vec::new();
    let mut stderr = String::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert!(stderr.len() <= 1024);
    let elapsed = now().checked_sub(start).unwrap();
    assert!(now() < until, "wait/PIPE EOF exceeded original cutoff");
    println!(
        "BACKEND_READY_CHILD role={role} pid={id} exit={status} elapsed_ns={} actual_wait=true both_PIPE_EOF=true",
        elapsed.as_nanos()
    );
    (status, stderr, id, elapsed)
}

#[test]
fn live_matching_peer_connects_without_protocol_bytes() {
    let directory = Directory::new();
    let path = directory.0.join("ready.sock");
    let listener = UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (status, stderr, helper, _) = run(&path, std::process::id(), "matching");
    assert!(
        status.success(),
        "matching live peer must be ready: {stderr}"
    );
    let (mut stream, _) = listener.accept().unwrap();
    let peer = rustix::net::sockopt::socket_peercred(&stream).unwrap();
    assert_eq!(peer.pid.as_raw_pid() as u32, helper);
    let mut data = [0; 1];
    assert_eq!(
        stream.read(&mut data).unwrap(),
        0,
        "probe must send no protocol bytes"
    );
}

#[test]
fn absent_endpoint_expires_at_original_five_second_deadline() {
    let directory = Directory::new();
    let (status, stderr, _, elapsed) = run(
        &directory.0.join("absent.sock"),
        std::process::id(),
        "absent",
    );
    assert_eq!(status.code(), Some(1));
    assert!(stderr.contains("socket deadline expired"), "{stderr}");
    assert!(
        elapsed >= Duration::from_secs(5),
        "absence must reach original deadline"
    );
}

#[test]
fn live_wrong_peer_is_refused_before_original_deadline() {
    let directory = Directory::new();
    let path = directory.0.join("wrong.sock");
    let _listener = UnixListener::bind(&path).unwrap();
    let wrong = std::process::id().checked_add(1).unwrap();
    let (status, stderr, _, elapsed) = run(&path, wrong, "wrong-peer");
    assert_eq!(status.code(), Some(1));
    assert!(stderr.contains("peer mismatch"), "{stderr}");
    assert!(elapsed < Duration::from_secs(5));
}
