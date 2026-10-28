//! Actual actor eligibility: root-admin IPC, real storage and original Source Observer.
//! Authored sender and raw hook trace only; no native compositor or sd_notify claim.
//! No original compositor, GUI, audio, ARM or hardware qualification.
use pokelinux_lifecycle::{boottime_ns, ipc};
use rustix::{
    fd::OwnedFd,
    process::{Pid, PidfdFlags, Signal},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read},
    os::unix::{fs::PermissionsExt, net::UnixDatagram},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPoll {
    pid: u32,
    observed_at_boottime_ns: u64,
    source_until: Option<u64>,
    stopping: bool,
}
static NEXT: AtomicU32 = AtomicU32::new(0);
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
struct Case {
    root: PathBuf,
    child: Child,
    until: u64,
    source_until: u64,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    out_eof: bool,
    err_eof: bool,
    status: Option<ExitStatus>,
    closed: bool,
    stopped: bool,
    trace: UnixDatagram,
    polls: Vec<RawPoll>,
    trace_limit: usize,
}
impl Case {
    fn new(mode: &str) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from(
            std::env::var_os("STARTUP_STATUS_TEST_ROOT").expect("isolated root test directory"),
        )
        .join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        for name in ["runtime", "policy"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(
                root.join(name),
                fs::Permissions::from_mode(if name == "policy" { 0o700 } else { 0o755 }),
            )
            .unwrap();
        }
        fs::write(root.join("profile.json"),br#"{"apps":[{"app":"org.omarchy.Sounds","uid":11042,"executable":"/usr/bin/sleep","args":["30"]}]}"#).unwrap();
        let start = boottime_ns();
        let until = start + 8_000_000_000;
        let source_until = start + 5_000_000_000;
        let (trace, trace_sender) = UnixDatagram::pair().unwrap();
        trace.set_nonblocking(true).unwrap();
        trace_sender.set_nonblocking(true).unwrap();
        let trace_sender: OwnedFd = trace_sender.into();
        let child = Command::new(
            std::env::var_os("STARTUP_STATUS_ACTOR").expect("explicit built actor helper"),
        )
        .arg(mode)
        .arg(&root)
        .arg(source_until.to_string())
        .arg(until.to_string())
        .stdin(Stdio::from(trace_sender))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
        // Retain custody before any fallible pipe/descriptor setup.
        let mut c = Self {
            root,
            child,
            until,
            source_until,
            stdout: vec![],
            stderr: vec![],
            out_eof: false,
            err_eof: false,
            status: None,
            closed: false,
            stopped: false,
            trace,
            polls: vec![],
            // This case observes the entire original five-second startup
            // budget, rather than a short immediate-error window. Retain every
            // hook record within a fixed, sub-megabyte typed-record bound.
            trace_limit: if mode == "gated_stale" { 16_384 } else { 2048 },
        };
        for fd in [
            c.child.stdout.as_ref().unwrap() as &dyn std::os::fd::AsFd,
            c.child.stderr.as_ref().unwrap() as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd).unwrap();
            rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        }
        loop {
            c.pump();
            if c.root.join("runtime/admin.sock").exists() {
                break;
            }
            assert!(boottime_ns() < start + 2_000_000_000, "admin setup cutoff");
            assert!(
                c.child.try_wait().unwrap().is_none(),
                "actor setup exit: {:?}",
                c.stderr
            );
            tick();
        }
        c
    }
    fn pump(&mut self) {
        loop {
            let mut packet = [0; 257];
            match self.trace.recv(&mut packet) {
                Ok(n) => {
                    assert!(n > 0 && n <= 256, "bounded raw datagram");
                    let raw: RawPoll = serde_json::from_slice(&packet[..n]).unwrap();
                    assert_eq!(raw.pid, self.child.id());
                    assert!(raw.observed_at_boottime_ns <= boottime_ns());
                    if let Some(previous) = self.polls.last() {
                        assert!(previous.observed_at_boottime_ns <= raw.observed_at_boottime_ns);
                    }
                    self.polls.push(raw);
                    assert!(
                        self.polls.len() <= self.trace_limit,
                        "bounded raw actor trace"
                    );
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("raw trace: {e}"),
            }
        }
        fn read(stream: &mut impl Read, bytes: &mut Vec<u8>, eof: &mut bool) {
            let mut data = [0; 2048];
            loop {
                match stream.read(&mut data) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        bytes.extend_from_slice(&data[..n]);
                        assert!(bytes.len() <= 16384)
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("pipe: {e}"),
                }
            }
        }
        read(
            self.child.stdout.as_mut().unwrap(),
            &mut self.stdout,
            &mut self.out_eof,
        );
        read(
            self.child.stderr.as_mut().unwrap(),
            &mut self.stderr,
            &mut self.err_eof,
        );
    }
    fn call(&mut self, request: Value) -> Value {
        assert!(boottime_ns() < self.until - 1_000_000_000);
        self.pump();
        let fd =
            ipc::connect(&self.root.join("runtime/admin.sock")).expect("actual root admin connect");
        let peer = rustix::net::sockopt::socket_peercred(&fd).unwrap();
        assert_eq!(peer.pid.as_raw_nonzero().get() as u32, self.child.id());
        assert_eq!(peer.uid.as_raw(), 0);
        let reply = ipc::call(fd, &request).expect("actual root-admin IPC");
        assert!(boottime_ns() < self.until);
        reply
    }
    fn snapshot(&mut self) -> Value {
        let v = self.call(json!({"op":"snapshot"}));
        assert_eq!(v["result"], "snapshot");
        v
    }
    fn ready(&mut self) -> Value {
        loop {
            let v = self.snapshot();
            if v["ready"] == true {
                return v;
            }
            assert!(
                boottime_ns() < self.source_until - 1_000_000_000,
                "actual storage ready setup"
            );
            tick();
        }
    }
    fn allow_source(&self) {
        fs::write(self.root.join("allow-source"), b"authored-source-only").unwrap();
    }
    fn observed_locked(&mut self) -> Value {
        loop {
            let v = self.ready();
            if source(&v)["state"] == "observed" {
                assert_eq!(v["locked"], true, "locked accepted Source is eligible");
                assert!(source(&v)["last_accepted"]["sequence"].as_u64().unwrap() > 0);
                return v;
            }
            assert!(boottime_ns() < self.source_until - 1_000_000_000);
            tick();
        }
    }
    fn failed(&mut self) -> Value {
        loop {
            let v = self.snapshot();
            if source(&v)["state"] == "failed" {
                return v;
            }
            assert!(boottime_ns() < self.source_until - 1_000_000_000);
            tick();
        }
    }
    fn raw_window(&mut self, duration: u64) {
        let until = boottime_ns() + duration;
        assert!(until < self.source_until);
        while boottime_ns() < until {
            self.pump();
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "actual actor remains alive"
            );
            tick();
        }
        self.pump();
    }
    fn assert_no_publication(&mut self) {
        self.pump();
        assert!(!self.polls.is_empty(), "actual hook was called");
        assert!(self.polls.iter().all(|r| r.source_until.is_none()));
    }
    fn stop(&mut self) -> u64 {
        assert!(!self.stopped);
        let reply = self.call(json!({"op":"stop"}));
        assert_eq!(reply["result"], "ok");
        self.stopped = true;
        boottime_ns()
    }
    fn no_later_publication(&mut self, since: u64) {
        self.pump();
        assert!(
            self.polls
                .iter()
                .filter(|r| r.observed_at_boottime_ns >= since)
                .all(|r| r.source_until.is_none()),
            "terminal actor cannot expose Source eligibility"
        );
    }
    fn source(&mut self) -> OwnedFd {
        loop {
            self.pump();
            let text = String::from_utf8_lossy(&self.stdout);
            if let Some(pid) = text.lines().find_map(|l| {
                l.strip_prefix("SOURCE_STATUS_ACTUAL_CHILD pid=")
                    .and_then(|v| v.parse::<i32>().ok())
            }) {
                return rustix::process::pidfd_open(
                    Pid::from_raw(pid).unwrap(),
                    PidfdFlags::NONBLOCK,
                )
                .unwrap();
            }
            assert!(boottime_ns() < self.source_until);
            tick();
        }
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        if !self.stopped {
            self.stop();
        }
        while self.status.is_none() || !self.out_eof || !self.err_eof {
            self.pump();
            if self.status.is_none() {
                self.status = self.child.try_wait().unwrap();
            }
            assert!(
                boottime_ns() < self.until,
                "actual actor wait/PIPE EOF cutoff"
            );
            tick();
        }
        self.pump();
        assert!(self.status.unwrap().success(), "actor {:?}", self.stderr);
        assert!(
            self.polls
                .iter()
                .filter(|r| r.stopping)
                .all(|r| r.source_until.is_none())
        );
        assert!(
            String::from_utf8_lossy(&self.stdout).contains("SOURCE_STATUS_ACTUAL_WAIT"),
            "actual Source child wait receipt"
        );
        println!(
            "STARTUP_STATUS_ACTOR_WAIT pid={} status={:?} observed_at_boottime_ns={} original_until_ns={} stdout_EOF={} stderr_EOF={}\n{}",
            self.child.id(),
            self.status,
            boottime_ns(),
            self.until,
            self.out_eof,
            self.err_eof,
            String::from_utf8_lossy(&self.stdout)
        );
        self.closed = true;
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.closed {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.close()));
            if !self.closed {
                let _ = self.child.kill();
                while self.child.try_wait().ok().flatten().is_none() {
                    if boottime_ns() >= self.until {
                        std::process::abort();
                    }
                    self.pump();
                    tick();
                }
                self.pump();
                assert!(
                    self.out_eof && self.err_eof,
                    "assertion cleanup needs actual PIPE EOF"
                );
            }
        }
    }
}
fn source(v: &Value) -> &Value {
    assert!(v["source"].is_object(), "MISSING_OBSERVED_SOURCE_STATUS");
    &v["source"]
}
fn failure(v: &Value, kind: &str, before: u64, after: u64) {
    let s = source(v);
    assert_eq!(s["state"], "failed");
    let f = &s["first_failure"];
    assert_eq!(f["category"], "provider_error");
    assert_eq!(f["error_kind"], kind);
    let time = f["observed_at_boottime_ns"].as_u64().unwrap();
    assert!(before <= time && time <= after);
}

