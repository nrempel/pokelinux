use crate::protocol::{ConversationView, DataId, MessageView, Page, Query, StorageStatus};
use pokelinux_inbox::{ConversationId, ImportLease, Inbox, ReadLease};
use pokelinux_messaging::{receipt::ReceiptOwner, receive::ReferenceWidth};
use std::path::Path;
/// Synchronous private storage component. The supervisor, not this object,
/// keeps stop and authority checks responsive while filesystem calls execute.
pub struct Store {
    raw: ReceiptOwner,
    inbox: Inbox,
    read: ReadLease,
    import: ImportLease,
    status: StorageStatus,
}
impl Store {
    pub fn open(raw: &Path, inbox: &Path) -> Result<Self, &'static str> {
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
            read: control
                .read_lease()
                .map_err(|_| "local worker read unavailable")?,
            import: control
                .import_lease()
                .map_err(|_| "local projection unavailable")?,
            inbox,
            status: StorageStatus::LocalStorageUnavailable,
        })
    }
    pub fn reconcile(&mut self) -> Result<StorageStatus, &'static str> {
        self.status = StorageStatus::LocalStorageUnavailable;
        let snapshot = self
            .raw
            .snapshot(32)
            .map_err(|_| "raw snapshot unavailable")?;
        if snapshot.total > 32 {
            self.status = StorageStatus::ProjectionBacklogUnavailable;
            return Ok(self.status);
        }
        // Existing API is newest first. Import oldest first for local ordinals.
        for receipt in snapshot.latest.iter().rev() {
            self.inbox
                .import(&self.import, &self.raw, receipt.id)
                .map_err(|_| "projection transaction unavailable")?;
        }
        self.status = StorageStatus::Ready;
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
