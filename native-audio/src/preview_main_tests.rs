//! Controlled original admission/work deadlines, not process custody evidence.
use super::*;
#[test]
fn preview_uses_the_supplied_start_cutoff_and_one_frozen_work_budget() {
    let now = BootTime::from_elapsed(Duration::from_nanos(10_000_000_000));
    let (start, work) = preview_deadlines(now, 10_250_000_000).unwrap();
    assert_eq!(start.elapsed(), Duration::from_nanos(10_250_000_000));
    assert_eq!(work.elapsed(), Duration::from_secs(16));
    assert!(preview_deadlines(start, 10_250_000_000).is_err());
}
#[test]
fn expired_or_over_one_second_preview_never_gets_a_fresh_budget() {
    let now = BootTime::from_elapsed(Duration::from_secs(10));
    for until in [0, 9_999_999_999, 10_000_000_000, 11_000_000_001, u64::MAX] {
        assert!(preview_deadlines(now, until).is_err());
    }
    assert!(preview_deadlines(now, 11_000_000_000).is_ok());
}

#[test]
fn preview_level_parser_rejects_missing_malformed_and_out_of_range_fields() {
    for percent in [0, 50, 100] {
        for muted in [false, true] {
            let level = preview_level(&json!({"level_percent":percent,"muted":muted})).unwrap();
            assert_eq!(level.percent(), percent as u8);
            assert_eq!(level.muted(), muted);
        }
    }
    for bad in [
        json!({}),
        json!({"level_percent":50}),
        json!({"muted":false}),
        json!({"level_percent":101,"muted":false}),
        json!({"level_percent":u64::MAX,"muted":false}),
        json!({"level_percent":-1,"muted":false}),
        json!({"level_percent":50.5,"muted":false}),
        json!({"level_percent":"50","muted":false}),
        json!({"level_percent":null,"muted":false}),
        json!({"level_percent":50,"muted":0}),
        json!({"level_percent":50,"muted":"false"}),
        json!({"level_percent":50,"muted":null}),
    ] {
        assert_eq!(preview_level(&bad), Err("invalid_preview_level"), "{bad}");
    }
}
