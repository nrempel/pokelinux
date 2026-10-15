//! Bounded owned registration wire fields, never IMS state or authority.
//! The owner selects an exact raw route and decodes the entire payload with
//! native-ims before copying these fields and rechecking its publication gate.
use omarchy_ims::registration::RegistrationWire as BorrowedRegistrationWire;

/// Original class wire code, qualified by the SDK routing oracle at826b820.
/// The observed enum ordinal is never used as a message identifier.
pub const MESSAGE_ID: i32 = 3012;

pub const MAX_URI_BYTES: usize = omarchy_ims::registration::MAX_URI_BYTES as usize;
pub const MAX_REASON_BYTES: usize = omarchy_ims::registration::MAX_REASON_BYTES as usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationWire {
    pub registration_state_wire: u8,
    pub feature_wire: u8,
    pub feature_mask_wire: i32,
    pub status_code_wire: u16,
    pub feature_mask_ext_wire: Option<i32>,
    pub sip_delegate_mask_wire: Option<i64>,
    pub feature_ext_wire: Option<u16>,
    uri_bytes: Vec<u8>,
    reason_bytes: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireBounds {
    UriTooLong,
    ReasonTooLong,
}
impl RegistrationWire {
    /// Own already supplied fields after checking byte limits. This performs
    /// no packet decoding, text conversion, enum validation or publication.
    pub fn new(fields: BorrowedRegistrationWire<'_>) -> Result<Self, WireBounds> {
        if fields.uri_bytes.len() > MAX_URI_BYTES {
            return Err(WireBounds::UriTooLong);
        }
        if fields.reason_bytes.len() > MAX_REASON_BYTES {
            return Err(WireBounds::ReasonTooLong);
        }
        Ok(Self {
            registration_state_wire: fields.registration_state_wire,
            feature_wire: fields.feature_wire,
            feature_mask_wire: fields.feature_mask_wire,
            status_code_wire: fields.status_code_wire,
            feature_mask_ext_wire: fields.feature_mask_ext_wire,
            sip_delegate_mask_wire: fields.sip_delegate_mask_wire,
            feature_ext_wire: fields.feature_ext_wire,
            uri_bytes: fields.uri_bytes.to_vec(),
            reason_bytes: fields.reason_bytes.to_vec(),
        })
    }
    pub fn uri_bytes(&self) -> &[u8] {
        &self.uri_bytes
    }
    pub fn reason_bytes(&self) -> &[u8] {
        &self.reason_bytes
    }
}
