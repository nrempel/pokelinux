//! One-shot Linux wireless discovery inside a caller-owned exclusive radio epoch.
//!
//! Scan and explicit open-association primitives never configure IP, abort or retry a mutation.
//! The caller must own its process through actual wait and stdout EOF. Closing its
//! sockets or reaping it does not prove cancellation of an admitted kernel scan.
mod association;
mod wire;
pub use association::{
    Associated, AssociationRequest, DisconnectRequest, Disconnected, associate_open_once,
    disconnect_once,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Request {
    pub ifindex: u32,
    pub frequency_mhz: u32,
    pub original_until_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bss {
    pub bssid_hex: String,
    pub ssid_hex: String,
    pub frequency_mhz: u32,
    pub last_seen_boottime_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Completed {
    pub request: Request,
    pub wiphy: u32,
    pub wdev: u64,
    pub started_boottime_ns: u64,
    pub completed_boottime_ns: u64,
    pub bss: Vec<Bss>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Unsupported,
    Expired,
    Malformed,
    MissingAttribute(u16),
    MissingAssociationAttribute {
        stage: u8,
        command: u8,
        attribute: u16,
    },
    MalformedAssociation(u8),
    Loss,
    Limit,
    Identity,
    Contended,
    Aborted,
    Kernel(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Unavailable,
    InvalidRequest,
    Failed(Failure),
    Uncertain(Failure),
}

/// At most one scan on a newly created, exclusively administered radio epoch.
///
/// `ifindex` and the retained namespace/wiphy/wdev checks establish identity, not
/// exclusive authority. nl80211 supplies no generic per-scan correlation token.
/// Callers cannot use this primitive safely on a shared or recovered radio epoch.
/// The original absolute CLOCK_BOOTTIME cutoff must be within five seconds.
/// Kernel last-seen timestamps outside the operation span are refused; none are
/// synthesized from age, TSF or the current clock. Physical driver timing and
/// vendor initialization require separate backend qualification.
pub fn scan_once(request: Request) -> Result<Completed, Error> {
    let started = wire::boottime_ns();
    if request.ifindex == 0
        || request.frequency_mhz == 0
        || request.frequency_mhz > 100_000
        || request.original_until_ns <= started
        || request.original_until_ns - started > 5_000_000_000
    {
        return Err(Error::InvalidRequest);
    }
    let mut dispatched = false;
    wire::scan(request, started, &mut dispatched)
        .map_err(|failure| classify_failure(failure, dispatched))
}
fn classify_failure(failure: Failure, dispatched: bool) -> Error {
    if dispatched {
        Error::Uncertain(failure)
    } else if failure == Failure::Unsupported {
        Error::Unavailable
    } else {
        Error::Failed(failure)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_enoent_is_not_wireless_unavailability() {
        assert_eq!(
            classify_failure(Failure::Kernel(2), false),
            Error::Failed(Failure::Kernel(2))
        );
    }
}
