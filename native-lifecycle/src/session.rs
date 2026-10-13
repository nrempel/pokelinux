//! Internal compositor facts supplied by the owned native observer. No app IPC,
//! grant, launch or unlock capability is represented by this boundary.
use serde::{Deserialize, Serialize};
use std::io;
pub const SAMPLE_BUDGET_NS: u64 = 100_000_000;
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFocus {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
    pub pidfs_device: u64,
    pub pidfs_inode: u64,
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Snapshot {
    pub source: String,
    pub revision: u64,
    pub sequence: u64,
    pub sampled_at_ns: u64,
    pub until: u64,
    pub locked: bool,
    pub focus: Option<NativeFocus>,
}
/// Poll is bounded and nonblocking. None/Err supplies no positive authority;
/// the actor retains policy and must match its exact launched app pidfd.
pub trait Provider {
    fn poll(&mut self) -> io::Result<Option<Snapshot>>;
}
