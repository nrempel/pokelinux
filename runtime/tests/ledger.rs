#![cfg(target_os = "linux")]
use omarchy_phone_core::ledger::{Admission, Ledger, Outcome};
use omarchy_phone_core::{Identity, Request, decode_request};
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::PathBuf};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("phone-ledger-{}", uuid::Uuid::new_v4().simple())))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn request(epoch: &Identity, id: u32, number: &str) -> Request {
    let value = serde_json::json!({"v":1,"id":format!("{id:032x}"),"epoch":epoch,
        "session":"cccccccccccccccccccccccccccccccc", "command":{"kind":"dial",
        "generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":number}});
    decode_request(format!("{value}\n").as_bytes()).unwrap()
}

fn ringing(episode: &str) -> omarchy_phone_core::watch::Snapshot {
    let id = Identity::try_from("dddddddddddddddddddddddddddddddd".to_owned()).unwrap();
    let body = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,
        "generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,
        "source":"android.telecom.InCallService","source_state":"observed",
        "ringing":[{"id":7,"episode":episode}]}))
    .unwrap();
    omarchy_phone_core::watch::decode_snapshot(&body, &id).unwrap()
}
#[test]
fn dismissal_journal_survives_restart_and_does_not_suppress_a_new_episode() {
    use omarchy_phone_core::notification_policy::Dismissals;
    let dir = Scratch::new();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut policy = Dismissals::default();
    policy.dismiss(&current, &current);
    let mut ledger = Ledger::open(&dir.0).unwrap();
    ledger.store_dismissals(&policy).unwrap();
    drop(ledger);
    let ledger = Ledger::open(&dir.0).unwrap();
    let restored = ledger.load_dismissals().unwrap();
    assert!(!restored.has_undismissed(&current));
    assert!(restored.has_undismissed(&ringing("cccccccccccccccccccccccccccccccc")));
}
#[test]
fn failed_dismissal_commit_keeps_previous_state_and_command_journal() {
    use omarchy_phone_core::notification_policy::Dismissals;
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut policy = Dismissals::default();
    policy.dismiss(&current, &current);
    ledger.store_dismissals(&policy).unwrap();
    let req = request(ledger.epoch(), 1, "5550100");
    ledger.reserve(&req).unwrap();
    let fault = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    fault.execute_batch("CREATE TRIGGER reject_dismissal BEFORE INSERT ON notification_episodes BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END;").unwrap();
    policy.observe(&ringing("cccccccccccccccccccccccccccccccc"));
    assert!(ledger.store_dismissals(&policy).is_err());
    assert!(!ledger.load_dismissals().unwrap().has_undismissed(&current));
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Pending)
    );
}
#[test]
fn corrupt_dismissal_identity_is_rejected_without_resetting_it() {
    use omarchy_phone_core::notification_policy::Dismissals;
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut policy = Dismissals::default();
    policy.dismiss(&current, &current);
    ledger.store_dismissals(&policy).unwrap();
    let fault = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    fault
        .execute("UPDATE notification_state SET generation='invalid'", [])
        .unwrap();
    assert!(ledger.load_dismissals().is_err());
    assert_eq!(
        fault
            .query_row("SELECT generation FROM notification_state", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "invalid"
    );
}

#[test]
fn oversized_or_orphaned_dismissal_rows_are_rejected() {
    let dir = Scratch::new();
    let ledger = Ledger::open(&dir.0).unwrap();
    assert!(ledger.load_dismissals().is_ok());
    let mut fault = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    let tx = fault.transaction().unwrap();
    tx.execute(
        "INSERT INTO notification_state VALUES (1, ?1)",
        ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
    )
    .unwrap();
    for id in 1..=65 {
        tx.execute(
            "INSERT INTO notification_episodes VALUES (?1, ?2, 1)",
            rusqlite::params![id, format!("{id:032x}")],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    assert!(ledger.load_dismissals().is_err());
    fault
        .execute("DELETE FROM notification_episodes WHERE call_id > 1", [])
        .unwrap();
    fault.execute("DELETE FROM notification_state", []).unwrap();
    assert!(ledger.load_dismissals().is_err());
}
#[test]
fn admission_is_durable_and_duplicates_never_dispatch_again() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let req = request(ledger.epoch(), 1, "5550100");
    assert_eq!(ledger.reserve(&req).unwrap(), Admission::New);
    let observer = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    assert_eq!(
        observer
            .query_row(
                "SELECT count(*) FROM operations WHERE outcome='pending'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Pending)
    );
    assert!(
        ledger
            .reserve(&request(ledger.epoch(), 1, "5550101"))
            .is_err()
    );
    assert!(
        ledger
            .reserve(&request(ledger.epoch(), 2, "5550100"))
            .is_err()
    );
    ledger.finish(req.id(), Outcome::DialInvoked).unwrap();
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::DialInvoked)
    );
    assert!(ledger.finish(req.id(), Outcome::Unknown).is_err());
    assert!(ledger.finish(req.id(), Outcome::ControlSubmitted).is_err());
    assert!(
        ledger
            .acknowledge(&ledger.epoch().clone(), req.id())
            .is_err()
    );
}
#[test]
fn restart_preserves_epoch_and_marks_pending_unknown_until_acknowledged() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let req = request(ledger.epoch(), 1, "5550100");
    ledger.reserve(&req).unwrap();
    assert!(Ledger::open(&dir.0).is_err(), "only one owner");
    drop(ledger);
    let mut ledger = Ledger::open(&dir.0).unwrap();
    assert_eq!(ledger.epoch(), req.epoch());
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Unknown)
    );
    let next = request(ledger.epoch(), 2, "5550101");
    assert!(ledger.reserve(&next).is_err());
    assert_eq!(ledger.unresolved().unwrap().len(), 1);
    ledger
        .acknowledge(&ledger.epoch().clone(), req.id())
        .unwrap();
    ledger
        .acknowledge(&ledger.epoch().clone(), req.id())
        .unwrap();
    assert!(ledger.unresolved().unwrap().is_empty());
    assert_eq!(ledger.reserve(&next).unwrap(), Admission::New);
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Unknown)
    );
    assert!(ledger.finish(next.id(), Outcome::ControlSubmitted).is_err());
    ledger.finish(next.id(), Outcome::DialNotInvoked).unwrap();
}
#[test]
fn refuses_wrong_epoch_and_insecure_or_symlinked_storage() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    assert!(
        ledger
            .reserve(&request(
                &Identity::try_from("a".repeat(32)).unwrap(),
                1,
                "5550100"
            ))
            .is_err()
    );
    drop(ledger);
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Ledger::open(&dir.0).is_err());
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o700)).unwrap();
    let link = Scratch::new();
    std::os::unix::fs::symlink(&dir.0, &link.0).unwrap();
    assert!(Ledger::open(&link.0).is_err());
    fs::remove_file(&link.0).unwrap();
    assert_eq!(
        fs::metadata(dir.0.join("journal.sqlite3"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn failed_commit_never_grants_dispatch_or_overwrites_a_receipt() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let req = request(ledger.epoch(), 1, "5550100");
    let injector = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    injector.execute_batch("CREATE TABLE failure_parent(id INTEGER PRIMARY KEY);
        CREATE TABLE failure_child(id INTEGER REFERENCES failure_parent(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER fail_admission AFTER INSERT ON operations BEGIN INSERT INTO failure_child VALUES(1); END;").unwrap();
    assert!(ledger.reserve(&req).is_err());
    assert_eq!(
        injector
            .query_row("SELECT count(*) FROM operations", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        0
    );
    injector
        .execute_batch("DROP TRIGGER fail_admission;")
        .unwrap();
    assert_eq!(ledger.reserve(&req).unwrap(), Admission::New);
    injector.execute_batch("CREATE TRIGGER fail_receipt AFTER UPDATE ON operations BEGIN INSERT INTO failure_child VALUES(1); END;").unwrap();
    assert!(ledger.finish(req.id(), Outcome::DialInvoked).is_err());
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Pending)
    );
    injector
        .execute_batch("DROP TRIGGER fail_receipt;")
        .unwrap();
    ledger.finish(req.id(), Outcome::DialInvoked).unwrap();
    drop(ledger);
    let bytes = fs::read(dir.0.join("journal.sqlite3")).unwrap();
    assert!(!bytes.windows(7).any(|s| s == b"5550100"));
}

#[test]
fn rejects_foreign_schema_and_symlinked_database_without_migration() {
    let dir = Scratch::new();
    drop(Ledger::open(&dir.0).unwrap());
    let path = dir.0.join("journal.sqlite3");
    let foreign = rusqlite::Connection::open(&path).unwrap();
    foreign.pragma_update(None, "user_version", 2).unwrap();
    assert!(Ledger::open(&dir.0).is_err());
    assert_eq!(
        foreign
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        2
    );
    drop(foreign);
    let target = dir.0.join("saved.sqlite3");
    fs::rename(&path, &target).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(Ledger::open(&dir.0).is_err());
}

// Run on every Linux architecture. The parent owns the exact child handle and
// kills it only after both production journal operations have committed.
#[test]
#[ignore = "spawned only by the SIGKILL integration test"]
fn crash_child() {
    let path = PathBuf::from(std::env::var_os("PHONE_JOURNAL_CRASH_TEST").unwrap());
    let mut ledger = Ledger::open(&path).unwrap();
    ledger
        .reserve(&request(ledger.epoch(), 1, "5550100"))
        .unwrap();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut policy = omarchy_phone_core::notification_policy::Dismissals::default();
    policy.dismiss(&current, &current);
    ledger.store_dismissals(&policy).unwrap();
    fs::write(path.join("committed"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}

#[test]
fn sigkill_after_commits_recovers_dismissal_and_marks_command_unknown() {
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = Scratch::new();
    let mut child = Child(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child", "--ignored"])
            .env("PHONE_JOURNAL_CRASH_TEST", &dir.0)
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !dir.0.join("committed").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before commit"
        );
        assert!(std::time::Instant::now() < deadline, "child did not commit");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        Ledger::open(&dir.0).is_err(),
        "live child retains journal ownership"
    );
    // Check visibility from another connection before the kill. A readiness
    // marker alone is not evidence that the SQLite transaction committed.
    let reader = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    assert_eq!(
        reader
            .query_row(
                "SELECT dismissed FROM notification_episodes WHERE call_id=7",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        reader
            .query_row("SELECT outcome FROM operations", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "pending"
    );
    drop(reader);
    child.0.kill().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(child.0.wait().unwrap().signal(), Some(9));
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let restored = ledger.load_dismissals().unwrap();
    assert!(!restored.has_undismissed(&ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")));
    assert!(restored.has_undismissed(&ringing("cccccccccccccccccccccccccccccccc")));
    let req = request(ledger.epoch(), 1, "5550100");
    assert_eq!(
        ledger.reserve(&req).unwrap(),
        Admission::Existing(Outcome::Unknown)
    );
    assert!(
        ledger
            .reserve(&request(ledger.epoch(), 2, "5550101"))
            .is_err()
    );
}
#[test]
fn receipt_lookup_is_read_only_and_never_admits_an_absent_command() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let epoch = ledger.epoch().clone();
    let req = request(&epoch, 1, "5550100");
    let before = fs::read(dir.0.join("journal.sqlite3")).unwrap();
    assert_eq!(ledger.lookup(&epoch, req.id()).unwrap(), None);
    assert_eq!(fs::read(dir.0.join("journal.sqlite3")).unwrap(), before);
    // Absence does not install a cancellation or reservation: later arrival
    // can still be admitted. The UI must not label absence as cancellation.
    assert_eq!(ledger.reserve(&req).unwrap(), Admission::New);
    let stored = ledger.lookup(&epoch, req.id()).unwrap().unwrap();
    assert_eq!(stored.outcome(), Outcome::Pending);
    assert!(!stored.acknowledged());
    ledger.finish(req.id(), Outcome::DialInvoked).unwrap();
    drop(ledger);
    let ledger = Ledger::open(&dir.0).unwrap();
    let stored = ledger.lookup(&epoch, req.id()).unwrap().unwrap();
    assert_eq!(stored.outcome(), Outcome::DialInvoked);
    assert!(!stored.acknowledged());
}
#[test]
fn recovery_is_bound_to_epoch_and_acknowledgement_preserves_unknown() {
    let dir = Scratch::new();
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let epoch = ledger.epoch().clone();
    let foreign = Identity::try_from("ffffffffffffffffffffffffffffffff".to_string()).unwrap();
    let first = request(&epoch, 1, "5550100");
    ledger.reserve(&first).unwrap();
    assert!(ledger.acknowledge(&epoch, first.id()).is_err());
    ledger.finish(first.id(), Outcome::Unknown).unwrap();
    assert!(ledger.lookup(&foreign, first.id()).is_err());
    assert!(ledger.acknowledge(&foreign, first.id()).is_err());
    assert!(
        !ledger
            .lookup(&epoch, first.id())
            .unwrap()
            .unwrap()
            .acknowledged()
    );
    ledger.acknowledge(&epoch, first.id()).unwrap();
    ledger.acknowledge(&epoch, first.id()).unwrap();
    let second = request(&epoch, 2, "5550101");
    ledger.reserve(&second).unwrap();
    ledger.finish(second.id(), Outcome::Unknown).unwrap();
    ledger.acknowledge(&epoch, first.id()).unwrap();
    assert_eq!(ledger.unresolved().unwrap(), vec![second.id().clone()]);
    drop(ledger);
    let mut ledger = Ledger::open(&dir.0).unwrap();
    let first_receipt = ledger.lookup(&epoch, first.id()).unwrap().unwrap();
    assert_eq!(first_receipt.outcome(), Outcome::Unknown);
    assert!(first_receipt.acknowledged());
    assert_eq!(
        ledger.reserve(&first).unwrap(),
        Admission::Existing(Outcome::Unknown)
    );
    assert_eq!(ledger.unresolved().unwrap(), vec![second.id().clone()]);
}
