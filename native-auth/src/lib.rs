//! Per-attempt Linux worker-tree ownership. This is not authentication authority.
//!
//! The root-only isolated entry is the first qualification target. There is no
//! production broker, application-selected program, service, account or cgroup.
#![forbid(unsafe_code)]
#![cfg(feature = "isolated-fixture")]

mod cgroup;
pub mod command;
pub mod owner;
pub mod parent;
pub mod record;

pub fn boottime_ns() -> u64 {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (value.tv_sec as u64) * 1_000_000_000 + value.tv_nsec as u64
}
