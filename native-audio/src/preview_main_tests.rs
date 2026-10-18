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
