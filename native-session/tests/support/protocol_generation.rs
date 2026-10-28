//! Chronological RED: real credentials/pidfds, authored Value wire, old API only.
use super::*;
use pokelinux_lifecycle::session::Provider;
use serde_json::{Value, json};

fn locked(generation: Value) -> Value {
    json!({"locked":true,"mapped":true,"lock_generation":generation,"protocol_unlock":null})
}
fn generic_unlock() -> Value {
    json!({"locked":false,"lock_generation":null,"protocol_unlock":null})
}
fn exact(generation: u64) -> Value {
    json!({"locked":false,"lock_generation":null,
        "protocol_unlock":{"generation":generation,"sequence":"this","observed_at_ns":"sample"}})
}
fn repeated() -> Value {
    json!({"locked":false,"lock_generation":null,"protocol_unlock":"repeat"})
}
fn send(peer: &mut Peer, specs: Vec<Value>) {
    peer.command(&format!(
        "generation-wire:{}",
        serde_json::to_string(&specs).unwrap()
    ));
}
fn admitted(observer: &mut Observer, reason: &str) -> Value {
    let status = observer.poll();
    assert!(
        status.is_ok(),
        "positive generation wire admission required ({reason}); actual={status:?}"
    );
    serde_json::to_value(
        observer
            .snapshot()
            .expect("admitted real packet must remain fresh"),
    )
    .unwrap()
}
fn establish_protocol(
    peer: &mut Peer,
    observer: &mut Observer,
    initial: Value,
    reason: &str,
) -> Value {
    send(peer, vec![initial.clone()]);
    observer
        .poll()
        .expect("valid protocol prerequisite is provisional");
    assert!(
        observer.snapshot().is_none(),
        "one protocol frame exported authority"
    );
    send(peer, vec![initial]);
    admitted(observer, reason)
}
fn terminal(observer: &mut Observer, peer: &mut Peer, reason: &str) {
    assert!(
        observer.poll().is_err(),
        "invalid generation transition admitted: {reason}"
    );
    assert!(
        observer.snapshot().is_none(),
        "invalid generation retained positive facts: {reason}"
    );
    send(peer, vec![locked(json!(1))]);
    assert!(
        observer.poll().is_err(),
        "late valid packet revived failed generation source: {reason}"
    );
    assert!(observer.snapshot().is_none());
}

#[test]
fn positive_known_generic_exact_and_sticky_heartbeats_preserve_event_revision() {
    let (mut p, mut o) = start(0);
    let first = establish_protocol(
        &mut p,
        &mut o,
        locked(json!(1)),
        "first accepted generation1 with real mapped peer",
    );
    assert_eq!(first["lock_generation"], 1);
    assert!(first["protocol_unlock"].is_null());
    assert_eq!(o.revision(), 1);
    let owned = Provider::poll(&mut o).unwrap().unwrap();
    assert_eq!(owned.locker.as_ref().unwrap().metadata.pid, p.child.id());
    assert_eq!(owned.revision, 1);
    send(&mut p, vec![generic_unlock()]);
    let generic = admitted(&mut o, "interim generic unlock without object cause");
    assert!(generic["lock_generation"].is_null() && generic["protocol_unlock"].is_null());
    assert_eq!(o.revision(), 2);
    send(&mut p, vec![exact(1)]);
    let event = admitted(&mut o, "exact-object protocol event after generic unlock");
    assert_eq!(event["protocol_unlock"]["generation"], 1);
    assert_eq!(event["protocol_unlock"]["sequence"], event["sequence"]);
    assert!(
        event["protocol_unlock"]["observed_at_ns"].as_u64().unwrap()
            >= generic["sampled_at_ns"].as_u64().unwrap()
    );
    assert_eq!(
        o.revision(),
        3,
        "new exact event must advance semantic revision"
    );
    let sticky_bytes = serde_json::to_vec(&event["protocol_unlock"]).unwrap();
    for _ in 0..2 {
        send(&mut p, vec![repeated()]);
        let heartbeat = admitted(&mut o, "same exact record on unlocked heartbeat");
        assert_eq!(
            serde_json::to_vec(&heartbeat["protocol_unlock"]).unwrap(),
            sticky_bytes
        );
        assert_eq!(
            o.revision(),
            3,
            "sticky event heartbeat must preserve revision"
        );
        let snapshot = Provider::poll(&mut o).unwrap().unwrap();
        assert_eq!(snapshot.revision, 3);
        assert!(!snapshot.locked && snapshot.locker.is_none());
    }
    let snapshot = Provider::poll(&mut o).unwrap().unwrap();
    assert!(snapshot.lock_generation.is_none());
    assert_eq!(
        serde_json::to_value(snapshot.protocol_unlock.unwrap()).unwrap(),
        event["protocol_unlock"]
    );
}

