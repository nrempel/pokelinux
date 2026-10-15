//! Bounded historical radio transport observations, not phone/IMS state or receipts.
//! Identity/attempt are provenance tags; the parent must also bind the actual
//! source connection and current Supervisor admission. This codec grants nothing.
#![forbid(unsafe_code)]
use crate::{
    owner::{Phase, Snapshot},
    receiver::ReceiverFailure,
    response::ResponseKind,
    supervisor::ProcessIdentity,
};
use std::io;

pub const MAX_PAYLOAD: usize = 2048;
pub const HEADER_LEN: usize = 44;
const RAW_FIXED_LEN: usize = 24;
pub const MAX_FRAME_LEN: usize = HEADER_LEN + RAW_FIXED_LEN + MAX_PAYLOAD;
const MAGIC: &[u8; 4] = b"RADO";
const VERSION: u8 = 1;
const ATTEMPT: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    Active {
        client_id: i32,
    },
    Raw {
        slot_id: i32,
        serial: i32,
        rilc_msg_id: i32,
        declared_length: i32,
        error: i32,
        payload: Vec<u8>,
    },
    Failed(ReceiverFailure),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub identity: ProcessIdentity,
    pub attempt: u64,
    pub revision: u64,
    pub kind: Kind,
}
fn malformed() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(
        bytes[at..at + 4]
            .try_into()
            .expect("validated frame bounds"),
    )
}
fn i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(
        bytes[at..at + 4]
            .try_into()
            .expect("validated frame bounds"),
    )
}
fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(
        bytes[at..at + 8]
            .try_into()
            .expect("validated frame bounds"),
    )
}
fn failure_code(reason: ReceiverFailure) -> u8 {
    match reason {
        ReceiverFailure::MalformedCallback => 1,
        ReceiverFailure::QueueFull => 2,
        ReceiverFailure::ConsumerGone => 3,
        ReceiverFailure::InternalFailure => 4,
        ReceiverFailure::PublisherGone => 5,
        ReceiverFailure::Cancelled => 6,
        ReceiverFailure::ServiceDied => 7,
        ReceiverFailure::MonitoringFailed => 8,
        ReceiverFailure::Expired => 9,
    }
}
fn failure(code: u8) -> io::Result<ReceiverFailure> {
    Ok(match code {
        1 => ReceiverFailure::MalformedCallback,
        2 => ReceiverFailure::QueueFull,
        3 => ReceiverFailure::ConsumerGone,
        4 => ReceiverFailure::InternalFailure,
        5 => ReceiverFailure::PublisherGone,
        6 => ReceiverFailure::Cancelled,
        7 => ReceiverFailure::ServiceDied,
        8 => ReceiverFailure::MonitoringFailed,
        9 => ReceiverFailure::Expired,
        _ => return Err(malformed()),
    })
}

impl Frame {
    /// Header: magic[0..4], version[4], kind[5], zero reserved[6..8],
    /// total length[8..12], incarnation[12..28], attempt[28..36], revision[36..44].
    /// Integers are LE. Raw body is five i32 fields, u32 payload size, then bytes.
    /// Max2116 bytes fits Linux PIPE_BUF4096; the writer must separately arrange
    /// a nonblocking pipe and one whole-frame write. This codec performs no IO.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        if self.attempt != ATTEMPT {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let (kind, body_len) = match &self.kind {
            Kind::Active { .. } => (1, 4),
            Kind::Raw { payload, .. } if payload.len() <= MAX_PAYLOAD => {
                (2, RAW_FIXED_LEN + payload.len())
            }
            Kind::Raw { .. } => return Err(io::ErrorKind::InvalidInput.into()),
            Kind::Failed(_) => (3, 1),
        };
        let length = HEADER_LEN + body_len;
        let mut bytes = Vec::with_capacity(length);
        bytes.extend(MAGIC);
        bytes.extend([VERSION, kind, 0, 0]);
        bytes.extend((length as u32).to_le_bytes());
        bytes.extend(self.identity.0);
        bytes.extend(self.attempt.to_le_bytes());
        bytes.extend(self.revision.to_le_bytes());
        match &self.kind {
            Kind::Active { client_id } => bytes.extend(client_id.to_le_bytes()),
            Kind::Raw {
                slot_id,
                serial,
                rilc_msg_id,
                declared_length,
                error,
                payload,
            } => {
                for value in [slot_id, serial, rilc_msg_id, declared_length, error] {
                    bytes.extend(value.to_le_bytes());
                }
                bytes.extend((payload.len() as u32).to_le_bytes());
                bytes.extend(payload);
            }
            Kind::Failed(reason) => bytes.push(failure_code(*reason)),
        }
        Ok(bytes)
    }
}

