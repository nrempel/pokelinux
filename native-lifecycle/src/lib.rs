//! Linux credentials are authority; app labels are trusted installation metadata.
pub mod authority;
pub mod ipc;
pub mod messages;
pub mod service;
pub mod storage;
use serde::{Deserialize, Serialize};

pub const MAX_PACKET: usize = 4096;
pub const MAX_APPS: usize = 16;
pub const MAX_PERMITS: usize = 64;
pub const MAX_RECEIPTS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Capture,
    SendMessage,
    MessagesRead,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppProfile {
    pub app: String,
    pub uid: u32,
    pub executable: String,
    pub args: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub apps: Vec<AppProfile>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppRequest {
    MessagesRead { request: messages::ReadRequest },
    Query {},
    Prepare { permission: Permission, ttl_ms: u64 },
    Admit { permit: String },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminRequest {
    Launch {
        app: String,
    },
    Grant {
        app: String,
        permission: Permission,
        allowed: bool,
    },
    Terminate {
        app: String,
    },
    Background {
        app: String,
        background: bool,
    },
    Locked {
        locked: bool,
    },
    Suspended {
        suspended: bool,
    },
    Snapshot {},
    MessagesStatus {},
    MessagesRestart {},
    Stop {},
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    MessagesStorage {
        incoming_started: u64,
        incoming_raw_committed: u64,
        incoming_completed: u64,
        receiving: bool,
        incoming_failure: Option<String>,
        occupied: bool,
        busy: bool,
        available: bool,
        pending_publications: usize,
        backpressure: u64,
    },
    Ok,
    Denied,
    Unavailable,
    Prepared {
        permit: String,
    },
    AdmittedUncertain {
        receipt: String,
    },
    Snapshot {
        principals: usize,
        permits: usize,
        receipts: usize,
        locked: bool,
        suspended: bool,
        degraded: bool,
        ready: bool,
        storage_occupied: bool,
        storage_busy: bool,
    },
}

pub fn boottime_ns() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(t.tv_nsec).unwrap())
        .unwrap()
}
pub fn fresh_id() -> std::io::Result<String> {
    let mut bytes = [0u8; 16];
    rustix::rand::getrandom(&mut bytes, rustix::rand::GetRandomFlags::empty())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
