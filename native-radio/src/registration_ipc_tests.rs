//! Exact bounded current registration transport representation.
use super::*;
use crate::owner::{CommittedObservation, Observation};
use crate::registration::RegistrationWire;
use std::sync::Arc;

fn identity() -> ProcessIdentity {
    ProcessIdentity([0x12; 16])
}
fn wire(uri: &[u8], reason: &[u8]) -> RegistrationWire {
    RegistrationWire::new(omarchy_ims::registration::RegistrationWire {
        registration_state_wire: 255,
        feature_wire: 128,
        feature_mask_wire: i32::MIN,
        status_code_wire: u16::MAX,
        uri_bytes: uri,
        reason_bytes: reason,
        feature_mask_ext_wire: Some(i32::MIN),
        sip_delegate_mask_wire: Some(i64::MIN),
        feature_ext_wire: Some(u16::MAX),
    })
    .unwrap()
}
fn snapshot(wire: RegistrationWire) -> Snapshot {
    Snapshot {
        session: None,
        revision: 11,
        phase: Phase::Active { client_id: 42 },
        observation: Some(Arc::new(CommittedObservation {
            revision: 9,
            kind: Observation::RegistrationWire {
                slot_id: 1,
                message_id: 3012,
                wire,
            },
        })),
        request: None,
    }
}
fn expected() -> Vec<u8> {
    let mut bytes = b"RADO".to_vec();
    bytes.extend([1, 4, 0, 0]);
    bytes.extend(88u32.to_le_bytes());
    bytes.extend([0x12; 16]);
    bytes.extend(1u64.to_le_bytes());
    bytes.extend(9u64.to_le_bytes());
    bytes.extend(1i32.to_le_bytes());
    bytes.extend(3012i32.to_le_bytes());
    bytes.extend([255, 128]);
    bytes.extend(u16::MAX.to_le_bytes());
    bytes.extend(i32::MIN.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend([7, 0, 0, 0]);
    bytes.extend(i32::MIN.to_le_bytes());
    bytes.extend(i64::MIN.to_le_bytes());
    bytes.extend(u16::MAX.to_le_bytes());
    bytes.extend([0, 0]);
    bytes.extend([0, 255, 192, 0]);
    bytes
}
#[test]
fn registration_frame_is_exact_and_keeps_its_commit_revision() {
    let snapshot = snapshot(wire(&[0, 255], &[192, 0]));
    let frame = from_snapshot(identity(), &snapshot).unwrap().unwrap();
    assert_eq!(frame.revision, 9);
    assert_eq!(frame.encode().unwrap(), expected());
    assert_eq!(
        decode_exact(&expected()).unwrap().encode().unwrap(),
        expected()
    );
}
#[test]
fn terminal_lifecycle_wins_over_retained_registration_history() {
    let mut snapshot = snapshot(wire(&[], &[]));
    snapshot.phase = Phase::Failed(ReceiverFailure::ServiceDied);
    let frame = from_snapshot(identity(), &snapshot).unwrap().unwrap();
    assert_eq!(frame.revision, 11);
    assert_eq!(frame.kind, Kind::Failed(ReceiverFailure::ServiceDied));
}
#[test]
fn registration_frame_maximum_stays_below_existing_pipe_and_status_bounds() {
    let snapshot = snapshot(wire(&vec![0xff; 1000], &vec![0; 256]));
    let bytes = from_snapshot(identity(), &snapshot)
        .unwrap()
        .unwrap()
        .encode()
        .unwrap();
    assert_eq!(bytes.len(), 1340);
    assert!(bytes.len() <= MAX_FRAME_LEN && bytes.len() <= 4096);
    assert_eq!(decode_exact(&bytes).unwrap().encode().unwrap(), bytes);
}
#[test]
fn registration_frame_rejects_noncanonical_fields_and_lengths() {
    let bytes = expected();
    for offset in [
        HEADER_LEN + 21,
        HEADER_LEN + 22,
        HEADER_LEN + 23,
        HEADER_LEN + 38,
        HEADER_LEN + 39,
    ] {
        let mut bad = bytes.clone();
        bad[offset] = 1;
        assert!(decode_exact(&bad).is_err());
    }
    for flags in [2, 6, 8, 255] {
        let mut bad = bytes.clone();
        bad[HEADER_LEN + 20] = flags;
        assert!(decode_exact(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad[HEADER_LEN + 20] = 0;
    assert!(decode_exact(&bad).is_err());
    for (offset, length) in [(HEADER_LEN + 16, 1001u16), (HEADER_LEN + 18, 257u16)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 2].copy_from_slice(&length.to_le_bytes());
        assert!(decode_exact(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad[HEADER_LEN + 4..HEADER_LEN + 8].copy_from_slice(&3013i32.to_le_bytes());
    assert!(decode_exact(&bad).is_err());
    let mut bad = bytes;
    bad[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(&(-1i32).to_le_bytes());
    assert!(decode_exact(&bad).is_err());
}
