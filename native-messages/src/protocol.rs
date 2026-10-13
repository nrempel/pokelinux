use serde::{Deserialize, Deserializer, Serialize};
pub const MAX_PACKET: usize = 4096;
pub const MAX_CONVERSATIONS: usize = 8;
pub const MAX_MESSAGES: usize = 2;
/// A local data identifier, never an authorization or modem receipt token.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DataId(String);
impl DataId {
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        if s.len() != 32
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("canonical local data ID required");
        }
        Ok(Self(s.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for DataId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Conversations {
        after: Option<DataId>,
        limit: usize,
    },
    Messages {
        conversation: DataId,
        after_ordinal: u64,
        limit: usize,
    },
}
impl Query {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Conversations { limit, .. } if (1..=MAX_CONVERSATIONS).contains(limit) => Ok(()),
            Self::Messages {
                after_ordinal,
                limit,
                ..
            } if *after_ordinal <= 4096 && (1..=MAX_MESSAGES).contains(limit) => Ok(()),
            _ => Err("query outside bound"),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationView {
    pub id: DataId,
    pub sender: String,
    pub slot: u8,
    pub sender_kind: u8,
    pub messages: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageView {
    pub receipt: DataId,
    pub ordinal: u64,
    pub sender: String,
    pub text: String,
    pub timestamp_millis: i64,
    pub reply_path: bool,
    pub concat_width: Option<u8>,
    pub concat_reference: Option<u16>,
    pub concat_total: Option<u8>,
    pub concat_sequence: Option<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "page", rename_all = "snake_case", deny_unknown_fields)]
pub enum Page {
    Conversations { items: Vec<ConversationView> },
    Messages { items: Vec<MessageView> },
}
// Sensitive page DTOs deliberately have no blanket Debug implementations.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageStatus {
    Ready,
    ProjectionBacklogUnavailable,
    LocalStorageUnavailable,
}
