#![cfg(target_os = "linux")]
use pokelinux_messaging::outbox::{Outbox, PartState, TransportResult};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
};
const ID: &str = "0123456789abcdef0123456789abcdef";
const OTHER: &str = "1123456789abcdef0123456789abcdef";
const PHONE: &str = "+15550100";
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("sms-outbox-{}", uuid::Uuid::new_v4().simple())))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn database(d: &Directory) -> rusqlite::Connection {
    rusqlite::Connection::open(d.0.join("outbox.sqlite3")).unwrap()
}
#[test]
fn reference_reservation_duplicate_identity_and_restart_are_atomic() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    assert!(Outbox::open(&d.0).is_err());
    let text = "A".repeat(161);
    let first = box_.prepare(ID, PHONE, &text, false).unwrap();
    assert_eq!(first.parts, vec![PartState::Ready; 2]);
    assert!(first.reference.is_some());
    assert_eq!(box_.prepare(ID, PHONE, &text, false).unwrap(), first);
    assert!(box_.prepare(ID, PHONE, "changed", false).is_err());
    let second = box_.prepare(OTHER, PHONE, &text, false).unwrap();
    assert_ne!(first.reference, second.reference);
    drop(box_);
    let mut box_ = Outbox::open(&d.0).unwrap();
    assert_eq!(box_.prepare(ID, PHONE, &text, false).unwrap(), first);
    let db = database(&d);
    assert_eq!(
        db.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
    // synchronous is connection-local; durable mode is checked inside the crate.
    assert_eq!(
        fs::metadata(d.0.join("outbox.sqlite3"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
#[test]
fn dispatch_is_committed_before_payload_is_exposed_and_restart_forbids_retry() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    box_.prepare(ID, PHONE, &"A".repeat(161), false).unwrap();
    let permit = box_.claim(ID, 0).unwrap();
    assert!(!permit.tpdu().is_empty());
    assert_eq!(
        database(&d)
            .query_row(
                "SELECT state FROM parts WHERE message=?1 AND part=0",
                [ID],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "uncertain"
    );
    assert!(box_.claim(ID, 0).is_err());
    assert!(box_.claim(ID, 1).is_err());
    drop(permit);
    drop(box_);
    let mut box_ = Outbox::open(&d.0).unwrap();
    assert_eq!(
        box_.lookup(ID).unwrap().parts,
        vec![PartState::Uncertain, PartState::Ready]
    );
    assert!(box_.claim(ID, 0).is_err());
    assert!(box_.claim(ID, 1).is_err());
}
#[test]
fn permit_from_previous_owner_cannot_change_a_restarted_outbox() {
    let d = Directory::new();
    let mut old = Outbox::open(&d.0).unwrap();
    old.prepare(ID, PHONE, "hello", false).unwrap();
    let permit = old.claim(ID, 0).unwrap();
    drop(old);
    let mut next = Outbox::open(&d.0).unwrap();
    assert!(next.finish(permit, TransportResult::Accepted).is_err());
    assert_eq!(next.lookup(ID).unwrap().parts, vec![PartState::Uncertain]);
}
#[test]
fn cancellation_survives_late_acceptance_and_duplicate_prepare() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    let text = "A".repeat(161);
    box_.prepare(ID, PHONE, &text, false).unwrap();
    let permit = box_.claim(ID, 0).unwrap();
    box_.cancel(ID).unwrap();
    box_.finish(permit, TransportResult::Accepted).unwrap();
    let receipt = box_.prepare(ID, PHONE, &text, false).unwrap();
    assert!(receipt.cancelled);
    assert_eq!(
        receipt.parts,
        vec![PartState::TransportAccepted, PartState::Ready]
    );
    assert!(box_.claim(ID, 1).is_err());
    drop(box_);
    let mut box_ = Outbox::open(&d.0).unwrap();
    assert!(box_.lookup(ID).unwrap().cancelled);
    assert!(box_.claim(ID, 1).is_err());
}
#[test]
fn parts_are_ordered_and_transport_acceptance_never_means_delivery() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    box_.prepare(ID, PHONE, &"A".repeat(161), false).unwrap();
    assert!(box_.claim(ID, 1).is_err());
    let first = box_.claim(ID, 0).unwrap();
    box_.finish(first, TransportResult::Accepted).unwrap();
    let second = box_.claim(ID, 1).unwrap();
    box_.finish(second, TransportResult::NotSubmitted).unwrap();
    assert_eq!(
        box_.lookup(ID).unwrap().parts,
        vec![PartState::TransportAccepted, PartState::NotSubmitted]
    );
    assert!(box_.claim(ID, 1).is_err());
}
#[test]
fn real_database_abort_never_returns_a_dispatch_permit() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    box_.prepare(ID, PHONE, "hello", false).unwrap();
    let db = database(&d);
    db.execute_batch("CREATE TRIGGER fail_claim BEFORE UPDATE ON parts BEGIN SELECT RAISE(ABORT,'test fault'); END;").unwrap();
    assert!(box_.claim(ID, 0).is_err());
    assert_eq!(box_.lookup(ID).unwrap().parts, vec![PartState::Ready]);
    db.execute_batch("DROP TRIGGER fail_claim;").unwrap();
    assert!(box_.claim(ID, 0).is_ok());
}
#[test]
fn private_storage_and_unsafe_lock_objects_are_checked_before_io() {
    let d = Directory::new();
    fs::create_dir(&d.0).unwrap();
    fs::set_permissions(&d.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Outbox::open(&d.0).is_err());
    fs::set_permissions(&d.0, fs::Permissions::from_mode(0o700)).unwrap();
    let outside = d.0.join("outside");
    fs::write(&outside, b"do not touch").unwrap();
    symlink(&outside, d.0.join("owner.lock")).unwrap();
    assert!(Outbox::open(&d.0).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"do not touch");
    fs::remove_file(d.0.join("owner.lock")).unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        d.0.join("owner.lock"),
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    assert!(Outbox::open(&d.0).is_err());
}

#[test]
fn payload_transaction_failure_does_not_consume_a_reference() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    let db = database(&d);
    db.execute_batch("CREATE TRIGGER fail_part BEFORE INSERT ON parts BEGIN SELECT RAISE(ABORT,'test fault'); END;").unwrap();
    assert!(box_.prepare(ID, PHONE, &"A".repeat(161), false).is_err());
    assert!(box_.lookup(ID).is_err());
    assert_eq!(
        db.query_row("SELECT next_reference FROM metadata", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_part;").unwrap();
    assert_eq!(
        box_.prepare(ID, PHONE, &"A".repeat(161), false)
            .unwrap()
            .reference,
        Some(0)
    );
}
#[test]
fn exhausted_reference_space_is_never_wrapped_and_single_parts_still_work() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    let db = database(&d);
    // Fault/volume fixture represents a full lifetime reservation history.
    db.execute("UPDATE metadata SET next_reference=65535", [])
        .unwrap();
    assert_eq!(
        box_.prepare(ID, PHONE, &"A".repeat(161), false)
            .unwrap()
            .reference,
        Some(65535)
    );
    assert!(box_.prepare(OTHER, PHONE, &"A".repeat(161), false).is_err());
    assert!(box_.lookup(OTHER).is_err());
    assert_eq!(
        box_.prepare(OTHER, PHONE, "hello", false)
            .unwrap()
            .reference,
        None
    );
}
#[test]
fn receipt_capacity_failure_preserves_existing_identities() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    let first = box_.prepare(ID, PHONE, "hello", false).unwrap();
    let db = database(&d);
    db.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<4095) INSERT INTO messages(id,digest,recipient,body,report,total) SELECT printf('%032x',x),zeroblob(32),'+15550100','fixture',0,1 FROM n;").unwrap();
    assert!(box_.prepare(OTHER, PHONE, "hello", false).is_err());
    assert_eq!(box_.prepare(ID, PHONE, "hello", false).unwrap(), first);
}
#[test]
fn invalid_identities_and_existing_schema_are_never_reinitialized() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    for id in ["", "../escape", "0123456789ABCDEF0123456789ABCDEF"] {
        assert!(box_.prepare(id, PHONE, "hello", false).is_err());
    }
    box_.prepare(ID, PHONE, "hello", false).unwrap();
    drop(box_);
    let db = database(&d);
    db.execute_batch("PRAGMA user_version=99;").unwrap();
    drop(db);
    assert!(Outbox::open(&d.0).is_err());
    let db = database(&d);
    assert_eq!(
        db.query_row("SELECT count(*) FROM messages WHERE id=?1", [ID], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn silent_database_write_refusal_cannot_return_a_dispatch_permit() {
    let d = Directory::new();
    let mut box_ = Outbox::open(&d.0).unwrap();
    box_.prepare(ID, PHONE, "hello", false).unwrap();
    let db = database(&d);
    db.execute_batch(
        "CREATE TRIGGER ignore_claim BEFORE UPDATE ON parts BEGIN SELECT RAISE(IGNORE); END;",
    )
    .unwrap();
    assert!(box_.claim(ID, 0).is_err());
    assert_eq!(box_.lookup(ID).unwrap().parts, vec![PartState::Ready]);
}
