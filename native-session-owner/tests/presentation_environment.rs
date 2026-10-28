//! Actual subprocess controls. Run only in an isolated root container, serially.
#![cfg(feature = "isolated-fixture")]
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio},
    time::{Duration, Instant},
};
const ROOT: &str = "/run/user/1000";
const BODY: &str = "WAYLAND_DISPLAY=wayland-1\nHYPRLAND_INSTANCE_SIGNATURE=fixture_compositor_1\n";

fn helper(command: &str) -> Command {
    let mut process =
        Command::new(std::env::var_os("PRESENTATION_HELPER").expect("installed helper"));
    process
        .arg(command)
        .env_clear()
        .env("WAYLAND_DISPLAY", "wayland-1")
        .env("HYPRLAND_INSTANCE_SIGNATURE", "fixture_compositor_1");
    if command != "clear" {
        process.uid(1000).gid(1000);
    }
    process
}
fn run(command: &str) -> Output {
    helper(command).output().unwrap()
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
struct Case;
impl Case {
    fn new() -> Self {
        assert_eq!(rustix::process::geteuid().as_raw(), 0, "isolated root only");
        fs::create_dir_all("/run/user").unwrap();
        assert!(
            !Path::new(ROOT).exists(),
            "never reuse another case runtime"
        );
        fs::create_dir(ROOT).unwrap();
        fs::set_permissions(ROOT, fs::Permissions::from_mode(0o700)).unwrap();
        rustix::fs::chown(
            ROOT,
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        Self
    }
}
fn reclaim(path: &Path) {
    let metadata = fs::symlink_metadata(path).unwrap();
    if metadata.is_dir() {
        rustix::fs::chown(
            path,
            Some(rustix::process::Uid::ROOT),
            Some(rustix::process::Gid::ROOT),
        )
        .unwrap();
        for entry in fs::read_dir(path).unwrap() {
            reclaim(&entry.unwrap().path());
        }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        reclaim(Path::new(ROOT));
        fs::remove_dir_all(ROOT).unwrap();
    }
}
struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}
impl Peer {
    fn new(sockets: &[&str]) -> Self {
        let mut child = Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/support/presentation-peer.py"
            ))
            .args(sockets)
            .uid(1000)
            .gid(1000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut peer = Self {
            child,
            input,
            output,
        };
        assert_eq!(peer.line(), "READY");
        peer
    }
    fn line(&mut self) -> String {
        let mut line = String::new();
        assert_ne!(self.output.read_line(&mut line).unwrap(), 0);
        line.trim_end().to_owned()
    }
    fn request(&mut self, request: Value) -> String {
        writeln!(self.input.as_mut().unwrap(), "{request}").unwrap();
        self.input.as_mut().unwrap().flush().unwrap();
        self.line()
    }
    fn write(&mut self, body: &str, mode: u32) {
        assert_eq!(
            self.request(json!({"op":"write","body":body,"mode":mode})),
            "OK"
        );
    }
    fn read(&mut self) -> Value {
        serde_json::from_str(&self.request(json!({"op":"read"}))).unwrap()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        drop(self.input.take());
        let cutoff = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= cutoff {
                self.child.kill().unwrap();
                self.child.wait().unwrap();
                panic!("fixture exceeded original cutoff");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let mut tail = Vec::new();
        self.output.read_to_end(&mut tail).unwrap();
        assert!(status.success(), "fixture actual wait {status}");
        eprintln!("fixture actual_wait={status} stdout_EOF=true");
    }
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_publish_and_wait_same_live_peer() {
    let _case = Case::new();
    let mut peer = Peer::new(&["wayland", "control"]);
    success(run("publish"));
    assert_eq!(peer.read(), BODY);
    success(run("wait"));
    success(run("clear"));
    assert_eq!(peer.read(), Value::Null);
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_publish_rejects_missing_and_injected_values() {
    let _case = Case::new();
    let mut peer = Peer::new(&["wayland", "control"]);
    for bad in ["", "../other", "signature\nEVIL=1", "$(id)", "quoted value"] {
        assert!(
            !helper("publish")
                .env("HYPRLAND_INSTANCE_SIGNATURE", bad)
                .output()
                .unwrap()
                .status
                .success()
        );
        assert_eq!(peer.read(), Value::Null);
    }
    assert!(
        !helper("publish")
            .env_remove("HYPRLAND_INSTANCE_SIGNATURE")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !helper("publish")
            .env("WAYLAND_DISPLAY", "wayland-2")
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_stale_file_requires_clear_before_new_epoch() {
    let _case = Case::new();
    let mut peer = Peer::new(&["wayland", "control"]);
    peer.write("old epoch\n", 0o600);
    assert!(!run("publish").status.success());
    assert_eq!(peer.read(), "old epoch\n");
    success(run("clear"));
    success(run("publish"));
    success(run("wait"));
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_symlink_is_rejected_without_following_or_deleting_it() {
    let _case = Case::new();
    let mut peer = Peer::new(&[]);
    assert_eq!(peer.request(json!({"op":"symlink"})), "OK");
    assert!(!run("publish").status.success());
    assert!(!run("wait").status.success());
    assert!(!run("clear").status.success());
    assert_eq!(peer.request(json!({"op":"sentinel"})), "\"keep\"");
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_invalid_file_shape_is_strict() {
    let _case = Case::new();
    let mut peer = Peer::new(&[]);
    for body in [
        "",
        "WAYLAND_DISPLAY=wayland-1\n",
        "WAYLAND_DISPLAY=wayland-1\nEVIL=value\n",
        "WAYLAND_DISPLAY=wayland-1\nHYPRLAND_INSTANCE_SIGNATURE=../escape\n",
        "WAYLAND_DISPLAY=wayland-1\nHYPRLAND_INSTANCE_SIGNATURE=fixture_compositor_1\nEVIL=1\n",
    ] {
        peer.write(body, 0o600);
        assert!(!run("wait").status.success());
        assert_eq!(peer.request(json!({"op":"remove"})), "OK");
    }
    peer.write(BODY, 0o644);
    assert!(!run("wait").status.success());
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_hardlink_is_rejected_by_wait_and_clear() {
    let _case = Case::new();
    let mut peer = Peer::new(&[]);
    peer.write(BODY, 0o600);
    assert_eq!(peer.request(json!({"op":"hardlink"})), "OK");
    assert!(!run("wait").status.success());
    assert!(!run("clear").status.success());
    assert_eq!(peer.read(), BODY);
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_two_different_live_peers_are_rejected() {
    let _case = Case::new();
    let mut wayland = Peer::new(&["wayland"]);
    let _control = Peer::new(&["control"]);
    assert!(!run("publish").status.success());
    wayland.write(BODY, 0o600);
    assert!(!run("wait").status.success());
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_dead_peer_is_rejected() {
    let _case = Case::new();
    {
        let _peer = Peer::new(&["wayland", "control"]);
        success(run("publish"));
    }
    assert!(!run("wait").status.success());
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_absence_expires_original_five_second_budget() {
    let _case = Case::new();
    let _peer = Peer::new(&[]);
    let started = Instant::now();
    let output = run("wait");
    let elapsed = started.elapsed();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("timed out"));
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed < Duration::from_secs(6),
        "{elapsed:?}"
    );
}
#[test]
#[ignore = "isolated fixed UID1000 runtime; explicit serial run only"]
fn contract_wait_accepts_publication_inside_original_budget() {
    let _case = Case::new();
    let _peer = Peer::new(&["wayland", "control"]);
    let mut waiter = helper("wait")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    success(run("publish"));
    let cutoff = Instant::now() + Duration::from_secs(2);
    loop {
        if waiter.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= cutoff {
            waiter.kill().unwrap();
            waiter.wait().unwrap();
            panic!("waiter failed to close");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    success(waiter.wait_with_output().unwrap());
}

#[test]
#[ignore = "isolated fixed UID1000 runtime and native fixed-path shell fixture"]
fn contract_launch_exec_keeps_pid_and_allowlisted_environment() {
    let _case = Case::new();
    let _peer = Peer::new(&["wayland", "control"]);
    success(run("publish"));
    let child = helper("launch")
        .env("WAYLAND_DISPLAY", "wrong-display")
        .env("HYPRLAND_INSTANCE_SIGNATURE", "wrong-signature")
        .env("PRESENTATION_TEST_KEEP", "keep-static")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let original_pid = child.id();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "launch actual_exit={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["pid"], original_pid);
    assert_eq!(actual["exe"], "/usr/bin/quickshell");
    assert_eq!(
        actual["args"],
        json!([
            "/usr/bin/quickshell",
            "-n",
            "-p",
            "/usr/share/pokelinux/omarchy/shell"
        ])
    );
    assert_eq!(actual["display"], "wayland-1");
    assert_eq!(actual["signature"], "fixture_compositor_1");
    assert_eq!(actual["keep"], "keep-static");
    assert_eq!(actual["unexpected"], false);
}
