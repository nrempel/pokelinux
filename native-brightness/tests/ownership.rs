//! Controlled-clock policy cases; real filesystem and process boundaries are
//! qualified separately. Calling retired here is a policy input, not a wait.
use pokelinux_brightness::model::{Action, Observation, Owner, Rejected};
const SECOND: u64 = 1_000_000_000;
const START: u64 = 10 * SECOND;
fn value(requested: u32) -> Observation {
    Observation {
        max: 400,
        requested,
        actual: Some(120),
    }
}
fn seed(owner: &mut Owner, now: u64) -> u64 {
    let ticket = owner
        .observe(now)
        .expect("one initial observation must be admitted");
    assert!(owner.retired(ticket, Some(value(200)), now + 1));
    owner.view(now + 1).revision
}
#[test]
fn empty_or_expired_views_never_authorize_a_set() {
    let mut owner = Owner::new();
    assert_eq!(
        owner.set(75, 0, START + SECOND, START),
        Err(Rejected::Unavailable)
    );
    let revision = seed(&mut owner, START);
    let historical = owner.view(START + 1);
    assert!(historical.observation.is_some());
    let later = START + 3600 * SECOND;
    assert_eq!(owner.view(later).observation, None);
    assert_eq!(
        owner.set(75, revision, later + SECOND, later),
        Err(Rejected::Unavailable)
    );
    assert!(
        historical.observation.is_some(),
        "a historical clone can remain positive without authority"
    );
}
#[test]
fn read_publication_is_anchored_to_collection_and_requires_retirement() {
    let mut owner = Owner::new();
    let ticket = owner.observe(START).expect("initial observation");
    assert_eq!(ticket.action, Action::Observe);
    assert_eq!(ticket.until_ns, START + SECOND);
    assert_eq!(ticket.publication_until_ns, START + 2 * SECOND);
    assert!(owner.view(START + 1).pending);
    assert_eq!(owner.view(START + 1).observation, None);
    assert!(owner.observe(START + 2).is_none(), "one slot only");
    assert!(owner.retired(ticket, Some(value(200)), START + SECOND / 2));
    let view = owner.view(START + SECOND / 2);
    assert_eq!(view.observation, Some(value(200)));
    assert_eq!(view.until_ns, START + 2 * SECOND);
    assert!(!view.pending);
    assert_eq!(owner.view(START + 2 * SECOND).observation, None);
}
#[test]
fn mutation_invalidates_old_revision_and_checks_independent_attribute_readback() {
    let mut owner = Owner::new();
    let revision = seed(&mut owner, START);
    assert_eq!(
        owner.set(75, revision + 1, START + SECOND, START + 2),
        Err(Rejected::StaleRevision)
    );
    let ticket = owner
        .set(75, revision, START + SECOND, START + 3)
        .expect("explicit current intent");
    assert_eq!(ticket.action, Action::Set { percent: 75 });
    assert_eq!(ticket.baseline, Some(value(200)));
    let pending = owner.view(START + 4);
    assert!(pending.pending && pending.revision > revision);
    assert_eq!(
        pending.observation, None,
        "admitted mutation invalidates prior observed state"
    );
    assert_eq!(
        owner.set(25, revision, START + SECOND, START + 5),
        Err(Rejected::Busy)
    );
    assert!(owner.retired(ticket, Some(value(300)), START + 6));
    let observed = owner.view(START + 7);
    assert_eq!(observed.observation, Some(value(300)));
    assert_eq!(
        observed.observation.unwrap().actual,
        Some(120),
        "requested readback is distinct from optional driver actual"
    );
    assert!(!observed.pending && !observed.last_set_uncertain);
    assert_eq!(
        owner.set(25, revision, START + SECOND, START + 8),
        Err(Rejected::StaleRevision)
    );
}
#[test]
fn cancellation_retains_slot_and_late_success_cannot_acknowledge_the_set() {
    let mut owner = Owner::new();
    let revision = seed(&mut owner, START);
    let ticket = owner.set(75, revision, START + SECOND, START + 2).unwrap();
    assert_eq!(owner.cancel(START + 3), Some(ticket));
    let fenced = owner.view(START + 4);
    assert!(fenced.pending && fenced.last_set_uncertain);
    assert_eq!(fenced.requested_percent, None);
    assert_eq!(fenced.observation, None);
    assert!(
        owner.observe(START + 5).is_none(),
        "cancel is not process retirement"
    );
    assert!(!owner.retired(ticket, Some(value(300)), START + 6));
    assert!(!owner.view(START + 7).pending);
    let read = owner
        .observe(START + 8)
        .expect("fresh readonly reconciliation after retirement");
    assert_eq!(read.action, Action::Observe);
    assert!(owner.retired(read, Some(value(300)), START + 9));
    let view = owner.view(START + 10);
    assert_eq!(view.observation, Some(value(300)));
    assert!(
        view.last_set_uncertain,
        "fresh values do not retrospectively acknowledge old intent"
    );
    let next = owner
        .set(25, view.revision, START + SECOND, START + 11)
        .unwrap();
    assert_eq!(
        next.action,
        Action::Set { percent: 25 },
        "only a new explicit intent dispatches another Set"
    );
}
#[test]
fn original_boottime_deadline_fences_queued_success_without_releasing_custody() {
    let mut owner = Owner::new();
    let revision = seed(&mut owner, START);
    let until = START + SECOND / 4;
    let ticket = owner.set(75, revision, until, START + 2).unwrap();
    for now in [START + 3, START + SECOND / 8] {
        assert!(owner.view(now).pending);
    }
    assert_eq!(
        ticket.until_ns, until,
        "polling must not renew the original deadline"
    );
    let expired = owner.view(until);
    assert!(expired.pending && expired.last_set_uncertain);
    assert_eq!(expired.observation, None);
    assert_eq!(expired.requested_percent, None);
    assert!(owner.observe(until + 1).is_none());
    assert!(!owner.retired(ticket, Some(value(300)), until + 2));
    let view = owner.view(until + 3);
    assert!(!view.pending && view.last_set_uncertain);
    assert_eq!(view.observation, None);
}
#[test]
fn invalid_ranges_mismatched_readback_and_changed_max_never_confirm() {
    let mut owner = Owner::new();
    let revision = seed(&mut owner, START);
    assert_eq!(
        owner.set(101, revision, START + SECOND, START + 2),
        Err(Rejected::Invalid)
    );
    assert_eq!(
        owner.set(75, revision, START + 2, START + 2),
        Err(Rejected::Expired)
    );
    assert_eq!(
        owner.set(75, revision, START + 2 * SECOND, START + 2),
        Err(Rejected::Invalid)
    );
    let ticket = owner.set(75, revision, START + SECOND, START + 3).unwrap();
    assert!(!owner.retired(ticket, Some(value(299)), START + 4));
    assert!(owner.view(START + 5).last_set_uncertain);
    let revision = seed(&mut owner, START + 6);
    let ticket = owner.set(75, revision, START + SECOND, START + 8).unwrap();
    assert!(!owner.retired(
        ticket,
        Some(Observation {
            max: 800,
            requested: 600,
            actual: None
        }),
        START + 9
    ));
    for invalid in [
        Observation {
            max: 0,
            requested: 0,
            actual: None,
        },
        Observation {
            max: 400,
            requested: 401,
            actual: None,
        },
        Observation {
            max: 400,
            requested: 200,
            actual: Some(401),
        },
    ] {
        let read = owner.observe(START + 10).unwrap();
        assert!(!owner.retired(read, Some(invalid), START + 11));
        assert_eq!(owner.view(START + 12).observation, None);
    }
}
#[test]
fn stale_retirement_cannot_release_or_publish_another_slot() {
    let mut owner = Owner::new();
    let old = owner.observe(START).unwrap();
    owner.cancel(START + 1);
    assert!(!owner.retired(old, None, START + 2));
    let current = owner.observe(START + 3).unwrap();
    assert_ne!(old.id, current.id);
    assert!(!owner.retired(old, Some(value(100)), START + 4));
    assert!(owner.view(START + 5).pending && owner.observe(START + 6).is_none());
    assert!(owner.retired(current, Some(value(200)), START + 7));
    assert_eq!(owner.view(START + 8).observation, Some(value(200)));
}
