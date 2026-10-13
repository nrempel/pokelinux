//! Durable local incoming occurrences, with test-backend-only ACK ownership.
//! Equal payloads are distinct receipts. No modem receive identity, deduplication,
//! successful network acknowledgement or safe replay is inferred by this module.
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
};
pub const MAX_RECEIPTS: usize = 4096;
pub const MAX_RAW_BYTES: usize = 512;
pub const MAX_SLOTS: u8 = 8;
pub const MAX_PENDING: usize = 8;
pub const MAX_SNAPSHOT: usize = 32;
pub const MAX_SCAN_PAGE: usize = 32;
#[derive(Debug)]
pub enum Error {
    Refused(&'static str),
    Io(std::io::Error),
    Database(rusqlite::Error),
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendScope {
    Unqualified,
    TestBackend,
}
impl BackendScope {
    fn value(self) -> &'static str {
        match self {
            Self::Unqualified => "unqualified",
            Self::TestBackend => "test_backend",
        }
    }
    fn parse(value: &str) -> Result<Self> {
        match value {
            "unqualified" => Ok(Self::Unqualified),
            "test_backend" => Ok(Self::TestBackend),
            _ => Err(Error::Refused("invalid stored backend scope")),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Source {
    owner: uuid::Uuid,
    incarnation: uuid::Uuid,
    slot: Slot,
    test_backend: bool,
}
impl Source {
    pub fn slot(self) -> Slot {
        self.slot
    }
    pub fn incarnation(self) -> uuid::Uuid {
        self.incarnation
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReceiptId(uuid::Uuid);
impl std::fmt::Display for ReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.simple())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AckAttempt(uuid::Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingKind {
    Deliver,
    StatusReport,
}
impl IncomingKind {
    fn value(self) -> &'static str {
        match self {
            Self::Deliver => "deliver",
            Self::StatusReport => "status_report",
        }
    }
    fn parse(value: &str) -> Result<Self> {
        match value {
            "deliver" => Ok(Self::Deliver),
            "status_report" => Ok(Self::StatusReport),
            _ => Err(Error::Refused("invalid stored incoming kind")),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckState {
    Stored,
    Uncertain,
    TestBackendAccepted,
}
impl AckState {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "stored" => Ok(Self::Stored),
            "uncertain" => Ok(Self::Uncertain),
            "test_backend_accepted" => Ok(Self::TestBackendAccepted),
            _ => Err(Error::Refused("invalid stored ACK state")),
        }
    }
}
pub struct StoredReceipt {
    pub id: ReceiptId,
    pub slot: Slot,
    pub origin_owner: uuid::Uuid,
    pub source_incarnation: uuid::Uuid,
    pub scope: BackendScope,
    pub kind: IncomingKind,
    pub ack: AckState,
    pub cancelled: bool,
    raw: Vec<u8>,
}
impl StoredReceipt {
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptSummary {
    pub id: ReceiptId,
    pub slot: Slot,
    pub kind: IncomingKind,
    pub ack: AckState,
    pub cancelled: bool,
    pub raw_length: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct ControlSnapshot {
    pub stopped: bool,
    pub permission: bool,
    pub active_sources: usize,
    pub pending: usize,
}
#[derive(Debug)]
pub struct Snapshot {
    pub total: usize,
    pub stopped: bool,
    pub permission: bool,
    pub active_sources: usize,
    pub pending: usize,
    pub latest: Vec<ReceiptSummary>,
}
/// Owner-bound data cursor over a fixed append-only sequence upper bound.
/// This is not read/ACK authority and carries no raw payload or modem token.
pub struct ReceiptScan {
    owner: uuid::Uuid,
    upper: i64,
    after: i64,
}
impl ReceiptScan {
    pub fn total(&self) -> usize {
        self.upper as usize
    }
    pub fn remaining(&self) -> usize {
        (self.upper - self.after) as usize
    }
    pub fn complete(&self) -> bool {
        self.after == self.upper
    }
}
/// A completed local enumeration prefix, usable only by the same open owner.
/// Copies remain data checkpoints, never transferable permission.
#[derive(Clone, Copy)]
pub struct ReceiptCheckpoint {
    owner: uuid::Uuid,
    through: i64,
}
pub struct ReceiptPage {
    pub items: Vec<ReceiptSummary>,
    pub complete: bool,
}
/// Non-cloneable local one-use permission. This contains no modem target token,
/// raw payload, serial, success flag or GSM cause; use only dispatch_test_ack.
pub struct AckPermit {
    owner: uuid::Uuid,
    source: Source,
    receipt: ReceiptId,
    attempt: AckAttempt,
}
impl AckPermit {
    pub fn attempt(&self) -> AckAttempt {
        self.attempt
    }
    pub fn receipt(&self) -> ReceiptId {
        self.receipt
    }
}
#[derive(Debug, Clone, Copy)]
pub struct TestAckRequest {
    pub source: Source,
    pub receipt: ReceiptId,
    pub attempt: AckAttempt,
}
/// The injected adapter qualifies test-backend ownership only. Production modem
/// correlation, success/cause mapping and ACK replay remain unqualified.
#[derive(Debug, Clone, Copy)]
pub struct TestDispatchError;
pub trait TestAckSink {
    fn dispatch(&mut self, request: TestAckRequest) -> std::result::Result<(), TestDispatchError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSignal {
    IpcReturned,
    RequestAcknowledged,
    FinalAccepted,
    Uncertain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ignored,
    Pending,
    TestBackendAccepted,
    Uncertain,
}
struct Window {
    source: Source,
    receipt: ReceiptId,
    attempt: Option<AckAttempt>,
}
#[derive(Clone, Copy)]
struct Pending {
    source: Source,
    receipt: ReceiptId,
    dispatched: bool,
}
pub struct ReceiptOwner {
    db: Connection,
    owner: uuid::Uuid,
    sources: HashMap<Slot, Source>,
    windows: HashMap<Slot, Window>,
    pending: HashMap<AckAttempt, Pending>,
    permission: bool,
    stopped: bool,
    // SQLite is closed before the permanent lock inode is released.
    _lock: File,
}
fn private_file(path: &Path, new: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32);
    if new {
        options.create_new(true);
    } else {
        options.create(true);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
    {
        return Err(Error::Refused(
            "private regular receipt journal files required",
        ));
    }
    Ok(file)
}
fn uuid(value: &str) -> Result<uuid::Uuid> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Refused("invalid stored local identity"));
    }
    uuid::Uuid::parse_str(value).map_err(|_| Error::Refused("invalid stored local identity"))
}
impl ReceiptOwner {
    pub fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() {
            return Err(Error::Refused("absolute receipt directory required"));
        }
        match fs::DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => File::open(
                directory
                    .parent()
                    .ok_or(Error::Refused("missing receipt directory parent"))?,
            )?
            .sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o777 != 0o700
            || fs::canonicalize(directory)? != directory
        {
            return Err(Error::Refused(
                "private receipt directory without symlinks required",
            ));
        }
        let lock = private_file(&directory.join("owner.lock"), false)?;
        lock.try_lock()
            .map_err(|_| Error::Refused("receipt store already owned or locking unavailable"))?;
        let path = directory.join("receipts.sqlite3");
        let journal = directory.join("receipts.sqlite3-journal");
        match fs::symlink_metadata(&journal) {
            Ok(_) => {
                private_file(&journal, false)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
        let (file, fresh) = match private_file(&path, true) {
            Ok(file) => (file, true),
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                (private_file(&path, false)?, false)
            }
            Err(e) => return Err(e),
        };
        let mut db = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        db.busy_timeout(std::time::Duration::ZERO)?;
        db.execute_batch("PRAGMA journal_mode=DELETE;PRAGMA synchronous=EXTRA;PRAGMA foreign_keys=ON;PRAGMA trusted_schema=OFF;PRAGMA max_page_count=32768;")?;
        if fresh {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE receipts(seq INTEGER PRIMARY KEY,id TEXT NOT NULL UNIQUE CHECK(length(id)=32),slot INTEGER NOT NULL CHECK(slot BETWEEN 0 AND 7),origin_owner TEXT NOT NULL CHECK(length(origin_owner)=32),source_incarnation TEXT NOT NULL CHECK(length(source_incarnation)=32),scope TEXT NOT NULL CHECK(scope IN('unqualified','test_backend')),kind TEXT NOT NULL CHECK(kind IN('deliver','status_report')),raw BLOB NOT NULL CHECK(length(raw) BETWEEN 0 AND 512),ack TEXT NOT NULL DEFAULT 'stored' CHECK(ack IN('stored','uncertain','test_backend_accepted')),cancelled INTEGER NOT NULL DEFAULT 0 CHECK(cancelled IN(0,1)),attempt TEXT UNIQUE CHECK(length(attempt)=32)) STRICT;PRAGMA user_version=1;")?;
            tx.commit()?;
        } else if db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? != 1 {
            return Err(Error::Refused("unsupported receipt schema; no migrations"));
        }
        if db.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))? != "delete"
            || db.pragma_query_value(None, "synchronous", |r| r.get::<_, u32>(0))? != 3
            || db.pragma_query_value(None, "foreign_keys", |r| r.get::<_, u32>(0))? != 1
        {
            return Err(Error::Refused("durable receipt journal unavailable"));
        }
        file.sync_all()?;
        File::open(directory)?.sync_all()?;
        let value = Self {
            db,
            owner: uuid::Uuid::new_v4(),
            sources: HashMap::new(),
            windows: HashMap::new(),
            pending: HashMap::new(),
            permission: false,
            stopped: false,
            _lock: lock,
        };
        if value.count()? > MAX_RECEIPTS {
            return Err(Error::Refused("stored receipt count outside bound"));
        }
        Ok(value)
    }
    fn count(&self) -> Result<usize> {
        Ok(self.db.query_row(
            "SELECT count(*) FROM (SELECT seq FROM receipts LIMIT 4097)",
            [],
            |r| r.get::<_, u32>(0),
        )? as usize)
    }
    fn active(&self, source: Source) -> bool {
        !self.stopped
            && source.owner == self.owner
            && self.sources.get(&source.slot) == Some(&source)
    }
    pub fn set_test_ack_permission(&mut self, allowed: bool) {
        if self.stopped || self.permission == allowed {
            return;
        }
        self.permission = allowed;
        self.sources.clear();
        self.windows.clear();
        self.pending.clear();
    }
    pub fn attach(&mut self, slot: Slot, scope: BackendScope) -> Result<Source> {
        if self.stopped || slot.0 >= MAX_SLOTS {
            return Err(Error::Refused(
                "stopped receipt owner or slot outside bound",
            ));
        }
        if let Some(old) = self.sources.get(&slot).copied() {
            self.revoke(old);
        }
        let source = Source {
            owner: self.owner,
            incarnation: uuid::Uuid::new_v4(),
            slot,
            test_backend: scope == BackendScope::TestBackend,
        };
        self.sources.insert(slot, source);
        Ok(source)
    }
    pub fn revoke(&mut self, source: Source) {
        if self.active(source) {
            self.sources.remove(&source.slot);
        }
        if self
            .windows
            .get(&source.slot)
            .is_some_and(|w| w.source == source)
        {
            self.windows.remove(&source.slot);
        }
        self.pending.retain(|_, p| p.source != source);
    }
    pub fn control(&self) -> ControlSnapshot {
        ControlSnapshot {
            stopped: self.stopped,
            permission: self.permission,
            active_sources: self.sources.len(),
            pending: self.pending.len(),
        }
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.permission = false;
        self.sources.clear();
        self.windows.clear();
        self.pending.clear();
    }
    fn persist(&mut self, source: Source, kind: IncomingKind, raw: &[u8]) -> Result<ReceiptId> {
        if raw.len() > MAX_RAW_BYTES {
            return Err(Error::Refused("raw receipt payload outside bound"));
        }
        let id = ReceiptId(uuid::Uuid::new_v4());
        let scope = if source.test_backend {
            BackendScope::TestBackend
        } else {
            BackendScope::Unqualified
        };
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: u32 = tx.query_row(
            "SELECT count(*) FROM (SELECT seq FROM receipts LIMIT 4097)",
            [],
            |r| r.get(0),
        )?;
        if count >= MAX_RECEIPTS as u32 {
            return Err(Error::Refused("durable receipt capacity exhausted"));
        }
        let last: i64 = tx.query_row("SELECT COALESCE(max(seq),0) FROM receipts", [], |r| {
            r.get(0)
        })?;
        if last != i64::from(count) {
            return Err(Error::Refused("invalid append-only receipt sequence"));
        }
        let next = last
            .checked_add(1)
            .ok_or(Error::Refused("receipt sequence exhausted"))?;
        let changed=tx.execute("INSERT INTO receipts(seq,id,slot,origin_owner,source_incarnation,scope,kind,raw) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![next,id.to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string(),scope.value(),kind.value(),raw])?;
        if changed != 1 {
            return Err(Error::Refused("raw receipt was not stored"));
        }
        // Byte equality here validates this atomic admission, never deduplicates
        // a network arrival or derives its local receipt identity from content.
        let valid:bool=tx.query_row("SELECT raw=?2 AND origin_owner=?3 AND source_incarnation=?4 AND slot=?5 AND scope=?6 AND kind=?7 AND ack='stored' AND cancelled=0 AND attempt IS NULL AND seq=?8 FROM receipts WHERE id=?1",params![id.to_string(),raw,source.owner.simple().to_string(),source.incarnation.simple().to_string(),source.slot.0,scope.value(),kind.value(),next],|r|r.get(0))?;
        if !valid {
            return Err(Error::Refused("raw receipt admission changed"));
        }
        tx.commit()?;
        Ok(id)
    }
    /// Always create a new local occurrence for each bounded admitted arrival,
    /// even for stale/foreign sources. No content-derived key is consulted.
    pub fn admit(&mut self, source: Source, kind: IncomingKind, raw: &[u8]) -> Result<ReceiptId> {
        let id = match self.persist(source, kind, raw) {
            Ok(id) => id,
            Err(e) => {
                self.revoke(source);
                return Err(e);
            }
        };
        if self.active(source) {
            if self.windows.contains_key(&source.slot) {
                self.revoke(source);
            } else if self.permission && source.test_backend {
                self.windows.insert(
                    source.slot,
                    Window {
                        source,
                        receipt: id,
                        attempt: None,
                    },
                );
            }
        }
        Ok(id)
    }
    pub fn eligible(&self, source: Source, id: ReceiptId) -> bool {
        self.permission
            && source.test_backend
            && self.active(source)
            && self
                .windows
                .get(&source.slot)
                .is_some_and(|w| w.source == source && w.receipt == id && w.attempt.is_none())
    }
    pub fn lookup(&self, id: ReceiptId) -> Result<StoredReceipt> {
        let data:(u8,String,String,String,String,Option<Vec<u8>>,String,bool)=self.db.query_row("SELECT slot,origin_owner,source_incarnation,scope,kind,CASE WHEN length(raw)<=512 THEN raw ELSE NULL END,ack,cancelled FROM receipts WHERE id=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
        if data.0 >= MAX_SLOTS {
            return Err(Error::Refused("stored slot outside bound"));
        }
        Ok(StoredReceipt {
            id,
            slot: Slot(data.0),
            origin_owner: uuid(&data.1)?,
            source_incarnation: uuid(&data.2)?,
            scope: BackendScope::parse(&data.3)?,
            kind: IncomingKind::parse(&data.4)?,
            raw: data
                .5
                .ok_or(Error::Refused("stored raw payload outside bound"))?,
            ack: AckState::parse(&data.6)?,
            cancelled: data.7,
        })
    }
    pub fn snapshot(&self, limit: usize) -> Result<Snapshot> {
        if !(1..=MAX_SNAPSHOT).contains(&limit) {
            return Err(Error::Refused("snapshot limit outside bound"));
        }
        let mut q=self.db.prepare("SELECT id,slot,kind,ack,cancelled,length(raw) FROM receipts ORDER BY seq DESC LIMIT ?1")?;
        let mut rows = q.query([limit as u32])?;
        let mut latest = Vec::with_capacity(limit);
        while let Some(r) = rows.next()? {
            let raw_length = r.get::<_, u32>(5)? as usize;
            let slot: u8 = r.get(1)?;
            if raw_length > MAX_RAW_BYTES || slot >= MAX_SLOTS {
                return Err(Error::Refused("stored receipt summary outside bound"));
            }
            latest.push(ReceiptSummary {
                id: ReceiptId(uuid(&r.get::<_, String>(0)?)?),
                slot: Slot(slot),
                kind: IncomingKind::parse(&r.get::<_, String>(2)?)?,
                ack: AckState::parse(&r.get::<_, String>(3)?)?,
                cancelled: r.get(4)?,
                raw_length,
            });
        }
        Ok(Snapshot {
            total: self.count()?,
            stopped: self.stopped,
            permission: self.permission,
            active_sources: self.sources.len(),
            pending: self.pending.len(),
            latest,
        })
    }
    fn scan_upper(&self) -> Result<i64> {
        // A bounded scalar inventory verifies the existing append-only schema.
        // Raw deletion/reordering is unsupported; no migration or retention is inferred.
        let (count,min,max):(i64,i64,i64)=self.db.query_row("SELECT count(*),COALESCE(min(seq),0),COALESCE(max(seq),0) FROM (SELECT seq FROM receipts LIMIT 4097)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if count < 0 || count > MAX_RECEIPTS as i64 || max != count || (count > 0 && min != 1) {
            return Err(Error::Refused(
                "receipt enumeration sequence outside append-only bound",
            ));
        }
        Ok(max)
    }
    pub fn scan(&self, after: Option<&ReceiptCheckpoint>) -> Result<ReceiptScan> {
        let upper = self.scan_upper()?;
        let after = if let Some(checkpoint) = after {
            if checkpoint.owner != self.owner
                || checkpoint.through < 0
                || checkpoint.through > upper
            {
                return Err(Error::Refused("stale or foreign receipt checkpoint"));
            }
            checkpoint.through
        } else {
            0
        };
        Ok(ReceiptScan {
            owner: self.owner,
            upper,
            after,
        })
    }
    pub fn scan_current(&self, scan: &ReceiptScan) -> Result<bool> {
        if scan.owner != self.owner {
            return Err(Error::Refused("stale or foreign receipt scan"));
        }
        Ok(self.scan_upper()? == scan.upper)
    }
    pub fn scan_page(&self, scan: &mut ReceiptScan, limit: usize) -> Result<ReceiptPage> {
        if scan.owner != self.owner || !(1..=MAX_SCAN_PAGE).contains(&limit) {
            return Err(Error::Refused("stale scan or page limit outside bound"));
        }
        let mut q=self.db.prepare("SELECT seq,id,slot,kind,ack,cancelled,length(raw) FROM receipts WHERE seq>?1 AND seq<=?2 ORDER BY seq LIMIT ?3")?;
        let mut rows = q.query(params![scan.after, scan.upper, limit as u32])?;
        let mut items = Vec::with_capacity(limit);
        let mut next = scan.after;
        while let Some(r) = rows.next()? {
            let seq: i64 = r.get(0)?;
            if next.checked_add(1) != Some(seq) || seq > scan.upper {
                return Err(Error::Refused("receipt scan membership changed"));
            }
            let raw_length = r.get::<_, u32>(6)? as usize;
            let slot: u8 = r.get(2)?;
            if raw_length > MAX_RAW_BYTES || slot >= MAX_SLOTS {
                return Err(Error::Refused("stored receipt summary outside bound"));
            }
            items.push(ReceiptSummary {
                id: ReceiptId(uuid(&r.get::<_, String>(1)?)?),
                slot: Slot(slot),
                kind: IncomingKind::parse(&r.get::<_, String>(3)?)?,
                ack: AckState::parse(&r.get::<_, String>(4)?)?,
                cancelled: r.get(5)?,
                raw_length,
            });
            next = seq;
        }
        if items.len() < limit && next != scan.upper {
            return Err(Error::Refused(
                "receipt scan ended before captured upper bound",
            ));
        }
        // Advance only after the entire bounded page validates; errors preserve cursor.
        scan.after = next;
        Ok(ReceiptPage {
            items,
            complete: scan.complete(),
        })
    }
    pub fn checkpoint(&self, scan: &ReceiptScan) -> Result<ReceiptCheckpoint> {
        if scan.owner != self.owner || !scan.complete() {
            return Err(Error::Refused("receipt scan incomplete or foreign"));
        }
        Ok(ReceiptCheckpoint {
            owner: self.owner,
            through: scan.upper,
        })
    }
    fn persist_claim(&mut self, source: Source, id: ReceiptId, attempt: AckAttempt) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("UPDATE receipts SET ack='uncertain',attempt=?2 WHERE id=?1 AND slot=?3 AND origin_owner=?4 AND source_incarnation=?5 AND scope='test_backend' AND ack='stored' AND cancelled=0 AND attempt IS NULL",params![id.to_string(),attempt.0.simple().to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string()])?;
        if changed != 1 {
            return Err(Error::Refused("receipt no longer eligible for ACK claim"));
        }
        let valid:bool=tx.query_row("SELECT ack='uncertain' AND attempt=?2 AND cancelled=0 AND slot=?3 AND origin_owner=?4 AND source_incarnation=?5 AND scope='test_backend' AND length(raw)<=512 FROM receipts WHERE id=?1",params![id.to_string(),attempt.0.simple().to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string()],|r|r.get(0))?;
        if !valid {
            return Err(Error::Refused("ACK uncertainty was not stored"));
        }
        tx.commit()?;
        Ok(())
    }
    /// An ACK claim never reuses a reopened receipt or refunds a lost permit.
    /// This API is confined to explicit injected/synthetic test backend scope.
    pub fn claim_test_ack(&mut self, source: Source, id: ReceiptId) -> Result<AckPermit> {
        if !self.eligible(source, id) || self.pending.len() >= MAX_PENDING {
            return Err(Error::Refused(
                "no live test ACK window or pending capacity exhausted",
            ));
        }
        let attempt = AckAttempt(uuid::Uuid::new_v4());
        if let Err(error) = self.persist_claim(source, id, attempt) {
            self.revoke(source);
            return Err(error);
        }
        self.pending.insert(
            attempt,
            Pending {
                source,
                receipt: id,
                dispatched: false,
            },
        );
        self.windows
            .get_mut(&source.slot)
            .ok_or(Error::Refused("missing current ACK window"))?
            .attempt = Some(attempt);
        Ok(AckPermit {
            owner: self.owner,
            source,
            receipt: id,
            attempt,
        })
    }
    fn current_attempt(&self, source: Source, id: ReceiptId, attempt: AckAttempt) -> bool {
        self.permission
            && source.test_backend
            && self.active(source)
            && self.windows.get(&source.slot).is_some_and(|w| {
                w.source == source && w.receipt == id && w.attempt == Some(attempt)
            })
            && self
                .pending
                .get(&attempt)
                .is_some_and(|p| p.source == source && p.receipt == id)
    }
    /// Consume the permit through the owner immediately before the injected
    /// adapter boundary. Permission, cancellation and source loss are rechecked.
    pub fn dispatch_test_ack(
        &mut self,
        permit: AckPermit,
        sink: &mut impl TestAckSink,
    ) -> Result<Outcome> {
        let AckPermit {
            owner,
            source,
            receipt,
            attempt,
        } = permit;
        if owner != self.owner
            || !self.current_attempt(source, receipt, attempt)
            || self.pending.get(&attempt).is_none_or(|p| p.dispatched)
        {
            return Ok(Outcome::Ignored);
        }
        let valid=self.db.query_row("SELECT ack='uncertain' AND attempt=?2 AND cancelled=0 AND slot=?3 AND origin_owner=?4 AND source_incarnation=?5 AND scope='test_backend' AND length(raw)<=512 FROM receipts WHERE id=?1",params![receipt.to_string(),attempt.0.simple().to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string()],|r|r.get::<_,bool>(0));
        match valid {
            Ok(true) => (),
            Ok(false) => {
                self.revoke(source);
                return Ok(Outcome::Ignored);
            }
            Err(e) => {
                self.revoke(source);
                return Err(e.into());
            }
        }
        self.pending
            .get_mut(&attempt)
            .ok_or(Error::Refused("missing ACK claim"))?
            .dispatched = true;
        let request = TestAckRequest {
            source,
            receipt,
            attempt,
        };
        if sink.dispatch(request).is_err() {
            self.revoke(source);
            return Ok(Outcome::Uncertain);
        }
        Ok(Outcome::Pending)
    }
    fn persist_test_completion(
        &mut self,
        source: Source,
        id: ReceiptId,
        attempt: AckAttempt,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("UPDATE receipts SET ack='test_backend_accepted' WHERE id=?1 AND attempt=?2 AND slot=?3 AND origin_owner=?4 AND source_incarnation=?5 AND scope='test_backend' AND ack='uncertain' AND cancelled=0",params![id.to_string(),attempt.0.simple().to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string()])?;
        if changed != 1 {
            return Err(Error::Refused("test ACK completion was not stored"));
        }
        let valid:bool=tx.query_row("SELECT ack='test_backend_accepted' AND attempt=?2 AND cancelled=0 AND slot=?3 AND origin_owner=?4 AND source_incarnation=?5 AND scope='test_backend' FROM receipts WHERE id=?1",params![id.to_string(),attempt.0.simple().to_string(),source.slot.0,source.owner.simple().to_string(),source.incarnation.simple().to_string()],|r|r.get(0))?;
        if !valid {
            return Err(Error::Refused("test ACK completion changed"));
        }
        tx.commit()?;
        Ok(())
    }
    /// Only an exact, adapter-qualified test result may finish a dispatched
    /// command. It proves no modem/network acknowledgement or receive identity.
    pub fn observe_test_ack(
        &mut self,
        source: Source,
        attempt: AckAttempt,
        signal: TestSignal,
    ) -> Result<Outcome> {
        let Some(pending) = self.pending.get(&attempt).copied() else {
            return Ok(Outcome::Ignored);
        };
        if pending.source != source
            || !pending.dispatched
            || !self.current_attempt(source, pending.receipt, attempt)
        {
            return Ok(Outcome::Ignored);
        }
        match signal {
            TestSignal::IpcReturned | TestSignal::RequestAcknowledged => Ok(Outcome::Pending),
            TestSignal::Uncertain => {
                self.revoke(source);
                Ok(Outcome::Uncertain)
            }
            TestSignal::FinalAccepted => {
                self.pending.remove(&attempt);
                self.windows.remove(&source.slot);
                if let Err(error) = self.persist_test_completion(source, pending.receipt, attempt) {
                    self.revoke(source);
                    return Err(error);
                }
                Ok(Outcome::TestBackendAccepted)
            }
        }
    }
    pub fn cancel(&mut self, id: ReceiptId) -> Result<()> {
        // Cancellation loses knowledge of the producer's last-incoming window.
        // Revoke its source before I/O; later arrivals on it stay quarantined.
        // Explicit test reattachment is local authority only, never a modem drain.
        if let Some(source) = self
            .windows
            .values()
            .find(|w| w.receipt == id)
            .map(|w| w.source)
        {
            self.revoke(source);
        }
        // Local cancellation fences held permits even if durable storage fails.
        self.windows.retain(|_, w| w.receipt != id);
        self.pending.retain(|_, p| p.receipt != id);
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE receipts SET cancelled=1 WHERE id=?1",
            [id.to_string()],
        )?;
        if changed != 1
            || !tx.query_row(
                "SELECT cancelled FROM receipts WHERE id=?1",
                [id.to_string()],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err(Error::Refused("receipt cancellation was not stored"));
        }
        tx.commit()?;
        Ok(())
    }
}
