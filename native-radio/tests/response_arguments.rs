use pokelinux_radio::response::{RadioResponse, ResponseKind, decode_response_arguments};
use rsbinder::{Deserialize, Parcel};

struct Raw(RadioResponse);
impl Deserialize for Raw {
    fn deserialize(parcel: &mut Parcel) -> rsbinder::Result<Self> {
        decode_response_arguments(1, parcel).map(Self)
    }
}
struct Segmented(RadioResponse);
impl Deserialize for Segmented {
    fn deserialize(parcel: &mut Parcel) -> rsbinder::Result<Self> {
        decode_response_arguments(2, parcel).map(Self)
    }
}
fn fixtures() -> Vec<(usize, u32, Vec<u8>)> {
    include_str!("fixtures/response-arguments.tsv")
        .lines()
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 4);
            assert_eq!(fields[0], "responseArgs");
            let (pairs, remainder) = fields[3].as_bytes().as_chunks::<2>();
            assert!(remainder.is_empty());
            let bytes = pairs
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            (
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                bytes,
            )
        })
        .collect()
}

#[test]
fn original_receiver_argument_fixtures_decode_to_explicit_events() {
    let rows = fixtures();
    assert_eq!(rows.len(), 4);
    for (case, code, bytes) in rows {
        let response = if code == 1 {
            rsbinder::from_bytes::<Raw>(&bytes).map(|r| r.0)
        } else {
            rsbinder::from_bytes::<Segmented>(&bytes).map(|r| r.0)
        };
        if case == 3 {
            assert!(response.is_err(), "original receiver rejects trailing data");
            continue;
        }
        let response = response.unwrap();
        assert_eq!(response.info.serial, if case == 1 { 99 } else { 42 });
        assert_eq!(response.info.slotId, 99);
        assert_eq!(response.info.rilcMsgId, 999);
        assert_eq!(response.info.length, 100);
        assert_eq!(response.info.error, 0);
        assert_eq!(response.payload, [1, 2, 3]);
        assert_eq!(
            response.kind,
            if code == 1 {
                ResponseKind::Raw
            } else {
                ResponseKind::Segmented { parameters: [0, 3] }
            }
        );
    }
}

#[test]
fn malformed_arguments_never_produce_an_event() {
    for (case, code, bytes) in fixtures() {
        if case == 3 {
            continue;
        }
        for end in 0..bytes.len() {
            let accepted = if code == 1 {
                rsbinder::from_bytes::<Raw>(&bytes[..end]).is_ok()
            } else {
                rsbinder::from_bytes::<Segmented>(&bytes[..end]).is_ok()
            };
            assert!(!accepted, "accepted truncated case {case} at {end}");
        }
    }
    let mut unknown = Parcel::new();
    assert!(decode_response_arguments(99, &mut unknown).is_err());
    assert_eq!(unknown.data_position(), 0);
    // A null typed metadata object cannot identify a pending request.
    assert!(rsbinder::from_bytes::<Raw>(&0_i32.to_le_bytes()).is_err());
}

#[test]
fn null_or_oversized_payload_lengths_are_rejected() {
    let (_, _, original) = fixtures().remove(0);
    for length in [-1_i32, i32::MAX] {
        let mut bytes = original.clone();
        // Original typed metadata occupies 28 bytes, followed by byte-array length.
        bytes[28..32].copy_from_slice(&length.to_le_bytes());
        assert!(rsbinder::from_bytes::<Raw>(&bytes).is_err());
    }
}
