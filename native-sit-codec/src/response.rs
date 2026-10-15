//! A complete base144-byte setup response, limited to the measured original shape.
//! Token comparison is local wire correlation, not provider or APN authorization.
use crate::Protocol;
use std::num::NonZeroU8;

pub const BASE_SETUP_RESPONSE_BYTES: usize = 144;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnsupportedLength,
    WrongType,
    UnsupportedHeader,
    WrongMessage,
    DeclaredLength,
    TokenMismatch,
    HeaderError(u8),
    PdpStatus(u16),
    InvalidContext,
    UnsupportedActivity,
    UnsupportedProtocol,
    UnsupportedSelector,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressInfo<const N: usize> {
    pub address: [u8; N],
    pub dns: [[u8; N]; 2],
    pub pcscf: [[u8; N]; 3],
}
/// Selector presence is independent of address bytes being all zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerFamilies<const COUNT: usize> {
    pub ipv4: Option<[[u8; 4]; COUNT]>,
    pub ipv6: Option<[[u8; 16]; COUNT]>,
}
/// Parsed observations only. Numeric activity is the original parser's output;
/// neither it nor an address establishes a live network or permission to use it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseSetupResponse {
    pub token: u32,
    pub context: NonZeroU8,
    pub reported_activity: u8,
    pub protocol: Protocol,
    pub dns_selector: u8,
    pub pcscf_selector: u8,
    pub ipv4: Option<[u8; 4]>,
    pub ipv6: Option<[u8; 16]>,
    pub dns: ServerFamilies<2>,
    pub pcscf: ServerFamilies<3>,
    /// Original base item127 -> DataCall124 -> modern report MTUV4.
    /// A wire observation, without a range check or network configuration policy.
    pub reported_mtu_v4: u32,
    pub opaque_u8_at_131: u8,
}
impl BaseSetupResponse {
    pub fn parse(packet: &[u8], expected_token: u32) -> Result<Self, DecodeError> {
        let body = checked_body(packet, expected_token)?;
        let protocol = match body[4] {
            1 => Protocol::Ipv4,
            2 => Protocol::Ipv6,
            3 => Protocol::Ipv4v6,
            5 => Protocol::NonIp,
            _ => unreachable!("protocol checked before decoding"),
        };
        Ok(Self {
            token: expected_token,
            context: NonZeroU8::new(body[2]).expect("context checked before decoding"),
            // Preserve the original base mapper's1/2 translation. No connected
            // or APN-ready interpretation is assigned to these numeric codes.
            reported_activity: match body[3] {
                1 => 2,
                2 => 1,
                value => value,
            },
            protocol,
            dns_selector: body[25],
            pcscf_selector: body[66],
            ipv4: matches!(protocol, Protocol::Ipv4 | Protocol::Ipv4v6).then(|| octets(body, 5)),
            ipv6: matches!(protocol, Protocol::Ipv6 | Protocol::Ipv4v6).then(|| octets(body, 9)),
            dns: ServerFamilies {
                ipv4: (body[25] & 1 != 0).then(|| [26, 46].map(|offset| octets(body, offset))),
                ipv6: (body[25] & 2 != 0).then(|| [30, 50].map(|offset| octets(body, offset))),
            },
            pcscf: ServerFamilies {
                ipv4: (body[66] & 1 != 0)
                    .then(|| std::array::from_fn(|i| octets(body, 67 + i * 4))),
                ipv6: (body[66] & 2 != 0)
                    .then(|| std::array::from_fn(|i| octets(body, 79 + i * 16))),
            },
            reported_mtu_v4: u32::from_le_bytes(octets(body, 127)),
            opaque_u8_at_131: body[131],
        })
    }
    /// The original getter omits a family when its local-address protocol is absent.
    pub fn address_info_v4(&self) -> Option<AddressInfo<4>> {
        self.ipv4.map(|address| AddressInfo {
            address,
            dns: self.dns.ipv4.unwrap_or([[0; 4]; 2]),
            pcscf: self.pcscf.ipv4.unwrap_or([[0; 4]; 3]),
        })
    }
    pub fn address_info_v6(&self) -> Option<AddressInfo<16>> {
        self.ipv6.map(|address| AddressInfo {
            address,
            dns: self.dns.ipv6.unwrap_or([[0; 16]; 2]),
            pcscf: self.pcscf.ipv6.unwrap_or([[0; 16]; 3]),
        })
    }
}
fn checked_body(packet: &[u8], expected_token: u32) -> Result<&[u8], DecodeError> {
    if packet.len() != BASE_SETUP_RESPONSE_BYTES {
        return Err(DecodeError::UnsupportedLength);
    }
    if packet[0] != 1 {
        return Err(DecodeError::WrongType);
    }
    if packet[1] != 0 || packet[11] != 0 {
        return Err(DecodeError::UnsupportedHeader);
    }
    if u16::from_le_bytes([packet[2], packet[3]]) != 0x600 {
        return Err(DecodeError::WrongMessage);
    }
    if u16::from_le_bytes([packet[4], packet[5]]) != BASE_SETUP_RESPONSE_BYTES as u16 {
        return Err(DecodeError::DeclaredLength);
    }
    if u32::from_le_bytes(packet[6..10].try_into().expect("bounded header")) != expected_token {
        return Err(DecodeError::TokenMismatch);
    }
    if packet[10] != 0 {
        return Err(DecodeError::HeaderError(packet[10]));
    }
    let body = &packet[12..];
    let status = u16::from_le_bytes([body[0], body[1]]);
    if status != 0 {
        return Err(DecodeError::PdpStatus(status));
    }
    if body[2] == 0 {
        return Err(DecodeError::InvalidContext);
    }
    if body[3] > 2 {
        return Err(DecodeError::UnsupportedActivity);
    }
    if !matches!(body[4], 1 | 2 | 3 | 5) {
        return Err(DecodeError::UnsupportedProtocol);
    }
    if body[25] > 3 || body[66] > 3 {
        return Err(DecodeError::UnsupportedSelector);
    }
    Ok(body)
}

fn octets<const N: usize>(body: &[u8], offset: usize) -> [u8; N] {
    body[offset..offset + N]
        .try_into()
        .expect("fixed base item bounds")
}