/// Decode one complete prefix and report the exact consumed length. None means
/// incomplete input, not EOF acceptance. Caller bounds its incremental buffer;
/// call decode_exact on a remaining EOF tail to reject truncation/trailing bytes.
/// Bytes beyond one frame belong to the next frame and are not consumed here.
pub fn decode(bytes: &[u8]) -> io::Result<Option<(Frame, usize)>> {
    if bytes.len() < HEADER_LEN {
        return Ok(None);
    }
    if &bytes[..4] != MAGIC
        || bytes[4] != VERSION
        || bytes[6..8] != [0, 0]
        || u64_at(bytes, 28) != ATTEMPT
    {
        return Err(malformed());
    }
    let length = u32_at(bytes, 8) as usize;
    match bytes[5] {
        1 if length == HEADER_LEN + 4 => {}
        2 if (HEADER_LEN + RAW_FIXED_LEN..=MAX_FRAME_LEN).contains(&length) => {}
        3 if length == HEADER_LEN + 1 => {}
        _ => return Err(malformed()),
    }
    if bytes[5] == 2 && bytes.len() >= HEADER_LEN + RAW_FIXED_LEN {
        let payload_len = u32_at(bytes, HEADER_LEN + 20) as usize;
        if payload_len > MAX_PAYLOAD || length != HEADER_LEN + RAW_FIXED_LEN + payload_len {
            return Err(malformed());
        }
    }
    if bytes.len() < length {
        return Ok(None);
    }
    let body = &bytes[HEADER_LEN..length];
    let kind = match bytes[5] {
        1 => Kind::Active {
            client_id: i32_at(body, 0),
        },
        2 => Kind::Raw {
            slot_id: i32_at(body, 0),
            serial: i32_at(body, 4),
            rilc_msg_id: i32_at(body, 8),
            declared_length: i32_at(body, 12),
            error: i32_at(body, 16),
            payload: body[RAW_FIXED_LEN..].to_vec(),
        },
        3 => Kind::Failed(failure(body[0])?),
        _ => return Err(malformed()),
    };
    Ok(Some((
        Frame {
            identity: ProcessIdentity(bytes[12..28].try_into().map_err(|_| malformed())?),
            attempt: ATTEMPT,
            revision: u64_at(bytes, 36),
            kind,
        },
        length,
    )))
}

/// Decode exactly one frame; reject a truncated frame or any trailing bytes.
pub fn decode_exact(bytes: &[u8]) -> io::Result<Frame> {
    match decode(bytes)? {
        None => Err(io::ErrorKind::UnexpectedEof.into()),
        Some((frame, consumed)) if consumed == bytes.len() => Ok(frame),
        Some(_) => Err(malformed()),
    }
}

