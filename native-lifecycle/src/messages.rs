//! Local Messages data provider. This owns no modem effect authority.
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub query: serde_json::Value,
    pub ttl_ms: u64,
}
pub enum Event {
    Complete { token: u64, packet: Vec<u8> },
    Failed { token: u64 },
}
/// Child owners retain occupied capacity until their actual child is reaped.
/// A remote provider reports its local connection/transaction ownership; closing
/// it does not certify retirement of children retained by the external service.
/// The lifecycle actor checks current authority at publication, outside storage IO.
pub trait Provider {
    /// Read lifetime incarnation; independent of incoming source identity.
    fn incarnation(&self) -> &str;
    /// Owned local resources, including terminal children not yet reaped.
    fn occupied(&self) -> bool;
    /// Owned local read capacity. Direct child owners retain it until actual reap.
    fn read_occupied(&self) -> bool;
    fn busy(&self) -> bool;
    fn available(&self) -> bool;
    fn incoming_progress(&self) -> (u64, u64, u64) {
        (0, 0, 0)
    }
    fn receiving(&self) -> bool {
        false
    }
    fn incoming_failure(&self) -> Option<&str> {
        None
    }
    fn start(&mut self, query: serde_json::Value, until: u64) -> io::Result<u64>;
    fn poll(&mut self) -> io::Result<Option<Event>>;
    /// Cancel only this exact pending read; receiving remains separately owned.
    fn cancel_read(&mut self, token: u64);
    fn stop(&mut self);
    fn restart(&mut self) -> io::Result<()>;
    fn drain_until(&mut self, until: u64) -> bool;
}
pub const APP: &str = "org.omarchy.Messages";
pub const DESKTOP_UID: u32 = 1000;
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopRequest {
    LaunchMessages {},
}
