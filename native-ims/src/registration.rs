//! Original-compared raw registration-indication fields, without IMS authority.
//! Only a complete bounded packet returns a value. No state/RAT, URI identity,
//! feature support or semantic publication is inferred from these wire fields.
use crate::reader::{DataReader, ReadError};

// Original instance limits clamp larger declarations. Native policy refuses them.
pub const MAX_URI_BYTES: u16 = 1000;
pub const MAX_REASON_BYTES: u16 = 256;

#[derive(Debug, PartialEq, Eq)]
pub struct RegistrationWire<'a> {
    pub registration_state_wire: u8,
    pub feature_wire: u8,
    pub feature_mask_wire: i32,
    pub status_code_wire: u16,
    pub uri_bytes: &'a [u8],
    pub reason_bytes: &'a [u8],
    pub feature_mask_ext_wire: Option<i32>,
    pub sip_delegate_mask_wire: Option<i64>,
    pub feature_ext_wire: Option<u16>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    Input(ReadError),
    UriTooLong,
    ReasonTooLong,
}
impl From<ReadError> for DecodeError {
    fn from(value: ReadError) -> Self {
        Self::Input(value)
    }
}

pub fn decode(bytes: &[u8]) -> Result<RegistrationWire<'_>, DecodeError> {
    let length = i32::try_from(bytes.len()).map_err(|_| ReadError::InputTooLarge)?;
    decode_with_length(bytes, length)
}

/// The original caller supplies a length starting at byte zero. Invalid ranges
/// fail instead of being clamped; bytes outside a valid range are not consumed.
/// Extension availability follows the observed original greedy read order. No
/// protocol version is invented; leftover bytes refuse the complete packet.
pub fn decode_with_length(bytes: &[u8], length: i32) -> Result<RegistrationWire<'_>, DecodeError> {
    let mut reader = DataReader::from_range(bytes, 0, length)?;
    let registration_state_wire = reader.read_u8()?;
    let feature_wire = reader.read_u8()?;
    let feature_mask_wire = reader.read_i32()?;
    let status_code_wire = reader.read_u16()?;
    let uri_length = reader.read_u16()?;
    let reason_length = reader.read_u16()?;
    if uri_length > MAX_URI_BYTES {
        return Err(DecodeError::UriTooLong);
    }
    if reason_length > MAX_REASON_BYTES {
        return Err(DecodeError::ReasonTooLong);
    }
    let uri_bytes = reader.read_bytes(i32::from(uri_length))?;
    let reason_bytes = reader.read_bytes(i32::from(reason_length))?;
    let feature_mask_ext_wire = if reader.remaining() >= 4 {
        Some(reader.read_i32()?)
    } else {
        None
    };
    let sip_delegate_mask_wire = if reader.remaining() >= 8 {
        Some(reader.read_i64()?)
    } else {
        None
    };
    let feature_ext_wire = if reader.remaining() >= 2 {
        Some(reader.read_u16()?)
    } else {
        None
    };
    reader.finish()?;
    Ok(RegistrationWire {
        registration_state_wire,
        feature_wire,
        feature_mask_wire,
        status_code_wire,
        uri_bytes,
        reason_bytes,
        feature_mask_ext_wire,
        sip_delegate_mask_wire,
        feature_ext_wire,
    })
}
