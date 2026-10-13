//! Local receipt projection and display grouping only. No raw PDU store, ACK,
//! network identity, transport retry, multipart assembly or delivery authority.
#![cfg(target_os = "linux")]
use pokelinux_messaging::{
    Encoding,
    receipt::{IncomingKind, ReceiptId, ReceiptOwner, Slot, StoredReceipt},
    receive::{IncomingConcatenation, ReceiveError, ReferenceWidth, decode_delivery},
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior, params};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
    sync::{Arc, Mutex},
};
pub const MAX_KEYS: usize = 4096;
pub const MAX_TEXT_MESSAGES: usize = 2048;
pub const MAX_CONVERSATIONS: usize = 512;
pub const MAX_PAGE: usize = 32;
pub const MAX_BODY_BYTES: usize = 640;
pub const MAX_SENDER_BYTES: usize = 64;
#[derive(Debug)]
pub enum Error {
    Refused(&'static str),
    Io(std::io::Error),
    Database(rusqlite::Error),
    Receipt(pokelinux_messaging::receipt::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReceiptKey(uuid::Uuid);
impl From<ReceiptId> for ReceiptKey {
    fn from(id: ReceiptId) -> Self {
        Self(
            uuid::Uuid::parse_str(&id.to_string())
                .expect("opaque receipt UUID has stable canonical representation"),
        )
    }
}
impl std::fmt::Display for ReceiptKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.simple())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationId(uuid::Uuid);
impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.simple())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionState {
    Text,
    Malformed,
    Unsupported,
    StatusReport,
    Cancelled,
    Deleted,
    Retained,
}
impl ProjectionState {
    fn value(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Malformed => "malformed",
            Self::Unsupported => "unsupported",
            Self::StatusReport => "status_report",
            Self::Cancelled => "cancelled",
            Self::Deleted => "deleted",
            Self::Retained => "retained",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "text" => Ok(Self::Text),
            "malformed" => Ok(Self::Malformed),
            "unsupported" => Ok(Self::Unsupported),
            "status_report" => Ok(Self::StatusReport),
            "cancelled" => Ok(Self::Cancelled),
            "deleted" => Ok(Self::Deleted),
            "retained" => Ok(Self::Retained),
            _ => Err(Error::Refused("invalid projection state")),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ImportResult {
    pub receipt: ReceiptKey,
    pub state: ProjectionState,
    pub conversation: Option<ConversationId>,
    pub existing: bool,
}
struct Access {
    owner: uuid::Uuid,
    import_epoch: u64,
    read_epoch: u64,
    import_allowed: bool,
    read_allowed: bool,
    stopped: bool,
}
#[derive(Clone)]
pub struct Control(Arc<Mutex<Access>>);
#[derive(Clone, Copy)]
pub struct ImportLease {
    owner: uuid::Uuid,
    epoch: u64,
}
#[derive(Clone, Copy)]
pub struct ReadLease {
    owner: uuid::Uuid,
    epoch: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct ControlSnapshot {
    pub import_allowed: bool,
    pub read_allowed: bool,
    pub stopped: bool,
}
impl Access {
    fn advance(&mut self, import: bool) -> Result<()> {
        let current = if import {
            self.import_epoch
        } else {
            self.read_epoch
        };
        if let Some(next) = current.checked_add(1) {
            if import {
                self.import_epoch = next
            } else {
                self.read_epoch = next
            }
            Ok(())
        } else {
            self.stopped = true;
            self.read_allowed = false;
            self.import_allowed = false;
            Err(Error::Refused("access generation exhausted"))
        }
    }
}
impl Control {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Access {
            owner: uuid::Uuid::new_v4(),
            import_epoch: 1,
            read_epoch: 1,
            import_allowed: false,
            read_allowed: false,
            stopped: false,
        })))
    }
    fn access(&self) -> Result<std::sync::MutexGuard<'_, Access>> {
        self.0
            .lock()
            .map_err(|_| Error::Refused("local access authority failed"))
    }
    pub fn set_import_allowed(&self, allowed: bool) -> Result<()> {
        let mut a = self.access()?;
        if a.stopped {
            return Err(Error::Refused("inbox stopped"));
        }
        if a.import_allowed != allowed {
            a.advance(true)?;
            a.import_allowed = allowed
        }
        Ok(())
    }
    pub fn set_read_allowed(&self, allowed: bool) -> Result<()> {
        let mut a = self.access()?;
        if a.stopped {
            return Err(Error::Refused("inbox stopped"));
        }
        if a.read_allowed != allowed {
            a.advance(false)?;
            a.read_allowed = allowed
        }
        Ok(())
    }
    pub fn import_lease(&self) -> Result<ImportLease> {
        let a = self.access()?;
        if !a.import_allowed || a.stopped {
            return Err(Error::Refused("local import denied"));
        }
        Ok(ImportLease {
            owner: a.owner,
            epoch: a.import_epoch,
        })
    }
    pub fn read_lease(&self) -> Result<ReadLease> {
        let a = self.access()?;
        if !a.read_allowed || a.stopped {
            return Err(Error::Refused("local read denied"));
        }
        Ok(ReadLease {
            owner: a.owner,
            epoch: a.read_epoch,
        })
    }
    fn check_import(&self, l: &ImportLease) -> Result<()> {
        let a = self.access()?;
        if !a.stopped && a.import_allowed && a.owner == l.owner && a.import_epoch == l.epoch {
            Ok(())
        } else {
            Err(Error::Refused("local import revoked"))
        }
    }
    fn check_read(&self, l: &ReadLease) -> Result<()> {
        let a = self.access()?;
        if !a.stopped && a.read_allowed && a.owner == l.owner && a.read_epoch == l.epoch {
            Ok(())
        } else {
            Err(Error::Refused("local read revoked"))
        }
    }
    pub fn stop(&self) -> Result<()> {
        let mut a = self.access()?;
        a.stopped = true;
        a.import_allowed = false;
        a.read_allowed = false;
        Ok(())
    }
    pub fn snapshot(&self) -> Result<ControlSnapshot> {
        let a = self.access()?;
        Ok(ControlSnapshot {
            import_allowed: a.import_allowed,
            read_allowed: a.read_allowed,
            stopped: a.stopped,
        })
    }
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
        return Err(Error::Refused("private regular journal files required"));
    }
    Ok(file)
}
pub struct Inbox {
    db: Connection,
    control: Control,
    _lock: File,
}
impl Inbox {
    pub fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() {
            return Err(Error::Refused("absolute state directory required"));
        }
        match fs::DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => File::open(directory.parent().ok_or(Error::Refused("missing parent"))?)?
                .sync_all()?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o777 != 0o700
            || fs::canonicalize(directory)? != directory
        {
            return Err(Error::Refused(
                "private state directory without symlinks required",
            ));
        }
        let lock = private_file(&directory.join("owner.lock"), false)?;
        lock.try_lock()
            .map_err(|_| Error::Refused("inbox already owned or locking unavailable"))?;
        let path = directory.join("inbox.sqlite3");
        // A crash may leave a hot rollback journal; inspect it before SQLite
        // recovery. The private directory excludes other UIDs, not same-UID apps.
        let journal = directory.join("inbox.sqlite3-journal");
        match fs::symlink_metadata(&journal) {
            Ok(_) => {
                private_file(&journal, false)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        let (file, fresh) = match private_file(&path, true) {
            Ok(file) => (file, true),
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                (private_file(&path, false)?, false)
            }
            Err(error) => return Err(error),
        };
        let mut db = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        db.busy_timeout(std::time::Duration::ZERO)?;
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA secure_delete=ON; PRAGMA max_page_count=32768;")?;
        if fresh {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE conversations(id TEXT PRIMARY KEY CHECK(length(id)=32), slot INTEGER NOT NULL CHECK(slot BETWEEN 0 AND 7), sender_kind INTEGER NOT NULL CHECK(sender_kind IN(129,145,161,208)), sender TEXT NOT NULL CHECK(length(CAST(sender AS BLOB)) BETWEEN 1 AND 64), UNIQUE(slot,sender_kind,sender)) STRICT;
CREATE TABLE records(seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL CHECK(length(id)=32), slot INTEGER NOT NULL CHECK(slot BETWEEN 0 AND 7), origin_owner TEXT NOT NULL CHECK(length(origin_owner)=32), source_incarnation TEXT NOT NULL CHECK(length(source_incarnation)=32), kind INTEGER NOT NULL CHECK(kind IN(0,1)), state TEXT NOT NULL CHECK(state IN('text','malformed','unsupported','status_report','cancelled','deleted','retained')), conversation TEXT REFERENCES conversations(id), body TEXT CHECK(length(CAST(body AS BLOB))<=640), encoding INTEGER CHECK(encoding IN(0,1)), timestamp INTEGER, concat_width INTEGER CHECK(concat_width IN(8,16)), concat_ref INTEGER CHECK(concat_ref BETWEEN 0 AND 65535), concat_total INTEGER CHECK(concat_total BETWEEN 1 AND 255), concat_sequence INTEGER CHECK(concat_sequence BETWEEN 1 AND concat_total), reply INTEGER CHECK(reply IN(0,1)), class INTEGER CHECK(class BETWEEN 0 AND 3), CHECK((state='text' AND conversation IS NOT NULL AND body IS NOT NULL AND encoding IS NOT NULL AND timestamp IS NOT NULL AND reply IS NOT NULL) OR (state!='text' AND conversation IS NULL AND body IS NULL AND encoding IS NULL AND timestamp IS NULL AND concat_width IS NULL AND concat_ref IS NULL AND concat_total IS NULL AND concat_sequence IS NULL AND reply IS NULL AND class IS NULL)), CHECK((concat_width IS NULL AND concat_ref IS NULL AND concat_total IS NULL AND concat_sequence IS NULL) OR (concat_width IS NOT NULL AND concat_ref IS NOT NULL AND concat_total IS NOT NULL AND concat_sequence IS NOT NULL))) STRICT;
CREATE INDEX records_conversation ON records(conversation,seq);
PRAGMA user_version=1;")?;
            tx.commit()?;
        } else if db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? != 1 {
            return Err(Error::Refused("unsupported inbox schema; no migrations"));
        }
        // Validate actual connection settings; unknown PRAGMAs can be ignored.
        if db.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))? != "delete"
            || db.pragma_query_value(None, "synchronous", |r| r.get::<_, u32>(0))? != 3
            || db.pragma_query_value(None, "foreign_keys", |r| r.get::<_, u32>(0))? != 1
        {
            return Err(Error::Refused("durable journal configuration unavailable"));
        }
        file.sync_all()?;
        File::open(directory)?.sync_all()?;
        counts(&db)?;
        Ok(Self {
            db,
            control: Control::new(),
            _lock: lock,
        })
    }

    pub fn control(&self) -> Control {
        self.control.clone()
    }
    pub fn import(
        &mut self,
        lease: &ImportLease,
        receipts: &ReceiptOwner,
        id: ReceiptId,
    ) -> Result<ImportResult> {
        self.control.check_import(lease)?;
        let receipt = receipts.lookup(id).map_err(Error::Receipt)?;
        let decoded = Decoded::receipt(&receipt)?;
        let key = ReceiptKey::from(id);
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(r) = record(&tx, key)? {
            r.validate()?;
            if !r.provenance(&receipt) {
                return Err(Error::Refused(
                    "local receipt key reused with changed provenance",
                ));
            }
            if matches!(
                r.validate()?,
                ProjectionState::Deleted | ProjectionState::Retained
            ) {
                let result = r.result(true)?;
                drop(tx);
                self.control.check_import(lease)?;
                return Ok(result);
            }
            if receipt.cancelled && r.validate()? != ProjectionState::Cancelled {
                erase(&tx, key, ProjectionState::Cancelled)?;
                let result = record(&tx, key)?
                    .ok_or(Error::Refused("cancelled projection absent"))?
                    .result(true)?;
                self.control.check_import(lease)?;
                tx.commit()?;
                self.control.check_import(lease)?;
                return Ok(result);
            }
            if !decoded.matches(&r)? {
                return Err(Error::Refused(
                    "local receipt key reused with changed decoded content",
                ));
            }
            let result = r.result(true)?;
            drop(tx);
            self.control.check_import(lease)?;
            return Ok(result);
        }
        let count = counts(&tx)?;
        if count.keys >= MAX_KEYS {
            return Err(Error::Refused("receipt tombstone capacity exhausted"));
        }
        let conversation = if let Some(v) = &decoded.received {
            if count.text_messages >= MAX_TEXT_MESSAGES {
                return Err(Error::Refused("text retention capacity exhausted"));
            }
            let kind = decoded
                .sender_kind
                .ok_or(Error::Refused("sender kind absent"))?;
            let existing: Option<String> = tx
                .query_row(
                    "SELECT id FROM conversations WHERE slot=?1 AND sender_kind=?2 AND sender=?3",
                    params![receipt.slot.0, kind as u8, v.sender],
                    |r| r.get(0),
                )
                .optional()?;
            Some(if let Some(value) = existing {
                uuid(&value)?
            } else {
                if count.conversations >= MAX_CONVERSATIONS {
                    return Err(Error::Refused("conversation retention capacity exhausted"));
                }
                let value = uuid::Uuid::new_v4();
                if tx.execute(
                    "INSERT INTO conversations(id,slot,sender_kind,sender) VALUES(?1,?2,?3,?4)",
                    params![
                        value.simple().to_string(),
                        receipt.slot.0,
                        kind as u8,
                        v.sender
                    ],
                )? != 1
                {
                    return Err(Error::Refused("conversation not stored"));
                }
                value
            })
        } else {
            None
        };
        let v = decoded.received.as_ref();
        let concat = v.and_then(|v| v.concatenation);
        if tx.execute("INSERT INTO records(id,slot,origin_owner,source_incarnation,kind,state,conversation,body,encoding,timestamp,concat_width,concat_ref,concat_total,concat_sequence,reply,class) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![id.to_string(),receipt.slot.0,receipt.origin_owner.simple().to_string(),receipt.source_incarnation.simple().to_string(),u8::from(receipt.kind==IncomingKind::StatusReport),decoded.state.value(),conversation.map(|id|id.simple().to_string()),v.map(|v|v.text.as_str()),v.map(|v|u8::from(v.encoding==Encoding::Utf16)),v.map(|v|v.timestamp_millis),concat.map(|v|if v.width==ReferenceWidth::Eight{8}else{16}),concat.map(|v|v.reference),concat.map(|v|v.total),concat.map(|v|v.sequence),v.map(|v|v.reply_path),v.and_then(|v|v.message_class)])?!=1{return Err(Error::Refused("projection not stored"))}
        let stored = record(&tx, key)?.ok_or(Error::Refused("projection absent"))?;
        if !stored.provenance(&receipt) || !decoded.matches(&stored)? {
            return Err(Error::Refused("projection admission changed"));
        }
        let result = stored.result(false)?;
        self.control.check_import(lease)?;
        tx.commit()?;
        self.control.check_import(lease)?;
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SenderKind {
    UnknownNumeric = 0x81,
    InternationalNumeric = 0x91,
    NationalNumeric = 0xa1,
    Alphanumeric = 0xd0,
}
impl SenderKind {
    fn parse(v: u8) -> Result<Self> {
        match v {
            0x81 => Ok(Self::UnknownNumeric),
            0x91 => Ok(Self::InternationalNumeric),
            0xa1 => Ok(Self::NationalNumeric),
            0xd0 => Ok(Self::Alphanumeric),
            _ => Err(Error::Refused("unsupported sender kind")),
        }
    }
}
// Sensitive decoded content deliberately has no blanket Debug implementation.
pub struct Message {
    pub receipt: ReceiptKey,
    pub ordinal: u64,
    pub conversation: ConversationId,
    pub slot: Slot,
    pub sender: String,
    pub sender_kind: SenderKind,
    pub text: String,
    pub encoding: Encoding,
    pub timestamp_millis: i64,
    pub concatenation: Option<IncomingConcatenation>,
    pub reply_path: bool,
    pub message_class: Option<u8>,
}
pub struct Conversation {
    pub id: ConversationId,
    pub slot: Slot,
    pub sender: String,
    pub sender_kind: SenderKind,
    pub messages: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub keys: usize,
    pub text_messages: usize,
    pub conversations: usize,
}
fn uuid(value: &str) -> Result<uuid::Uuid> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Refused("invalid stored local UUID"));
    }
    uuid::Uuid::parse_str(value).map_err(|_| Error::Refused("invalid stored local UUID"))
}
fn page(limit: usize) -> Result<u32> {
    if !(1..=MAX_PAGE).contains(&limit) {
        return Err(Error::Refused("page outside bound"));
    }
    Ok(limit as u32)
}
fn counts(db: &Connection) -> Result<Stats> {
    let value = Stats {
        keys: db.query_row(
            "SELECT count(*) FROM (SELECT id FROM records LIMIT 4097)",
            [],
            |r| r.get::<_, u32>(0),
        )? as usize,
        text_messages: db.query_row(
            "SELECT count(*) FROM (SELECT id FROM records WHERE state='text' LIMIT 2049)",
            [],
            |r| r.get::<_, u32>(0),
        )? as usize,
        conversations: db.query_row(
            "SELECT count(*) FROM (SELECT id FROM conversations LIMIT 513)",
            [],
            |r| r.get::<_, u32>(0),
        )? as usize,
    };
    if value.keys > MAX_KEYS
        || value.text_messages > MAX_TEXT_MESSAGES
        || value.conversations > MAX_CONVERSATIONS
    {
        return Err(Error::Refused("stored inbox exceeds bound"));
    }
    Ok(value)
}
const RECORD_QUERY: &str = "SELECT r.seq,r.id,r.slot,r.origin_owner,r.source_incarnation,r.kind,r.state,r.conversation,CASE WHEN length(CAST(r.body AS BLOB))<=640 THEN r.body ELSE NULL END,r.encoding,r.timestamp,r.concat_width,r.concat_ref,r.concat_total,r.concat_sequence,r.reply,r.class,c.slot,c.sender_kind,CASE WHEN length(CAST(c.sender AS BLOB))<=64 THEN c.sender ELSE NULL END FROM records r LEFT JOIN conversations c ON r.conversation=c.id";
struct Record {
    ordinal: i64,
    id: String,
    slot: u8,
    origin: String,
    source: String,
    kind: u8,
    state: String,
    conversation: Option<String>,
    body: Option<String>,
    encoding: Option<u8>,
    timestamp: Option<i64>,
    width: Option<u8>,
    reference: Option<u16>,
    total: Option<u8>,
    sequence: Option<u8>,
    reply: Option<bool>,
    class: Option<u8>,
    conversation_slot: Option<u8>,
    sender_kind: Option<u8>,
    sender: Option<String>,
}
impl Record {
    fn read(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            ordinal: r.get(0)?,
            id: r.get(1)?,
            slot: r.get(2)?,
            origin: r.get(3)?,
            source: r.get(4)?,
            kind: r.get(5)?,
            state: r.get(6)?,
            conversation: r.get(7)?,
            body: r.get(8)?,
            encoding: r.get(9)?,
            timestamp: r.get(10)?,
            width: r.get(11)?,
            reference: r.get(12)?,
            total: r.get(13)?,
            sequence: r.get(14)?,
            reply: r.get(15)?,
            class: r.get(16)?,
            conversation_slot: r.get(17)?,
            sender_kind: r.get(18)?,
            sender: r.get(19)?,
        })
    }
    fn validate(&self) -> Result<ProjectionState> {
        uuid(&self.id)?;
        uuid(&self.origin)?;
        uuid(&self.source)?;
        if self.ordinal < 1 || self.slot > 7 || self.kind > 1 {
            return Err(Error::Refused("invalid local receipt provenance"));
        }
        let state = ProjectionState::parse(&self.state)?;
        if state == ProjectionState::Text {
            self.message()?;
        } else if self.conversation.is_some()
            || self.body.is_some()
            || self.encoding.is_some()
            || self.timestamp.is_some()
            || self.width.is_some()
            || self.reference.is_some()
            || self.total.is_some()
            || self.sequence.is_some()
            || self.reply.is_some()
            || self.class.is_some()
        {
            return Err(Error::Refused("excluded projection retains content"));
        }
        Ok(state)
    }
    fn message(&self) -> Result<Message> {
        if self.state != "text"
            || self.kind != 0
            || self.slot > 7
            || self.conversation_slot != Some(self.slot)
        {
            return Err(Error::Refused("invalid text conversation"));
        }
        let sender = self
            .sender
            .as_ref()
            .ok_or(Error::Refused("missing sender"))?;
        let body = self.body.as_ref().ok_or(Error::Refused("missing text"))?;
        if sender.is_empty() || sender.len() > MAX_SENDER_BYTES || body.len() > MAX_BODY_BYTES {
            return Err(Error::Refused("stored text outside bound"));
        }
        let encoding = match self.encoding {
            Some(0) => Encoding::Gsm7,
            Some(1) => Encoding::Utf16,
            _ => return Err(Error::Refused("invalid text encoding")),
        };
        let concatenation = match (self.width, self.reference, self.total, self.sequence) {
            (None, None, None, None) => None,
            (Some(width), Some(reference), Some(total), Some(sequence))
                if total > 0 && sequence > 0 && sequence <= total =>
            {
                Some(IncomingConcatenation {
                    width: match width {
                        8 if reference <= 255 => ReferenceWidth::Eight,
                        16 => ReferenceWidth::Sixteen,
                        _ => return Err(Error::Refused("invalid concat reference")),
                    },
                    reference,
                    total,
                    sequence,
                })
            }
            _ => return Err(Error::Refused("invalid concat metadata")),
        };
        if self.class.is_some_and(|v| v > 3) {
            return Err(Error::Refused("invalid message class"));
        }
        Ok(Message {
            receipt: ReceiptKey(uuid(&self.id)?),
            ordinal: u64::try_from(self.ordinal)
                .map_err(|_| Error::Refused("invalid local ordinal"))?,
            conversation: ConversationId(uuid(
                self.conversation
                    .as_deref()
                    .ok_or(Error::Refused("missing conversation"))?,
            )?),
            slot: Slot(self.slot),
            sender: sender.clone(),
            sender_kind: SenderKind::parse(
                self.sender_kind
                    .ok_or(Error::Refused("missing sender kind"))?,
            )?,
            text: body.clone(),
            encoding,
            timestamp_millis: self.timestamp.ok_or(Error::Refused("missing timestamp"))?,
            concatenation,
            reply_path: self.reply.ok_or(Error::Refused("missing reply path"))?,
            message_class: self.class,
        })
    }
    fn provenance(&self, r: &StoredReceipt) -> bool {
        self.id == r.id.to_string()
            && self.slot == r.slot.0
            && self.origin == r.origin_owner.simple().to_string()
            && self.source == r.source_incarnation.simple().to_string()
            && self.kind == u8::from(r.kind == IncomingKind::StatusReport)
    }
    fn same_provenance(&self, r: &Self) -> bool {
        self.id == r.id
            && self.slot == r.slot
            && self.origin == r.origin
            && self.source == r.source
            && self.kind == r.kind
            && self.ordinal == r.ordinal
    }
    fn result(&self, existing: bool) -> Result<ImportResult> {
        Ok(ImportResult {
            receipt: ReceiptKey(uuid(&self.id)?),
            state: self.validate()?,
            conversation: self
                .conversation
                .as_deref()
                .map(uuid)
                .transpose()?
                .map(ConversationId),
            existing,
        })
    }
}
fn record(db: &Connection, key: ReceiptKey) -> Result<Option<Record>> {
    Ok(db
        .query_row(
            &format!("{RECORD_QUERY} WHERE r.id=?1"),
            [key.to_string()],
            Record::read,
        )
        .optional()?)
}
struct Decoded {
    state: ProjectionState,
    sender_kind: Option<SenderKind>,
    received: Option<pokelinux_messaging::receive::Received>,
}
impl Decoded {
    fn receipt(r: &StoredReceipt) -> Result<Self> {
        let (state, received) = if r.cancelled {
            (ProjectionState::Cancelled, None)
        } else if r.kind == IncomingKind::StatusReport {
            (ProjectionState::StatusReport, None)
        } else {
            match decode_delivery(r.raw()) {
                Ok(v) => (ProjectionState::Text, Some(v)),
                Err(ReceiveError::Malformed) => (ProjectionState::Malformed, None),
                Err(ReceiveError::Unsupported) => (ProjectionState::Unsupported, None),
            }
        };
        let sender_kind = if let Some(v) = &received {
            if v.sender.is_empty()
                || v.sender.len() > MAX_SENDER_BYTES
                || v.text.len() > MAX_BODY_BYTES
            {
                return Err(Error::Refused("decoded text outside inbox bounds"));
            }
            Some(SenderKind::parse(
                *r.raw()
                    .get(usize::from(r.raw()[0]) + 3)
                    .ok_or(Error::Refused("qualified sender kind absent"))?,
            )?)
        } else {
            None
        };
        Ok(Self {
            state,
            sender_kind,
            received,
        })
    }
    fn matches(&self, r: &Record) -> Result<bool> {
        if r.validate()? != self.state {
            return Ok(false);
        }
        match &self.received {
            Some(v) => {
                let m = r.message()?;
                Ok(m.sender == v.sender
                    && Some(m.sender_kind) == self.sender_kind
                    && m.text == v.text
                    && m.encoding == v.encoding
                    && m.timestamp_millis == v.timestamp_millis
                    && m.concatenation == v.concatenation
                    && m.reply_path == v.reply_path
                    && m.message_class == v.message_class)
            }
            None => Ok(true),
        }
    }
}
fn erase(db: &Connection, key: ReceiptKey, state: ProjectionState) -> Result<()> {
    if !matches!(
        state,
        ProjectionState::Cancelled | ProjectionState::Deleted | ProjectionState::Retained
    ) {
        return Err(Error::Refused("invalid content removal"));
    }
    let before = record(db, key)?.ok_or(Error::Refused("unknown receipt key"))?;
    if db.execute("UPDATE records SET state=?2,conversation=NULL,body=NULL,encoding=NULL,timestamp=NULL,concat_width=NULL,concat_ref=NULL,concat_total=NULL,concat_sequence=NULL,reply=NULL,class=NULL WHERE id=?1",params![key.to_string(),state.value()])?!=1{return Err(Error::Refused("projection removal not stored"))}
    let after = record(db, key)?.ok_or(Error::Refused("projection removal missing"))?;
    if !before.same_provenance(&after) || after.validate()? != state {
        return Err(Error::Refused("projection removal changed"));
    }
    db.execute("DELETE FROM conversations WHERE NOT EXISTS(SELECT 1 FROM records WHERE conversation=conversations.id)",[])?;
    if db.query_row("SELECT EXISTS(SELECT 1 FROM conversations WHERE NOT EXISTS(SELECT 1 FROM records WHERE conversation=conversations.id))",[],|r|r.get::<_,bool>(0))?{return Err(Error::Refused("private sender label cleanup not stored"))}
    Ok(())
}
impl Inbox {
    pub fn stats(&self, lease: &ReadLease) -> Result<Stats> {
        self.control.check_read(lease)?;
        let value = counts(&self.db)?;
        self.control.check_read(lease)?;
        Ok(value)
    }
    pub fn state(&self, lease: &ReadLease, key: ReceiptKey) -> Result<Option<ProjectionState>> {
        self.control.check_read(lease)?;
        let value = record(&self.db, key)?.map(|r| r.validate()).transpose()?;
        self.control.check_read(lease)?;
        Ok(value)
    }
    pub fn message(&self, lease: &ReadLease, key: ReceiptKey) -> Result<Option<Message>> {
        self.control.check_read(lease)?;
        let value = match record(&self.db, key)? {
            Some(r) if r.validate()? == ProjectionState::Text => Some(r.message()?),
            _ => None,
        };
        self.control.check_read(lease)?;
        Ok(value)
    }
    pub fn conversations(
        &self,
        lease: &ReadLease,
        after: Option<ConversationId>,
        limit: usize,
    ) -> Result<Vec<Conversation>> {
        self.control.check_read(lease)?;
        let limit = page(limit)?;
        let mut q=self.db.prepare("SELECT c.id,c.slot,c.sender_kind,CASE WHEN length(CAST(c.sender AS BLOB))<=64 THEN c.sender ELSE NULL END,count(r.id) FROM conversations c JOIN records r ON r.conversation=c.id WHERE c.id>?1 GROUP BY c.id ORDER BY c.id LIMIT ?2")?;
        let mut rows = q.query(params![
            after.map(|id| id.to_string()).unwrap_or_default(),
            limit
        ])?;
        let mut out = Vec::with_capacity(limit as usize);
        while let Some(r) = rows.next()? {
            let slot: u8 = r.get(1)?;
            let sender: String = r.get(3)?;
            let messages: u32 = r.get(4)?;
            if slot > 7
                || sender.is_empty()
                || sender.len() > MAX_SENDER_BYTES
                || messages == 0
                || messages > MAX_TEXT_MESSAGES as u32
            {
                return Err(Error::Refused("conversation outside bounds"));
            }
            out.push(Conversation {
                id: ConversationId(uuid(&r.get::<_, String>(0)?)?),
                slot: Slot(slot),
                sender_kind: SenderKind::parse(r.get(2)?)?,
                sender,
                messages: messages as usize,
            })
        }
        drop(rows);
        drop(q);
        self.control.check_read(lease)?;
        Ok(out)
    }
    pub fn messages(
        &self,
        lease: &ReadLease,
        conversation: ConversationId,
        after_ordinal: u64,
        limit: usize,
    ) -> Result<Vec<Message>> {
        self.control.check_read(lease)?;
        let limit = page(limit)?;
        let after =
            i64::try_from(after_ordinal).map_err(|_| Error::Refused("ordinal outside bound"))?;
        let mut q = self.db.prepare(&format!(
            "{RECORD_QUERY} WHERE r.conversation=?1 AND r.seq>?2 ORDER BY r.seq LIMIT ?3"
        ))?;
        let mut rows = q.query(params![conversation.to_string(), after, limit])?;
        let mut out = Vec::with_capacity(limit as usize);
        while let Some(r) = rows.next()? {
            let r = Record::read(r)?;
            if r.validate()? != ProjectionState::Text {
                return Err(Error::Refused("conversation contains excluded receipt"));
            }
            out.push(r.message()?)
        }
        drop(rows);
        drop(q);
        self.control.check_read(lease)?;
        Ok(out)
    }
    pub fn delete(&mut self, lease: &ImportLease, key: ReceiptKey) -> Result<()> {
        self.control.check_import(lease)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        erase(&tx, key, ProjectionState::Deleted)?;
        self.control.check_import(lease)?;
        tx.commit()?;
        self.control.check_import(lease)?;
        Ok(())
    }
    pub fn retain_latest(
        &mut self,
        lease: &ImportLease,
        keep: usize,
        batch: usize,
    ) -> Result<usize> {
        self.control.check_import(lease)?;
        if keep > MAX_TEXT_MESSAGES {
            return Err(Error::Refused("retention outside bounds"));
        }
        let batch = page(batch)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let keys = {
            let mut q = tx.prepare(
                "SELECT id FROM records WHERE state='text' ORDER BY seq DESC LIMIT ?1 OFFSET ?2",
            )?;
            let values = q
                .query_map(params![batch, keep as u32], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            values
                .into_iter()
                .map(|s| uuid(&s).map(ReceiptKey))
                .collect::<Result<Vec<_>>>()?
        };
        for key in &keys {
            erase(&tx, *key, ProjectionState::Retained)?
        }
        self.control.check_import(lease)?;
        tx.commit()?;
        self.control.check_import(lease)?;
        Ok(keys.len())
    }
}
#[cfg(test)]
mod access_storage_tests {
    use super::*;
    use pokelinux_messaging::receipt::BackendScope;
    struct TestState(std::path::PathBuf);
    impl Drop for TestState {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn prepared() -> (TestState, ReceiptOwner, Inbox, ReceiptId) {
        let path = std::env::temp_dir().join(format!("inbox-gate-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        let mut receipts = ReceiptOwner::open(&path.join("raw")).unwrap();
        let source = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
        let bytes = "000008915155100000006201213141510005e8329bfd06";
        let bytes: Vec<_> = (0..bytes.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&bytes[i..i + 2], 16).unwrap())
            .collect();
        let id = receipts
            .admit(source, IncomingKind::Deliver, &bytes)
            .unwrap();
        let inbox = Inbox::open(&path.join("projection")).unwrap();
        (TestState(path), receipts, inbox, id)
    }
    #[test]
    fn read_generation_is_rechecked_after_actual_sqlite_work_before_text_publication() {
        let (_d, receipts, mut inbox, id) = prepared();
        let c = inbox.control();
        c.set_import_allowed(true).unwrap();
        inbox
            .import(&c.import_lease().unwrap(), &receipts, id)
            .unwrap();
        c.set_read_allowed(true).unwrap();
        let lease = c.read_lease().unwrap();
        let changed = c.clone();
        inbox
            .db
            .progress_handler(
                1,
                Some(move || {
                    changed.set_read_allowed(false).unwrap();
                    false
                }),
            )
            .unwrap();
        assert!(
            inbox.message(&lease, ReceiptKey::from(id)).is_err(),
            "read revoked inside SQLite must not publish decoded text"
        );
    }
    #[test]
    fn import_generation_is_rechecked_during_actual_sqlite_transaction() {
        let (_d, receipts, mut inbox, id) = prepared();
        let c = inbox.control();
        c.set_import_allowed(true).unwrap();
        let lease = c.import_lease().unwrap();
        let changed = c.clone();
        inbox
            .db
            .progress_handler(
                1,
                Some(move || {
                    changed.set_import_allowed(false).unwrap();
                    false
                }),
            )
            .unwrap();
        assert!(
            inbox.import(&lease, &receipts, id).is_err(),
            "import revoked inside SQLite must not publish projection success"
        );
        let count: u32 = inbox
            .db
            .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 0,
            "revocation observed before commit must roll back admission"
        );
    }
    #[test]
    fn import_generation_is_rechecked_after_actual_commit_even_when_storage_cannot_be_interrupted()
    {
        let (d, receipts, mut inbox, id) = prepared();
        let c = inbox.control();
        c.set_import_allowed(true).unwrap();
        let lease = c.import_lease().unwrap();
        let changed = c.clone();
        inbox
            .db
            .commit_hook(Some(move || {
                changed.set_import_allowed(false).unwrap();
                false
            }))
            .unwrap();
        assert!(
            inbox.import(&lease, &receipts, id).is_err(),
            "revocation during SQLite commit must suppress publication afterward"
        );
        let reader = Connection::open(d.0.join("projection/inbox.sqlite3")).unwrap();
        let count: u32 = reader
            .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 1,
            "actual committed projection remains durable; no pretend rollback after commit"
        );
    }
}
#[cfg(test)]
mod generation_exhaustion_test {
    use super::*;
    #[test]
    fn generation_exhaustion_cannot_leave_old_authority_active() {
        let c = Control::new();
        c.set_import_allowed(true).unwrap();
        c.access().unwrap().import_epoch = u64::MAX;
        let lease = c.import_lease().unwrap();
        assert!(c.set_import_allowed(false).is_err());
        assert!(
            c.check_import(&lease).is_err(),
            "exhausted generation must revoke old authority"
        );
        assert!(c.snapshot().unwrap().stopped);
    }
}
