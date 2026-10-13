//! Appearance is public presentation; it never supplies session or action authority.
pub mod collector;
pub mod ipc;
pub mod palette;
pub mod sandbox;
pub mod service;

use serde::{Deserialize, Serialize};
use std::io;

pub const MAX_PACKET: usize = 4096;
pub const COLLECTION_BUDGET_NS: u64 = 1_000_000_000;
pub const REFRESH_NS: u64 = 250_000_000;
pub const DESKTOP_UID: u32 = 1000;

pub fn boottime_ns() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (t.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(t.tv_nsec as u64)
}
pub fn fresh_id() -> io::Result<String> {
    let mut bytes = [0; 16];
    let mut count = 0;
    while count < bytes.len() {
        count +=
            rustix::rand::getrandom(&mut bytes[count..], rustix::rand::GetRandomFlags::empty())?;
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub home: String,
    pub runtime: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub owner: String,
    pub revision: u32,
    /// Canonical decimal preserves exact u64 nanoseconds across JSON/Qt.
    pub until_ns: String,
    pub available: bool,
    pub palette: Option<palette::Palette>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Job {
    owner: String,
    token: u64,
    until_ns: u64,
    config: Configuration,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Completion {
    owner: String,
    token: u64,
    until_ns: u64,
    palette: Option<palette::Palette>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ack {
    owner: String,
    token: u64,
}
