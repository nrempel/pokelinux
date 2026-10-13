#![cfg(target_os = "linux")]
use pokelinux_messaging::receipt::{
    AckState, BackendScope, IncomingKind, Outcome, ReceiptOwner, Slot, TestAckRequest, TestAckSink,
    TestDispatchError, TestSignal,
};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("pokelinux-incoming-{}", uuid::Uuid::new_v4())))
    }
    fn owner(&self) -> ReceiptOwner {
        ReceiptOwner::open(&self.0).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Sink {
    path: PathBuf,
    requests: Vec<TestAckRequest>,
    raw: Vec<u8>,
    fail: bool,
}
impl Sink {
    fn new(directory: &Directory, raw: &[u8]) -> Self {
        Self {
            path: directory.0.join("receipts.sqlite3"),
            requests: Vec::new(),
            raw: raw.to_vec(),
            fail: false,
        }
    }
}
impl TestAckSink for Sink {
    fn dispatch(&mut self, request: TestAckRequest) -> std::result::Result<(), TestDispatchError> {
        let db = rusqlite::Connection::open(&self.path).unwrap();
        let (raw, state): (Vec<u8>, String) = db
            .query_row(
                "SELECT raw,ack FROM receipts WHERE id=?1",
                [request.receipt.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            raw, self.raw,
            "raw arrival must be committed before exposing ACK dispatch"
        );
        assert_eq!(
            state, "uncertain",
            "ACK uncertainty must be committed before injected no-radio dispatch"
        );
        self.requests.push(request);
        if self.fail {
            Err(TestDispatchError)
        } else {
            Ok(())
        }
    }
}
#[test]
fn sqlite_raw_and_uncertainty_precede_injected_ack_dispatch() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let raw = b"synthetic raw incoming\0\xff";
    let id = owner.admit(source, IncomingKind::Deliver, raw).unwrap();
    assert_eq!(owner.lookup(id).unwrap().raw(), raw);
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Stored);
    let permit = owner
        .claim_test_ack(source, id)
        .expect("durable one-shot ACK permit ownership missing");
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    let attempt = permit.attempt();
    let mut sink = Sink::new(&directory, raw);
    assert_eq!(
        owner.dispatch_test_ack(permit, &mut sink).unwrap(),
        Outcome::Pending
    );
    assert_eq!(sink.requests.len(), 1);
    assert_eq!(sink.requests[0].source.slot(), Slot(1));
    assert_eq!(sink.requests[0].receipt, id);
    for signal in [TestSignal::IpcReturned, TestSignal::RequestAcknowledged] {
        assert_eq!(
            owner.observe_test_ack(source, attempt, signal).unwrap(),
            Outcome::Pending
        );
        assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    }
    assert_eq!(
        owner
            .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::TestBackendAccepted
    );
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::TestBackendAccepted);
    assert_eq!(
        owner
            .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert!(owner.claim_test_ack(source, id).is_err());
}
#[test]
fn equal_bytes_are_distinct_local_occurrences_and_premature_second_arrival_revokes_the_window() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let first = owner.admit(source, IncomingKind::Deliver, b"same").unwrap();
    let second = owner
        .admit(source, IncomingKind::StatusReport, b"same")
        .unwrap();
    let third = owner.admit(source, IncomingKind::Deliver, b"same").unwrap();
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_ne!(first, third);
    for id in [first, second, third] {
        assert_eq!(owner.lookup(id).unwrap().raw(), b"same");
        assert!(!owner.eligible(source, id));
        assert!(owner.claim_test_ack(source, id).is_err());
    }
    assert_eq!(owner.snapshot(3).unwrap().total, 3);
}
#[test]
fn unqualified_backend_and_denied_permission_preserve_raw_without_ack_authority() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let id = owner.admit(source, IncomingKind::Deliver, b"").unwrap();
    assert!(owner.claim_test_ack(source, id).is_err());
    owner.set_test_ack_permission(true);
    assert!(owner.claim_test_ack(source, id).is_err());
    let unqualified = owner.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = owner
        .admit(unqualified, IncomingKind::Deliver, b"malformed\xff")
        .unwrap();
    assert_eq!(owner.lookup(id).unwrap().scope, BackendScope::Unqualified);
    assert_eq!(owner.lookup(id).unwrap().raw(), b"malformed\xff");
    assert!(owner.claim_test_ack(unqualified, id).is_err());
}
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires a real kernel process boundary; no ARM64 user-QEMU process claim"
)]
fn receipt_crash_child() {
    let Some(path) = std::env::var_os("POKELINUX_RECEIPT_CRASH_TEST") else {
        return;
    };
    let path = PathBuf::from(path);
    let mut owner = ReceiptOwner::open(&path).unwrap();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let id = owner
        .admit(source, IncomingKind::StatusReport, b"synthetic status raw")
        .unwrap();
    fs::write(path.join("test-raw-committed"), id.to_string()).unwrap();
    let _held = owner
        .claim_test_ack(source, id)
        .expect("child durable ACK claim missing");
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    fs::write(path.join("test-uncertainty-committed"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires a real kernel process boundary; no ARM64 user-QEMU process claim"
)]
fn sigkill_releases_single_owner_without_replaying_claimed_ack_or_deduplicating_redelivery() {
    let directory = Directory::new();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "receipt_crash_child",
                "--nocapture",
                "--include-ignored",
            ])
            .env("POKELINUX_RECEIPT_CRASH_TEST", &directory.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let end = Instant::now() + Duration::from_secs(10);
    while !directory.0.join("test-uncertainty-committed").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "actual child exited before durable ACK claim; raw admission marker={}",
            directory.0.join("test-raw-committed").exists()
        );
        assert!(
            Instant::now() < end,
            "child failed to commit ACK uncertainty"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ReceiptOwner::open(&directory.0).is_err());
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(9));
    let mut owner = directory.owner();
    let old = owner.snapshot(1).unwrap().latest[0].id;
    assert_eq!(owner.lookup(old).unwrap().raw(), b"synthetic status raw");
    assert_eq!(owner.lookup(old).unwrap().ack, AckState::Uncertain);
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    assert!(owner.claim_test_ack(source, old).is_err());
    let redelivery = owner
        .admit(source, IncomingKind::StatusReport, b"synthetic status raw")
        .unwrap();
    assert_ne!(redelivery, old);
    assert_eq!(owner.snapshot(2).unwrap().total, 2);
    assert_eq!(owner.lookup(old).unwrap().ack, AckState::Uncertain);
}

