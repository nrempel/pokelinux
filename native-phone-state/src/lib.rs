//! Device-neutral read-only observations. A cloned view is historical data,
//! never a lease, permission, command receipt, or proof of carrier connectivity.
#![forbid(unsafe_code)]

/// Origin in one provider's local revision namespace, not a vendor sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegistrationObservation {
    pub source_generation: u64,
    pub source_revision: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RegistrationAvailability {
    #[default]
    Unavailable,
    RegistrationObserved(RegistrationObservation),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegistrationView {
    /// Changes of this view, independent of transport commit revisions.
    pub revision: u64,
    pub availability: RegistrationAvailability,
    /// Last positive observation, retained when current availability is lost.
    pub last_observed: Option<RegistrationObservation>,
}

/// Coarse read-only battery state; this never grants charging or suspend control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChargingStatus {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatteryObservation {
    pub capacity_percent: u8,
    pub status: ChargingStatus,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatteryView {
    pub revision: u64,
    /// None always clears the last percentage/status, including on source loss.
    pub observation: Option<BatteryObservation>,
    /// Absolute Linux CLOCK_BOOTTIME; a historical clone can already be expired.
    pub until_ns: u64,
}
