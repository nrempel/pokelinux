//! Linux credentials are authority; app labels are trusted installation metadata.
mod apps;
pub mod audio;
mod audio_owner;
mod audio_session;
pub mod authority;
pub mod brightness;
mod brightness_session;
pub mod camera;
pub mod connectivity;
mod connectivity_session;
pub mod display;
pub mod ipc;
pub mod lock;
pub mod messages;
pub mod service;
pub mod startup;
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
    MediaRoute,
    DisplayBrightness,
    NetworkControl,
}

/// Installation-owned role; app request labels never establish controller authority.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppRole {
    Application,
    PermissionController {
        target: String,
        permission: Permission,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppProfile {
    pub role: AppRole,
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
    ConnectivitySet { request: connectivity::Request },
    ConnectivityStatus {},
    BrightnessSet { request: brightness::Request },
    BrightnessStatus {},
    MediaRoute { request: audio::Request },
    MediaPreviewPlay { request: audio::PreviewRequest },
    MediaPreviewMute { request: audio::PreviewMuteRequest },
    MediaPreviewStop {},
    MediaStatus { offset: usize },
    MediaReconnect {},
    CameraCapture { request: camera::Request },
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
    CameraStatus {},
    Stop {},
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    ConnectivityCompleted {
        state: connectivity::State,
    },
    ConnectivityState {
        state: connectivity::State,
        can_control: bool,
    },
    BrightnessCompleted {
        state: brightness::State,
    },
    BrightnessState {
        state: brightness::State,
        can_set: bool,
    },
    MediaSelected {
        state: audio::State,
    },
    MediaPreview {
        state: audio::State,
    },
    MediaStatus {
        state: audio::State,
        routes: Vec<audio::Route>,
        next_offset: Option<usize>,
        can_choose: bool,
        can_reconnect: bool,
    },
    CameraState {
        state: camera::State,
    },
    CameraStatus {
        occupied: bool,
        busy: bool,
        available: bool,
        pending: usize,
    },
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
        source: session::SourceObservation,
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

pub mod session;

#[cfg(test)]
mod permission_profile_tests;
