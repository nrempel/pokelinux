//! Current local phase and separately historical provider observations.
use crate::projection::Projection;
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Initializing,
    Observing,
    Unavailable,
    Stopping,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema: u8,
    pub server_pid: u32,
    pub worker_pid: u32,
    pub phase: Phase,
    pub until_ns: u64,
    pub sampled_at_ns: u64,
    #[serde(deserialize_with = "crate::wire::required_nullable")]
    pub fault: Option<String>,
    pub observation: Projection,
}
impl Status {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec(self).map_err(|_| io::ErrorKind::InvalidData)?;
        bytes.push(b'\n');
        if bytes.len() > crate::ipc::MAX_PACKET {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
