use pokelinux_power::{
    Application, AppliedState, ApplyResult, Arbiter, Error, Expiry, MAX_LEASES, MAX_OWNER_LEASES,
    OwnerId, ResourcePolicy, RetryState,
    clock::{BootTime, Clock},
};
mod support;
use std::{fs, io, time::Duration};
use support::TemporaryNode;
const A: OwnerId = OwnerId(1);
const B: OwnerId = OwnerId(2);
fn at(ms: u64) -> BootTime {
    BootTime::from_elapsed(Duration::from_millis(ms))
}
fn policies() -> Vec<ResourcePolicy> {
    vec![ResourcePolicy {
        key: "performance".into(),
        values: vec!["high".into(), "low".into(), "idle".into()],
        default_index: 2,
    }]
}
fn arbiter() -> Arbiter {
    Arbiter::new(policies(), Duration::from_millis(500)).unwrap()
}
fn desired(a: &mut Arbiter, ms: u64) -> String {
    a.snapshot("performance", at(ms)).unwrap().desired
}
fn apply(a: &mut Arbiter, value: &str, ms: u64) -> Application {
    let plan = a
        .begin_apply("performance", at(ms))
        .unwrap()
        .expect("application required");
    assert_eq!(plan.value(), value);
    a.finish_apply(&plan, ApplyResult::Applied, at(ms)).unwrap();
    plan
}
#[test]
fn owner_requests_preempt_restore_and_cancel_independently() {
    let mut a = arbiter();
    assert_eq!(desired(&mut a, 0), "idle");
    let low = a
        .acquire(A, "performance", "low", Expiry::UntilCancelled, at(0))
        .unwrap();
    assert_eq!(desired(&mut a, 0), "low");
    let high = a
        .acquire(B, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
    assert_eq!(desired(&mut a, 0), "high");
    assert_eq!(a.cancel(A, &high, at(0)), Err(Error::WrongOwner));
    assert_eq!(desired(&mut a, 0), "high");
    a.cancel(B, &high, at(0)).unwrap();
    assert_eq!(desired(&mut a, 0), "low");
    a.cancel(A, &low, at(0)).unwrap();
    assert_eq!(desired(&mut a, 0), "idle");
    assert_eq!(a.cancel(A, &low, at(0)), Err(Error::UnknownLease));
}
#[test]
fn renewal_never_shortens_and_indefinite_renewal_needs_explicit_cancel() {
    let mut a = arbiter();
    let lease = a
        .acquire(A, "performance", "high", Expiry::At(at(10_000)), at(0))
        .unwrap();
    assert!(a.renew(A, &lease, Expiry::At(at(20_000)), at(0)).unwrap());
    assert!(!a.renew(A, &lease, Expiry::At(at(5_000)), at(0)).unwrap());
    assert!(!a.renew(A, &lease, Expiry::At(at(20_000)), at(0)).unwrap());
    assert_eq!(desired(&mut a, 19_999), "high");
    assert!(
        a.renew(A, &lease, Expiry::UntilCancelled, at(19_999))
            .unwrap()
    );
    assert!(
        !a.renew(A, &lease, Expiry::At(at(30_000)), at(19_999))
            .unwrap()
    );
    assert_eq!(desired(&mut a, 40_000), "high");
    a.cancel(A, &lease, at(40_000)).unwrap();
    assert_eq!(desired(&mut a, 40_000), "idle");
}
#[test]
fn expiry_is_exact_and_an_expired_handle_cannot_resurrect_work() {
    let mut a = arbiter();
    let lease = a
        .acquire(A, "performance", "high", Expiry::At(at(100)), at(0))
        .unwrap();
    assert_eq!(desired(&mut a, 99), "high");
    assert_eq!(desired(&mut a, 100), "idle");
    assert_eq!(
        a.renew(A, &lease, Expiry::At(at(200)), at(100)),
        Err(Error::UnknownLease)
    );
    assert_eq!(
        a.acquire(A, "performance", "high", Expiry::At(at(100)), at(100))
            .unwrap_err(),
        Error::ExpiryElapsed
    );
    assert_eq!(a.lease_count(), 0);
}
#[test]
fn suspend_time_expires_native_policy_without_waiting_for_awake_time() {
    struct InjectedClock {
        boot_ms: u64,
        awake_ms: u64,
    }
    impl Clock for InjectedClock {
        fn now(&self) -> BootTime {
            at(self.boot_ms)
        }
    }
    let mut clock = InjectedClock {
        boot_ms: 100,
        awake_ms: 100,
    };
    let mut a = arbiter();
    a.acquire(
        A,
        "performance",
        "high",
        Expiry::after(clock.now(), Duration::from_secs(5)).unwrap(),
        clock.now(),
    )
    .unwrap();
    assert_eq!(desired(&mut a, clock.boot_ms), "high");
    clock.boot_ms += 6_000; // Controlled suspend/resume clock fixture; no real suspend.
    assert_eq!(clock.awake_ms, 100);
    assert_eq!(desired(&mut a, clock.boot_ms), "idle");
}
#[test]
fn owner_disconnect_removes_all_its_resources_without_touching_another_owner() {
    let mut p = policies();
    p.push(ResourcePolicy {
        key: "latency".into(),
        values: vec!["active".into(), "default".into()],
        default_index: 1,
    });
    let mut a = Arbiter::new(p, Duration::from_millis(500)).unwrap();
    a.acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
    a.acquire(A, "latency", "active", Expiry::UntilCancelled, at(0))
        .unwrap();
    a.acquire(B, "performance", "low", Expiry::UntilCancelled, at(0))
        .unwrap();
    assert_eq!(a.cancel_owner(A, at(0)), Ok(2));
    assert_eq!(desired(&mut a, 0), "low");
    assert_eq!(a.snapshot("latency", at(0)).unwrap().desired, "default");
    assert_eq!(a.lease_count(), 1);
}
#[test]
fn completion_after_cancel_reports_the_old_value_then_reconciles_default() {
    let mut a = arbiter();
    let lease = a
        .acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
    let old = a.begin_apply("performance", at(0)).unwrap().unwrap();
    assert_eq!(old.value(), "high");
    assert!(a.begin_apply("performance", at(0)).unwrap().is_none());
    a.cancel(A, &lease, at(1)).unwrap();
    a.finish_apply(&old, ApplyResult::Applied, at(2)).unwrap();
    let s = a.snapshot("performance", at(2)).unwrap();
    assert_eq!(s.desired, "idle");
    assert_eq!(s.applied, AppliedState::Confirmed("high".into()));
    let reset = a.begin_apply("performance", at(2)).unwrap().unwrap();
    assert_eq!(reset.value(), "idle");
    assert_eq!(
        a.finish_apply(&old, ApplyResult::Applied, at(2)),
        Err(Error::StaleToken)
    );
    assert!(a.snapshot("performance", at(2)).unwrap().pending);
    a.finish_apply(&reset, ApplyResult::Applied, at(2)).unwrap();
    assert!(a.begin_apply("performance", at(2)).unwrap().is_none());
}
#[test]
fn backend_replacement_fences_old_completion_and_reapplies_current_intent() {
    let mut a = arbiter();
    a.acquire(A, "performance", "high", Expiry::At(at(100)), at(0))
        .unwrap();
    let old = a.begin_apply("performance", at(0)).unwrap().unwrap();
    a.backend_lost("performance", at(1)).unwrap();
    let next = a.begin_apply("performance", at(1)).unwrap().unwrap();
    assert_eq!(
        a.finish_apply(&old, ApplyResult::Applied, at(1)),
        Err(Error::StaleToken)
    );
    assert_eq!(
        a.snapshot("performance", at(1)).unwrap().applied,
        AppliedState::Unknown
    );
    a.finish_apply(&next, ApplyResult::Applied, at(1)).unwrap();
    a.backend_lost("performance", at(100)).unwrap();
    apply(&mut a, "idle", 100); // Expired boost is not replayed.
}
#[test]
fn handles_from_a_previous_component_instance_are_never_admitted() {
    let mut old = arbiter();
    let lease = old
        .acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
    let plan = old.begin_apply("performance", at(0)).unwrap().unwrap();
    let mut next = arbiter();
    next.acquire(A, "performance", "low", Expiry::UntilCancelled, at(0))
        .unwrap();
    let pending = next.begin_apply("performance", at(0)).unwrap().unwrap();
    assert_eq!(next.cancel(A, &lease, at(0)), Err(Error::StaleToken));
    assert_eq!(
        next.finish_apply(&plan, ApplyResult::Applied, at(0)),
        Err(Error::StaleToken)
    );
    next.finish_apply(&pending, ApplyResult::Applied, at(0))
        .unwrap();
    assert_eq!(desired(&mut next, 0), "low");
}
#[test]
fn invalid_profiles_values_and_clock_regression_do_not_change_owned_requests() {
    for p in [
        vec![],
        vec![ResourcePolicy {
            key: "/sys/power/state".into(),
            values: vec!["mem".into()],
            default_index: 0,
        }],
        vec![ResourcePolicy {
            key: "performance".into(),
            values: vec!["high".into(), "high".into()],
            default_index: 0,
        }],
    ] {
        assert_eq!(
            Arbiter::new(p, Duration::from_millis(500)).err(),
            Some(Error::InvalidPolicy)
        );
    }
    let mut a = arbiter();
    let lease = a
        .acquire(A, "performance", "high", Expiry::At(at(200)), at(100))
        .unwrap();
    assert_eq!(
        a.acquire(
            A,
            "performance",
            "unavailable",
            Expiry::UntilCancelled,
            at(100)
        )
        .unwrap_err(),
        Error::UnknownValue
    );
    assert_eq!(a.cancel(A, &lease, at(99)), Err(Error::ClockRegressed));
    assert_eq!(desired(&mut a, 100), "high");
    assert_eq!(
        Expiry::after(
            BootTime::from_elapsed(Duration::MAX),
            Duration::from_nanos(1)
        ),
        Err(Error::DeadlineOverflow)
    );
}
#[test]
fn owner_and_global_capacity_are_bounded_and_reclaimed() {
    let mut a = arbiter();
    for _ in 0..MAX_OWNER_LEASES {
        a.acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
            .unwrap();
    }
    assert_eq!(
        a.acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
            .unwrap_err(),
        Error::OwnerCapacity
    );
    for i in MAX_OWNER_LEASES..MAX_LEASES {
        a.acquire(
            OwnerId(10 + i as u64),
            "performance",
            "low",
            Expiry::UntilCancelled,
            at(0),
        )
        .unwrap();
    }
    assert_eq!(
        a.acquire(B, "performance", "high", Expiry::UntilCancelled, at(0))
            .unwrap_err(),
        Error::LeaseCapacity
    );
    assert_eq!(a.cancel_owner(A, at(0)).unwrap(), MAX_OWNER_LEASES);
    a.acquire(B, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
}
#[test]
fn failed_application_is_unknown_keeps_history_and_waits_before_retry() {
    let mut a = arbiter();
    apply(&mut a, "idle", 0);
    a.acquire(A, "performance", "high", Expiry::UntilCancelled, at(1))
        .unwrap();
    let p = a.begin_apply("performance", at(1)).unwrap().unwrap();
    a.finish_apply(&p, ApplyResult::Failed, at(1)).unwrap();
    let s = a.snapshot("performance", at(1)).unwrap();
    assert_eq!(s.desired, "high");
    assert_eq!(s.applied, AppliedState::Unknown);
    assert_eq!(s.last_confirmed, Some("idle".into()));
    assert!(s.failed);
    assert_eq!(s.retry, RetryState::After(at(501)));
    assert!(a.begin_apply("performance", at(500)).unwrap().is_none());
    apply(&mut a, "high", 501);
    assert!(!a.snapshot("performance", at(501)).unwrap().failed);
}
#[test]
fn an_overflowing_retry_deadline_cannot_wrap_or_create_a_busy_loop() {
    let mut a = arbiter();
    let now = BootTime::from_elapsed(Duration::MAX);
    let p = a.begin_apply("performance", now).unwrap().unwrap();
    a.finish_apply(&p, ApplyResult::Failed, now).unwrap();
    assert_eq!(
        a.snapshot("performance", now).unwrap().retry,
        RetryState::Exhausted
    );
    assert!(a.begin_apply("performance", now).unwrap().is_none());
}
#[test]
fn real_temporary_file_failure_retries_the_current_request_and_restores_default() {
    let node = TemporaryNode::new();
    node.write("stale");
    let mut a = arbiter();
    let p = a.begin_apply("performance", at(0)).unwrap().unwrap();
    node.execute(&p).unwrap();
    a.finish_apply(&p, ApplyResult::Applied, at(0)).unwrap();
    assert_eq!(node.read(), "idle");
    let lease = a
        .acquire(A, "performance", "high", Expiry::UntilCancelled, at(1))
        .unwrap();
    fs::remove_file(node.path()).unwrap();
    let p = a.begin_apply("performance", at(1)).unwrap().unwrap();
    assert_eq!(
        node.execute(&p).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    a.finish_apply(&p, ApplyResult::Failed, at(1)).unwrap();
    node.write("unrelated");
    assert!(a.begin_apply("performance", at(500)).unwrap().is_none());
    let p = a.begin_apply("performance", at(501)).unwrap().unwrap();
    assert_eq!(p.value(), "high");
    node.execute(&p).unwrap();
    a.finish_apply(&p, ApplyResult::Applied, at(501)).unwrap();
    assert_eq!(node.read(), "high");
    a.cancel(A, &lease, at(502)).unwrap();
    let p = a.begin_apply("performance", at(502)).unwrap().unwrap();
    node.execute(&p).unwrap();
    a.finish_apply(&p, ApplyResult::Applied, at(502)).unwrap();
    assert_eq!(node.read(), "idle");
}
#[test]
#[cfg(target_os = "linux")]
fn actual_linux_boot_clock_is_monotonic_and_can_schedule_a_native_lease() {
    let clock = pokelinux_power::clock::BootClock;
    let before = clock.now();
    let mut a = arbiter();
    a.acquire(
        A,
        "performance",
        "high",
        Expiry::after(before, Duration::from_secs(3600)).unwrap(),
        before,
    )
    .unwrap();
    let after = clock.now();
    assert!(after >= before);
    assert_eq!(a.snapshot("performance", after).unwrap().desired, "high");
}

#[test]
fn dispatch_marks_current_application_unknown_until_completion() {
    let mut a = arbiter();
    apply(&mut a, "idle", 0);
    a.acquire(A, "performance", "high", Expiry::UntilCancelled, at(1))
        .unwrap();
    let plan = a.begin_apply("performance", at(1)).unwrap().unwrap();
    let state = a.snapshot("performance", at(1)).unwrap();
    assert!(state.pending);
    assert_eq!(state.applied, AppliedState::Unknown);
    assert_eq!(state.last_confirmed, Some("idle".into()));
    a.finish_apply(&plan, ApplyResult::Applied, at(2)).unwrap();
    assert_eq!(
        a.snapshot("performance", at(2)).unwrap().applied,
        AppliedState::Confirmed("high".into())
    );
}
#[test]
fn configured_default_is_not_an_active_request_with_implicit_priority() {
    let mut a = Arbiter::new(
        vec![ResourcePolicy {
            key: "performance".into(),
            values: vec!["idle".into(), "high".into()],
            default_index: 0,
        }],
        Duration::from_millis(500),
    )
    .unwrap();
    let lease = a
        .acquire(A, "performance", "high", Expiry::UntilCancelled, at(0))
        .unwrap();
    assert_eq!(desired(&mut a, 0), "high");
    a.cancel(A, &lease, at(0)).unwrap();
    assert_eq!(desired(&mut a, 0), "idle");
}

#[test]
fn event_loop_deadline_tracks_renewal_expiry_retry_and_pending_application() {
    let mut a = arbiter();
    assert_eq!(a.next_deadline(), None);
    let lease = a
        .acquire(A, "performance", "high", Expiry::At(at(100)), at(0))
        .unwrap();
    assert_eq!(a.next_deadline(), Some(at(100)));
    a.renew(A, &lease, Expiry::At(at(200)), at(0)).unwrap();
    assert_eq!(a.next_deadline(), Some(at(200)));
    a.advance(at(200)).unwrap();
    assert_eq!(a.next_deadline(), None);
    let plan = a.begin_apply("performance", at(200)).unwrap().unwrap();
    a.finish_apply(&plan, ApplyResult::Failed, at(200)).unwrap();
    assert_eq!(a.next_deadline(), Some(at(700)));
    let plan = a.begin_apply("performance", at(700)).unwrap().unwrap();
    assert_eq!(a.next_deadline(), None);
    a.finish_apply(&plan, ApplyResult::Applied, at(700))
        .unwrap();
    assert_eq!(a.next_deadline(), None);
}
