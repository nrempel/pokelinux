//! Durable, single-owner SMS dispatch admission. A permit is returned only after
//! committing uncertainty; dropping it, cancellation and restart grant no retry.
//! App permissions and actual modem/IMS result qualification remain separate.
use crate::plan_submit;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
};
pub const MAX_MESSAGES: usize = 4096;
#[derive(Debug)]
pub enum Error {
    Refused(&'static str),
    Io(std::io::Error),
    Database(rusqlite::Error),
    Payload(crate::Error),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartState {
    Ready,
    Uncertain,
    TransportAccepted,
    NotSubmitted,
}
impl PartState {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "ready" => Ok(Self::Ready),
            "uncertain" => Ok(Self::Uncertain),
            "accepted" => Ok(Self::TransportAccepted),
            "not_submitted" => Ok(Self::NotSubmitted),
            _ => Err(Error::Refused("invalid stored part state")),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub cancelled: bool,
    pub reference: Option<u16>,
    pub parts: Vec<PartState>,
}
/// Only a qualified transport adapter may classify these outcomes. Accepted
/// means transport acceptance, never delivery; NotSubmitted requires proof that
/// remote submission did not occur. Neither result permits retry of this part.
#[derive(Debug, Clone, Copy)]
pub enum TransportResult {
    Accepted,
    NotSubmitted,
}
/// Non-cloneable, owner-specific one-use dispatch permission. Payloads and
/// message identities are intentionally absent from Debug output.
pub struct DispatchPermit {
    owner: uuid::Uuid,
    id: String,
    part: usize,
    tpdu: Vec<u8>,
}
impl DispatchPermit {
    pub fn tpdu(&self) -> &[u8] {
        &self.tpdu
    }
}
pub struct Outbox {
    db: Connection,
    owner: uuid::Uuid,
    // Close SQLite before releasing the permanent lock inode.
    _lock: File,
}
fn identity(id: &str) -> Result<()> {
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Refused("32 lowercase hex identity required"));
    }
    Ok(())
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
impl Outbox {
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
            .map_err(|_| Error::Refused("outbox already owned or locking unavailable"))?;
        let path = directory.join("outbox.sqlite3");
        // A crash may leave a hot rollback journal; inspect it before SQLite
        // recovery. The private directory excludes other UIDs, not same-UID apps.
        let journal = directory.join("outbox.sqlite3-journal");
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
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA max_page_count=32768;")?;
        if fresh {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE metadata(singleton INTEGER PRIMARY KEY CHECK(singleton=1), next_reference INTEGER NOT NULL CHECK(next_reference BETWEEN 0 AND 65536)) STRICT;
                INSERT INTO metadata VALUES(1,0);
                CREATE TABLE messages(id TEXT PRIMARY KEY CHECK(length(id)=32), digest BLOB NOT NULL CHECK(length(digest)=32), recipient TEXT NOT NULL, body TEXT NOT NULL, report INTEGER NOT NULL CHECK(report IN(0,1)), reference INTEGER UNIQUE CHECK(reference BETWEEN 0 AND 65535), total INTEGER NOT NULL CHECK(total BETWEEN 1 AND 32), cancelled INTEGER NOT NULL DEFAULT 0 CHECK(cancelled IN(0,1))) STRICT;
                CREATE TABLE parts(message TEXT NOT NULL REFERENCES messages(id), part INTEGER NOT NULL CHECK(part BETWEEN 0 AND 31), tpdu BLOB NOT NULL CHECK(length(tpdu) BETWEEN 7 AND 157), state TEXT NOT NULL CHECK(state IN('ready','uncertain','accepted','not_submitted')), PRIMARY KEY(message,part)) STRICT;
                PRAGMA user_version=1;")?;
            tx.commit()?;
        } else if db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? != 1 {
            return Err(Error::Refused("unsupported outbox schema; no migrations"));
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
        Ok(Self {
            db,
            owner: uuid::Uuid::new_v4(),
            _lock: lock,
        })
    }
    /// Prepare explicitly, atomically reserving all payloads and a never-reused
    /// reference. Duplicate identities return receipts; changed intents fail.
    pub fn prepare(
        &mut self,
        id: &str,
        recipient: &str,
        text: &str,
        report: bool,
    ) -> Result<Receipt> {
        identity(id)?;
        let initial = plan_submit(recipient, text, report, 0).map_err(Error::Payload)?;
        let mut hash = Sha256::new();
        for field in [recipient.as_bytes(), text.as_bytes(), &[u8::from(report)]] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field);
        }
        let digest = hash.finalize();
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<Vec<u8>> = tx
            .query_row("SELECT digest FROM messages WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(existing) = existing {
            if existing.as_slice() != &digest[..] {
                return Err(Error::Refused("identity reused for changed message"));
            }
            drop(tx);
            return self.lookup(id);
        }
        let count: u32 = tx.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?;
        if count >= MAX_MESSAGES as u32 {
            return Err(Error::Refused("outbox receipt capacity exhausted"));
        }
        let reference = if initial.len() > 1 {
            let next: u32 = tx.query_row(
                "SELECT next_reference FROM metadata WHERE singleton=1",
                [],
                |r| r.get(0),
            )?;
            if next > u32::from(u16::MAX) {
                return Err(Error::Refused("concatenation references exhausted"));
            }
            tx.execute(
                "UPDATE metadata SET next_reference=next_reference+1 WHERE singleton=1",
                [],
            )?;
            Some(next as u16)
        } else {
            None
        };
        let parts = if let Some(reference) = reference {
            plan_submit(recipient, text, report, reference).map_err(Error::Payload)?
        } else {
            initial
        };
        tx.execute("INSERT INTO messages(id,digest,recipient,body,report,reference,total) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,&digest[..],recipient,text,report,reference,parts.len() as u32])?;
        for (index, part) in parts.iter().enumerate() {
            tx.execute(
                "INSERT INTO parts VALUES(?1,?2,?3,'ready')",
                params![id, index as u32, part.tpdu],
            )?;
        }
        tx.commit()?;
        self.lookup(id)
    }
    pub fn lookup(&self, id: &str) -> Result<Receipt> {
        identity(id)?;
        let (cancelled, reference, total): (bool, Option<u16>, u8) = self.db.query_row(
            "SELECT cancelled,reference,total FROM messages WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let total = usize::from(total);
        if !(1..=crate::MAX_PARTS).contains(&total) {
            return Err(Error::Refused("invalid stored part count"));
        }
        let mut query = self
            .db
            .prepare("SELECT part,state FROM parts WHERE message=?1 ORDER BY part LIMIT 33")?;
        let mut rows = query.query([id])?;
        let mut parts = Vec::with_capacity(total);
        while let Some(row) = rows.next()? {
            if usize::from(row.get::<_, u8>(0)?) != parts.len() || parts.len() >= total {
                return Err(Error::Refused("invalid stored part ordering"));
            }
            parts.push(PartState::parse(&row.get::<_, String>(1)?)?);
        }
        if parts.len() != total {
            return Err(Error::Refused("incomplete stored message"));
        }
        Ok(Receipt {
            cancelled,
            reference,
            parts,
        })
    }
    /// Explicitly claim one part. Earlier parts must be transport-accepted. The
    /// durable uncertain state is set before returning a non-cloneable payload.
    pub fn claim(&mut self, id: &str, part: usize) -> Result<DispatchPermit> {
        identity(id)?;
        if part >= crate::MAX_PARTS {
            return Err(Error::Refused("part outside bound"));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (cancelled, total): (bool, u8) = tx.query_row(
            "SELECT cancelled,total FROM messages WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if cancelled || part >= usize::from(total) {
            return Err(Error::Refused("message cancelled or part absent"));
        }
        let accepted: u32 = tx.query_row(
            "SELECT count(*) FROM parts WHERE message=?1 AND part<?2 AND state='accepted'",
            params![id, part as u32],
            |r| r.get(0),
        )?;
        if accepted != part as u32 {
            return Err(Error::Refused("earlier part not accepted"));
        }
        let payload: Option<Vec<u8>> = tx
            .query_row(
                "SELECT tpdu FROM parts WHERE message=?1 AND part=?2 AND state='ready'",
                params![id, part as u32],
                |r| r.get(0),
            )
            .optional()?;
        let tpdu = payload.ok_or(Error::Refused("part already claimed; no retry"))?;
        if !(7..=157).contains(&tpdu.len()) {
            return Err(Error::Refused("invalid stored payload bound"));
        }
        let changed = tx.execute(
            "UPDATE parts SET state='uncertain' WHERE message=?1 AND part=?2 AND state='ready'",
            params![id, part as u32],
        )?;
        if changed != 1
            || tx.query_row(
                "SELECT state FROM parts WHERE message=?1 AND part=?2",
                params![id, part as u32],
                |r| r.get::<_, String>(0),
            )? != "uncertain"
        {
            return Err(Error::Refused("dispatch uncertainty was not stored"));
        }
        tx.commit()?;
        Ok(DispatchPermit {
            owner: self.owner,
            id: id.into(),
            part,
            tpdu,
        })
    }
    /// Store one qualified local transport result. A cancelled message stays
    /// cancelled even if a held request succeeds later. No delivery is inferred.
    pub fn finish(&mut self, permit: DispatchPermit, result: TransportResult) -> Result<()> {
        if permit.owner != self.owner {
            return Err(Error::Refused("stale dispatch owner"));
        }
        let state = match result {
            TransportResult::Accepted => "accepted",
            TransportResult::NotSubmitted => "not_submitted",
        };
        let changed = self.db.execute(
            "UPDATE parts SET state=?3 WHERE message=?1 AND part=?2 AND state='uncertain'",
            params![permit.id, permit.part as u32, state],
        )?;
        if changed != 1 {
            return Err(Error::Refused("immutable dispatch receipt"));
        }
        Ok(())
    }
    pub fn cancel(&mut self, id: &str) -> Result<()> {
        identity(id)?;
        if self
            .db
            .execute("UPDATE messages SET cancelled=1 WHERE id=?1", [id])?
            != 1
        {
            return Err(Error::Refused("unknown message"));
        }
        Ok(())
    }
}
