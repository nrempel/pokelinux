//! Installed Messages data provider. Current publication belongs to the shared
//! lifecycle actor; this provider has no app grant, modem send/ACK or simulator.
pub mod projection;
pub mod protocol;
pub mod worker;

pub mod incoming;
