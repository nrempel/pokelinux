//! Durable admission only. A `New` result permits the service owner to dispatch
//! once; every other result forbids dispatch. Client cancellation cannot erase
//! an admission. No phone numbers or replayable command payloads are stored.
use crate::{Command, Identity, Request};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Database(rusqlite::Error),
    Refused(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
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
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    NotSent,
    Pending,
    Unknown,
    DialInvoked,
    DialNotInvoked,
    ControlSubmitted,
    ControlRejected,
}
impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Self::NotSent => "not_sent",
            Self::Pending => "pending",
            Self::Unknown => "unknown",
            Self::DialInvoked => "dial_invoked",
            Self::DialNotInvoked => "dial_not_invoked",
            Self::ControlSubmitted => "control_submitted",
            Self::ControlRejected => "control_rejected",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "not_sent" => Ok(Self::NotSent),
            "pending" => Ok(Self::Pending),
            "unknown" => Ok(Self::Unknown),
            "dial_invoked" => Ok(Self::DialInvoked),
            "dial_not_invoked" => Ok(Self::DialNotInvoked),
            "control_submitted" => Ok(Self::ControlSubmitted),
            "control_rejected" => Ok(Self::ControlRejected),
            _ => Err(Error::Refused("invalid stored outcome")),
        }
    }
    fn valid_for(self, kind: &str) -> bool {
        matches!(self, Self::Unknown | Self::NotSent)
            || matches!(
                (kind, self),
                ("dial", Self::DialInvoked | Self::DialNotInvoked)
                    | ("control", Self::ControlSubmitted | Self::ControlRejected)
            )
    }
}
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct StoredReceipt {
    outcome: Outcome,
    acknowledged: bool,
}
impl StoredReceipt {
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn acknowledged(&self) -> bool {
        self.acknowledged
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    New,
    Existing(Outcome),
}
pub struct Ledger {
    db: Connection,
    epoch: Identity,
    // Drop the connection before releasing ownership.
    _owner: File,
}
fn private_file(path: &Path, create_new: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o777 != 0o600
        || meta.nlink() != 1
    {
        return Err(Error::Refused(
            "journal files must be private regular files",
        ));
    }
    Ok(file)
}
impl Ledger {
    pub fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() {
            return Err(Error::Refused("absolute journal directory required"));
        }
        match fs::DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => {
                File::open(directory.parent().ok_or(Error::Refused("missing parent"))?)?
                    .sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
        let meta = fs::symlink_metadata(directory)?;
        if !meta.is_dir()
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o777 != 0o700
            || fs::canonicalize(directory)? != directory
        {
            return Err(Error::Refused(
                "journal directory must be private and must not use symlinks",
            ));
        }
        let owner = private_file(&directory.join("owner.lock"), false)?;
        owner
            .try_lock()
            .map_err(|_| Error::Refused("journal already owned or locking unavailable"))?;
        let path = directory.join("journal.sqlite3");
        let (file, fresh) = match private_file(&path, true) {
            Ok(f) => (f, true),
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
        db.execute_batch(
            "PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA foreign_keys=ON;",
        )?;
        if fresh {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE metadata(epoch TEXT NOT NULL) STRICT;
                CREATE TABLE operations(id TEXT PRIMARY KEY, digest BLOB NOT NULL CHECK(length(digest)=32),
                    kind TEXT NOT NULL CHECK(kind IN ('dial','control')),
                    outcome TEXT NOT NULL CHECK(outcome IN ('not_sent','pending','unknown','dial_invoked','dial_not_invoked','control_submitted','control_rejected')),
                    acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(acknowledged IN (0,1))) STRICT;
                CREATE TABLE notification_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                    generation TEXT NOT NULL) STRICT;
                CREATE TABLE notification_episodes(call_id INTEGER PRIMARY KEY CHECK(call_id BETWEEN 1 AND 2147483647),
                    episode TEXT NOT NULL UNIQUE, dismissed INTEGER NOT NULL CHECK(dismissed IN (0,1))) STRICT;
                PRAGMA user_version=3;")?;
            tx.execute(
                "INSERT INTO metadata VALUES (?1)",
                [uuid::Uuid::new_v4().simple().to_string()],
            )?;
            tx.commit()?;
        } else if db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? != 3 {
            return Err(Error::Refused("unsupported journal schema; no migrations"));
        }
        let epoch: String = db.query_row("SELECT epoch FROM metadata", [], |r| r.get(0))?;
        let epoch = Identity::try_from(epoch).map_err(Error::Refused)?;
        // The owner has just started: nothing from an earlier process can still
        // be safely dispatched or classified from a missing reply.
        db.execute(
            "UPDATE operations SET outcome='unknown' WHERE outcome='pending'",
            [],
        )?;
        file.sync_all()?;
        File::open(directory)?.sync_all()?;
        Ok(Self {
            db,
            epoch,
            _owner: owner,
        })
    }
    pub fn epoch(&self) -> &Identity {
        &self.epoch
    }
    /// Replace one bounded presentation state atomically. Caller must keep a
    /// failed save distinct from durable success; this never admits a command.
    pub fn store_dismissals(
        &mut self,
        state: &crate::notification_policy::Dismissals,
    ) -> Result<()> {
        let (generation, records) = state.stored();
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM notification_episodes", [])?;
        tx.execute("DELETE FROM notification_state", [])?;
        if let Some(generation) = generation {
            tx.execute(
                "INSERT INTO notification_state VALUES (1, ?1)",
                [&generation.0],
            )?;
        }
        for (id, episode, dismissed) in records {
            tx.execute(
                "INSERT INTO notification_episodes VALUES (?1, ?2, ?3)",
                params![id, episode.0, dismissed],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn load_dismissals(&self) -> Result<crate::notification_policy::Dismissals> {
        let generation: Option<String> = self
            .db
            .query_row(
                "SELECT generation FROM notification_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let generation = generation
            .map(Identity::try_from)
            .transpose()
            .map_err(Error::Refused)?;
        // Read at most one row beyond the format bound, so corrupt files cannot
        // turn restoration into an unbounded allocation.
        let mut query = self.db.prepare("SELECT call_id, episode, dismissed FROM notification_episodes ORDER BY call_id LIMIT 65")?;
        let mut rows = query.query([])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let id: u32 = row.get(0)?;
            let episode: String = row.get(1)?;
            let dismissed = match row.get::<_, i64>(2)? {
                0 => false,
                1 => true,
                _ => return Err(Error::Refused("invalid stored dismissal flag")),
            };
            records.push((
                id,
                Identity::try_from(episode).map_err(Error::Refused)?,
                dismissed,
            ));
        }
        crate::notification_policy::Dismissals::restore(generation, records).map_err(Error::Refused)
    }
    pub fn reserve(&mut self, request: &Request) -> Result<Admission> {
        self.reserve_fenced(request, true)
    }
    pub(crate) fn reserve_fenced(&mut self, request: &Request, current: bool) -> Result<Admission> {
        self.check_epoch(request.epoch())?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(Vec<u8>, String)> = tx
            .query_row(
                "SELECT digest,outcome FROM operations WHERE id=?1",
                [&request.id().0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let digest = request.command_digest();
        if let Some((stored, outcome)) = existing {
            if stored != digest {
                return Err(Error::Refused(
                    "request identity reused with changed command",
                ));
            }
            return Ok(Admission::Existing(Outcome::parse(&outcome)?));
        }
        if !current {
            return Err(Error::Refused("stale admission token"));
        }
        let blocked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE outcome='pending' OR (outcome='unknown' AND acknowledged=0))", [], |r| r.get(0))?;
        if blocked {
            return Err(Error::Refused("unfinished operation requires recovery"));
        }
        let kind = match request.command() {
            Command::Dial { .. } => "dial",
            Command::Control { .. } => "control",
        };
        tx.execute(
            "INSERT INTO operations(id,digest,kind,outcome) VALUES (?1,?2,?3,'pending')",
            params![request.id().0, &digest[..], kind],
        )?;
        tx.commit()?;
        Ok(Admission::New)
    }
    pub fn finish(&mut self, id: &Identity, outcome: Outcome) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let kind: Option<String> = tx
            .query_row(
                "SELECT kind FROM operations WHERE id=?1 AND outcome='pending'",
                [&id.0],
                |r| r.get(0),
            )
            .optional()?;
        if !kind.as_deref().is_some_and(|k| outcome.valid_for(k)) {
            return Err(Error::Refused("invalid outcome or immutable receipt"));
        }
        tx.execute(
            "UPDATE operations SET outcome=?2 WHERE id=?1",
            params![id.0, outcome.name()],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Observation only. None is not a cancellation and grants no retry authority.
    pub fn lookup(&self, epoch: &Identity, id: &Identity) -> Result<Option<StoredReceipt>> {
        self.check_epoch(epoch)?;
        let row: Option<(String, bool)> = self
            .db
            .query_row(
                "SELECT outcome,acknowledged FROM operations WHERE id=?1",
                [&id.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(outcome, acknowledged)| {
            Ok(StoredReceipt {
                outcome: Outcome::parse(&outcome)?,
                acknowledged,
            })
        })
        .transpose()
    }
    fn check_epoch(&self, epoch: &Identity) -> Result<()> {
        if epoch != &self.epoch {
            return Err(Error::Refused("stale journal epoch"));
        }
        Ok(())
    }
    pub fn acknowledge(&mut self, epoch: &Identity, id: &Identity) -> Result<()> {
        self.check_epoch(epoch)?;
        let changed = self.db.execute(
            "UPDATE operations SET acknowledged=1 WHERE id=?1 AND outcome='unknown'",
            [&id.0],
        )?;
        if changed != 1 {
            return Err(Error::Refused("only unknown outcomes can be acknowledged"));
        }
        Ok(())
    }
    pub fn unresolved(&self) -> Result<Vec<Identity>> {
        let mut stmt = self.db.prepare("SELECT id FROM operations WHERE outcome='pending' OR (outcome='unknown' AND acknowledged=0) ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Identity::try_from(r?).map_err(Error::Refused))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_journal_connection_uses_delete_extra_and_foreign_keys() {
        let directory = std::env::temp_dir().join(format!(
            "phone-durability-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let ledger = Ledger::open(&directory).unwrap();
        assert_eq!(
            ledger
                .db
                .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
                .unwrap(),
            "delete"
        );
        assert_eq!(
            ledger
                .db
                .pragma_query_value(None, "synchronous", |r| r.get::<_, u32>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            ledger
                .db
                .pragma_query_value(None, "foreign_keys", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        drop(ledger);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