fn awaiting(c: &mut Case) {
    let initial = c.ready();
    assert_eq!(initial["locked"], true);
    assert_eq!(source(&initial)["state"], "awaiting_first");
    assert!(source(&initial)["last_accepted"].is_null());
    assert!(source(&initial)["first_failure"].is_null());
    c.raw_window(40_000_000);
    c.assert_no_publication();
}
#[test]
#[ignore = "isolated root actor/storage, authored UID1000 Source and actual IPC"]
fn policy_ready_without_source_is_not_startup_eligible() {
    let mut c = Case::new("gated_locked");
    awaiting(&mut c);
    c.close();
    c.assert_no_publication();
}
#[test]
#[ignore = "isolated root actor/storage, authored UID1000 Source and actual IPC"]
fn rejected_first_source_never_becomes_startup_eligible() {
    let mut c = Case::new("gated_malformed");
    awaiting(&mut c);
    let before = boottime_ns();
    c.allow_source();
    let failed = c.failed();
    failure(&failed, "InvalidData", before, boottime_ns());
    assert!(source(&failed)["last_accepted"].is_null());
    c.raw_window(40_000_000);
    c.close();
    c.assert_no_publication();
}
#[test]
#[ignore = "isolated root actor/storage, authored UID1000 Source and actual IPC"]
fn stale_first_source_never_becomes_startup_eligible() {
    let mut c = Case::new("gated_stale");
    awaiting(&mut c);
    c.allow_source();
    let failed = loop {
        let state = c.snapshot();
        c.assert_no_publication();
        if source(&state)["state"] == "failed" {
            break state;
        }
        assert_eq!(source(&state)["state"], "awaiting_first");
        assert!(source(&state)["last_accepted"].is_null());
        assert!(source(&state)["first_failure"].is_null());
        // Observe the original startup deadline, retaining the original later
        // closure budget; no new allowance follows provisional timing expiry.
        assert!(boottime_ns() < c.until - 1_000_000_000);
        tick();
    };
    failure(&failed, "TimedOut", c.source_until, boottime_ns());
    assert!(source(&failed)["last_accepted"].is_null());
    assert!(source(&failed)["first_failure"]["observer_rejection"].is_null());
    c.close();
    c.assert_no_publication();
}
#[test]
#[ignore = "isolated root actor/storage, authored UID1000 Source and actual IPC"]
fn accepted_locked_source_is_eligible_then_stall_failure_is_terminal() {
    let mut c = Case::new("gated_locked");
    awaiting(&mut c);
    c.allow_source();
    loop {
        let accepted = c.observed_locked();
        let sampled = source(&accepted)["last_accepted"]["sampled_at_ns"]
            .as_u64()
            .unwrap();
        let exact_until = sampled
            .checked_add(pokelinux_session::SAMPLE_BUDGET_NS)
            .unwrap();
        c.pump();
        if c.polls.iter().any(|r| r.source_until == Some(exact_until)) {
            break;
        }
        assert!(boottime_ns() < c.source_until - 1_000_000_000);
        tick();
    }
    for raw in c.polls.iter().filter(|r| r.source_until.is_some()) {
        let until = raw.source_until.unwrap();
        assert!(!raw.stopping);
        assert!(until > 0, "actual copied Source cutoff");
        assert!(
            until
                <= raw
                    .observed_at_boottime_ns
                    .saturating_add(pokelinux_session::SAMPLE_BUDGET_NS)
        );
    }
    let source_pidfd = c.source();
    let before = boottime_ns();
    rustix::process::pidfd_send_signal(&source_pidfd, Signal::STOP).unwrap();
    let failed = c.failed();
    failure(&failed, "TimedOut", before, boottime_ns());
    let failed_observed = boottime_ns();
    c.raw_window(40_000_000);
    c.no_later_publication(failed_observed);
    let reply = c.snapshot();
    assert_eq!(source(&reply), source(&failed));
    c.pump();
    assert!(
        c.polls
            .iter()
            .any(|r| r.observed_at_boottime_ns >= failed_observed),
        "actual post-failure actor hook call"
    );
    let stop_ack = c.stop();
    c.close();
    c.no_later_publication(failed_observed);
    c.no_later_publication(stop_ack);
}
#[test]
#[ignore = "isolated root actor/storage, authored UID1000 Source and actual IPC"]
fn acknowledged_stop_before_source_cannot_later_publish() {
    let mut c = Case::new("gated_locked");
    awaiting(&mut c);
    let stop_ack = c.stop();
    c.allow_source();
    c.close();
    c.no_later_publication(stop_ack);
    c.assert_no_publication();
}
