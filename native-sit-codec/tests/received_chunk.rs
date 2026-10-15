use pokelinux_sit_codec::received_chunk::{CopyError, ReceivedChunk};

// Public fixtures render qualified private captures as reviewable hex text.
// Bound text and decoded bytes before accepting whitespace-separated byte pairs.
fn fixture_bytes(text: &str, provided_count: usize) -> Vec<u8> {
    assert!((1..=65_568).contains(&provided_count));
    assert!(
        text.len() <= provided_count * 4,
        "FIXTURE_HEX_TEXT_EXCEEDS_BOUND"
    );
    let mut bytes = Vec::with_capacity(provided_count);
    for pair in text.split_ascii_whitespace() {
        assert_eq!(pair.len(), 2, "FIXTURE_HEX_REQUIRES_BYTE_PAIRS");
        assert!(pair.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(
            bytes.len() < provided_count,
            "FIXTURE_HEX_EXCEEDS_BYTE_COUNT"
        );
        bytes.push(u8::from_str_radix(pair, 16).expect("FIXTURE_HEX_INVALID_BYTE"));
    }
    assert_eq!(
        bytes.len(),
        provided_count,
        "FIXTURE_HEX_INCOMPLETE_BYTE_COUNT"
    );
    bytes
}

// Each expected value is the full actual original-owned capture, independently
// converted to hex; no protocol encoder or decoder supplies expected bytes.
fn compare(input_text: &str, original_owned_text: &str, provided_count: usize) {
    let input = fixture_bytes(input_text, provided_count);
    let original_owned = fixture_bytes(original_owned_text, provided_count);
    assert_eq!(
        original_owned, input,
        "ORIGINAL_COPY_CAPTURE_MUST_PRESERVE_ALL_BYTES"
    );
    let chunk = ReceivedChunk::copy_from(&input).expect("MISSING_OWNED_RECEIVED_CHUNK_COPY");
    assert_eq!(chunk.as_bytes(), original_owned.as_slice());
}

#[test]
fn copies_complete_base_v4_response() {
    compare(
        include_str!("received-chunk-fixtures/base-v4-144.input.hex"),
        include_str!("received-chunk-fixtures/base-v4-144.owned-after-overwrite.hex"),
        144,
    );
}

#[test]
fn copies_complete_request_without_response_validation() {
    compare(
        include_str!("received-chunk-fixtures/request-983.input.hex"),
        include_str!("received-chunk-fixtures/request-983.owned-after-overwrite.hex"),
        983,
    );
}

#[test]
fn copies_provided_count_despite_inner_declared_length() {
    compare(
        include_str!("received-chunk-fixtures/declared-length-mismatch-144.input.hex"),
        include_str!(
            "received-chunk-fixtures/declared-length-mismatch-144.owned-after-overwrite.hex"
        ),
        144,
    );
}

#[test]
fn copies_only_the_sixteen_provided_bytes() {
    compare(
        include_str!("received-chunk-fixtures/provided-count-16-from-v4-144.input.hex"),
        include_str!(
            "received-chunk-fixtures/provided-count-16-from-v4-144.owned-after-overwrite.hex"
        ),
        16,
    );
}

#[test]
fn copies_both_coalesced_records_as_one_raw_chunk() {
    compare(
        include_str!("received-chunk-fixtures/coalesced-v4-288.input.hex"),
        include_str!("received-chunk-fixtures/coalesced-v4-288.owned-after-overwrite.hex"),
        288,
    );
}

#[test]
fn copies_opaque_non_rcm_bytes() {
    compare(
        include_str!("received-chunk-fixtures/opaque-32.input.hex"),
        include_str!("received-chunk-fixtures/opaque-32.owned-after-overwrite.hex"),
        32,
    );
}

#[test]
fn copies_every_byte_at_the_native_policy_bound() {
    compare(
        include_str!("received-chunk-fixtures/max-pattern-65568.input.hex"),
        include_str!("received-chunk-fixtures/max-pattern-65568.owned-after-overwrite.hex"),
        65_568,
    );
}

#[test]
fn owned_bytes_survive_complete_source_overwrite_and_drop() {
    let original_owned = fixture_bytes(
        include_str!("received-chunk-fixtures/base-v4-144.owned-after-overwrite.hex"),
        144,
    );
    let mut source = fixture_bytes(
        include_str!("received-chunk-fixtures/base-v4-144.input.hex"),
        144,
    );
    let chunk = ReceivedChunk::copy_from(&source).expect("MISSING_OWNED_RECEIVED_CHUNK_COPY");
    for byte in &mut source {
        *byte ^= 0x5a;
    }
    assert_ne!(source.as_slice(), original_owned.as_slice());
    assert_eq!(chunk.as_bytes(), original_owned.as_slice());
    drop(source);
    assert_eq!(chunk.as_bytes(), original_owned.as_slice());
}

#[test]
fn native_policy_refuses_empty_input() {
    assert!(matches!(
        ReceivedChunk::copy_from(&[]),
        Err(CopyError::Empty)
    ));
}

#[test]
fn native_policy_refuses_one_byte_above_the_selected_bound() {
    let source = [0xa5; 65_569];
    assert!(matches!(
        ReceivedChunk::copy_from(&source),
        Err(CopyError::TooLarge)
    ));
}
