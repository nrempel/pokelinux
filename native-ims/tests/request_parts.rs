use omarchy_ims::requests::{RequestPart, RequestParts};

#[test]
fn raw_requests_include_empty_payload_and_threshold() {
    for size in [0, 1, 2047, 2048] {
        let bytes = vec![0x81; size];
        let mut parts = RequestParts::new(&bytes).unwrap();
        assert_eq!(parts.next(), Some(RequestPart::Raw(&bytes)));
        assert_eq!(parts.next(), None);
        assert_eq!(parts.next(), None);
    }
}

#[test]
fn original_non_multiple_boundaries_match_and_exact_multiples_preserve_last_chunk() {
    for (size, expected) in [
        (2049, vec![2048, 1]),
        (4095, vec![2048, 2047]),
        (4096, vec![2048, 2048]),
        (4097, vec![2048, 2048, 1]),
        (6144, vec![2048, 2048, 2048]),
    ] {
        let payload = vec![0xff; size];
        let lengths: Vec<_> = RequestParts::new(&payload)
            .unwrap()
            .map(|part| match part {
                RequestPart::Segment { payload, .. } => payload.len(),
                _ => panic!("large request must be segmented"),
            })
            .collect();
        assert_eq!(lengths, expected);
    }
}

#[test]
fn every_byte_and_metadata_survive_all_small_boundary_sizes() {
    for size in 2049..=16385 {
        let bytes: Vec<u8> = (0..size)
            .map(|i| ((i * 31 + i / 2048) & 255) as u8)
            .collect();
        let mut restored = Vec::new();
        let mut parts = RequestParts::new(&bytes).unwrap();
        for (index, part) in parts.by_ref().enumerate() {
            let RequestPart::Segment {
                index: actual,
                total_length,
                payload,
            } = part
            else {
                panic!("wrong request route");
            };
            assert_eq!(actual, index as i32);
            assert_eq!(total_length, size);
            assert!(!payload.is_empty() && payload.len() <= 2048);
            // Fragments borrow the input; no payload copy or padding.
            assert_eq!(payload.as_ptr(), bytes[restored.len()..].as_ptr());
            restored.extend_from_slice(payload);
        }
        assert_eq!(restored, bytes);
        assert_eq!(parts.next(), None);
    }
}
