//! Native Data process and private diagnostic support. No APN or app authority.
pub mod config;
pub mod ipc;
pub mod output;
pub mod projection;
pub mod status;
pub mod wire;
pub fn now_ns() -> u64 {
    use pokelinux_process_owner::deadline::{BootClock, Clock};
    BootClock
        .now()
        .as_nanos()
        .try_into()
        .expect("bounded Linux uptime")
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../native-cellular-owner/src/fixture.rs"]
pub(crate) mod expected;

pub mod worker_policy;
