use pokelinux_radio::indication::{IndicationKind, RadioIndication, decode_indication_arguments};
use rsbinder::{Deserialize, Parcel};
struct Raw(RadioIndication);
struct Segment(RadioIndication);
impl Deserialize for Raw {
    fn deserialize(p: &mut Parcel) -> rsbinder::Result<Self> {
        decode_indication_arguments(1, p).map(Self)
    }
}
impl Deserialize for Segment {
    fn deserialize(p: &mut Parcel) -> rsbinder::Result<Self> {
        decode_indication_arguments(2, p).map(Self)
    }
}
fn arguments(segmented: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [33_i32, 0, 3] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[1, 2, 3, 0]);
    if segmented {
        for value in [1_i32, 6] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}
#[test]
fn raw_and_segmented_indications_keep_routing_and_segment_fields() {
    let raw = rsbinder::from_bytes::<Raw>(&arguments(false)).unwrap().0;
    assert_eq!(raw.message_id, 33);
    assert_eq!(raw.slot_id, 0);
    assert_eq!(raw.payload, [1, 2, 3]);
    assert_eq!(raw.kind, IndicationKind::Raw);
    let part = rsbinder::from_bytes::<Segment>(&arguments(true)).unwrap().0;
    assert_eq!(part.message_id, 33);
    assert_eq!(part.slot_id, 0);
    assert_eq!(part.payload, [1, 2, 3]);
    assert_eq!(
        part.kind,
        IndicationKind::Segment {
            index: 1,
            total_length: 6
        }
    );
}
#[test]
fn malformed_indications_produce_no_event() {
    for segmented in [false, true] {
        let bytes = arguments(segmented);
        for end in 0..bytes.len() {
            let accepted = if segmented {
                rsbinder::from_bytes::<Segment>(&bytes[..end]).is_ok()
            } else {
                rsbinder::from_bytes::<Raw>(&bytes[..end]).is_ok()
            };
            assert!(!accepted, "truncated at {end}");
        }
        let mut extra = bytes.clone();
        extra.extend_from_slice(&0_i32.to_le_bytes());
        assert!(if segmented {
            rsbinder::from_bytes::<Segment>(&extra).is_err()
        } else {
            rsbinder::from_bytes::<Raw>(&extra).is_err()
        });
    }
    for length in [-1_i32, i32::MAX] {
        let mut bytes = arguments(false);
        bytes[8..12].copy_from_slice(&length.to_le_bytes());
        assert!(rsbinder::from_bytes::<Raw>(&bytes).is_err());
    }
    let mut empty = Parcel::new();
    assert!(decode_indication_arguments(99, &mut empty).is_err());
    assert_eq!(empty.data_position(), 0);
}
