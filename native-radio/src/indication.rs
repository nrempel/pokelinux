//! Indication arguments recovered from the original stub and adapter.
//! Decoding preserves fragments; it does not reassemble or apply modem policy.
use rsbinder::{Parcel, Result, StatusCode};
#[derive(Debug, PartialEq, Eq)]
pub enum IndicationKind {
    Raw,
    Segment { index: i32, total_length: i32 },
}
#[derive(Debug, PartialEq, Eq)]
pub struct RadioIndication {
    pub message_id: i32,
    pub slot_id: i32,
    pub payload: Vec<u8>,
    pub kind: IndicationKind,
}
pub fn decode_indication_arguments(code: u32, input: &mut Parcel) -> Result<RadioIndication> {
    if code != 1 && code != 2 {
        return Err(StatusCode::UnknownTransaction);
    }
    let message_id = input.read_i32()?;
    let slot_id = input.read_i32()?;
    let payload = input.read::<Vec<u8>>()?;
    let kind = if code == 1 {
        IndicationKind::Raw
    } else {
        IndicationKind::Segment {
            index: input.read_i32()?,
            total_length: input.read_i32()?,
        }
    };
    if input.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(RadioIndication {
        message_id,
        slot_id,
        payload,
        kind,
    })
}
