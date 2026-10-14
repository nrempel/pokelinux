//! Internal compositor facts supplied by the owned native observer. No app IPC,
//! grant, launch or unlock capability is represented by this boundary.
use pokelinux_kernel_identity::ProcessView;
use serde::{Deserialize, Serialize};
use std::{io, os::fd::OwnedFd, sync::Arc};
pub const SAMPLE_BUDGET_NS: u64 = 100_000_000;
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFocus {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
}
/// Metadata is serializable; the retained kernel handle is internal authority input.
#[derive(Debug, Clone)]
pub struct OwnedFocus {
    pub metadata: NativeFocus,
    pidfd: Arc<OwnedFd>,
}
impl OwnedFocus {
    pub fn new(metadata: NativeFocus, pidfd: OwnedFd) -> Self {
        Self {
            metadata,
            pidfd: Arc::new(pidfd),
        }
    }
    pub fn pidfd(&self) -> &OwnedFd {
        &self.pidfd
    }
    /// Compare live kernel tasks in one pinned procfs namespace. Metadata alone
    /// and descriptor numbers never establish equivalence.
    pub fn equivalent(&self, other: &Self, view: &ProcessView) -> bool {
        self.metadata == other.metadata
            && view
                .matches(self.pidfd(), other.pidfd(), self.metadata.pid)
                .unwrap_or(false)
    }
    /// Exact retained-object sharing permits repeated observation of a sample,
    /// including after its focused app died. It does not establish live authority.
    pub(crate) fn same_reference(&self, other: &Self) -> bool {
        self.metadata == other.metadata && Arc::ptr_eq(&self.pidfd, &other.pidfd)
    }
}
/// Exact accepted-object protocol request, scoped to this Source incarnation.
/// This is historical fact, not authentication or permission to restore reads.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolUnlock {
    pub generation: u64,
    pub sequence: u64,
    pub observed_at_ns: u64,
}
pub fn valid_protocol_facts(
    locked: bool,
    sequence: u64,
    sampled_at_ns: u64,
    generation: Option<u64>,
    unlock: Option<&ProtocolUnlock>,
) -> bool {
    !generation.is_some_and(|value| value == 0 || !locked)
        && !unlock.is_some_and(|event| {
            locked
                || generation.is_some()
                || event.generation == 0
                || event.sequence == 0
                || event.sequence > sequence
                || event.observed_at_ns == 0
                || event.observed_at_ns > sampled_at_ns
        })
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub source: String,
    pub revision: u64,
    pub sequence: u64,
    pub sampled_at_ns: u64,
    pub until: u64,
    pub locked: bool,
    pub focus: Option<OwnedFocus>,
    /// Retained mapped-locker peer from the same sample. This is not an unlock
    /// capability. Consumers must revalidate live owned identity when using it.
    pub locker: Option<OwnedFocus>,
    /// Unknown for a lock already present when Source first attaches.
    pub lock_generation: Option<u64>,
    pub protocol_unlock: Option<ProtocolUnlock>,
}
/// Poll is bounded and nonblocking. None/Err supplies no positive authority;
/// the actor retains policy and must match its exact launched app pidfd.
pub trait Provider {
    fn poll(&mut self) -> io::Result<Option<Snapshot>>;
    /// Fence this source's local admission and signal any owned helper.
    fn stop(&mut self);
    /// True until owned helpers have actually been reaped, even after failure.
    fn occupied(&self) -> bool;
}
