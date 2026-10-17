use super::*;
const NOW: u64 = 10_000_000_000;
const ABSENT: &[u8] = br#"{"schema":1,"revision":8,"until_ns":"0","available":false,"capacity_percent":null,"status":null}
"#;
#[test]
fn unavailable_zero_deadline_is_an_accepted_transaction() {
    assert!(
        matches!(decode(ABSENT, NOW), Some(None)),
        "production unavailable snapshot must decode as valid unavailable"
    );
}
#[test]
fn unavailable_nonzero_deadline_is_rejected() {
    let text = std::str::from_utf8(ABSENT)
        .unwrap()
        .replace("\"until_ns\":\"0\"", "\"until_ns\":\"10000000001\"");
    assert!(
        decode(text.as_bytes(), NOW).is_none(),
        "unavailable state requires the zero deadline sentinel"
    );
}
#[test]
fn nullable_fields_are_required_even_for_unavailable_state() {
    assert!(serde_json::from_slice::<Wire>(ABSENT).is_ok());
    let text = std::str::from_utf8(ABSENT).unwrap();
    for missing in [
        text.replace(",\"capacity_percent\":null", ""),
        text.replace(",\"status\":null", ""),
    ] {
        assert!(
            serde_json::from_str::<Wire>(&missing).is_err(),
            "missing nullable field was accepted: {missing}"
        );
    }
    for inconsistent in [
        text.replace("\"capacity_percent\":null", "\"capacity_percent\":0"),
        text.replace("\"status\":null", "\"status\":\"unknown\""),
    ] {
        assert!(decode(inconsistent.as_bytes(), NOW).is_none());
    }
}