#[test]
fn positive_late_attachment_unknown_generation_keeps_real_mapped_owner() {
    let (mut p, mut o) = start(0);
    let frame = establish_protocol(
        &mut p,
        &mut o,
        locked(Value::Null),
        "late attachment mapped owner with unknown generation",
    );
    assert!(frame["lock_generation"].is_null());
    assert!(frame["protocol_unlock"].is_null());
    assert_eq!(frame["locker"]["pid"], p.child.id());
    let snapshot = Provider::poll(&mut o).unwrap().unwrap();
    let locker = snapshot.locker.as_ref().unwrap();
    assert!(
        pokelinux_kernel_identity::ProcessView::open()
            .unwrap()
            .matches(locker.pidfd(), locker.pidfd(), p.child.id())
            .unwrap()
    );
    assert!(
        rustix::io::fcntl_getfd(locker.pidfd())
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
}

#[test]
fn positive_queued_accepted_unlock_exact_unknown_relock_keeps_all_revisions() {
    let (mut p, mut o) = start(0);
    let initial = establish_protocol(
        &mut p,
        &mut o,
        generic_unlock(),
        "neutral authority before queued transitions",
    );
    let initial_revision = o.revision();
    let initial_sequence = initial["sequence"].as_u64().unwrap();
    send(
        &mut p,
        vec![
            locked(json!(1)),
            generic_unlock(),
            exact(1),
            locked(Value::Null),
            locked(json!(2)),
        ],
    );
    let frame = admitted(
        &mut o,
        "all five queued semantic transitions must be admitted in one bounded drain",
    );
    assert_eq!(frame["sequence"], initial_sequence + 5);
    assert_eq!(frame["lock_generation"], 2);
    assert!(frame["protocol_unlock"].is_null());
    assert_eq!(
        o.revision(),
        initial_revision + 5,
        "queued exact event and generation changes cannot coalesce away"
    );
    assert_eq!(
        Provider::poll(&mut o).unwrap().unwrap().revision,
        initial_revision + 5
    );
}

#[test]
fn positive_known_generation_survives_temporarily_unavailable_mapped_owner() {
    let (mut p, mut o) = start(0);
    establish_protocol(
        &mut p,
        &mut o,
        locked(json!(1)),
        "known generation with actual owned mapped locker",
    );
    let mut unavailable = locked(json!(1));
    unavailable["mapped"] = json!(false);
    send(&mut p, vec![unavailable]);
    let frame = admitted(
        &mut o,
        "accepted generation survives temporary mapping unavailability",
    );
    assert_eq!(frame["lock_generation"], 1);
    assert_eq!(frame["locker"]["state"], "unavailable");
    assert_eq!(o.revision(), 2);
    send(&mut p, vec![locked(json!(1))]);
    let frame = admitted(&mut o, "same known generation reacquires mapped owned peer");
    assert_eq!(frame["lock_generation"], 1);
    assert_eq!(o.revision(), 3);
}

#[test]
fn initial_generation_shape_refusals_are_terminal_but_old_unknown_field_is_not_semantic_proof() {
    let mut unseen = exact(1);
    unseen["protocol_unlock"]["sequence"] = json!(1);
    let mut future_sequence = exact(1);
    future_sequence["protocol_unlock"]["sequence"] = json!("future");
    let mut future_time = exact(1);
    future_time["protocol_unlock"]["observed_at_ns"] = json!("future");
    let mut zero_event = exact(0);
    zero_event["protocol_unlock"]["sequence"] = json!(0);
    zero_event["protocol_unlock"]["observed_at_ns"] = json!(0);
    let mut unlocked_generation = generic_unlock();
    unlocked_generation["lock_generation"] = json!(1);
    let mut locked_event = locked(json!(1));
    locked_event["protocol_unlock"] = exact(1)["protocol_unlock"].clone();
    for (reason, prefix, packet) in [
        ("zero current generation", vec![], locked(json!(0))),
        ("counter must start1", vec![], locked(json!(2))),
        ("unseen accepted generation", vec![], unseen),
        (
            "future event sequence",
            vec![locked(json!(1)), generic_unlock()],
            future_sequence,
        ),
        (
            "future event time",
            vec![locked(json!(1)), generic_unlock()],
            future_time,
        ),
        (
            "zero event fields",
            vec![locked(json!(1)), generic_unlock()],
            zero_event,
        ),
        ("generation on unlocked frame", vec![], unlocked_generation),
        (
            "event on locked frame",
            vec![locked(json!(1))],
            locked_event,
        ),
    ] {
        let (mut p, mut o) = start(0);
        if !prefix.is_empty() {
            let provisional = prefix.len() == 1;
            send(&mut p, prefix);
            if provisional {
                o.poll()
                    .expect("valid initial prerequisite remains provisional");
                assert!(o.snapshot().is_none());
            } else {
                admitted(&mut o, &format!("valid prerequisite before {reason}"));
            }
        }
        send(&mut p, vec![packet]);
        terminal(&mut o, &mut p, reason);
        eprintln!(
            "INITIAL_REFUSAL_CASE: {reason}; old strict unknown-field refusal is NOT new semantic validation proof"
        );
    }
}

fn after_valid_prefix(reason: &str, prefix: Vec<Value>, bad: Value) {
    let (mut p, mut o) = start(0);
    let mut prefix = prefix.into_iter();
    establish_protocol(
        &mut p,
        &mut o,
        prefix.next().expect("valid initial protocol state"),
        &format!("established prerequisite before {reason}"),
    );
    let remaining: Vec<_> = prefix.collect();
    if !remaining.is_empty() {
        send(&mut p, remaining);
        admitted(&mut o, &format!("valid prerequisite before {reason}"));
    }
    send(&mut p, vec![bad]);
    terminal(&mut o, &mut p, reason);
}

#[test]
fn terminal_exact_event_time_before_previous_sample() {
    let mut bad = exact(1);
    bad["protocol_unlock"]["observed_at_ns"] = json!("before-previous");
    after_valid_prefix(
        "event precedes previous sample",
        vec![locked(json!(1)), generic_unlock()],
        bad,
    );
}
#[test]
fn terminal_first_event_sequence_must_be_containing_frame() {
    let mut bad = exact(1);
    bad["protocol_unlock"]["sequence"] = json!(1);
    after_valid_prefix(
        "first event cites earlier sequence",
        vec![locked(json!(1)), generic_unlock()],
        bad,
    );
}
#[test]
fn terminal_event_generation_must_match_previously_accepted() {
    after_valid_prefix(
        "event names unseen generation2",
        vec![locked(json!(1)), generic_unlock()],
        exact(2),
    );
}
#[test]
fn terminal_sticky_record_mutation() {
    after_valid_prefix(
        "sticky record rewritten at later sequence/time",
        vec![locked(json!(1)), generic_unlock(), exact(1)],
        exact(1),
    );
}
#[test]
fn terminal_sticky_record_drop_on_unlocked_heartbeat() {
    after_valid_prefix(
        "sticky record dropped while still unlocked",
        vec![locked(json!(1)), generic_unlock(), exact(1)],
        generic_unlock(),
    );
}
#[test]
fn terminal_generation_reuse_after_unknown_new_lock() {
    after_valid_prefix(
        "generation1 reused in new lock",
        vec![
            locked(json!(1)),
            generic_unlock(),
            exact(1),
            locked(Value::Null),
        ],
        locked(json!(1)),
    );
}
#[test]
fn terminal_generation_counter_skip() {
    after_valid_prefix(
        "accepted counter skips2",
        vec![
            locked(json!(1)),
            generic_unlock(),
            exact(1),
            locked(Value::Null),
        ],
        locked(json!(3)),
    );
}
#[test]
fn terminal_current_generation_rollback() {
    after_valid_prefix(
        "current generation rolls back2to1",
        vec![
            locked(json!(1)),
            generic_unlock(),
            exact(1),
            locked(json!(2)),
        ],
        locked(json!(1)),
    );
}
#[test]
fn terminal_retired_event_replayed_after_unknown_new_lock() {
    after_valid_prefix(
        "old sticky event reused after unknown lock retired it",
        vec![
            locked(json!(1)),
            generic_unlock(),
            exact(1),
            locked(Value::Null),
            generic_unlock(),
        ],
        repeated(),
    );
}
#[test]
fn required_generation_field_cannot_be_omitted() {
    let (mut p, mut o) = start(0);
    let mut packet = generic_unlock();
    packet.as_object_mut().unwrap().remove("lock_generation");
    send(&mut p, vec![packet]);
    terminal(&mut o, &mut p, "required nullable lock_generation omitted");
}
#[test]
fn required_protocol_unlock_field_cannot_be_omitted() {
    let (mut p, mut o) = start(0);
    let mut packet = generic_unlock();
    packet.as_object_mut().unwrap().remove("protocol_unlock");
    send(&mut p, vec![packet]);
    terminal(&mut o, &mut p, "required nullable protocol_unlock omitted");
}
#[test]
fn both_new_required_fields_cannot_be_omitted() {
    let (mut p, mut o) = start(0);
    send(&mut p, vec![json!({"locked":false})]);
    terminal(
        &mut o,
        &mut p,
        "both required generation fields omitted; old wire has no compatibility",
    );
}
