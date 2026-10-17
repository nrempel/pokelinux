//! Public battery observations, independent of suspend and charging policy.
#![forbid(unsafe_code)]
use pokelinux_phone_state::{BatteryObservation, BatteryView, ChargingStatus};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub const COLLECTION_NS: u64 = 1_000_000_000;
pub const VALIDITY_NS: u64 = 2_000_000_000;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Collection {
    pub id: u64,
    pub until_ns: u64,
}

/// Observation policy only. The service separately retains actual child custody
/// through wait and PIPE EOF; retiring a token is never process reclamation.
#[derive(Default)]
pub struct BatteryModel {
    sequence: u64,
    pending: Option<Collection>,
    view: BatteryView,
}
impl BatteryModel {
    pub fn begin(&mut self, now: u64) -> Option<Collection> {
        self.view(now);
        if self.pending.is_some() {
            return None;
        }
        let id = self.sequence.checked_add(1)?;
        let until_ns = now.checked_add(COLLECTION_NS)?;
        now.checked_add(VALIDITY_NS)?;
        self.sequence = id;
        let collection = Collection { id, until_ns };
        self.pending = Some(collection);
        Some(collection)
    }
    /// Call only after the exact reader's actual wait and bounded PIPE EOF.
    /// A matching late completion retires its token but cannot publish data.
    pub fn complete(
        &mut self,
        collection: Collection,
        now: u64,
        value: Option<BatteryObservation>,
    ) -> bool {
        if self.pending != Some(collection) {
            return false;
        }
        self.pending = None;
        if now >= collection.until_ns || value.is_some_and(|v| v.capacity_percent > 100) {
            self.clear();
            return false;
        }
        self.view.revision = self.view.revision.saturating_add(1);
        self.view.observation = value;
        self.view.until_ns = if value.is_some() {
            collection.until_ns - COLLECTION_NS + VALIDITY_NS
        } else {
            0
        };
        true
    }
    fn clear(&mut self) {
        if self.view.observation.take().is_some() {
            self.view.revision = self.view.revision.saturating_add(1);
        }
        self.view.until_ns = 0;
    }
    pub fn view(&mut self, now: u64) -> BatteryView {
        if now >= self.view.until_ns {
            self.clear();
        }
        self.view
    }
    /// Revoke publication before signalling; the service still owns its child.
    pub fn withdraw(&mut self) {
        self.pending = None;
        self.clear();
    }
}

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn field<'a>(slot: &mut Option<&'a str>, value: &'a str) -> io::Result<()> {
    if slot.replace(value).is_some() {
        return Err(invalid());
    }
    Ok(())
}
/// Blocking, bounded-byte power_supply backend; run only in an owned reader.
/// Class entries may be real sysfs symlinks. The class root and selected supply
/// are deployment input, never a public client's path. One uevent read is a
/// sampled observation, not a hardware-atomic transaction.
pub fn read_supply(class_root: &Path, supply: &str) -> io::Result<BatteryObservation> {
    if !class_root.is_absolute()
        || supply.is_empty()
        || supply.len() > 128
        || !supply
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut bytes = Vec::new();
    File::open(class_root.join(supply).join("uevent"))?
        .take(4097)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4096 || bytes.last() != Some(&b'\n') {
        return Err(invalid());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
    let (mut kind, mut present, mut capacity, mut status) = (None, None, None, None);
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or_else(invalid)?;
        match key {
            "POWER_SUPPLY_TYPE" => {
                // Linux can repeat the same TYPE in one uevent. Conflicting
                // types remain invalid; other measured fields stay unique.
                if kind.is_some_and(|previous| previous != value) {
                    return Err(invalid());
                }
                kind = Some(value);
            }
            "POWER_SUPPLY_PRESENT" => field(&mut present, value)?,
            "POWER_SUPPLY_CAPACITY" => field(&mut capacity, value)?,
            "POWER_SUPPLY_STATUS" => field(&mut status, value)?,
            _ => {} // Other standard and driver-specific fields keep their units.
        }
    }
    if kind != Some("Battery") || present.is_some_and(|v| v != "1") {
        return Err(invalid());
    }
    let capacity = capacity.ok_or_else(invalid)?;
    if capacity.is_empty() || capacity.len() > 3 || !capacity.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let capacity_percent: u8 = capacity.parse().map_err(|_| invalid())?;
    if capacity_percent > 100 {
        return Err(invalid());
    }
    let status = match status {
        Some("Unknown") => ChargingStatus::Unknown,
        Some("Charging") => ChargingStatus::Charging,
        Some("Discharging") => ChargingStatus::Discharging,
        Some("Not charging") => ChargingStatus::NotCharging,
        Some("Full") => ChargingStatus::Full,
        _ => return Err(invalid()),
    };
    Ok(BatteryObservation {
        capacity_percent,
        status,
    })
}
