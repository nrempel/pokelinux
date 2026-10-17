#![forbid(unsafe_code)]

pub const CAPACITY: usize = 4096;
pub const OK: i32 = 0;
pub const EMPTY: i32 = 1;
pub const WAIT: i32 = 2;
pub const REFUSED: i32 = 3;
pub const EXPIRED: i32 = 4;
pub const INVALID: i32 = 5;
pub const STALE: i32 = 6;
pub const ABORT: i32 = 7;
pub const CONNECT: u32 = 1;
pub const SEND: u32 = 2;
pub const RECEIVE: u32 = 3;
pub const NOTIFY: u32 = 4;
pub const SLEEP: u32 = 5;
pub const CLOSE: u32 = 6;
pub const EXIT: u32 = 7;
pub const NEW: u32 = 0;
pub const CONNECTING: u32 = 1;
pub const SENDING: u32 = 2;
pub const RECEIVING: u32 = 3;
pub const COOLDOWN: u32 = 4;
pub const STOPPED: u32 = 5;
pub const UNAVAILABLE: u32 = 0;
pub const HISTORY: u32 = 1;
pub const DENIED: u32 = 2;
pub const CONNECTED: u32 = 1;
pub const CONNECT_FAILED: u32 = 2;
pub const SENT: u32 = 3;
pub const WOULD_BLOCK: u32 = 4;
pub const INTERRUPTED: u32 = 5;
pub const IO_FAILED: u32 = 6;
pub const CLOSED: u32 = 7;
pub const NOTIFY_QUEUED: u32 = 8;
pub const PATH_REFUSED: u32 = 9;
pub const SELECT: u32 = 1;
pub const NEXT_MESSAGES: u32 = 2;
pub const FIRST_MESSAGES: u32 = 3;
pub const NEXT_CONVERSATIONS: u32 = 4;
pub const FIRST_CONVERSATIONS: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Id {
    pub length: u32,
    pub bytes: [u8; 32],
}
impl Id {
    pub fn from_text(text: &str) -> Option<Self> {
        if text.len() != 32
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        let mut value = Self {
            length: 32,
            ..Self::default()
        };
        value.bytes.copy_from_slice(text.as_bytes());
        Some(value)
    }
    pub fn valid(&self) -> bool {
        self.length == 32
            && self
                .bytes
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    }
    pub fn text(&self) -> &str {
        // IDs are created only from validated ASCII, or the empty default.
        std::str::from_utf8(&self.bytes[..self.length.min(32) as usize]).unwrap_or("")
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub generation: u64,
    pub connection_epoch: u64,
    pub request_token: u64,
    pub event_token: u64,
    pub original_until_boottime_ns: u64,
    pub wake_boottime_ns: u64,
    pub action: u32,
    pub packet_length: u32,
    pub wait_slice_ms: u32,
    pub reserved: u32,
    pub packet: [u8; CAPACITY],
}
impl Default for Step {
    fn default() -> Self {
        Self {
            generation: 0,
            connection_epoch: 0,
            request_token: 0,
            event_token: 0,
            original_until_boottime_ns: 0,
            wake_boottime_ns: 0,
            action: 0,
            packet_length: 0,
            wait_slice_ms: 0,
            reserved: 0,
            packet: [0; CAPACITY],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct View {
    pub generation: u64,
    pub model_revision: u64,
    pub connection_epoch: u64,
    pub request_token: u64,
    pub original_until_boottime_ns: u64,
    pub queued_events: u64,
    pub stage: u32,
    pub status: u32,
    pub stopping: u32,
    pub worker_active: u32,
    pub conversations_length: u32,
    pub messages_length: u32,
    pub reserved0: u32,
    pub reserved1: u32,
    pub selected: Id,
    pub conversations_after: Id,
    pub after_ordinal: u64,
    pub conversations_json: [u8; CAPACITY],
    pub messages_json: [u8; CAPACITY],
}
impl Default for View {
    fn default() -> Self {
        Self {
            generation: 0,
            model_revision: 0,
            connection_epoch: 0,
            request_token: 0,
            original_until_boottime_ns: 0,
            queued_events: 0,
            stage: NEW,
            status: UNAVAILABLE,
            stopping: 0,
            worker_active: 0,
            conversations_length: 0,
            messages_length: 0,
            reserved0: 0,
            reserved1: 0,
            selected: Id::default(),
            conversations_after: Id::default(),
            after_ordinal: 0,
            conversations_json: [0; CAPACITY],
            messages_json: [0; CAPACITY],
        }
    }
}
