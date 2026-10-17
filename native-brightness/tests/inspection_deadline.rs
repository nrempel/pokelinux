//! Controlled policy clocks only. An explicit retired input is not a process
//! wait, pipe EOF, kernel suspend, or physical brightness observation.
use pokelinux_brightness::model::{Action, Observation, Owner};

const SECOND: u64 = 1_000_000_000;
const START: u64 = 10 * SECOND;

fn observed() -> Observation {
    Observation {
        max: 400,
        requested: 200,
        actual: Some(120),
    }
}

#[test]
fn earlier_caller_deadline_is_exact_without_shortening_publication_horizon() {
    let original_request_until = START + SECOND;
    let collection_started = START + 3 * SECOND / 4;
    let mut owner = Owner::new();
    let ticket = owner
        .observe_until(collection_started, original_request_until)
        .expect("a caller's remaining quarter-second must be admissible");
    assert_eq!(ticket.action, Action::Observe);
    assert_eq!(ticket.until_ns, original_request_until);
    assert_eq!(
        ticket.publication_until_ns,
        collection_started + 2 * SECOND,
        "publication remains anchored to collection start, never completion"
    );
    assert!(owner.retired(ticket, Some(observed()), original_request_until - 1));
    let view = owner.view(original_request_until - 1);
    assert_eq!(view.observation, Some(observed()));
    assert_eq!(view.until_ns, collection_started + 2 * SECOND);
    assert_eq!(
        owner.view(collection_started + 2 * SECOND).observation,
        None
    );
}

#[test]
fn completion_at_original_readonly_deadline_is_refused_until_slot_retirement() {
    let original_request_until = START + SECOND / 4;
    let mut owner = Owner::new();
    let ticket = owner
        .observe_until(START, original_request_until)
        .expect("valid caller deadline");
    let pending = owner.view(original_request_until);
    assert!(pending.pending);
    assert_eq!(pending.observation, None);
    assert!(
        owner
            .observe_until(original_request_until, original_request_until + SECOND)
            .is_none(),
        "expiry alone must not release the occupied worker slot"
    );
    assert!(
        !owner.retired(ticket, Some(observed()), original_request_until),
        "completion exactly at the original deadline is late"
    );
    let retired = owner.view(original_request_until);
    assert!(!retired.pending);
    assert_eq!(retired.observation, None);
    let next = owner
        .observe_until(original_request_until + 1, original_request_until + SECOND)
        .expect("explicit retirement releases the slot");
    assert_eq!(next.id, ticket.id + 1);
}

#[test]
fn expired_and_overlong_deadlines_preserve_view_and_do_not_consume_ticket_ids() {
    let now = START + 10;
    for until in [now - 1, now, now + SECOND + 1, u64::MAX] {
        let mut owner = Owner::new();
        let initial = owner.observe(START).expect("ordinary observation");
        assert!(owner.retired(initial, Some(observed()), START + 1));
        let before = owner.view(now);
        assert!(owner.observe_until(now, until).is_none());
        assert_eq!(
            owner.view(now),
            before,
            "refusal changes no live policy state"
        );
        let next = owner
            .observe_until(now, now + SECOND / 4)
            .expect("a refused deadline consumes no admission");
        assert_eq!(next.id, initial.id + 1, "refusal consumes no ticket ID");
        assert_eq!(next.until_ns, now + SECOND / 4);
    }
}

#[test]
fn ordinary_observe_and_exact_one_second_caller_budget_remain_valid() {
    let mut ordinary = Owner::new();
    let old = ordinary.observe(START).expect("existing ordinary API");
    assert_eq!(old.until_ns, START + SECOND);
    assert_eq!(old.publication_until_ns, START + 2 * SECOND);
    assert!(ordinary.retired(old, Some(observed()), START + SECOND - 1));

    let mut explicit = Owner::new();
    let ticket = explicit
        .observe_until(START, START + SECOND)
        .expect("exactly one second is within the maximum");
    assert_eq!(ticket.until_ns, START + SECOND);
    assert_eq!(ticket.publication_until_ns, START + 2 * SECOND);
    assert!(explicit.retired(ticket, Some(observed()), START + SECOND - 1));
}
