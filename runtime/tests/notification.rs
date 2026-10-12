use omarchy_phone_core::{
    notification::{Completion, Notifications},
    observer::{Lease, Observer},
    watch::decode_snapshot,
};
use std::time::Duration;
fn lease(start: u64) -> Lease {
    let mut observer = Observer::default();
    for offset in [0, 1] {
        let pending = observer.begin(Duration::from_secs(start + offset));
        let body = serde_json::to_vec(&serde_json::json!({
            "v":1,"id":pending.query().id(),"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "revision":1,"source":"android.telecom.InCallService","source_state":"observed",
            "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]
        }))
        .unwrap();
        let snapshot = decode_snapshot(&body, pending.query().id()).unwrap();
        observer
            .complete(pending, snapshot, Duration::from_secs(start + offset))
            .unwrap();
    }
    observer
        .lease(Duration::from_secs(start + 1))
        .unwrap()
        .clone()
}
#[test]
fn notify_is_serialized_and_timeout_is_bounded_by_remaining_lease() {
    let lease = lease(10);
    let mut state = Notifications::default();
    let request = state
        .begin(":1.7", &lease, true, Duration::from_millis(11501))
        .unwrap();
    assert_eq!(request.timeout_at(Duration::from_millis(11501)), Some(4499));
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_secs(12))
            .is_none()
    );
    assert!(matches!(
        state.complete(
            request,
            7,
            Some(":1.7"),
            Some(&lease),
            true,
            Duration::from_secs(12)
        ),
        Completion::Visible(_)
    ));
}
#[test]
fn late_reply_is_closed_on_original_owner_even_when_ids_are_reused() {
    let lease = lease(10);
    let mut state = Notifications::default();
    let request = state
        .begin(":1.7", &lease, true, Duration::from_secs(11))
        .unwrap();
    let Completion::Close(handle) = state.complete(
        request,
        7,
        Some(":1.8"),
        Some(&lease),
        true,
        Duration::from_secs(12),
    ) else {
        panic!("late owner was accepted")
    };
    assert_eq!(handle.owner(), ":1.7");
    assert_eq!(handle.id(), 7);
    assert_eq!(
        handle.presented().ringing()[0].id(),
        lease.snapshot().ringing()[0].id()
    );
    assert_eq!(
        handle.presented().ringing()[0].episode(),
        lease.snapshot().ringing()[0].episode()
    );
    assert_eq!(handle.presented().cursor(), lease.snapshot().cursor());
    assert!(
        state
            .begin(":1.8", &lease, true, Duration::from_secs(12))
            .is_some()
    );
}
#[test]
fn lost_call_expiry_lock_or_new_lease_reject_delayed_reply() {
    for case in 0..4 {
        let lease = lease(10);
        let next = super_lease();
        let mut state = Notifications::default();
        let request = state
            .begin(":1.7", &lease, true, Duration::from_secs(11))
            .unwrap();
        let current = match case {
            0 => None,
            3 => Some(&next),
            _ => Some(&lease),
        };
        let now = Duration::from_secs(if case == 1 { 16 } else { 12 });
        assert!(matches!(
            state.complete(request, 7, Some(":1.7"), current, case != 2, now),
            Completion::Close(_)
        ));
    }
}
fn super_lease() -> Lease {
    lease(11)
}
#[test]
fn invalid_owner_locked_or_expired_state_never_creates_a_request() {
    let lease = lease(10);
    let mut state = Notifications::default();
    for owner in ["", "org.freedesktop.Notifications", ":", ":bad name"] {
        assert!(
            state
                .begin(owner, &lease, true, Duration::from_secs(11))
                .is_none()
        );
    }
    assert!(
        state
            .begin(":1.7", &lease, false, Duration::from_secs(11))
            .is_none()
    );
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_secs(16))
            .is_none()
    );
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_micros(15999999))
            .is_none()
    );
    let request = state
        .begin(":1.7", &lease, true, Duration::from_secs(11))
        .unwrap();
    state.not_sent(request);
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_secs(12))
            .is_some()
    );
}

fn sample(observer: &mut Observer, t: u64) {
    let pending = observer.begin(Duration::from_secs(t));
    let body = serde_json::to_vec(&serde_json::json!({
        "v":1,"id":pending.query().id(),"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "revision":1,"source":"android.telecom.InCallService","source_state":"observed",
        "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]
    }))
    .unwrap();
    let snapshot = decode_snapshot(&body, pending.query().id()).unwrap();
    observer
        .complete(pending, snapshot, Duration::from_secs(t))
        .unwrap();
}
#[test]
fn uninterrupted_renewal_does_not_reject_pending_notification() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    sample(&mut observer, 11);
    let first = observer.lease(Duration::from_secs(11)).unwrap().clone();
    let mut state = Notifications::default();
    let request = state
        .begin(":1.7", &first, true, Duration::from_secs(11))
        .unwrap();
    sample(&mut observer, 12);
    let renewed = observer.lease(Duration::from_secs(12)).unwrap();
    assert_ne!(first.id(), renewed.id());
    assert!(matches!(
        state.complete(
            request,
            7,
            Some(":1.7"),
            Some(renewed),
            true,
            Duration::from_secs(12)
        ),
        Completion::Visible(_)
    ));
}
#[test]
fn loss_and_reconfirmation_do_not_restore_old_presentation_authority() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    sample(&mut observer, 11);
    let first = observer.lease(Duration::from_secs(11)).unwrap().clone();
    let mut state = Notifications::default();
    let request = state
        .begin(":1.7", &first, true, Duration::from_secs(11))
        .unwrap();
    observer.invalidate();
    sample(&mut observer, 12);
    sample(&mut observer, 13);
    let renewed = observer.lease(Duration::from_secs(13)).unwrap();
    assert!(matches!(
        state.complete(
            request,
            7,
            Some(":1.7"),
            Some(renewed),
            true,
            Duration::from_secs(13)
        ),
        Completion::Close(_)
    ));
}

#[test]
fn revocation_is_not_lost_when_permission_or_owner_returns() {
    let lease = lease(10);
    let mut state = Notifications::default();
    let request = state
        .begin(":1.7", &lease, true, Duration::from_secs(11))
        .unwrap();
    state.invalidate();
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_secs(12))
            .is_none()
    );
    assert!(matches!(
        state.complete(
            request,
            7,
            Some(":1.7"),
            Some(&lease),
            true,
            Duration::from_secs(12)
        ),
        Completion::Close(_)
    ));
    assert!(
        state
            .begin(":1.7", &lease, true, Duration::from_secs(12))
            .is_some()
    );
}
