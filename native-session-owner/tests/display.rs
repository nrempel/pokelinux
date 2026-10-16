//! Root-only actual Linux process/socket tests, no compositor/SDK Ready claim.
use pokelinux_display::{SourceIdentity, boottime_ns};
use pokelinux_lifecycle::{
    AppProfile, audio,
    display::{Context, Factory as _, State},
};
use pokelinux_session_owner::display::Factory;
use rustix::{
    event::{PollFd, PollFlags, poll},
    fd::OwnedFd,
    process::{Pid, PidfdFlags},
};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
static SERIAL: AtomicU32 = AtomicU32::new(0);
const INSTANCE: &str = "0123456789abcdef0123456789abcdef";
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
fn dead(fd: &OwnedFd) -> bool {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&rustix::time::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .unwrap();
    !fds[0].revents().is_empty()
}
fn child_set() -> BTreeSet<u32> {
    fs::read_to_string("/proc/thread-self/children")
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect()
}
struct RootDir(PathBuf);
impl RootDir {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let p = PathBuf::from(format!(
            "/run/session-display-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        fs::create_dir(p.join("listeners")).unwrap();
        Self(p)
    }
}
impl Drop for RootDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Peer {
    child: Child,
    until: u64,
    output: Vec<u8>,
    errors: Vec<u8>,
    rows: Vec<serde_json::Value>,
    stdout_eof: bool,
    stderr_eof: bool,
    pipes_ready: bool,
}
impl Peer {
    fn new(root: &Path, label: &str, uid: u32) -> Self {
        Self::with_delayed_bind(root, label, uid, false)
    }
    fn with_delayed_bind(root: &Path, label: &str, uid: u32, delayed: bool) -> Self {
        let dir = root.join(label);
        fs::create_dir(&dir).unwrap();
        rustix::fs::chown(
            &dir,
            Some(rustix::process::Uid::from_raw(uid)),
            Some(rustix::process::Gid::from_raw(uid)),
        )
        .unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let child = Command::new(helper())
            .args([
                if delayed {
                    "--delayed-peer"
                } else {
                    "--held-peer"
                },
                dir.join("wayland.sock").to_str().unwrap(),
                &uid.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let until = boottime_ns() + 8_000_000_000;
        // Retain custody before any fallible post-spawn descriptor setup.
        let mut peer = Self {
            child,
            until,
            output: vec![],
            errors: vec![],
            rows: vec![],
            stdout_eof: false,
            stderr_eof: false,
            pipes_ready: false,
        };
        for fd in [
            peer.child.stdout.as_ref().unwrap() as &dyn std::os::fd::AsFd,
            peer.child.stderr.as_ref().unwrap() as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd).unwrap();
            rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        }
        peer.pipes_ready = true;
        while boottime_ns() < until {
            peer.pump();
            if let Some(row) = peer.rows.first() {
                assert_eq!(
                    row["event"],
                    if delayed {
                        "waiting_for_bind"
                    } else {
                        "listening"
                    }
                );
                assert_eq!(row["pid"], peer.child.id());
                assert_eq!(row["uid"], uid);
                assert_eq!(row["gid"], uid);
                return peer;
            }
            assert!(
                peer.child.try_wait().unwrap().is_none(),
                "peer exited before ready"
            );
            tick();
        }
        panic!("original peer startup deadline");
    }
    fn pid(&self) -> Pid {
        Pid::from_raw(self.child.id() as i32).unwrap()
    }
    fn watch(&self) -> OwnedFd {
        rustix::process::pidfd_open(self.pid(), PidfdFlags::NONBLOCK).unwrap()
    }
    fn pump(&mut self) {
        let mut bytes = [0; 1024];
        loop {
            match self.child.stdout.as_mut().unwrap().read(&mut bytes) {
                Ok(0) => {
                    self.stdout_eof = true;
                    break;
                }
                Ok(n) => {
                    self.output.extend_from_slice(&bytes[..n]);
                    assert!(self.output.len() <= 2048);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
        let complete = self
            .output
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(&[][..], |end| &self.output[..end]);
        self.rows = complete
            .split(|b| *b == b'\n')
            .filter(|v| !v.is_empty())
            .map(|v| serde_json::from_slice(v).unwrap())
            .collect();
        loop {
            match self.child.stderr.as_mut().unwrap().read(&mut bytes) {
                Ok(0) => {
                    self.stderr_eof = true;
                    break;
                }
                Ok(n) => {
                    self.errors.extend_from_slice(&bytes[..n]);
                    assert!(self.errors.len() <= 2048);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if !self.pipes_ready {
            // Close partially prepared pipes without reading or claiming EOF.
            self.child.stdin.take();
            self.child.stdout.take();
            self.child.stderr.take();
            let _ = self.child.kill();
            let status = self.child.wait().unwrap();
            println!(
                "{}",
                serde_json::json!({"fixture_peer_actual_wait":true,"partial_setup":true,"pid":self.child.id(),"code":status.code(),"stdout_eof_after_wait":false,"stderr_eof_after_wait":false,"stdout_closed_without_EOF":true,"stderr_closed_without_EOF":true})
            );
            return;
        }
        if let Some(mut input) = self.child.stdin.take() {
            let _ = input.write_all(b"Q");
        }
        loop {
            self.pump();
            if let Some(status) = self.child.try_wait().unwrap() {
                self.pump();
                assert!(self.stdout_eof && self.stderr_eof);
                assert!(
                    status.success(),
                    "peer failed: {status:?} {:?}",
                    self.errors
                );
                println!(
                    "{}",
                    serde_json::json!({"fixture_peer_actual_wait":true,"pid":self.child.id(),"code":status.code(),"stdout_eof_after_wait":self.stdout_eof,"stderr_eof_after_wait":self.stderr_eof})
                );
                break;
            }
            if boottime_ns() >= self.until {
                let _ = self.child.kill();
                let _ = self.child.wait();
                std::process::abort();
            }
            tick();
        }
    }
}
fn helper() -> PathBuf {
    PathBuf::from(
        std::env::var_os("SESSION_DISPLAY_TEST_HELPER")
            .expect("root-installed fixed fixture helper"),
    )
}
fn source_path(root: &RootDir, label: &str) -> PathBuf {
    root.0.join(label).join("wayland.sock")
}
fn factory(root: &RootDir, peer: &Peer, label: &str) -> Factory {
    Factory::new(
        root.0.join("listeners"),
        source_path(root, label),
        peer.watch(),
        SourceIdentity {
            pid: peer.pid(),
            uid: 1000,
            gid: 1000,
        },
        helper(),
    )
    .unwrap()
}
fn profile() -> AppProfile {
    AppProfile {
        app: audio::APP.into(),
        uid: 11042,
        executable: "/usr/libexec/pokelinux/pokelinux-sounds".into(),
        args: vec![
            "/var/lib/pokelinux-sounds".into(),
            "/run/user/11042".into(),
            "/run/user/1000/wayland-0".into(),
            "/run/pokelinux/appearance.sock".into(),
            "/run/pokelinux/session/app.sock".into(),
            "/usr/share/pokelinux/appearance".into(),
        ],
    }
}
struct Guard(Option<Box<dyn Context>>, u64);
impl Guard {
    fn new(value: Box<dyn Context>) -> Self {
        Self(Some(value), boottime_ns() + 2_000_000_000)
    }
    fn value(&mut self) -> &mut (dyn Context + 'static) {
        self.0.as_deref_mut().unwrap()
    }
    fn reap(&mut self) {
        while self.value().occupied() && boottime_ns() < self.1 {
            let _ = self.value().poll();
            tick();
        }
        assert!(
            !self.value().occupied(),
            "SDK retained after original cleanup cutoff"
        );
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if self.0.is_some() {
            let _ = self.value().stop();
            self.reap();
            self.0.take();
        }
    }
}
fn refused(value: io::Result<Box<dyn Context>>) {
    let rejected = value.is_err();
    if let Ok(ctx) = value {
        let mut guard = Guard::new(ctx);
        let _ = guard.value().stop();
        guard.reap();
    }
    assert!(rejected, "non-Sounds profile admitted");
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn invalid_app_uid_and_argument_count_are_refused() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "peer", 1000);
    let mut f = factory(&root, &peer, "peer");
    let before = child_set();
    for case in [0, 1, 2, 3] {
        let mut p = profile();
        match case {
            0 => p.app = "org.omarchy.Other".into(),
            1 => p.uid = 11043,
            2 => {
                p.args.pop();
            }
            _ => p.args.push("extra".into()),
        }
        refused(f.start(&p, INSTANCE, boottime_ns() + 900_000_000));
    }
    assert_eq!(child_set(), before);
    assert_eq!(fs::read_dir(root.0.join("listeners")).unwrap().count(), 0);
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn messages_is_not_the_sounds_display_role() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "peer", 1000);
    let mut f = factory(&root, &peer, "peer");
    let mut p = profile();
    p.app = pokelinux_lifecycle::messages::APP.into();
    refused(f.start(&p, INSTANCE, boottime_ns() + 900_000_000));
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn wrong_source_role_and_mismatched_pidfd_are_refused() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "peer", 1000);
    for identity in [
        SourceIdentity {
            pid: peer.pid(),
            uid: 0,
            gid: 0,
        },
        SourceIdentity {
            pid: rustix::process::getpid(),
            uid: 1000,
            gid: 1000,
        },
    ] {
        assert!(
            Factory::new(
                root.0.join("listeners"),
                source_path(&root, "peer"),
                peer.watch(),
                identity,
                helper()
            )
            .is_err()
        );
    }
    assert_eq!(fs::read_dir(root.0.join("listeners")).unwrap().count(), 0);
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn actual_wrong_wayland_peer_is_refused_before_sdk_spawn() {
    let root = RootDir::new();
    let actual = Peer::new(&root.0, "actual", 1000);
    let expected = Peer::new(&root.0, "expected", 1000);
    let mut f = Factory::new(
        root.0.join("listeners"),
        source_path(&root, "actual"),
        expected.watch(),
        SourceIdentity {
            pid: expected.pid(),
            uid: 1000,
            gid: 1000,
        },
        helper(),
    )
    .unwrap();
    let before = child_set();
    refused(f.start(&profile(), INSTANCE, boottime_ns() + 900_000_000));
    assert_eq!(child_set(), before);
    assert_eq!(fs::read_dir(root.0.join("listeners")).unwrap().count(), 0);
    drop(actual);
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn expired_initialization_deadline_creates_no_sdk() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "peer", 1000);
    let mut f = factory(&root, &peer, "peer");
    let before = child_set();
    refused(f.start(&profile(), INSTANCE, boottime_ns()));
    assert_eq!(child_set(), before);
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn sounds_owns_preparing_sdk_listener_then_actual_reap() {
    let root = RootDir::new();
    let mut peer = Peer::new(&root.0, "peer", 1000);
    let mut f = factory(&root, &peer, "peer");
    let p = profile();
    let before = child_set();
    let until = boottime_ns() + 900_000_000;
    let mut guard = Guard::new(
        f.start(&p, INSTANCE, until)
            .expect("Sounds must be accepted"),
    );
    let new: Vec<_> = child_set().difference(&before).copied().collect();
    assert_eq!(new.len(), 1);
    let sdk = Pid::from_raw(new[0] as i32).unwrap();
    let watch = rustix::process::pidfd_open(sdk, PidfdFlags::NONBLOCK).unwrap();
    let args = guard.value().arguments().to_vec();
    assert_eq!(args.len(), 6);
    for i in [0, 1, 3, 4, 5] {
        assert_eq!(args[i], p.args[i]);
    }
    let path = PathBuf::from(&args[2]);
    assert_eq!(
        path,
        root.0.join("listeners").join(INSTANCE).join("wayland.sock")
    );
    let socket = fs::symlink_metadata(&path).unwrap();
    assert!(socket.file_type().is_socket());
    assert_eq!(
        (socket.uid(), socket.gid(), socket.mode() & 0o777),
        (0, 11042, 0o660)
    );
    while boottime_ns() < until {
        assert_eq!(guard.value().poll().unwrap(), State::Preparing);
        peer.pump();
        if peer
            .rows
            .iter()
            .any(|r| r["event"] == "actual_registry_request")
        {
            break;
        }
        tick();
    }
    assert!(
        peer.rows
            .iter()
            .any(|r| r["event"] == "actual_registry_request"),
        "real SDK emitted no request before original deadline"
    );
    guard.value().stop().unwrap();
    assert!(!path.exists());
    assert!(guard.value().occupied());
    while !dead(&watch) && boottime_ns() < guard.1 {
        tick();
    }
    assert!(dead(&watch));
    assert!(
        guard.value().occupied(),
        "pidfd death incorrectly cleared custody"
    );
    guard.reap();
    assert!(!PathBuf::from(format!("/proc/{}", sdk.as_raw_pid())).exists());
    assert_eq!(
        rustix::process::waitpid(Some(sdk), rustix::process::WaitOptions::NOHANG).unwrap_err(),
        rustix::io::Errno::CHILD
    );
}
#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn withdrawal_error_still_stops_and_reaps_sdk_preserving_replacement() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "peer", 1000);
    let mut f = factory(&root, &peer, "peer");
    let mut guard = Guard::new(
        f.start(&profile(), INSTANCE, boottime_ns() + 900_000_000)
            .expect("Sounds must be accepted"),
    );
    let path = PathBuf::from(&guard.value().arguments()[2]);
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"replacement").unwrap();
    assert!(guard.value().stop().is_err());
    assert!(guard.value().occupied());
    guard.reap();
    assert_eq!(fs::read(&path).unwrap(), b"replacement");
    drop(guard);
    assert!(path.is_file());
}

#[test]
#[ignore = "isolated root and real UID1000 process required"]
fn cold_source_may_bind_wayland_after_factory_construction() {
    let root = RootDir::new();
    let mut peer = Peer::with_delayed_bind(&root.0, "peer", 1000, true);
    let path = source_path(&root, "peer");
    assert!(!path.exists());
    let mut f = factory(&root, &peer, "peer");
    let before = child_set();
    refused(f.start(&profile(), INSTANCE, boottime_ns() + 900_000_000));
    assert_eq!(child_set(), before);
    assert_eq!(fs::read_dir(root.0.join("listeners")).unwrap().count(), 0);
    peer.child.stdin.as_mut().unwrap().write_all(b"B").unwrap();
    while boottime_ns() < peer.until {
        peer.pump();
        if peer.rows.iter().any(|row| row["event"] == "listening") {
            break;
        }
        assert!(peer.child.try_wait().unwrap().is_none());
        tick();
    }
    assert!(peer.rows.iter().any(|row| row["event"] == "listening"));
    assert!(fs::symlink_metadata(&path).unwrap().file_type().is_socket());
    let until = boottime_ns() + 900_000_000;
    let mut guard = Guard::new(
        f.start(&profile(), INSTANCE, until)
            .expect("same live Source bound later"),
    );
    while boottime_ns() < until {
        assert_eq!(guard.value().poll().unwrap(), State::Preparing);
        peer.pump();
        if peer
            .rows
            .iter()
            .any(|row| row["event"] == "actual_registry_request")
        {
            break;
        }
        tick();
    }
    assert!(
        peer.rows
            .iter()
            .any(|row| row["event"] == "actual_registry_request")
    );
    guard.value().stop().unwrap();
    guard.reap();
}
