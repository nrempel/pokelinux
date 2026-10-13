use pokelinux_inbox::{Inbox, ProjectionState};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("pokelinux-inbox-{}", uuid::Uuid::new_v4())))
    }
    fn path(&self, s: &str) -> PathBuf {
        self.0.join(s)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn raw() -> Vec<u8> {
    unhex("000008915155100000006201213141510005e8329bfd06")
}
fn setup(d: &Directory) -> (ReceiptOwner, Inbox) {
    fs::create_dir(&d.0).unwrap();
    (
        ReceiptOwner::open(&d.path("raw")).unwrap(),
        Inbox::open(&d.path("inbox")).unwrap(),
    )
}
#[test]
fn committed_local_receipt_is_projected_through_actual_sqlite() {
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let ctl = inbox.control();
    ctl.set_import_allowed(true).unwrap();
    let l = ctl.import_lease().unwrap();
    let result = inbox
        .import(&l, &receipts, id)
        .expect("committed text must be projected into actual SQLite");
    assert_eq!(result.state, ProjectionState::Text);
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    let body: String = db
        .query_row(
            "SELECT body FROM records WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(body, "hello");
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
    ignore = "requires real ARM64 kernel process boundary"
)]
fn inbox_crash_child() {
    let Some(path) = std::env::var_os("POKELINUX_INBOX_CRASH_TEST") else {
        return;
    };
    let d = Directory(PathBuf::from(path));
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    fs::write(d.path("raw-committed"), id.to_string()).unwrap();
    let c = inbox.control();
    c.set_import_allowed(true).unwrap();
    inbox
        .import(&c.import_lease().unwrap(), &receipts, id)
        .expect("child text projection missing");
    fs::write(d.path("projection-committed"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires real ARM64 kernel process boundary"
)]
fn sigkill_releases_private_inbox_with_committed_projection_intact() {
    let d = Directory::new();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "inbox_crash_child", "--include-ignored"])
            .env("POKELINUX_INBOX_CRASH_TEST", &d.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let end = Instant::now() + Duration::from_secs(10);
    while !d.path("projection-committed").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before durable text projection; raw committed={}",
            d.path("raw-committed").exists()
        );
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(Inbox::open(&d.path("inbox")).is_err());
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(9));
    let inbox = Inbox::open(&d.path("inbox")).unwrap();
    assert!(!inbox.control().snapshot().unwrap().read_allowed);
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    let body: String = db
        .query_row("SELECT body FROM records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(body, "hello");
}
fn leases(inbox: &Inbox) -> (pokelinux_inbox::ImportLease, pokelinux_inbox::ReadLease) {
    let c = inbox.control();
    c.set_import_allowed(true).unwrap();
    c.set_read_allowed(true).unwrap();
    (c.import_lease().unwrap(), c.read_lease().unwrap())
}
#[test]
fn exact_local_receipt_id_is_idempotent_equal_raw_occurrences_stay_distinct_after_restart() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let a = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let b = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let (import, read) = leases(&inbox);
    let first = inbox.import(&import, &receipts, a).unwrap();
    assert!(!first.existing);
    let duplicate = inbox.import(&import, &receipts, a).unwrap();
    assert!(duplicate.existing);
    assert_eq!(first.conversation, duplicate.conversation);
    let equal = inbox.import(&import, &receipts, b).unwrap();
    assert_ne!(first.receipt, equal.receipt);
    assert_eq!(first.conversation, equal.conversation);
    assert_eq!(inbox.stats(&read).unwrap().keys, 2);
    let rows = inbox
        .messages(&read, first.conversation.unwrap(), 0, 32)
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].receipt, ReceiptKey::from(a));
    assert_eq!(rows[1].receipt, ReceiptKey::from(b));
    drop(inbox);
    drop(receipts);
    let receipts = ReceiptOwner::open(&d.path("raw")).unwrap();
    let mut inbox = Inbox::open(&d.path("inbox")).unwrap();
    let (import, read) = leases(&inbox);
    assert!(inbox.import(&import, &receipts, a).unwrap().existing);
    assert_eq!(
        inbox
            .message(&read, ReceiptKey::from(a))
            .unwrap()
            .unwrap()
            .text,
        "hello"
    );
    assert_eq!(inbox.stats(&read).unwrap().keys, 2);
}
#[test]
fn status_malformed_unsupported_and_cancelled_receipts_never_become_text() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let (import, read) = leases(&inbox);
    let mut unsupported = raw();
    unsupported[9] = 4;
    for (kind, bytes, state, cancel) in [
        (
            IncomingKind::StatusReport,
            raw(),
            ProjectionState::StatusReport,
            false,
        ),
        (
            IncomingKind::Deliver,
            Vec::new(),
            ProjectionState::Malformed,
            false,
        ),
        (
            IncomingKind::Deliver,
            unsupported,
            ProjectionState::Unsupported,
            false,
        ),
        (
            IncomingKind::Deliver,
            raw(),
            ProjectionState::Cancelled,
            true,
        ),
    ] {
        let id = receipts.admit(s, kind, &bytes).unwrap();
        if cancel {
            receipts.cancel(id).unwrap()
        }
        let r = inbox.import(&import, &receipts, id).unwrap();
        assert_eq!(r.state, state);
        assert!(r.conversation.is_none());
        assert!(
            inbox
                .message(&read, ReceiptKey::from(id))
                .unwrap()
                .is_none()
        );
        assert!(inbox.import(&import, &receipts, id).unwrap().existing);
    }
    assert_eq!(inbox.stats(&read).unwrap().text_messages, 0);
    assert_eq!(inbox.stats(&read).unwrap().conversations, 0);
}
#[test]
fn cancelled_import_reconciliation_and_local_delete_preserve_tombstones_without_touching_raw() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let (import, read) = leases(&inbox);
    let a = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let b = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    inbox.import(&import, &receipts, a).unwrap();
    inbox.import(&import, &receipts, b).unwrap();
    receipts.cancel(a).unwrap();
    assert_eq!(
        inbox.import(&import, &receipts, a).unwrap().state,
        ProjectionState::Cancelled
    );
    assert!(inbox.message(&read, ReceiptKey::from(a)).unwrap().is_none());
    inbox.delete(&import, ReceiptKey::from(b)).unwrap();
    assert_eq!(
        inbox.import(&import, &receipts, b).unwrap().state,
        ProjectionState::Deleted
    );
    assert!(inbox.message(&read, ReceiptKey::from(b)).unwrap().is_none());
    assert_eq!(receipts.lookup(b).unwrap().raw(), raw());
    assert!(!receipts.lookup(b).unwrap().cancelled);
    assert_eq!(inbox.stats(&read).unwrap().keys, 2);
    assert_eq!(inbox.stats(&read).unwrap().conversations, 0);
    drop(inbox);
    let mut inbox = Inbox::open(&d.path("inbox")).unwrap();
    let (import, read) = leases(&inbox);
    assert_eq!(
        inbox.import(&import, &receipts, b).unwrap().state,
        ProjectionState::Deleted
    );
    assert!(inbox.message(&read, ReceiptKey::from(b)).unwrap().is_none());
}
#[test]
fn display_grouping_preserves_sender_kind_slot_exact_case_and_leading_digits() {
    use pokelinux_inbox::SenderKind;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let (import, read) = leases(&inbox);
    let mut unknown = raw();
    unknown[3] = 0x81;
    let mut national = unknown.clone();
    national[3] = 0xa1;
    let mut packed = [0u8; 7];
    for (i, b) in b"15550100".iter().enumerate() {
        for bit in 0..7 {
            if b & (1 << bit) != 0 {
                let n = i * 7 + bit;
                packed[n / 8] |= 1 << (n % 8)
            }
        }
    }
    let mut alpha = vec![0, 0, 14, 0xd0];
    alpha.extend(packed);
    alpha.extend_from_slice(&raw()[8..]);
    let mut leading = unknown.clone();
    leading[4] = 0x50;
    let cases = [
        (Slot(0), unknown, SenderKind::UnknownNumeric),
        (Slot(0), national, SenderKind::NationalNumeric),
        (Slot(0), alpha, SenderKind::Alphanumeric),
        (Slot(0), raw(), SenderKind::InternationalNumeric),
        (Slot(1), raw(), SenderKind::InternationalNumeric),
        (Slot(0), leading, SenderKind::UnknownNumeric),
    ];
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    for (slot, bytes, kind) in cases {
        let s = receipts.attach(slot, BackendScope::Unqualified).unwrap();
        let id = receipts.admit(s, IncomingKind::Deliver, &bytes).unwrap();
        let r = inbox.import(&import, &receipts, id).unwrap();
        let m = inbox.message(&read, r.receipt).unwrap().unwrap();
        assert_eq!(m.sender_kind, kind);
        assert_eq!(m.slot, slot);
        assert!(!ids.contains(&r.conversation));
        ids.push(r.conversation);
        labels.push(m.sender);
    }
    assert_eq!(labels[0], "15550100");
    assert_eq!(labels[0], labels[1]);
    assert_eq!(labels[0], labels[2]);
    assert_eq!(labels[3], "+15550100");
    assert_eq!(labels[3], labels[4]);
    assert_eq!(labels[5], "05550100");
    assert_eq!(inbox.stats(&read).unwrap().conversations, 6);
}
#[test]
fn retention_uses_local_insertion_order_and_never_resurrects_removed_text() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let (import, read) = leases(&inbox);
    let mut ids = Vec::new();
    for year in [0x62, 0x08, 0x09] {
        let mut bytes = raw();
        bytes[10] = year;
        let id = receipts.admit(s, IncomingKind::Deliver, &bytes).unwrap();
        inbox.import(&import, &receipts, id).unwrap();
        ids.push(id);
    }
    assert_eq!(inbox.retain_latest(&import, 1, 1).unwrap(), 1);
    assert_eq!(inbox.retain_latest(&import, 1, 32).unwrap(), 1);
    assert_eq!(inbox.retain_latest(&import, 1, 32).unwrap(), 0);
    for id in &ids[..2] {
        assert_eq!(
            inbox.import(&import, &receipts, *id).unwrap().state,
            ProjectionState::Retained
        );
        assert!(
            inbox
                .message(&read, ReceiptKey::from(*id))
                .unwrap()
                .is_none()
        );
    }
    assert!(
        inbox
            .message(&read, ReceiptKey::from(ids[2]))
            .unwrap()
            .is_some()
    );
    assert_eq!(inbox.stats(&read).unwrap().keys, 3);
    assert_eq!(inbox.stats(&read).unwrap().text_messages, 1);
}
#[test]
fn local_access_denies_by_default_fences_revocation_stop_foreign_and_restart_leases() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let c = inbox.control();
    assert!(c.import_lease().is_err());
    assert!(c.read_lease().is_err());
    let (import, read) = leases(&inbox);
    inbox.import(&import, &receipts, id).unwrap();
    c.set_read_allowed(false).unwrap();
    c.set_read_allowed(true).unwrap();
    assert!(inbox.message(&read, ReceiptKey::from(id)).is_err());
    assert!(inbox.stats(&read).is_err());
    assert!(inbox.conversations(&read, None, 1).is_err());
    assert!(inbox.state(&read, ReceiptKey::from(id)).is_err());
    c.set_import_allowed(false).unwrap();
    c.set_import_allowed(true).unwrap();
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert!(inbox.delete(&import, ReceiptKey::from(id)).is_err());
    let oldread = c.read_lease().unwrap();
    let oldimport = c.import_lease().unwrap();
    drop(inbox);
    let mut inbox = Inbox::open(&d.path("inbox")).unwrap();
    let (import, read) = leases(&inbox);
    assert!(inbox.message(&oldread, ReceiptKey::from(id)).is_err());
    assert!(inbox.import(&oldimport, &receipts, id).is_err());
    let foreign = Directory::new();
    fs::create_dir(&foreign.0).unwrap();
    let other = Inbox::open(&foreign.path("inbox")).unwrap();
    assert!(other.stats(&read).is_err());
    inbox.control().stop().unwrap();
    assert!(inbox.stats(&read).is_err());
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert!(inbox.control().set_read_allowed(true).is_err());
}
#[test]
fn sqlite_faults_roll_back_projection_and_refuse_changed_local_id_content() {
    use pokelinux_inbox::ReceiptKey;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let (import, read) = leases(&inbox);
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    db.execute_batch(
        "CREATE TRIGGER reject_record BEFORE INSERT ON records BEGIN SELECT RAISE(IGNORE);END;",
    )
    .unwrap();
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert_eq!(inbox.stats(&read).unwrap().keys, 0);
    assert_eq!(inbox.stats(&read).unwrap().conversations, 0);
    db.execute_batch("DROP TRIGGER reject_record;CREATE TRIGGER corrupt_record AFTER INSERT ON records BEGIN UPDATE records SET slot=7 WHERE id=NEW.id;END;").unwrap();
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert_eq!(inbox.stats(&read).unwrap().keys, 0);
    db.execute_batch("DROP TRIGGER corrupt_record;").unwrap();
    inbox.import(&import, &receipts, id).unwrap();
    db.execute_batch(
        "CREATE TRIGGER reject_remove BEFORE UPDATE ON records BEGIN SELECT RAISE(IGNORE);END;",
    )
    .unwrap();
    assert!(inbox.delete(&import, ReceiptKey::from(id)).is_err());
    assert_eq!(
        inbox
            .message(&read, ReceiptKey::from(id))
            .unwrap()
            .unwrap()
            .text,
        "hello"
    );
    db.execute_batch("DROP TRIGGER reject_remove;").unwrap();
    let rawdb = rusqlite::Connection::open(d.path("raw/receipts.sqlite3")).unwrap();
    let mut different = raw();
    *different.last_mut().unwrap() = 0;
    rawdb
        .execute(
            "UPDATE receipts SET raw=?2 WHERE id=?1",
            rusqlite::params![id.to_string(), different],
        )
        .unwrap();
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert_eq!(
        inbox
            .message(&read, ReceiptKey::from(id))
            .unwrap()
            .unwrap()
            .text,
        "hello"
    );
}
#[test]
fn tombstone_capacity_refuses_new_imports_without_dropping_identity_history() {
    use pokelinux_inbox::MAX_KEYS;
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let (import, read) = leases(&inbox);
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    db.execute_batch("WITH RECURSIVE n(x) AS(VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<4096) INSERT INTO records(id,slot,origin_owner,source_incarnation,kind,state) SELECT printf('%032x',x),0,printf('%032x',0),printf('%032x',0),0,'retained' FROM n;").unwrap();
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    assert!(inbox.import(&import, &receipts, id).is_err());
    assert_eq!(inbox.stats(&read).unwrap().keys, MAX_KEYS);
    assert_eq!(inbox.retain_latest(&import, 0, 32).unwrap(), 0);
    assert!(inbox.import(&import, &receipts, id).is_err());
}
#[test]
fn bounded_queries_and_memory_stop_with_sqlite_locked_are_explicit() {
    use pokelinux_inbox::{MAX_PAGE, MAX_TEXT_MESSAGES, ReceiptKey};
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let (import, read) = leases(&inbox);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let r = inbox.import(&import, &receipts, id).unwrap();
    assert!(inbox.conversations(&read, None, 0).is_err());
    assert!(inbox.conversations(&read, None, MAX_PAGE + 1).is_err());
    assert!(
        inbox
            .messages(&read, r.conversation.unwrap(), u64::MAX, 1)
            .is_err()
    );
    assert!(
        inbox
            .retain_latest(&import, MAX_TEXT_MESSAGES + 1, 1)
            .is_err()
    );
    assert!(inbox.retain_latest(&import, 0, 0).is_err());
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE;").unwrap();
    let start = Instant::now();
    assert!(inbox.message(&read, ReceiptKey::from(id)).is_err());
    inbox.control().stop().unwrap();
    assert!(inbox.control().snapshot().unwrap().stopped);
    assert!(start.elapsed() < Duration::from_secs(2));
    db.execute_batch("ROLLBACK;").unwrap();
}
#[test]
fn private_owner_modes_links_schema_and_raw_separation_are_checked() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    let d = Directory::new();
    let (_receipts, inbox) = setup(&d);
    assert!(Inbox::open(&d.path("inbox")).is_err());
    let p = d.path("inbox/inbox.sqlite3");
    assert_eq!(fs::metadata(d.path("inbox")).unwrap().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(&p).unwrap().mode() & 0o777, 0o600);
    let db = rusqlite::Connection::open(&p).unwrap();
    let columns: Vec<String> = db
        .prepare("SELECT name FROM pragma_table_info('records')")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        !columns
            .iter()
            .any(|c| c.contains("raw") || c.contains("digest"))
    );
    drop(db);
    drop(inbox);
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Inbox::open(&d.path("inbox")).is_err());
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    let hard = d.path("inbox/link");
    fs::hard_link(&p, &hard).unwrap();
    assert!(Inbox::open(&d.path("inbox")).is_err());
    fs::remove_file(hard).unwrap();
    let old = d.path("inbox/original");
    fs::rename(&p, &old).unwrap();
    symlink(&old, &p).unwrap();
    assert!(Inbox::open(&d.path("inbox")).is_err());
}
#[test]
fn removal_refuses_success_when_empty_conversation_label_cleanup_is_ignored() {
    let d = Directory::new();
    let (mut receipts, mut inbox) = setup(&d);
    let (import, read) = leases(&inbox);
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
    let r = inbox.import(&import, &receipts, id).unwrap();
    let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER retain_private_label BEFORE DELETE ON conversations BEGIN SELECT RAISE(IGNORE);END;").unwrap();
    assert!(
        inbox.delete(&import, r.receipt).is_err(),
        "removal must fail atomically when its final private sender label is retained"
    );
    assert!(inbox.message(&read, r.receipt).unwrap().is_some());
}
#[test]
fn text_and_conversation_capacity_require_explicit_retention_without_key_eviction() {
    for conversations in [false, true] {
        let d = Directory::new();
        let (mut receipts, mut inbox) = setup(&d);
        let (import, read) = leases(&inbox);
        let db = rusqlite::Connection::open(d.path("inbox/inbox.sqlite3")).unwrap();
        if conversations {
            db.execute_batch("WITH RECURSIVE n(x) AS(VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<512) INSERT INTO conversations(id,slot,sender_kind,sender) SELECT printf('%032x',x),0,129,printf('%d',x) FROM n;INSERT INTO records(id,slot,origin_owner,source_incarnation,kind,state,conversation,body,encoding,timestamp,reply) SELECT id,0,printf('%032x',0),printf('%032x',0),0,'text',id,'x',0,0,0 FROM conversations;").unwrap()
        } else {
            db.execute_batch("INSERT INTO conversations VALUES(printf('%032x',0),0,145,'+15550100');WITH RECURSIVE n(x) AS(VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<2048) INSERT INTO records(id,slot,origin_owner,source_incarnation,kind,state,conversation,body,encoding,timestamp,reply) SELECT printf('%032x',x),0,printf('%032x',0),printf('%032x',0),0,'text',printf('%032x',0),'x',0,0,0 FROM n;").unwrap()
        }
        let before = inbox.stats(&read).unwrap();
        let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
        let id = receipts.admit(s, IncomingKind::Deliver, &raw()).unwrap();
        assert!(inbox.import(&import, &receipts, id).is_err());
        assert_eq!(inbox.stats(&read).unwrap(), before);
        assert_eq!(
            inbox
                .retain_latest(&import, before.text_messages - 1, 1)
                .unwrap(),
            1
        );
        assert_eq!(inbox.stats(&read).unwrap().keys, before.keys);
        assert_eq!(
            inbox.import(&import, &receipts, id).unwrap().state,
            ProjectionState::Text
        );
        assert_eq!(inbox.stats(&read).unwrap().keys, before.keys + 1);
    }
}
