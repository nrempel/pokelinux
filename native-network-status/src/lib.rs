//! Coarse local kernel observations. No route, Internet or transport authority.
#![forbid(unsafe_code)]
use pokelinux_network_state::KernelSnapshot;
use serde::{Deserialize, Serialize};

pub const COLLECTION_NS: u64 = 1_000_000_000;
pub const VALIDITY_NS: u64 = 2_000_000_000;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub local_interfaces: u16,
    /// Observed address records; includes tentative/down/link-local records.
    pub address_records: u16,
}
impl Counts {
    pub fn valid(self) -> bool {
        self.local_interfaces <= 64 && self.address_records <= self.local_interfaces * 16
    }
    pub fn from_kernel(snapshot: &KernelSnapshot) -> Option<Self> {
        // Kernel IFF_LOOPBACK, never a guessed interface-name classification.
        let links: Vec<_> = snapshot
            .links
            .iter()
            .filter(|link| link.flags & 8 == 0)
            .collect();
        let value = Self {
            local_interfaces: u16::try_from(links.len()).ok()?,
            address_records: u16::try_from(
                links.iter().map(|link| link.addresses.len()).sum::<usize>(),
            )
            .ok()?,
        };
        value.valid().then_some(value)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Collection {
    pub id: u64,
    pub until_ns: u64,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct View {
    pub revision: u64,
    pub counts: Option<Counts>,
    pub until_ns: u64,
}
#[derive(Default)]
pub struct Model {
    sequence: u64,
    pending: Option<Collection>,
    view: View,
}
impl Model {
    pub fn begin(&mut self, now: u64) -> Option<Collection> {
        self.view(now);
        if self.pending.is_some() {
            return None;
        }
        let until_ns = now.checked_add(COLLECTION_NS)?;
        now.checked_add(VALIDITY_NS)?;
        self.sequence = self.sequence.checked_add(1)?;
        let collection = Collection {
            id: self.sequence,
            until_ns,
        };
        self.pending = Some(collection);
        Some(collection)
    }
    /// Call only after this exact Reader's actual wait and PIPE EOF.
    pub fn complete(&mut self, collection: Collection, now: u64, value: Option<Counts>) -> bool {
        if self.pending != Some(collection) {
            return false;
        }
        self.pending = None;
        if now >= collection.until_ns {
            self.clear();
            return false;
        }
        let Some(value) = value.filter(|value| value.valid()) else {
            self.clear();
            return false;
        };
        self.view(now);
        let Some(revision) = self.view.revision.checked_add(1) else {
            self.clear();
            return false;
        };
        self.view = View {
            revision,
            counts: Some(value),
            // begin checked this sum. Completion never renews the original lease.
            until_ns: collection.until_ns - COLLECTION_NS + VALIDITY_NS,
        };
        true
    }
    fn clear(&mut self) {
        if self.view.counts.take().is_some() {
            self.view.revision = self.view.revision.saturating_add(1);
        }
        self.view.until_ns = 0;
        // Cache expiry is not reader retirement and must not alter pending.
    }
    pub fn view(&mut self, now: u64) -> View {
        if now >= self.view.until_ns {
            self.clear();
        }
        self.view
    }
    pub fn withdraw(&mut self) {
        self.pending = None;
        self.clear();
    }
}
