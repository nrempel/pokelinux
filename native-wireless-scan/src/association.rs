//! Explicit open association and disconnection in an exclusively owned radio epoch.
use crate::{Error, classify_failure, wire};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationRequest {
    pub ifindex: u32,
    pub frequency_mhz: u32,
    pub bssid: [u8; 6],
    pub ssid: Vec<u8>,
    pub original_until_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Associated {
    pub request: AssociationRequest,
    pub wiphy: u32,
    pub wdev: u64,
    pub started_boottime_ns: u64,
    pub completed_boottime_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisconnectRequest {
    pub ifindex: u32,
    pub original_until_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disconnected {
    pub request: DisconnectRequest,
    pub wiphy: u32,
    pub wdev: u64,
    pub started_boottime_ns: u64,
    pub completed_boottime_ns: u64,
}

fn valid_cutoff(started: u64, until: u64) -> bool {
    until > started && until - started <= 5_000_000_000
}

/// Submit one open-system CONNECT restricted to the exact BSSID/frequency/SSID.
///
/// Caller-owned fresh-radio exclusivity is required; identities alone cannot
/// establish it. Completion requires the selected MLME success event and actual
/// associated-BSS readback. No credentials, retry, rollback or automatic
/// Disconnect is provided. A possibly dispatched failure remains uncertain;
/// worker/socket closure does not mean the kernel action was cancelled.
pub fn associate_open_once(request: AssociationRequest) -> Result<Associated, Error> {
    let started = wire::boottime_ns();
    if request.ifindex == 0
        || request.frequency_mhz == 0
        || request.frequency_mhz > 100_000
        || request.bssid == [0; 6]
        || request.bssid[0] & 1 != 0
        || request.ssid.is_empty()
        || request.ssid.len() > 32
        || !valid_cutoff(started, request.original_until_ns)
    {
        return Err(Error::InvalidRequest);
    }
    let mut dispatched = false;
    wire::associate(request, started, &mut dispatched)
        .map_err(|failure| classify_failure(failure, dispatched))
}

/// Submit one explicit local Disconnect, then require its event and readback.
///
/// This is a separately requested action on a confirmed, exclusively owned
/// association. It must not be used as implicit recovery for an uncertain
/// CONNECT. No retry, automatic replacement or deadline renewal occurs here.
pub fn disconnect_once(request: DisconnectRequest) -> Result<Disconnected, Error> {
    let started = wire::boottime_ns();
    if request.ifindex == 0 || !valid_cutoff(started, request.original_until_ns) {
        return Err(Error::InvalidRequest);
    }
    let mut dispatched = false;
    wire::disconnect(request, started, &mut dispatched)
        .map_err(|failure| classify_failure(failure, dispatched))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AssociationRequest {
        AssociationRequest {
            ifindex: 3,
            frequency_mhz: 2412,
            bssid: [2, 0, 0, 0, 0, 0],
            ssid: b"authored".to_vec(),
            original_until_ns: wire::boottime_ns() + 1_000_000_000,
        }
    }

    #[test]
    fn invalid_association_selectors_refuse_before_netlink() {
        for changed in [
            AssociationRequest {
                ifindex: 0,
                ..request()
            },
            AssociationRequest {
                frequency_mhz: 0,
                ..request()
            },
            AssociationRequest {
                bssid: [0; 6],
                ..request()
            },
            AssociationRequest {
                bssid: [1; 6],
                ..request()
            },
            AssociationRequest {
                ssid: vec![],
                ..request()
            },
            AssociationRequest {
                ssid: vec![0; 33],
                ..request()
            },
            AssociationRequest {
                original_until_ns: 0,
                ..request()
            },
        ] {
            assert_eq!(associate_open_once(changed), Err(Error::InvalidRequest));
        }
        assert_eq!(
            disconnect_once(DisconnectRequest {
                ifindex: 0,
                original_until_ns: wire::boottime_ns() + 1_000_000_000
            }),
            Err(Error::InvalidRequest)
        );
    }

    #[test]
    fn action_cutoff_is_original_and_never_widened() {
        assert!(!valid_cutoff(100, 100));
        assert!(!valid_cutoff(100, 99));
        assert!(valid_cutoff(100, 5_000_000_100));
        assert!(!valid_cutoff(100, 5_000_000_101));
        assert_eq!(
            classify_failure(crate::Failure::Expired, true),
            Error::Uncertain(crate::Failure::Expired)
        );
        assert_eq!(
            classify_failure(crate::Failure::Kernel(2), false),
            Error::Failed(crate::Failure::Kernel(2))
        );
    }
}
