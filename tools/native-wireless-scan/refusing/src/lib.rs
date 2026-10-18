//! Fixed-profile one-shot wireless scan API. This baseline performs no effects.

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
pub enum Error {
    Unavailable,
}

/// Explicit refusing baseline: no socket, kernel request, retry or child is created.
pub fn scan_once(_request: Request) -> Result<Completed, Error> {
    Err(Error::Unavailable)
}
