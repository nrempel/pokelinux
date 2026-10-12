use pokelinux_messaging::{
    outbox::{Outbox, PartState},
    transport::{EndpointId, MAX_ENDPOINTS, MAX_PENDING, Outcome, Signal, SubmissionOwner},
};
use std::path::PathBuf;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("pokelinux-sms-owner-{}", uuid::Uuid::new_v4())))
    }
    fn owner(&self) -> SubmissionOwner {
        SubmissionOwner::new(Outbox::open(&self.0).unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn id(n: usize) -> String {
    format!("{n:032x}")
}
fn prepare(owner: &mut SubmissionOwner, n: usize) -> String {
    let message = id(n);
    owner
        .prepare(&message, "+15550100", &"x".repeat(161), true)
        .unwrap();
    message
}
#[test]
fn one_way_and_intermediate_ack_preserve_uncertainty() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(EndpointId(1)).unwrap();
    let message = prepare(&mut owner, 1);
    let request = owner.begin(source, &message, 0).unwrap();
    assert_eq!(
        owner.receipt(&message).unwrap().parts[0],
        PartState::Uncertain
    );
    for signal in [Signal::IpcReturned, Signal::RequestAcknowledged] {
        assert_eq!(
            owner.observe(source, request.attempt, signal).unwrap(),
            Outcome::Pending
        );
        assert_eq!(
            owner.receipt(&message).unwrap().parts[0],
            PartState::Uncertain
        );
    }
    assert!(owner.begin(source, &message, 0).is_err());
    assert!(owner.begin(source, &message, 1).is_err());
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Accepted
    );
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert_eq!(
        owner.receipt(&message).unwrap().parts[0],
        PartState::TransportAccepted
    );
}
#[test]
fn replacement_and_foreign_source_cannot_finish_a_new_attempt() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let old = owner.attach(EndpointId(1)).unwrap();
    let first = prepare(&mut owner, 1);
    let pending = owner.begin(old, &first, 0).unwrap();
    let new = owner.attach(EndpointId(1)).unwrap();
    let second = prepare(&mut owner, 2);
    let next = owner.begin(new, &second, 0).unwrap();
    for (source, attempt) in [
        (old, pending.attempt),
        (old, next.attempt),
        (new, pending.attempt),
    ] {
        assert_eq!(
            owner
                .observe(source, attempt, Signal::SendAccepted)
                .unwrap(),
            Outcome::Ignored
        );
    }
    let other_directory = Directory::new();
    let mut other = other_directory.owner();
    let foreign = other.attach(EndpointId(1)).unwrap();
    assert_eq!(
        owner
            .observe(foreign, next.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    owner.revoke(old);
    assert_eq!(
        owner
            .observe(new, next.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Accepted
    );
    assert_eq!(
        owner.receipt(&first).unwrap().parts[0],
        PartState::Uncertain
    );
}
#[test]
fn cancellation_and_death_fence_consumed_work() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(EndpointId(1)).unwrap();
    let cancelled = prepare(&mut owner, 1);
    let request = owner.begin(source, &cancelled, 0).unwrap();
    owner.cancel(&cancelled).unwrap();
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert!(owner.receipt(&cancelled).unwrap().cancelled);
    assert!(owner.begin(source, &cancelled, 1).is_err());
    let dying = prepare(&mut owner, 2);
    let request = owner.begin(source, &dying, 0).unwrap();
    owner.revoke(source);
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert_eq!(
        owner.receipt(&dying).unwrap().parts[0],
        PartState::Uncertain
    );
    assert!(owner.begin(source, &dying, 0).is_err());
}
#[test]
fn restart_retains_uncertainty_and_rejects_old_callback_identity() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let old = owner.attach(EndpointId(1)).unwrap();
    let message = prepare(&mut owner, 1);
    let request = owner.begin(old, &message, 0).unwrap();
    drop(owner);
    let mut next = directory.owner();
    let source = next.attach(EndpointId(1)).unwrap();
    assert_eq!(
        next.observe(old, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert_eq!(
        next.receipt(&message).unwrap().parts[0],
        PartState::Uncertain
    );
    assert!(next.begin(source, &message, 0).is_err());
    assert!(next.begin(source, &message, 1).is_err());
}
#[test]
fn error_without_negative_submission_proof_is_terminal_uncertainty() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(EndpointId(1)).unwrap();
    let message = prepare(&mut owner, 1);
    let request = owner.begin(source, &message, 0).unwrap();
    let _ = owner
        .observe(source, request.attempt, Signal::Uncertain)
        .unwrap();
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert_eq!(
        owner.receipt(&message).unwrap().parts[0],
        PartState::Uncertain
    );
    assert!(owner.begin(source, &message, 0).is_err());
}
#[test]
fn failure_to_store_acceptance_never_produces_an_accepted_outcome() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(EndpointId(1)).unwrap();
    let message = prepare(&mut owner, 1);
    let request = owner.begin(source, &message, 0).unwrap();
    let database = rusqlite::Connection::open(directory.0.join("outbox.sqlite3")).unwrap();
    database.execute_batch("CREATE TRIGGER deny_accept BEFORE UPDATE OF state ON parts WHEN NEW.state='accepted' BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .is_err()
    );
    assert_eq!(
        owner.receipt(&message).unwrap().parts[0],
        PartState::Uncertain
    );
    assert_eq!(
        owner
            .observe(source, request.attempt, Signal::SendAccepted)
            .unwrap(),
        Outcome::Ignored
    );
    assert!(owner.begin(source, &message, 0).is_err());
}
#[test]
fn capacity_checks_precede_durable_claim() {
    let directory = Directory::new();
    let mut owner = directory.owner();
    let source = owner.attach(EndpointId(0)).unwrap();
    for n in 1..MAX_ENDPOINTS {
        owner.attach(EndpointId(n as u64)).unwrap();
    }
    assert!(owner.attach(EndpointId(MAX_ENDPOINTS as u64)).is_err());
    for n in 0..MAX_PENDING {
        let message = prepare(&mut owner, n);
        owner.begin(source, &message, 0).unwrap();
    }
    let overflow = prepare(&mut owner, MAX_PENDING);
    assert!(owner.begin(source, &overflow, 0).is_err());
    assert_eq!(owner.receipt(&overflow).unwrap().parts[0], PartState::Ready);
}
