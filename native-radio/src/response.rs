//! Vendor response argument decoding. The Binder dispatcher must validate
//! the interface token before calling this module; arguments start after it.
use crate::RadioExternalResponseInfo;
use rsbinder::{Parcel, Result, StatusCode};

#[derive(Debug, PartialEq, Eq)]
pub enum ResponseKind {
    Raw,
    /// Preserve the two wire integers until sender-side semantics are verified.
    /// Receiving a segment must not silently complete or discard a request.
    Segmented {
        parameters: [i32; 2],
    },
}

#[derive(Debug)]
pub struct RadioResponse {
    pub info: RadioExternalResponseInfo,
    pub payload: Vec<u8>,
    pub kind: ResponseKind,
}

/// Decode callback transaction 1 or 2, rejecting malformed or trailing data.
///
/// Returns an event only after the complete argument sequence has been read.
/// It does not mutate pending requests, validate modem policy, or reassemble
/// segments. Metadata fields are preserved even when their semantics are unknown.
pub fn decode_response_arguments(code: u32, parcel: &mut Parcel) -> Result<RadioResponse> {
    if code != 1 && code != 2 {
        return Err(StatusCode::UnknownTransaction);
    }
    let info = parcel.read::<RadioExternalResponseInfo>()?;
    let payload = parcel.read::<Vec<u8>>()?;
    let kind = if code == 1 {
        ResponseKind::Raw
    } else {
        ResponseKind::Segmented {
            parameters: [parcel.read::<i32>()?, parcel.read::<i32>()?],
        }
    };
    if parcel.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(RadioResponse {
        info,
        payload,
        kind,
    })
}