/// Snapshot clones are historical observations. This does not finalize requests,
/// decode semantic state, authenticate callers or itself publish to a parent.
/// declared_length is preserved independently of actual payload size; recovered
/// response fixtures permit those fields to differ. No new semantic rule is added.
pub fn from_snapshot(identity: ProcessIdentity, snapshot: &Snapshot) -> io::Result<Option<Frame>> {
    if snapshot
        .response
        .as_ref()
        .is_some_and(|response| response.kind != ResponseKind::Raw)
    {
        return Err(io::ErrorKind::Unsupported.into());
    }
    let kind = match snapshot.phase {
        Phase::Idle | Phase::Registering => return Ok(None),
        Phase::Failed(reason) => Kind::Failed(reason),
        Phase::Active { client_id } => match &snapshot.response {
            None => Kind::Active { client_id },
            Some(response) => {
                if response.payload.len() > MAX_PAYLOAD {
                    return Err(malformed());
                }
                Kind::Raw {
                    slot_id: response.info.slotId,
                    serial: response.info.serial,
                    rilc_msg_id: response.info.rilcMsgId,
                    declared_length: response.info.length,
                    error: response.info.error,
                    payload: response.payload.clone(),
                }
            }
        },
    };
    Ok(Some(Frame {
        identity,
        attempt: ATTEMPT,
        revision: snapshot.revision,
        kind,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(size: usize) -> Frame {
        Frame {
            identity: ProcessIdentity::from_hex("0123456789abcdef0123456789abcdef").unwrap(),
            attempt: ATTEMPT,
            revision: 7,
            kind: Kind::Raw {
                slot_id: -1,
                serial: 7,
                rilc_msg_id: 137,
                declared_length: 100,
                error: -19,
                payload: vec![7; size],
            },
        }
    }
    #[test]
    fn partial_input_never_releases_a_frame_and_exact_rejects_trailing_data() {
        let frame = raw(3);
        let bytes = frame.encode().unwrap();
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).unwrap().is_none());
            assert_eq!(
                decode_exact(&bytes[..end]).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
        assert_eq!(decode_exact(&bytes).unwrap(), frame);
        let mut joined = bytes.clone();
        joined.extend(&bytes);
        let (decoded, consumed) = decode(&joined).unwrap().unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
        assert_eq!(
            decode_exact(&joined).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
    #[test]
    fn malformed_headers_and_payload_lengths_fail_without_large_allocation() {
        let bytes = raw(3).encode().unwrap();
        for (offset, replacement) in [(0, 0), (4, 2), (5, 99), (6, 1), (28, 2)] {
            let mut invalid = bytes.clone();
            invalid[offset] = replacement;
            assert!(decode(&invalid).is_err());
        }
        for length in [0, HEADER_LEN as u32, (MAX_FRAME_LEN + 1) as u32, u32::MAX] {
            let mut invalid = bytes.clone();
            invalid[8..12].copy_from_slice(&length.to_le_bytes());
            assert!(decode(&invalid[..HEADER_LEN]).is_err());
        }
        for payload_len in [0, 4, (MAX_PAYLOAD + 1) as u32, u32::MAX] {
            let mut invalid = bytes.clone();
            invalid[HEADER_LEN + 20..HEADER_LEN + 24].copy_from_slice(&payload_len.to_le_bytes());
            assert!(decode(&invalid).is_err());
        }
        let mut unknown_failure = raw(0);
        unknown_failure.kind = Kind::Failed(ReceiverFailure::ServiceDied);
        let mut encoded = unknown_failure.encode().unwrap();
        encoded[HEADER_LEN] = 255;
        assert!(decode(&encoded).is_err());
    }
    #[test]
    fn payload_bound_preserves_a_single_atomic_pipe_write_size() {
        let bytes = raw(MAX_PAYLOAD).encode().unwrap();
        assert_eq!(bytes.len(), MAX_FRAME_LEN);
        assert!(bytes.len() <= 4096);
        assert_eq!(decode_exact(&bytes).unwrap(), raw(MAX_PAYLOAD));
        assert!(raw(MAX_PAYLOAD + 1).encode().is_err());
        let mut wrong_attempt = raw(0);
        wrong_attempt.attempt = 2;
        assert!(wrong_attempt.encode().is_err());
    }
    #[test]
    fn snapshot_conversion_refuses_segmented_or_oversized_raw_data() {
        use std::sync::Arc;
        let mut snapshot = Snapshot {
            session: None,
            revision: 7,
            phase: Phase::Active { client_id: 42 },
            response: Some(Arc::new(crate::response::RadioResponse {
                info: crate::RadioExternalResponseInfo {
                    slotId: 0,
                    serial: 7,
                    rilcMsgId: 137,
                    length: 1,
                    error: 0,
                },
                payload: vec![7],
                kind: ResponseKind::Segmented { parameters: [0, 1] },
            })),
            request: None,
        };
        let identity = raw(0).identity;
        assert_eq!(
            from_snapshot(identity, &snapshot).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        let response = Arc::get_mut(snapshot.response.as_mut().unwrap()).unwrap();
        response.kind = ResponseKind::Raw;
        response.payload = vec![7; MAX_PAYLOAD + 1];
        assert!(from_snapshot(identity, &snapshot).is_err());
    }
}
