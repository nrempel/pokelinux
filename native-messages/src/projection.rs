use crate::protocol::{ConversationView, DataId, MessageView, Page, Query, StorageStatus};
use pokelinux_inbox::{ConversationId, ImportLease, Inbox, ReadLease};
use pokelinux_messaging::{
    receipt::{
        BackendScope, IncomingKind, ReceiptCheckpoint, ReceiptId, ReceiptOwner, ReceiptScan, Slot,
        Source,
    },
    receive::ReferenceWidth,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
/// Synchronous private storage component. The supervisor, not this object,
/// keeps stop and authority checks responsive while filesystem calls execute.
pub const RECONCILE_BATCH: usize = 8;
pub struct Store {
    raw: ReceiptOwner,
    directory: PathBuf,
    inbox: Inbox,
    read: ReadLease,
    import: ImportLease,
    status: StorageStatus,
    scan: Option<ReceiptScan>,
    completed: Option<ReceiptCheckpoint>,
}
impl Store {
    pub fn open(raw: &Path, inbox: &Path) -> Result<Self, &'static str> {
        let directory = raw.to_path_buf();
        let raw = ReceiptOwner::open(raw).map_err(|_| "raw storage unavailable")?;
        let inbox = Inbox::open(inbox).map_err(|_| "projection storage unavailable")?;
        let control = inbox.control();
        // Service-owned projection is separate from current app read authority.
        control
            .set_read_allowed(true)
            .map_err(|_| "local worker read unavailable")?;
        control
            .set_import_allowed(true)
            .map_err(|_| "local projection unavailable")?;
        Ok(Self {
            raw,
            directory,
            read: control
                .read_lease()
                .map_err(|_| "local worker read unavailable")?,
            import: control
                .import_lease()
                .map_err(|_| "local projection unavailable")?,
            inbox,
            status: StorageStatus::LocalStorageUnavailable,
            scan: None,
            completed: None,
        })
    }
    /// Durable test-source attempt only. Presence never authorizes reattachment.
    /// Partial/invalid evidence remains unavailable; no cleanup or retry exists.
    pub fn claim_test_source(&self, generation: &str) -> Result<bool, &'static str> {
        DataId::parse(generation)?;
        let path = self
            .directory
            .join(format!("incoming-attempt-{generation}"));
        let expected = format!("incoming-test-v1:{generation}\n");
        // Test-only attachment history is also bounded; never evict an uncertainty marker.
        if !path
            .try_exists()
            .map_err(|_| "source attempt lookup unavailable")?
        {
            let mut attempts = 0;
            for entry in
                fs::read_dir(&self.directory).map_err(|_| "source attempt inventory unavailable")?
            {
                let entry = entry.map_err(|_| "source attempt inventory unavailable")?;
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("incoming-attempt-"))
                {
                    attempts += 1;
                    if attempts >= 8 {
                        break;
                    }
                }
            }
            if attempts >= 8 {
                return Err("durable source attempt capacity exhausted");
            }
        }

        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(expected.as_bytes())
                    .map_err(|_| "source attempt write unavailable")?;
                file.sync_all()
                    .map_err(|_| "source attempt sync unavailable")?;
                File::open(&self.directory)
                    .and_then(|f| f.sync_all())
                    .map_err(|_| "source attempt directory sync unavailable")?;
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| "source attempt metadata unavailable")?;
                if !metadata.is_file()
                    || metadata.nlink() != 1
                    || metadata.uid() != rustix::process::geteuid().as_raw()
                    || metadata.mode() & 0o777 != 0o600
                    || metadata.len() != expected.len() as u64
                {
                    return Err("invalid durable source attempt");
                }
                let mut old = String::new();
                OpenOptions::new()
                    .read(true)
                    .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                    .open(path)
                    .and_then(|f| f.take(128).read_to_string(&mut old))
                    .map_err(|_| "source attempt read unavailable")?;
                if old != expected {
                    return Err("invalid durable source attempt");
                }
                Ok(false)
            }
            Err(_) => Err("source attempt storage unavailable"),
        }
    }
    /// Typed service-owned source. Test ACK permission remains denied permanently.
    pub fn attach_test_incoming(&mut self, slot: Slot) -> Result<Source, &'static str> {
        self.raw
            .attach(slot, BackendScope::TestBackend)
            .map_err(|_| "incoming source unavailable")
    }
    pub fn admit_incoming(
        &mut self,
        source: Source,
        kind: IncomingKind,
        raw: &[u8],
    ) -> Result<ReceiptId, &'static str> {
        let id = self
            .raw
            .admit(source, kind, raw)
            .map_err(|_| "raw occurrence commit unavailable")?;
        self.status = StorageStatus::ProjectionBacklogUnavailable;
        Ok(id)
    }
    /// One bounded projection turn, not a synchronous scan of all raw receipts.
    /// Ready requires a completed fixed upper bound plus a current-tail check.
    pub fn reconcile(&mut self) -> Result<StorageStatus, &'static str> {
        self.status = StorageStatus::ProjectionBacklogUnavailable;
        let result = self.reconcile_turn();
        if result.is_err() {
            self.scan = None;
            self.status = StorageStatus::LocalStorageUnavailable;
        }
        result
    }
    fn reconcile_turn(&mut self) -> Result<StorageStatus, &'static str> {
        if self.scan.is_none() {
            self.scan = Some(
                self.raw
                    .scan(self.completed.as_ref())
                    .map_err(|_| "raw enumeration unavailable")?,
            );
        }
        let scan = self.scan.as_mut().ok_or("missing receipt scan")?;
        let page = self
            .raw
            .scan_page(scan, RECONCILE_BATCH)
            .map_err(|_| "raw enumeration page unavailable")?;
        for receipt in page.items {
            self.inbox
                .import(&self.import, &self.raw, receipt.id)
                .map_err(|_| "projection transaction unavailable")?;
        }
        if page.complete {
            let completed = self
                .raw
                .checkpoint(scan)
                .map_err(|_| "incomplete receipt enumeration")?;
            if self
                .raw
                .scan_current(scan)
                .map_err(|_| "raw enumeration tail unavailable")?
            {
                self.status = StorageStatus::Ready;
            }
            self.completed = Some(completed);
            self.scan = None;
        }
        Ok(self.status)
    }
    fn conversation(&self, key: &DataId) -> Result<ConversationId, &'static str> {
        let mut after = None;
        // A bounded lookup avoids converting external data identifiers into authority.
        for _ in 0..16 {
            let page = self
                .inbox
                .conversations(&self.read, after, 32)
                .map_err(|_| "conversation lookup unavailable")?;
            if let Some(c) = page.iter().find(|c| c.id.to_string() == key.as_str()) {
                return Ok(c.id);
            }
            if page.len() < 32 {
                break;
            }
            after = page.last().map(|c| c.id);
        }
        Err("unknown local conversation")
    }
    pub fn query(&self, query: &Query) -> Result<Page, &'static str> {
        query.validate()?;
        if self.status != StorageStatus::Ready {
            return Err("projection unavailable");
        }
        match query {
            Query::Conversations { after, limit } => {
                let after = after.as_ref().map(|id| self.conversation(id)).transpose()?;
                let values = self
                    .inbox
                    .conversations(&self.read, after, *limit)
                    .map_err(|_| "conversation page unavailable")?;
                Ok(Page::Conversations {
                    items: values
                        .into_iter()
                        .map(|c| {
                            Ok(ConversationView {
                                id: DataId::parse(&c.id.to_string())?,
                                sender: c.sender,
                                slot: c.slot.0,
                                sender_kind: c.sender_kind as u8,
                                messages: c.messages,
                            })
                        })
                        .collect::<Result<_, &'static str>>()?,
                })
            }
            Query::Messages {
                conversation,
                after_ordinal,
                limit,
            } => {
                let id = self.conversation(conversation)?;
                let values = self
                    .inbox
                    .messages(&self.read, id, *after_ordinal, *limit)
                    .map_err(|_| "message page unavailable")?;
                Ok(Page::Messages {
                    items: values
                        .into_iter()
                        .map(|m| {
                            Ok(MessageView {
                                receipt: DataId::parse(&m.receipt.to_string())?,
                                ordinal: m.ordinal,
                                sender: m.sender,
                                text: m.text,
                                timestamp_millis: m.timestamp_millis,
                                reply_path: m.reply_path,
                                concat_width: m.concatenation.as_ref().map(|c| match c.width {
                                    ReferenceWidth::Eight => 8,
                                    ReferenceWidth::Sixteen => 16,
                                }),
                                concat_reference: m.concatenation.as_ref().map(|c| c.reference),
                                concat_total: m.concatenation.as_ref().map(|c| c.total),
                                concat_sequence: m.concatenation.as_ref().map(|c| c.sequence),
                            })
                        })
                        .collect::<Result<_, &'static str>>()?,
                })
            }
        }
    }
    pub fn status(&self) -> StorageStatus {
        self.status
    }
}
