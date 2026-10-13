use omarchy_ims::reader::{MAX_INPUT_BYTES, ReadError};
use omarchy_ims::registration::{DecodeError, decode, decode_with_length};
fn bytes(v: &str) -> Vec<u8> {
    if v == "-" {
        return vec![];
    }
    assert!(v.len().is_multiple_of(2));
    v.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn optional<T: std::str::FromStr>(v: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    if v == "-" {
        None
    } else {
        Some(v.parse().unwrap())
    }
}
fn packet(uri: u16, reason: u16, body: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut p = vec![255, 128];
    p.extend(i32::MIN.to_le_bytes());
    p.extend(u16::MAX.to_le_bytes());
    p.extend(uri.to_le_bytes());
    p.extend(reason.to_le_bytes());
    p.extend(body);
    p.extend(tail);
    p
}
#[test]
fn complete_original_synthetic_registration_fields_match() {
    let mut matches = 0;
    let mut refusals = 0;
    for line in include_str!("fixtures/shannon-registration.tsv").lines() {
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 17);
        let input = bytes(f[1]);
        let result = decode_with_length(&input, f[2].parse().unwrap());
        if f[4] == "match" {
            let v = result.unwrap_or_else(|e| panic!("{}: {e:?}", f[0]));
            assert_eq!(
                (
                    v.registration_state_wire,
                    v.feature_wire,
                    v.feature_mask_wire,
                    v.status_code_wire
                ),
                (
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                    f[7].parse().unwrap(),
                    f[8].parse().unwrap()
                ),
                "{}",
                f[0]
            );
            assert_eq!(
                (v.uri_bytes.len(), v.reason_bytes.len()),
                (f[9].parse().unwrap(), f[10].parse().unwrap()),
                "{}",
                f[0]
            );
            assert_eq!(
                (
                    v.feature_mask_ext_wire,
                    v.sip_delegate_mask_wire,
                    v.feature_ext_wire
                ),
                (optional(f[11]), optional(f[12]), optional(f[13])),
                "{}",
                f[0]
            );
            assert_eq!(
                (v.uri_bytes, v.reason_bytes),
                (bytes(f[14]).as_slice(), bytes(f[15]).as_slice()),
                "{}",
                f[0]
            );
            let combined = [v.uri_bytes, v.reason_bytes].concat();
            if !combined.is_empty() && std::str::from_utf8(&combined).is_ok() {
                assert_eq!(
                    combined,
                    bytes(f[16]),
                    "{} original text changed valid bytes",
                    f[0]
                );
            }
            matches += 1;
        } else {
            let e = match f[4] {
                "InputTooLarge" => DecodeError::Input(ReadError::InputTooLarge),
                "InvalidRange" => DecodeError::Input(ReadError::InvalidRange),
                "Truncated" => DecodeError::Input(ReadError::Truncated),
                "TrailingData" => DecodeError::Input(ReadError::TrailingData),
                "UriTooLong" => DecodeError::UriTooLong,
                "ReasonTooLong" => DecodeError::ReasonTooLong,
                _ => panic!("unknown authored policy"),
            };
            assert_eq!(result, Err(e), "{}", f[0]);
            refusals += 1;
        }
    }
    assert_eq!((matches, refusals), (37, 84));
}
#[test]
fn minimal_complete_packet_has_no_old_optional_fields_or_sentinel_text() {
    let p = packet(0, 0, &[], &[]);
    let v = decode(&p).unwrap();
    assert_eq!(
        (
            v.feature_mask_ext_wire,
            v.sip_delegate_mask_wire,
            v.feature_ext_wire
        ),
        (None, None, None)
    );
    assert!(v.uri_bytes.is_empty() && v.reason_bytes.is_empty());
    assert_eq!(
        (
            v.registration_state_wire,
            v.feature_wire,
            v.feature_mask_wire,
            v.status_code_wire
        ),
        (255, 128, i32::MIN, u16::MAX)
    );
}
#[test]
fn independently_present_tail_fields_use_the_observed_greedy_layout() {
    let tail: Vec<u8> = (1..=14).collect();
    for (n, mask, delegate, feature) in [
        (0, None, None, None),
        (2, None, None, Some(513)),
        (4, Some(67305985), None, None),
        (6, Some(67305985), None, Some(1541)),
        (12, Some(67305985), Some(867798387104613893), None),
        (14, Some(67305985), Some(867798387104613893), Some(3597)),
    ] {
        let p = packet(0, 0, &[], &tail[..n]);
        let v = decode(&p).unwrap();
        assert_eq!(
            (
                v.feature_mask_ext_wire,
                v.sip_delegate_mask_wire,
                v.feature_ext_wire
            ),
            (mask, delegate, feature)
        );
    }
}
#[test]
fn opaque_body_retains_nul_and_non_utf8_and_original_byte_boundaries() {
    let p = packet(3, 2, &[65, 0, 66, 255, 192], &[]);
    let v = decode(&p).unwrap();
    assert_eq!(v.uri_bytes, &[65, 0, 66]);
    assert_eq!(v.reason_bytes, &[255, 192]);
    assert_eq!(v.uri_bytes.as_ptr(), p[12..].as_ptr());
    assert_eq!(v.reason_bytes.as_ptr(), p[15..].as_ptr());
}
#[test]
fn overlength_wire_fields_are_refused_without_original_clamping() {
    assert_eq!(
        decode(&packet(1001, 0, &vec![0; 1000], &[])),
        Err(DecodeError::UriTooLong)
    );
    assert_eq!(
        decode(&packet(0, 257, &vec![0; 256], &[])),
        Err(DecodeError::ReasonTooLong)
    );
}
#[test]
fn truncated_header_and_body_never_return_partial_fields() {
    let p = packet(4, 3, b"sip\0ok\0", &[]);
    for n in 0..p.len() {
        assert_eq!(
            decode_with_length(&p, n as i32),
            Err(DecodeError::Input(ReadError::Truncated)),
            "prefix {n}"
        );
    }
    assert_eq!(decode(&p).unwrap().reason_bytes, b"ok\0");
}
#[test]
fn unconsumed_tail_is_refused_even_when_original_updates_fields() {
    for n in [1, 3, 5, 7, 8, 9, 10, 11, 13, 15, 16, 17] {
        assert_eq!(
            decode(&packet(0, 0, &[], &vec![0; n])),
            Err(DecodeError::Input(ReadError::TrailingData)),
            "tail {n}"
        );
    }
}
#[test]
fn invalid_ranges_and_oversized_inputs_cannot_be_clamped_into_success() {
    let p = packet(0, 0, &[], &[]);
    for n in [-1, p.len() as i32 + 1, i32::MAX] {
        assert_eq!(
            decode_with_length(&p, n),
            Err(DecodeError::Input(ReadError::InvalidRange))
        );
    }
    assert_eq!(
        decode(&vec![0; MAX_INPUT_BYTES + 1]),
        Err(DecodeError::Input(ReadError::InputTooLarge))
    );
}
