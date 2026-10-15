//! Bounded Samsung SIT codecs. Wire observations grant no transport or APN authority.
#![forbid(unsafe_code)]
use std::num::NonZeroU8;

pub mod modern_report;
pub mod response;

pub const INITIAL_LTE_SETUP_V3_BYTES: usize = 983;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Ipv4,
    Ipv6,
    Ipv4v6,
    NonIp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    InvalidApn,
}
/// One initial token-zero packet in a measured original encoder subset.
/// This carries no acquired APN, interface assignment or dispatch authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitialLteSetupV3 {
    context: NonZeroU8,
    apn: String,
    protocol: Protocol,
}
impl InitialLteSetupV3 {
    pub fn new(context: NonZeroU8, apn: &str, protocol: Protocol) -> Result<Self, Error> {
        if apn.is_empty()
            || apn.len() > 99
            || !apn
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        {
            return Err(Error::InvalidApn);
        }
        Ok(Self {
            context,
            apn: apn.into(),
            protocol,
        })
    }
    pub fn encode(&self) -> Result<[u8; INITIAL_LTE_SETUP_V3_BYTES], Error> {
        let mut packet = [0; INITIAL_LTE_SETUP_V3_BYTES];
        // Original complete 12-byte RCM request header: initial token is zero.
        packet[2..4].copy_from_slice(&0x0600u16.to_le_bytes());
        packet[4..6].copy_from_slice(&(INITIAL_LTE_SETUP_V3_BYTES as u16).to_le_bytes());
        packet[12] = self.context.get();
        packet[13] = 14; // This first measured subset fixes the radio argument to LTE.
        packet[16..16 + self.apn.len()].copy_from_slice(self.apn.as_bytes());
        packet[218] = match self.protocol {
            Protocol::Ipv4 => 1,
            Protocol::Ipv6 => 2,
            Protocol::Ipv4v6 => 3,
            Protocol::NonIp => 5,
        };
        // Preserve the original APN factory's default scalar. Its policy meaning
        // remains unresolved; this pure codec grants no provider capability.
        packet[241..245].copy_from_slice(&1u32.to_le_bytes());
        packet[245] = 2; // Actual longer-format branch discriminator.
        Ok(packet)
    }
}
#[cfg(test)]
mod tests;
