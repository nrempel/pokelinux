use omarchy_phone_core::{
    Identity,
    observer::Observer,
    watch::{Snapshot, decode_snapshot},
};
use serde_json::json;
use std::time::Duration;
fn time(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}
fn id(c: char) -> Identity {
    c.to_string().repeat(32).try_into().unwrap()
}
fn snapshot(
    request: &Identity,
    generation: char,
    revision: u64,
    state: &str,
    ringing: bool,
) -> Snapshot {
    let calls = if ringing {
        json!([{"id":7,"episode":id('c')}])
    } else {
        json!([])
    };
    decode_snapshot(&serde_json::to_vec(&json!({"v":1,"id":request,"generation":id(generation),"revision":revision,"source":"android.telecom.InCallService","source_state":state,"ringing":calls})).unwrap(),request).unwrap()
}
#[test]
fn idle_ringing_requires_short_confirmation_and_lease_starts_before_request() {
    let mut observer = Observer::default();
    let idle = observer.begin(time(0));
    assert_eq!(idle.query().wait_ms(), 25000);
    let reply = snapshot(idle.query().id(), 'a', 1, "observed", true);
    assert!(observer.complete(idle, reply, time(20)).unwrap());
    assert!(observer.lease(time(20)).is_none());
    let confirm = observer.begin(time(20));
    assert_eq!(confirm.query().wait_ms(), 0);
    let reply = snapshot(confirm.query().id(), 'a', 1, "observed", true);
    observer.complete(confirm, reply, time(23)).unwrap();
    assert_eq!(observer.lease(time(23)).unwrap().expires(), time(25));
    assert_eq!(
        observer.lease(time(23)).unwrap().snapshot().ringing().len(),
        1
    );
    assert!(observer.lease(time(25)).is_none());
}
#[test]
fn late_confirmation_cannot_present_and_resume_discards_inflight_reply() {
    let mut observer = Observer::default();
    let idle = observer.begin(time(0));
    let reply = snapshot(idle.query().id(), 'a', 1, "observed", true);
    observer.complete(idle, reply, time(1)).unwrap();
    let confirm = observer.begin(time(1));
    let reply = snapshot(confirm.query().id(), 'a', 1, "observed", true);
    assert!(observer.complete(confirm, reply, time(6)).is_err());
    assert!(observer.lease(time(6)).is_none());
    let pending = observer.begin(time(7));
    let reply = snapshot(pending.query().id(), 'a', 2, "observed", true);
    observer.invalidate();
    assert!(!observer.complete(pending, reply, time(8)).unwrap());
    assert!(observer.lease(time(8)).is_none());
}
fn ringing(observer: &mut Observer) {
    let idle = observer.begin(time(0));
    let reply = snapshot(idle.query().id(), 'a', 1, "observed", true);
    observer.complete(idle, reply, time(1)).unwrap();
    let confirm = observer.begin(time(1));
    let reply = snapshot(confirm.query().id(), 'a', 1, "observed", true);
    observer.complete(confirm, reply, time(2)).unwrap();
}
#[test]
fn renewal_fences_old_expiry_and_newer_reply_fences_old_confirmation() {
    let mut observer = Observer::default();
    ringing(&mut observer);
    let old_lease = observer.lease(time(2)).unwrap().id().clone();
    let pending = observer.begin(time(2));
    assert_eq!(pending.query().wait_ms(), 1000);
    let newer = observer.begin(time(3));
    let reply = snapshot(newer.query().id(), 'a', 2, "observed", true);
    observer.complete(newer, reply, time(4)).unwrap();
    assert!(!observer.expire(&old_lease, time(6)));
    assert!(observer.lease(time(6)).is_some());
    let old_reply = snapshot(pending.query().id(), 'a', 1, "observed", false);
    assert!(!observer.complete(pending, old_reply, time(6)).unwrap());
    assert!(observer.lease(time(6)).is_some());
    let lease = observer.lease(time(6)).unwrap().id().clone();
    assert!(observer.expire(&lease, time(8)));
    assert_eq!(observer.begin(time(8)).query().wait_ms(), 0);
}
#[test]
fn source_loss_and_errors_withdraw_and_revision_regression_is_rejected() {
    for state in ["detached", "unavailable", "observed"] {
        let mut observer = Observer::default();
        ringing(&mut observer);
        let request = observer.begin(time(2));
        let reply = snapshot(request.query().id(), 'a', 2, state, false);
        observer.complete(request, reply, time(3)).unwrap();
        assert!(observer.lease(time(3)).is_none());
    }
    let mut observer = Observer::default();
    ringing(&mut observer);
    let request = observer.begin(time(2));
    let reply = snapshot(request.query().id(), 'a', 1, "observed", true);
    observer.complete(request, reply, time(3)).unwrap();
    observer.invalidate();
    assert!(observer.lease(time(3)).is_none());
    let request = observer.begin(time(3));
    let reply = snapshot(request.query().id(), 'a', 5, "observed", true);
    observer.complete(request, reply, time(4)).unwrap();
    let request = observer.begin(time(4));
    let reply = snapshot(request.query().id(), 'a', 4, "observed", true);
    assert!(observer.complete(request, reply, time(4)).is_err());
    assert!(observer.lease(time(4)).is_none());
}

#[test]
fn late_transport_failure_cannot_withdraw_a_newer_lease() {
    let mut observer = Observer::default();
    ringing(&mut observer);
    let old = observer.begin(time(2));
    let new = observer.begin(time(3));
    let reply = snapshot(new.query().id(), 'b', 1, "observed", true);
    observer.complete(new, reply, time(4)).unwrap();
    assert!(!observer.failed(old));
    assert!(observer.lease(time(4)).is_some());
    let current = observer.begin(time(4));
    assert!(observer.failed(current));
    assert!(observer.lease(time(4)).is_none());
}

#[test]
fn continuity_changes_on_expiry_generation_or_episode_even_without_empty_sample() {
    for case in 0..3 {
        let mut observer = Observer::default();
        ringing(&mut observer);
        let original = observer.lease(time(2)).unwrap().continuity().clone();
        let start = if case == 0 { 6 } else { 3 };
        let pending = observer.begin(time(start));
        let bytes = serde_json::to_vec(&json!({
            "v":1,"id":pending.query().id(),"generation":id(if case == 1 {'b'} else {'a'}),
            "revision":2,"source":"android.telecom.InCallService","source_state":"observed",
            "ringing":[{"id":7,"episode":id(if case == 2 {'d'} else {'c'})}]
        }))
        .unwrap();
        let reply = decode_snapshot(&bytes, pending.query().id()).unwrap();
        observer.complete(pending, reply, time(start)).unwrap();
        assert_ne!(observer.lease(time(start)).unwrap().continuity(), &original);
    }
}
