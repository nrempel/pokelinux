//! Actual root-admin IPC and real authored UID1000 Source/Observer/child controls.
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
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
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
}
impl Case {
    fn new(mode: &str) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from(
            std::env::var_os("SOURCE_STATUS_TEST_ROOT").expect("isolated root test directory"),
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
        let child = Command::new(
            std::env::var_os("SOURCE_STATUS_ACTOR").expect("explicit built actor helper"),
        )
        .arg(mode)
        .arg(&root)
        .arg(source_until.to_string())
        .arg(until.to_string())
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
    fn observed(&mut self) -> Value {
        loop {
            let v = self.ready();
            if v["locked"] == false {
                return v;
            }
            assert!(
                boottime_ns() < self.source_until - 1_000_000_000,
                "actual accepted Source setup"
            );
            tick();
        }
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
    fn wait_locked(&mut self) -> (Value, u64, u64) {
        let before = boottime_ns();
        loop {
            let v = self.snapshot();
            if v["locked"] == true {
                return (v, before, boottime_ns());
            }
            assert!(boottime_ns() < self.until - 2_000_000_000);
            tick();
        }
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        let reply = self.call(json!({"op":"stop"}));
        assert_eq!(reply["result"], "ok");
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
        assert!(self.status.unwrap().success(), "actor {:?}", self.stderr);
        println!(
            "SOURCE_STATUS_ACTOR_WAIT pid={} status={:?} observed_at_boottime_ns={} original_until_ns={} stdout_EOF={} stderr_EOF={}\n{}",
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
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn snapshot_without_source_reports_not_configured() {
    let mut c = Case::new("none");
    let v = c.ready();
    c.close();
    let s = source(&v);
    assert_eq!(s["state"], "not_configured");
    assert!(s["last_accepted"].is_null() && s["first_failure"].is_null());
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn snapshot_awaiting_first_then_original_timeout_retains_first_failure() {
    let mut c = Case::new("quiet");
    let initial = c.ready();
    assert_eq!(initial["locked"], true);
    let before = boottime_ns();
    while boottime_ns() < c.source_until + 30_000_000 {
        c.pump();
        tick();
    }
    let failed = c.snapshot();
    let after = boottime_ns();
    let repeated = c.snapshot();
    c.close();
    let s = source(&initial);
    assert_eq!(s["state"], "awaiting_first");
    assert!(s["last_accepted"].is_null() && s["first_failure"].is_null());
    failure(&failed, "TimedOut", before, after);
    assert!(source(&failed)["last_accepted"].is_null());
    assert_eq!(source(&failed), source(&repeated));
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn snapshot_accepted_sample_then_real_stall_retains_first_failure() {
    let mut c = Case::new("valid");
    let initial = c.observed();
    assert_eq!(
        initial["locked"], false,
        "real valid Source must reach authority"
    );
    let fd = c.source();
    let before = boottime_ns();
    rustix::process::pidfd_send_signal(&fd, Signal::STOP).unwrap();
    let (failed, _, after) = c.wait_locked();
    let repeated = c.snapshot();
    c.close();
    let s = source(&initial);
    assert_eq!(s["state"], "observed");
    assert!(s["last_accepted"]["sequence"].as_u64().unwrap() > 0);
    failure(&failed, "TimedOut", before, after);
    assert_eq!(source(&failed), source(&repeated));
    assert!(
        source(&failed)["last_accepted"]["sequence"]
            .as_u64()
            .unwrap()
            >= s["last_accepted"]["sequence"].as_u64().unwrap()
    );
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn snapshot_actual_source_child_exit_reports_disconnect() {
    let mut c = Case::new("valid");
    let initial = c.observed();
    assert_eq!(initial["locked"], false);
    let fd = c.source();
    let before = boottime_ns();
    rustix::process::pidfd_send_signal(&fd, Signal::KILL).unwrap();
    let (failed, _, after) = c.wait_locked();
    let repeated = c.snapshot();
    c.close();
    assert_eq!(source(&initial)["state"], "observed");
    let kind = source(&failed)["first_failure"]["error_kind"]
        .as_str()
        .unwrap();
    assert!(
        matches!(kind, "NotConnected" | "UnexpectedEof"),
        "actual child disconnect kind: {kind}"
    );
    failure(&failed, kind, before, after);
    assert_eq!(source(&failed), source(&repeated));
    assert!(
        source(&failed)["last_accepted"]["sequence"]
            .as_u64()
            .unwrap()
            > 0
    );
}

struct ResumeActor(OwnedFd);
impl Drop for ResumeActor {
    fn drop(&mut self) {
        // Restore the owned actor on every assertion path before Case cleanup.
        let _ = rustix::process::pidfd_send_signal(&self.0, Signal::CONT);
    }
}
// These assertions compile against the pre-diagnostic production Snapshot API.
// Each case completes real failure controls and owned child cleanup before RED.
fn actual_rejection(command: &str, stage: &str, reason: &str) {
    let mut c = Case::new("diagnostic");
    fs::set_permissions(&c.root, fs::Permissions::from_mode(0o777)).unwrap();
    let initial = c.observed();
    let actor = ResumeActor(
        rustix::process::pidfd_open(
            Pid::from_raw(c.child.id() as i32).unwrap(),
            PidfdFlags::NONBLOCK,
        )
        .unwrap(),
    );
    rustix::process::pidfd_send_signal(&actor.0, Signal::STOP).unwrap();
    // Wait until the owned actor is actually stopped before queueing wire inputs.
    loop {
        let status = fs::read_to_string(format!("/proc/{}/status", c.child.id())).unwrap();
        if status
            .lines()
            .any(|line| line.starts_with("State:") && line.contains("T (stopped)"))
        {
            break;
        }
        assert!(boottime_ns() < c.until - 3_000_000_000);
        tick();
    }
    let before = boottime_ns();
    fs::write(c.root.join("reject"), command).unwrap();
    while !c.root.join("rejected-input-ready").exists() {
        assert!(boottime_ns() < c.until - 3_000_000_000);
        c.pump();
        tick();
    }
    rustix::process::pidfd_send_signal(&actor.0, Signal::CONT).unwrap();
    let (failed, _, after) = c.wait_locked();
    let repeated = c.snapshot();
    c.close();
    failure(&failed, "InvalidData", before, after);
    assert_eq!(source(&failed), source(&repeated));
    assert!(
        source(&initial)["last_accepted"]["sequence"]
            .as_u64()
            .unwrap()
            > 0
    );
    let rejection = &source(&failed)["first_failure"]["observer_rejection"];
    assert!(
        rejection.is_object(),
        "MISSING_FIRST_OBSERVER_REJECTION: {command}"
    );
    assert_eq!(rejection["stage"], stage);
    assert_eq!(rejection["reason"], reason);
    let validated = rejection["validated_at_boottime_ns"].as_u64().unwrap();
    assert!(before <= validated && validated <= after);
    let ordinal = rejection["drain_ordinal"].as_u64().unwrap();
    assert!((1..=17).contains(&ordinal));
    let previous = &rejection["previous_accepted"];
    assert!(
        previous["sequence"].as_u64().unwrap()
            >= source(&initial)["last_accepted"]["sequence"]
                .as_u64()
                .unwrap()
    );
    let sample = rejection["sampled_at_ns"].as_u64();
    match command {
        "queued_expired" => {
            let sample = sample.unwrap();
            assert!(sample + pokelinux_session::SAMPLE_BUDGET_NS <= validated);
            assert!(sample >= previous["sampled_at_ns"].as_u64().unwrap());
            assert!(
                sample
                    <= previous["sampled_at_ns"].as_u64().unwrap()
                        + pokelinux_session::SAMPLE_BUDGET_NS
            );
            assert_eq!(
                rejection["sequence"].as_u64().unwrap(),
                previous["sequence"].as_u64().unwrap() + 1
            );
        }
        "sample_gap" => {
            let sample = sample.unwrap();
            assert!(
                sample
                    > previous["sampled_at_ns"].as_u64().unwrap()
                        + pokelinux_session::SAMPLE_BUDGET_NS
            );
            assert!(
                sample <= validated && validated < sample + pokelinux_session::SAMPLE_BUDGET_NS
            );
        }
        "sequence" => assert!(
            rejection["sequence"].as_u64().unwrap() > previous["sequence"].as_u64().unwrap() + 1
        ),
        "eof" | "zero_bytes" => assert_eq!(rejection["receive"]["received_bytes"], 0),
        "truncated" => assert_eq!(rejection["receive"]["payload_truncated"], true),
        "malformed" => assert!(sample.is_none() && rejection["sequence"].is_null()),
        "drain_limit" => assert_eq!(ordinal, 17),
        _ => unreachable!(),
    }
    // Bounded structure has no packet, arbitrary parser/OS text or peer identity.
    let keys: std::collections::BTreeSet<_> = rejection
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "stage",
            "reason",
            "drain_ordinal",
            "sequence",
            "sampled_at_ns",
            "previous_accepted",
            "validated_at_boottime_ns",
            "receive"
        ]
        .into_iter()
        .collect()
    );
    let text = rejection.to_string();
    for forbidden in [
        "private-frame-marker",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "focus",
        "locker",
        "uid",
        "pid",
        "source",
    ] {
        assert!(
            !text.contains(forbidden),
            "sensitive diagnostic: {forbidden}"
        );
    }
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_queued_expired_frame_is_distinct_from_sample_gap() {
    actual_rejection("queued_expired", "validation", "sample_expired");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_sampling_gap_is_distinct_from_queued_expiry() {
    actual_rejection("sample_gap", "validation", "sample_gap");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_sequence_gap_retains_parsed_sample() {
    actual_rejection("sequence", "validation", "sequence");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_eof_is_distinct_from_malformed_frame() {
    actual_rejection("eof", "receive", "receive_zero_bytes");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_truncation_retains_bounded_receive_flags() {
    actual_rejection("truncated", "receive", "payload_truncated");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_malformed_frame_keeps_no_raw_data() {
    actual_rejection("malformed", "decode", "malformed_frame");
}
#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_drain_limit_retains_original_ordinal() {
    actual_rejection("drain_limit", "drain", "drain_limit");
}

#[test]
#[ignore = "isolated root, UID transitions, built actor/Source helpers"]
fn rejection_actual_zero_length_packet_reports_only_zero_bytes() {
    actual_rejection("zero_bytes", "receive", "receive_zero_bytes");
}
