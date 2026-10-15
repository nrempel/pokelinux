//! Bounded add-PDN request bytes. This does not acquire a bearer, submit a
//! request, establish IMS registration, or authorize an APN context.
use std::net::{Ipv4Addr, Ipv6Addr};

pub const MESSAGE_ID: i32 = 105;
/// Local supported bound; no vendor limit or count-clamping policy is inferred.
pub const MAX_PCSCF_ADDRESSES: usize = 8;
pub const MAX_ENCODED_BYTES: usize = 663 + 4 * MAX_PCSCF_ADDRESSES + 16 * MAX_PCSCF_ADDRESSES;

/// Already selected civil components, not a wall-clock/timezone conversion API.
/// Milliseconds and timezone names have no representation in this wire field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc_offset_minutes: i16,
}
impl WireTime {
    fn valid(self) -> bool {
        if !(2000..=2255).contains(&self.year)
            || !(1..=12).contains(&self.month)
            || self.hour > 23
            || self.minute > 59
            || self.second > 59
        {
            return false;
        }
        let leap = self.year.is_multiple_of(4)
            && (!self.year.is_multiple_of(100) || self.year.is_multiple_of(400));
        let days = match self.month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        self.day >= 1 && self.day <= days
    }
}
/// Vendor wire fields only. Selector0/2 is not a cellular/WiFi or IMS-state enum.
/// Slice lengths supply counts; caller count/array disagreement is unrepresentable.
#[derive(Clone)]
pub struct AddPdn<'a> {
    pub pdn_type_wire: u8,
    pub ip_type_wire: u8,
    pub pdn_ipv4: Ipv4Addr,
    pub pdn_ipv6: Ipv6Addr,
    pub hd_enabled_wire: u8,
    pub operator_id_wire: u8,
    pub registration_style_wire: u8,
    pub new_pdn_wire: u8,
    pub volte_auto_config_wire: u8,
    pub rcs_auto_config_wire: u8,
    pub offering_type_wire: u16,
    pub registration_type_wire: u8,
    pub init_type_wire: u8,
    pub ims_net_type_wire: u8,
    pub mac: Option<[u8; 6]>,
    pub pcscf_ipv4: &'a [Ipv4Addr],
    pub pcscf_ipv6: &'a [Ipv6Addr],
    pub primary_dns_ipv4: Ipv4Addr,
    pub primary_dns_ipv6: Ipv6Addr,
    pub secondary_dns_ipv4: Ipv4Addr,
    pub secondary_dns_ipv6: Ipv6Addr,
    pub last_access_network_wire: u8,
    pub last_access_time: Option<WireTime>,
    pub time: Option<WireTime>,
    pub tac_or_lac_wire: u32,
    pub cell_id32_wire: u32,
    pub remaining_timer_wire: u32,
    pub mcc_mnc: Option<&'a str>,
    pub country: Option<&'a str>,
    pub mtu_wire: u16,
    pub rcs_capability_wire: u32,
    pub vendor_capability_wire: u16,
    pub extended_rcs_capability_wire: u32,
    pub e911_aid: Option<&'a str>,
    pub band_wire: u32,
    pub cell_id64_wire: u64,
    pub network_interface_index_wire: u32,
    pub sip_delegate_capability_wire: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    UnsupportedSelector,
    TooManyIpv4,
    TooManyIpv6,
    InvalidTime,
    InvalidMccMnc,
    InvalidCountry,
    InvalidE911Aid,
}
impl AddPdn<'_> {
    fn validate(&self) -> Result<(), EncodeError> {
        if !matches!(self.ims_net_type_wire, 0 | 2) {
            return Err(EncodeError::UnsupportedSelector);
        }
        if self.pcscf_ipv4.len() > MAX_PCSCF_ADDRESSES {
            return Err(EncodeError::TooManyIpv4);
        }
        if self.pcscf_ipv6.len() > MAX_PCSCF_ADDRESSES {
            return Err(EncodeError::TooManyIpv6);
        }
        if self.last_access_time.is_some_and(|time| !time.valid())
            || self.time.is_some_and(|time| !time.valid())
        {
            return Err(EncodeError::InvalidTime);
        }
        if self
            .mcc_mnc
            .is_some_and(|text| text.len() != 6 || !text.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(EncodeError::InvalidMccMnc);
        }
        if self
            .country
            .is_some_and(|text| text.len() > 2 || !text.is_ascii())
        {
            return Err(EncodeError::InvalidCountry);
        }
        if self
            .e911_aid
            .is_some_and(|text| text.len() > 32 || !text.is_ascii())
        {
            return Err(EncodeError::InvalidE911Aid);
        }
        Ok(())
    }
}
/// Reject unsupported inputs before emitting anything; no silent clamps, DNS,
/// default charset, external state or cached optional bytes participate.
pub fn encode(config: &AddPdn<'_>) -> Result<Vec<u8>, EncodeError> {
    config.validate()?;
    let mut output = Vec::with_capacity(MAX_ENCODED_BYTES);
    output.extend_from_slice(&[config.pdn_type_wire, config.ip_type_wire]);
    output.extend_from_slice(&config.pdn_ipv4.octets());
    output.extend_from_slice(&config.pdn_ipv6.octets());
    output.extend_from_slice(&[
        config.hd_enabled_wire,
        config.operator_id_wire,
        config.registration_style_wire,
        config.new_pdn_wire,
        config.volte_auto_config_wire,
        config.rcs_auto_config_wire,
    ]);
    output.extend_from_slice(&config.offering_type_wire.to_le_bytes());
    output.extend_from_slice(&[
        config.registration_type_wire,
        config.init_type_wire,
        config.ims_net_type_wire,
    ]);
    output.extend_from_slice(&config.mac.unwrap_or([0; 6]));
    output.push(config.pcscf_ipv4.len() as u8);
    for address in config.pcscf_ipv4 {
        output.extend_from_slice(&address.octets());
    }
    output.push(config.pcscf_ipv6.len() as u8);
    for address in config.pcscf_ipv6 {
        output.extend_from_slice(&address.octets());
    }
    output.extend_from_slice(&config.primary_dns_ipv4.octets());
    output.extend_from_slice(&config.primary_dns_ipv6.octets());
    output.extend_from_slice(&config.secondary_dns_ipv4.octets());
    output.extend_from_slice(&config.secondary_dns_ipv6.octets());
    if config.ims_net_type_wire == 2 {
        output.push(config.last_access_network_wire);
        append_time(&mut output, config.last_access_time);
        append_time(&mut output, config.time);
        // The main field stores the low 16 bits; the tail carries the full value.
        output.extend_from_slice(&(config.tac_or_lac_wire as u16).to_le_bytes());
        output.extend_from_slice(&config.cell_id32_wire.to_le_bytes());
    } else {
        output.resize(output.len() + 23, 0);
    }
    output.extend_from_slice(&config.remaining_timer_wire.to_le_bytes());
    if config.ims_net_type_wire == 2 {
        // Each MCC/MNC digit is its numeric value, not ASCII or packed BCD.
        if let Some(value) = config.mcc_mnc {
            output.extend(value.bytes().map(|digit| digit - b'0'));
        } else {
            output.extend_from_slice(&[0; 6]);
        }
        let country_end = output.len() + 2;
        output.extend_from_slice(config.country.unwrap_or("").as_bytes());
        output.resize(country_end, 0);
        output.extend_from_slice(&config.mtu_wire.to_le_bytes());
    } else {
        output.resize(output.len() + 10, 0);
    }
    output.extend_from_slice(&config.rcs_capability_wire.to_le_bytes());
    output.extend_from_slice(&config.vendor_capability_wire.to_le_bytes());
    output.extend_from_slice(&config.extended_rcs_capability_wire.to_le_bytes());
    let aid = config.e911_aid.unwrap_or("").as_bytes();
    output.push(aid.len() as u8);
    let aid_end = output.len() + 32;
    output.extend_from_slice(aid);
    output.resize(aid_end, 0);
    let additional_length: u16 = if config.ims_net_type_wire == 2 {
        48
    } else {
        12
    };
    output.extend_from_slice(&additional_length.to_le_bytes());
    let additional_end = output.len() + 500;
    output.extend_from_slice(&[1, 0, 8, 0]);
    output.extend_from_slice(&config.sip_delegate_capability_wire.to_le_bytes());
    if config.ims_net_type_wire == 2 {
        output.extend_from_slice(&[2, 0, 4, 0]);
        output.extend_from_slice(&config.band_wire.to_le_bytes());
        output.extend_from_slice(&[4, 0, 8, 0]);
        output.extend_from_slice(&config.cell_id64_wire.to_le_bytes());
        output.extend_from_slice(&[5, 0, 4, 0]);
        output.extend_from_slice(&config.tac_or_lac_wire.to_le_bytes());
        output.extend_from_slice(&[6, 0, 4, 0]);
        output.extend_from_slice(&config.network_interface_index_wire.to_le_bytes());
    }
    // getBytes returns the entire backing array, not just its logical TLV length.
    output.resize(additional_end, 0);
    Ok(output)
}

fn append_time(output: &mut Vec<u8>, time: Option<WireTime>) {
    if let Some(time) = time {
        output.extend_from_slice(&[
            (time.year - 2000) as u8,
            time.month,
            time.day,
            time.hour,
            time.minute,
            time.second,
        ]);
        output.extend_from_slice(&time.utc_offset_minutes.to_le_bytes());
    } else {
        output.extend_from_slice(&[0; 8]);
    }
}
