//! Owned observations for the measured version22/base-response subset.
//! Interface and gateway strings are presentation, without link or APN authority.
use crate::{
    Protocol,
    response::{BaseSetupResponse, ServerFamilies},
};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportError {
    InvalidPresentationIndex,
    InvalidResponse,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModernSetupReport22 {
    pub cause: i32,
    pub opaque_i64_at_8: i64,
    pub cid: i32,
    pub active: i32,
    pub protocol_code: i32,
    pub mtu_v4: i32,
    pub mtu_v6: i32,
    pub ifname: Option<String>,
    pub address_text: Option<String>,
    pub dnses: Vec<String>,
    pub gateways: Vec<String>,
    pub pcscf: Vec<String>,
    pub structured_addresses: [(); 0],
    pub qos_sessions: [(); 0],
    pub traffic_descriptors: [(); 0],
}
impl ModernSetupReport22 {
    pub fn project(
        response: &BaseSetupResponse,
        presentation_index: u8,
    ) -> Result<Self, ReportError> {
        if presentation_index > 99 {
            return Err(ReportError::InvalidPresentationIndex);
        }
        let v4 = matches!(response.protocol, Protocol::Ipv4 | Protocol::Ipv4v6);
        let v6 = matches!(response.protocol, Protocol::Ipv6 | Protocol::Ipv4v6);
        if response.reported_activity > 2
            || response.dns_selector > 3
            || response.pcscf_selector > 3
            || response.ipv4.is_some() != v4
            || response.ipv6.is_some() != v6
            || response.dns.ipv4.is_some() != (response.dns_selector & 1 != 0)
            || response.dns.ipv6.is_some() != (response.dns_selector & 2 != 0)
            || response.pcscf.ipv4.is_some() != (response.pcscf_selector & 1 != 0)
            || response.pcscf.ipv6.is_some() != (response.pcscf_selector & 2 != 0)
        {
            return Err(ReportError::InvalidResponse);
        }
        let mut report = Self {
            cause: 65535,
            opaque_i64_at_8: 0,
            cid: i32::from(response.context.get()),
            active: i32::from(response.reported_activity),
            protocol_code: 0,
            mtu_v4: 0,
            mtu_v6: 0,
            ifname: None,
            address_text: None,
            dnses: Vec::new(),
            gateways: Vec::new(),
            pcscf: Vec::new(),
            structured_addresses: [],
            qos_sessions: [],
            traffic_descriptors: [],
        };
        if response.reported_activity == 0 {
            return Ok(report);
        }
        report.cause = 0;
        report.protocol_code = match response.protocol {
            Protocol::Ipv4 => 0,
            Protocol::Ipv6 => 1,
            Protocol::Ipv4v6 => 2,
            Protocol::NonIp => 4,
        };
        // Preserve the original scalar's32-bit pattern, not an MTU range policy.
        report.mtu_v4 = response.reported_mtu_v4 as i32;
        report.ifname = Some(format!("rmnet{presentation_index}"));
        let mut local = Vec::new();
        if let Some(address) = response.ipv6 {
            local.push(format!("{}/64", ipv6_text(address)));
            if address != [0; 16] {
                report.gateways.push("::".to_owned());
            }
        }
        if let Some(address) = response.ipv4 {
            local.push(Ipv4Addr::from(address).to_string());
            if address != [0; 4] {
                report.gateways.push("0.0.0.0".to_owned());
            }
        }
        // Non-IP retains a present empty string. Gateway strings are defaults
        // in this report shape, never discovered kernel routes or remote peers.
        report.address_text = Some(local.join(" "));
        report.dnses = server_text(&response.dns);
        report.pcscf = server_text(&response.pcscf);
        Ok(report)
    }
}

fn server_text<const COUNT: usize>(families: &ServerFamilies<COUNT>) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(addresses) = &families.ipv6 {
        values.extend(
            addresses
                .iter()
                .filter(|a| **a != [0; 16])
                .map(|a| ipv6_text(*a)),
        );
    }
    if let Some(addresses) = &families.ipv4 {
        values.extend(
            addresses
                .iter()
                .filter(|a| **a != [0; 4])
                .map(|a| Ipv4Addr::from(*a).to_string()),
        );
    }
    values
}
fn ipv6_text(address: [u8; 16]) -> String {
    // Captured Bionic IPv4-compatible spelling differs from Rust Display when
    // the low32 bits occupy both final16-bit groups. Keep that observed form;
    // all other values use the native Rust presentation policy. Finite original
    // comparisons do not prove arbitrary inet_ntop equivalence.
    if address[..12] == [0; 12] && address[12..14] != [0; 2] {
        let tail: [u8; 4] = address[12..].try_into().expect("fixed IPv6 tail");
        format!("::{}", Ipv4Addr::from(tail))
    } else {
        Ipv6Addr::from(address).to_string()
    }
}