#[test]
fn altered_admission_metadata_is_not_accepted_as_a_durable_receipt() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER alter_admission AFTER INSERT ON receipts BEGIN UPDATE receipts SET slot=7 WHERE id=NEW.id; END;").unwrap();
    assert!(
        owner.admit(source, IncomingKind::Deliver, b"raw").is_err(),
        "changed slot provenance must fail admission atomically"
    );
    assert_eq!(owner.snapshot(1).unwrap().total, 0);
}

#[test]
fn held_permits_are_rechecked_at_dispatch_after_cancel_death_replacement_permission_stop_or_new_arrival()
 {
    for cause in 0..6 {
        let directory = Directory::new();
        let mut owner = directory.owner();
        owner.set_test_ack_permission(true);
        let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
        let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
        let permit = owner.claim_test_ack(source, id).unwrap();
        let attempt = permit.attempt();
        match cause {
            0 => owner.cancel(id).unwrap(),
            1 => owner.revoke(source),
            2 => {
                owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
            }
            3 => {
                owner.set_test_ack_permission(false);
                owner.set_test_ack_permission(true);
            }
            4 => owner.stop(),
            5 => {
                owner
                    .admit(source, IncomingKind::StatusReport, b"x")
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let mut sink = Sink::new(&directory, b"x");
        assert_eq!(
            owner.dispatch_test_ack(permit, &mut sink).unwrap(),
            Outcome::Ignored,
            "cause {cause}"
        );
        assert!(sink.requests.is_empty());
        assert_eq!(
            owner
                .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
                .unwrap(),
            Outcome::Ignored
        );
        assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
        assert!(owner.claim_test_ack(source, id).is_err());
    }
}
#[test]
fn consumed_completions_cannot_reactivate_after_cancel_death_permission_replacement_or_stop() {
    for cause in 0..5 {
        let directory = Directory::new();
        let mut owner = directory.owner();
        owner.set_test_ack_permission(true);
        let source = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
        let id = owner
            .admit(source, IncomingKind::StatusReport, b"x")
            .unwrap();
        let permit = owner.claim_test_ack(source, id).unwrap();
        let attempt = permit.attempt();
        let mut sink = Sink::new(&directory, b"x");
        owner.dispatch_test_ack(permit, &mut sink).unwrap();
        match cause {
            0 => owner.cancel(id).unwrap(),
            1 => owner.revoke(source),
            2 => {
                owner.set_test_ack_permission(false);
                owner.set_test_ack_permission(true);
            }
            3 => {
                owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
            }
            4 => owner.stop(),
            _ => unreachable!(),
        }
        assert_eq!(
            owner
                .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
                .unwrap(),
            Outcome::Ignored,
            "cause {cause}"
        );
        assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    }
}
#[test]
fn source_slot_attempt_owner_and_actual_dispatch_are_all_required_for_test_completion() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let a = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let b = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let first = owner.admit(a, IncomingKind::Deliver, b"x").unwrap();
    let second = owner.admit(b, IncomingKind::StatusReport, b"x").unwrap();
    let pa = owner.claim_test_ack(a, first).unwrap();
    let pb = owner.claim_test_ack(b, second).unwrap();
    let aa = pa.attempt();
    let ab = pb.attempt();
    assert_eq!(
        owner
            .observe_test_ack(a, aa, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    let other = Directory::new();
    let mut foreign = other.owner();
    let source = foreign.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let mut sink = Sink::new(&directory, b"x");
    owner.dispatch_test_ack(pa, &mut sink).unwrap();
    owner.dispatch_test_ack(pb, &mut sink).unwrap();
    for (wrong, attempt) in [(b, aa), (a, ab), (source, aa)] {
        assert_eq!(
            owner
                .observe_test_ack(wrong, attempt, TestSignal::FinalAccepted)
                .unwrap(),
            Outcome::Ignored
        );
    }
    assert_eq!(
        owner
            .observe_test_ack(a, aa, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::TestBackendAccepted
    );
    assert_eq!(
        owner
            .observe_test_ack(b, ab, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::TestBackendAccepted
    );
    assert_eq!(owner.lookup(first).unwrap().slot, Slot(0));
    assert_eq!(owner.lookup(second).unwrap().slot, Slot(1));
}
#[test]
fn reopening_does_not_restore_stored_or_uncertain_ack_windows_and_old_permits_are_ignored() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let a = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let b = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let stored = owner.admit(a, IncomingKind::Deliver, b"x").unwrap();
    let uncertain = owner.admit(b, IncomingKind::Deliver, b"x").unwrap();
    let permit = owner.claim_test_ack(b, uncertain).unwrap();
    let attempt = permit.attempt();
    drop(owner);
    let mut next = directory.owner();
    next.set_test_ack_permission(true);
    let fresh = next.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let mut sink = Sink::new(&directory, b"x");
    assert_eq!(
        next.dispatch_test_ack(permit, &mut sink).unwrap(),
        Outcome::Ignored
    );
    assert!(sink.requests.is_empty());
    assert_eq!(
        next.observe_test_ack(b, attempt, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert!(next.claim_test_ack(fresh, stored).is_err());
    assert!(next.claim_test_ack(fresh, uncertain).is_err());
    assert_eq!(next.lookup(stored).unwrap().ack, AckState::Stored);
    assert_eq!(next.lookup(uncertain).unwrap().ack, AckState::Uncertain);
    let current = next.admit(fresh, IncomingKind::Deliver, b"x").unwrap();
    assert_ne!(current, stored);
    assert_ne!(current, uncertain);
    assert_eq!(next.snapshot(3).unwrap().total, 3);
}
#[test]
fn dropping_a_permit_or_an_uncertain_sink_result_never_refunds_ack_authority() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
    let attempt = {
        let permit = owner.claim_test_ack(source, id).unwrap();
        permit.attempt()
    };
    assert!(owner.claim_test_ack(source, id).is_err());
    assert_eq!(
        owner
            .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    let source = owner.attach(Slot(1), BackendScope::TestBackend).unwrap();
    let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
    let permit = owner.claim_test_ack(source, id).unwrap();
    let attempt = permit.attempt();
    let mut sink = Sink::new(&directory, b"x");
    sink.fail = true;
    assert_eq!(
        owner.dispatch_test_ack(permit, &mut sink).unwrap(),
        Outcome::Uncertain
    );
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
    assert_eq!(
        owner
            .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert!(owner.claim_test_ack(source, id).is_err());
}
#[test]
fn only_qualified_test_completion_opens_a_following_incoming_window() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let first = owner.admit(source, IncomingKind::Deliver, b"same").unwrap();
    let permit = owner.claim_test_ack(source, first).unwrap();
    let attempt = permit.attempt();
    let mut sink = Sink::new(&directory, b"same");
    owner.dispatch_test_ack(permit, &mut sink).unwrap();
    owner
        .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
        .unwrap();
    let second = owner
        .admit(source, IncomingKind::StatusReport, b"same")
        .unwrap();
    assert_ne!(first, second);
    assert!(owner.eligible(source, second));
    assert!(owner.claim_test_ack(source, first).is_err());
    let permit = owner.claim_test_ack(source, second).unwrap();
    assert_eq!(permit.receipt(), second);
    assert_eq!(
        owner.lookup(first).unwrap().ack,
        AckState::TestBackendAccepted
    );
}
#[test]
fn database_failures_never_emit_eligibility_or_completion_success() {
    for phase in 0..3 {
        let directory = Directory::new();
        let mut owner = directory.owner();
        owner.set_test_ack_permission(true);
        let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
        let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
        if phase == 0 {
            db.execute_batch("CREATE TRIGGER deny_raw BEFORE INSERT ON receipts BEGIN SELECT RAISE(IGNORE); END;").unwrap();
            assert!(owner.admit(source, IncomingKind::Deliver, b"x").is_err());
            assert_eq!(owner.snapshot(1).unwrap().total, 0);
            continue;
        }
        let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
        if phase == 1 {
            db.execute_batch("CREATE TRIGGER deny_claim BEFORE UPDATE OF ack ON receipts WHEN NEW.ack='uncertain' BEGIN SELECT RAISE(IGNORE); END;").unwrap();
            assert!(owner.claim_test_ack(source, id).is_err());
            assert_eq!(owner.lookup(id).unwrap().ack, AckState::Stored);
            assert!(!owner.eligible(source, id));
            continue;
        }
        let permit = owner.claim_test_ack(source, id).unwrap();
        let attempt = permit.attempt();
        let mut sink = Sink::new(&directory, b"x");
        owner.dispatch_test_ack(permit, &mut sink).unwrap();
        db.execute_batch("CREATE TRIGGER deny_complete BEFORE UPDATE OF ack ON receipts WHEN NEW.ack='test_backend_accepted' BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        assert!(
            owner
                .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
                .is_err()
        );
        assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
        assert_eq!(
            owner
                .observe_test_ack(source, attempt, TestSignal::FinalAccepted)
                .unwrap(),
            Outcome::Ignored
        );
    }
}
#[test]
fn cancellation_storage_failure_still_fences_a_held_permit() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
    let permit = owner.claim_test_ack(source, id).unwrap();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER deny_cancel BEFORE UPDATE OF cancelled ON receipts BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(owner.cancel(id).is_err());
    let mut sink = Sink::new(&directory, b"x");
    assert_eq!(
        owner.dispatch_test_ack(permit, &mut sink).unwrap(),
        Outcome::Ignored
    );
    assert!(sink.requests.is_empty());
    assert_eq!(owner.lookup(id).unwrap().ack, AckState::Uncertain);
}
#[test]
fn raw_slot_and_snapshot_bounds_precede_ack_admission() {
    use pokelinux_messaging::receipt::{MAX_RAW_BYTES, MAX_SLOTS, MAX_SNAPSHOT};
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let raw = vec![0xff; MAX_RAW_BYTES];
    let id = owner
        .admit(source, IncomingKind::StatusReport, &raw)
        .unwrap();
    assert_eq!(owner.lookup(id).unwrap().raw(), raw);
    assert!(
        owner
            .admit(source, IncomingKind::Deliver, &[0; MAX_RAW_BYTES + 1])
            .is_err()
    );
    assert_eq!(owner.snapshot(1).unwrap().total, 1);
    assert!(
        owner
            .attach(Slot(MAX_SLOTS), BackendScope::TestBackend)
            .is_err()
    );
    assert!(owner.snapshot(0).is_err());
    assert!(owner.snapshot(MAX_SNAPSHOT + 1).is_err());
}
#[test]
fn stop_and_control_snapshot_remain_responsive_while_sqlite_is_exclusively_locked() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    let id = owner.admit(source, IncomingKind::Deliver, b"x").unwrap();
    let permit = owner.claim_test_ack(source, id).unwrap();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE;").unwrap();
    let begin = Instant::now();
    owner.stop();
    let control = owner.control();
    assert!(control.stopped);
    assert!(!control.permission);
    assert_eq!(control.pending, 0);
    assert_eq!(control.active_sources, 0);
    assert!(owner.snapshot(1).is_err());
    assert!(begin.elapsed() < Duration::from_secs(2));
    let mut sink = Sink::new(&directory, b"x");
    assert_eq!(
        owner.dispatch_test_ack(permit, &mut sink).unwrap(),
        Outcome::Ignored
    );
    assert!(sink.requests.is_empty());
    db.execute_batch("ROLLBACK;").unwrap();
    let late = owner
        .admit(source, IncomingKind::Deliver, b"late after stop")
        .unwrap();
    assert_eq!(owner.lookup(late).unwrap().raw(), b"late after stop");
    assert!(!owner.eligible(source, late));
    assert!(owner.attach(Slot(0), BackendScope::TestBackend).is_err());
}
#[test]
fn capacity_exhaustion_preserves_existing_raw_receipts_and_revokes_new_ack_eligibility() {
    use pokelinux_messaging::receipt::{MAX_RECEIPTS, MAX_SNAPSHOT};
    let directory = Directory::new();
    let mut owner = directory.owner();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<4096) INSERT INTO receipts(id,slot,origin_owner,source_incarnation,scope,kind,raw) SELECT printf('%032x',x),0,printf('%032x',0),printf('%032x',0),'unqualified','deliver',X'78' FROM n;").unwrap();
    owner.set_test_ack_permission(true);
    let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
    assert!(
        owner
            .admit(source, IncomingKind::Deliver, b"overflow")
            .is_err()
    );
    let snapshot = owner.snapshot(MAX_SNAPSHOT).unwrap();
    assert_eq!(snapshot.total, MAX_RECEIPTS);
    assert_eq!(snapshot.latest.len(), MAX_SNAPSHOT);
    assert_eq!(owner.control().active_sources, 0);
}
#[test]
fn private_file_modes_links_and_exclusive_owner_are_checked() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    let directory = Directory::new();
    let owner = directory.owner();
    assert!(ReceiptOwner::open(&directory.0).is_err());
    assert_eq!(fs::metadata(&directory.0).unwrap().mode() & 0o777, 0o700);
    let db = directory.0.join("receipts.sqlite3");
    assert_eq!(fs::metadata(&db).unwrap().mode() & 0o777, 0o600);
    drop(owner);
    fs::set_permissions(&db, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(ReceiptOwner::open(&directory.0).is_err());
    fs::set_permissions(&db, fs::Permissions::from_mode(0o600)).unwrap();
    let linked = directory.0.join("extra-link");
    fs::hard_link(&db, &linked).unwrap();
    assert!(ReceiptOwner::open(&directory.0).is_err());
    fs::remove_file(linked).unwrap();
    let original = directory.0.join("original.sqlite3");
    fs::rename(&db, &original).unwrap();
    symlink(&original, &db).unwrap();
    assert!(ReceiptOwner::open(&directory.0).is_err());
}

#[test]
fn cancellation_revokes_the_incoming_source_until_explicit_test_reattachment() {
    for dispatched in [false, true] {
        let directory = Directory::new();
        let mut owner = directory.owner();
        owner.set_test_ack_permission(true);
        let source = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
        let id = owner.admit(source, IncomingKind::Deliver, b"old").unwrap();
        let permit = owner.claim_test_ack(source, id).unwrap();
        if dispatched {
            let mut sink = Sink::new(&directory, b"old");
            assert_eq!(
                owner.dispatch_test_ack(permit, &mut sink).unwrap(),
                Outcome::Pending
            );
        }
        owner.cancel(id).unwrap();
        let late = owner
            .admit(source, IncomingKind::StatusReport, b"new")
            .unwrap();
        assert_eq!(owner.lookup(late).unwrap().raw(), b"new");
        assert!(
            !owner.eligible(source, late),
            "cancellation must not forget uncertain last-incoming ordering and reopen the same source"
        );
        assert!(owner.claim_test_ack(source, late).is_err());
        assert_eq!(owner.control().active_sources, 0);
        let replacement = owner.attach(Slot(0), BackendScope::TestBackend).unwrap();
        assert!(owner.claim_test_ack(replacement, late).is_err());
        let fresh = owner
            .admit(replacement, IncomingKind::Deliver, b"new")
            .unwrap();
        assert_ne!(late, fresh);
        assert!(owner.eligible(replacement, fresh));
    }
}

#[test]
fn fixed_upper_paged_enumeration_preserves_equal_ids_and_defers_new_tail() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let ids: Vec<_> = (0..33)
        .map(|_| {
            owner
                .admit(source, IncomingKind::Deliver, b"equal")
                .unwrap()
        })
        .collect();
    let mut scan = owner.scan(None).unwrap();
    assert_eq!(scan.total(), 33);
    assert!(owner.checkpoint(&scan).is_err());
    assert!(owner.scan_page(&mut scan, 0).is_err());
    assert!(owner.scan_page(&mut scan, 33).is_err());
    let first = owner.scan_page(&mut scan, 8).unwrap();
    assert!(!first.complete);
    let extra = owner
        .admit(source, IncomingKind::Deliver, b"equal")
        .unwrap();
    let mut seen: Vec<_> = first.items.into_iter().map(|r| r.id).collect();
    while !scan.complete() {
        let page = owner.scan_page(&mut scan, 8).unwrap();
        assert!(page.items.len() <= 8);
        seen.extend(page.items.into_iter().map(|r| r.id));
    }
    assert_eq!(seen, ids);
    assert_eq!(scan.total(), 33);
    assert!(!owner.scan_current(&scan).unwrap());
    let checkpoint = owner.checkpoint(&scan).unwrap();
    let mut tail = owner.scan(Some(&checkpoint)).unwrap();
    assert_eq!(tail.total(), 34);
    assert_eq!(tail.remaining(), 1);
    assert_eq!(owner.scan_page(&mut tail, 8).unwrap().items[0].id, extra);
    assert!(owner.scan_current(&tail).unwrap());
    let tail_checkpoint = owner.checkpoint(&tail).unwrap();
    assert!(!owner.control().permission);
    assert_eq!(owner.control().pending, 0);
    drop(owner);
    let reopened = directory.owner();
    assert!(reopened.scan(Some(&tail_checkpoint)).is_err());
    assert!(reopened.scan_page(&mut tail, 8).is_err());
    let mut fresh = reopened.scan(None).unwrap();
    let mut again = Vec::new();
    while !fresh.complete() {
        again.extend(
            reopened
                .scan_page(&mut fresh, 32)
                .unwrap()
                .items
                .into_iter()
                .map(|r| r.id),
        );
    }
    let mut expected = ids;
    expected.push(extra);
    assert_eq!(again, expected);
}
#[test]
fn bounded_full4096_enumeration_has_no_silent_latest32_truncation() {
    let directory = Directory::new();
    let owner = directory.owner();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<4096) INSERT INTO receipts(id,slot,origin_owner,source_incarnation,scope,kind,raw) SELECT printf('%032x',x),0,printf('%032x',0),printf('%032x',0),'unqualified','deliver',X'78' FROM n;").unwrap();
    let mut scan = owner.scan(None).unwrap();
    assert_eq!(scan.total(), 4096);
    let mut seen = std::collections::HashSet::new();
    let mut pages = 0;
    while !scan.complete() {
        let page = owner.scan_page(&mut scan, 32).unwrap();
        assert_eq!(page.items.len(), 32);
        pages += 1;
        for row in page.items {
            assert!(seen.insert(row.id));
        }
    }
    assert_eq!(pages, 128);
    assert_eq!(seen.len(), 4096);
    assert_eq!(owner.snapshot(32).unwrap().latest.len(), 32);
}
#[test]
fn changed_captured_membership_refuses_without_advancing_page_or_ack_control() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(Slot(0), BackendScope::Unqualified).unwrap();
    for _ in 0..33 {
        owner
            .admit(source, IncomingKind::Deliver, b"equal")
            .unwrap();
    }
    let mut scan = owner.scan(None).unwrap();
    owner.scan_page(&mut scan, 8).unwrap();
    let remaining = scan.remaining();
    let db = rusqlite::Connection::open(directory.0.join("receipts.sqlite3")).unwrap();
    db.execute("DELETE FROM receipts WHERE seq=12", []).unwrap();
    assert!(owner.scan_page(&mut scan, 8).is_err());
    assert_eq!(scan.remaining(), remaining);
    assert!(owner.scan_current(&scan).is_err());
    assert!(owner.scan(None).is_err());
    assert!(!owner.control().permission);
    assert_eq!(owner.control().pending, 0);
    assert_eq!(owner.snapshot(32).unwrap().total, 32);
}
