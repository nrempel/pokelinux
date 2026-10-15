use pokelinux_process_owner::{
    deadline::{Clock, Deadline},
    supervisor::ProcessIdentity,
};
use pokelinux_sit_attempt::{Attempt, Disposition, Error, Setup, Snapshot, Withdrawal};
use pokelinux_sit_codec::{Protocol, modern_report::ModernSetupReport22, response::DecodeError};
use std::{
    num::NonZeroU8,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

const UNTIL: u64 = 1_000;
struct ManualClock(AtomicU64);
impl ManualClock {
    fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    fn set(&self, now: u64) {
        self.0.store(now, Ordering::SeqCst);
    }
}
impl Clock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::SeqCst))
    }
}
struct DecodeExpiryClock(AtomicUsize);
impl Clock for DecodeExpiryClock {
    fn now(&self) -> Duration {
        // new, dispatch, predecode are live; the postdecode check expires.
        let call = self.0.fetch_add(1, Ordering::SeqCst);
        Duration::from_nanos(if call < 3 { 0 } else { UNTIL })
    }
}
fn source() -> ProcessIdentity {
    ProcessIdentity::from_bytes([1; 16])
}
fn other_source() -> ProcessIdentity {
    ProcessIdentity::from_bytes([2; 16])
}
fn setup() -> Setup {
    Setup::new(
        NonZeroU8::new(1).unwrap(),
        "pokelinux.invalid",
        Protocol::Ipv4,
        0,
    )
    .unwrap()
}
fn owner(clock: Arc<dyn Clock>) -> Attempt {
    Attempt::new(
        setup(),
        source(),
        clock,
        Deadline::at(Duration::from_nanos(UNTIL)),
    )
    .unwrap()
}
fn bytes(text: &str) -> Vec<u8> {
    let text: String = text.split_whitespace().collect();
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn response() -> Vec<u8> {
    bytes(include_str!("fixtures/base-v4.hex"))
}
fn expected_report() -> ModernSetupReport22 {
    ModernSetupReport22 {
        cause: 0,
        opaque_i64_at_8: 0,
        cid: 1,
        active: 2,
        protocol_code: 0,
        mtu_v4: 1420,
        mtu_v6: 0,
        ifname: Some("rmnet0".to_owned()),
        address_text: Some("192.0.2.7".to_owned()),
        dnses: vec!["192.0.2.53".to_owned(), "198.51.100.53".to_owned()],
        gateways: vec!["0.0.0.0".to_owned()],
        pcscf: vec![
            "203.0.113.1".to_owned(),
            "203.0.113.2".to_owned(),
            "203.0.113.3".to_owned(),
        ],
        structured_addresses: [],
        qos_sessions: [],
        traffic_descriptors: [],
    }
}
fn sent(owner: &Attempt) {
    let packet = owner.take_dispatch().unwrap();
    assert_eq!(
        packet.bytes().as_slice(),
        bytes(include_str!("fixtures/initial-v4.hex"))
    );
    assert_eq!(
        owner.snapshot().unwrap().disposition,
        Disposition::OutcomeUnknown
    );
    let _discarded_packet = packet;
}
fn unobserved(snapshot: Snapshot, disposition: Disposition, withdrawal: Option<Withdrawal>) {
    assert_eq!(snapshot.disposition, disposition);
    assert_eq!(snapshot.withdrawal, withdrawal);
    assert!(snapshot.current.is_none());
    assert!(snapshot.history.is_none());
}

#[test]
fn validates_setup_and_deadline_before_attempt_admission() {
    let context = NonZeroU8::new(1).unwrap();
    assert_eq!(
        Setup::new(context, "", Protocol::Ipv4, 0).err(),
        Some(Error::InvalidSetup(pokelinux_sit_codec::Error::InvalidApn))
    );
    assert_eq!(
        Setup::new(context, "pokelinux.invalid", Protocol::Ipv4, 100).err(),
        Some(Error::InvalidPresentationIndex)
    );
    let clock = Arc::new(ManualClock::new());
    clock.set(UNTIL);
    assert_eq!(
        Attempt::new(
            setup(),
            source(),
            clock,
            Deadline::at(Duration::from_nanos(UNTIL))
        )
        .err(),
        Some(Error::Deadline)
    );
}

#[test]
fn dispatch_is_once_and_discarded_packet_keeps_uncertainty() {
    let owner = owner(Arc::new(ManualClock::new()));
    unobserved(owner.snapshot().unwrap(), Disposition::NotSent, None);
    sent(&owner);
    assert_eq!(owner.take_dispatch().err(), Some(Error::AlreadyDispatched));
    unobserved(owner.snapshot().unwrap(), Disposition::OutcomeUnknown, None);
}

#[test]
fn cancelled_before_dispatch_remains_not_sent_and_readable() {
    let owner = owner(Arc::new(ManualClock::new()));
    owner.cancel().unwrap();
    assert_eq!(owner.take_dispatch().err(), Some(Error::Cancelled));
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::NotSent,
        Some(Withdrawal::Cancelled),
    );
}

#[test]
fn exact_deadline_before_dispatch_remains_not_sent_without_timer() {
    let clock = Arc::new(ManualClock::new());
    let owner = owner(clock.clone());
    clock.set(UNTIL);
    assert_eq!(owner.take_dispatch().err(), Some(Error::Deadline));
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::NotSent,
        Some(Withdrawal::Deadline),
    );
}

#[test]
fn response_before_dispatch_is_not_admitted() {
    let owner = owner(Arc::new(ManualClock::new()));
    assert_eq!(
        owner.prepare_response(source(), &response()).err(),
        Some(Error::NotDispatched)
    );
    unobserved(owner.snapshot().unwrap(), Disposition::NotSent, None);
}

#[test]
fn wrong_source_token_context_and_length_do_not_consume_valid_candidate() {
    let owner = owner(Arc::new(ManualClock::new()));
    sent(&owner);
    assert_eq!(
        owner.prepare_response(other_source(), &response()).err(),
        Some(Error::WrongSource)
    );
    let mut wrong = response();
    wrong[6] = 1;
    assert_eq!(
        owner.prepare_response(source(), &wrong).err(),
        Some(Error::Decode(DecodeError::TokenMismatch))
    );
    let mut wrong = response();
    wrong[14] = 2;
    assert_eq!(
        owner.prepare_response(source(), &wrong).err(),
        Some(Error::ContextMismatch)
    );
    for length in [0, 12, 143] {
        assert_eq!(
            owner
                .prepare_response(source(), &response()[..length])
                .err(),
            Some(Error::Decode(DecodeError::UnsupportedLength))
        );
    }
    let mut too_long = response();
    too_long.push(0);
    assert_eq!(
        owner.prepare_response(source(), &too_long).err(),
        Some(Error::Decode(DecodeError::UnsupportedLength))
    );
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    assert_eq!(candidate.source(), source());
    assert_eq!(candidate.record().as_slice(), response());
    assert_eq!(candidate.report(), &expected_report());
    unobserved(owner.snapshot().unwrap(), Disposition::OutcomeUnknown, None);
}

#[test]
fn one_valid_candidate_is_issued_without_reset_or_second_response_slot() {
    let owner = owner(Arc::new(ManualClock::new()));
    sent(&owner);
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    assert_eq!(
        owner.prepare_response(source(), &response()).err(),
        Some(Error::DuplicateResponse)
    );
    drop(candidate);
    assert_eq!(
        owner.prepare_response(source(), &response()).err(),
        Some(Error::DuplicateResponse)
    );
    assert_eq!(owner.take_dispatch().err(), Some(Error::AlreadyDispatched));
}

#[test]
fn candidate_from_another_attempt_cannot_be_committed() {
    let clock = Arc::new(ManualClock::new());
    let first = owner(clock.clone());
    let second = owner(clock);
    sent(&first);
    sent(&second);
    let candidate = first.prepare_response(source(), &response()).unwrap();
    assert_eq!(second.commit(candidate), Err(Error::WrongAttempt));
    unobserved(
        second.snapshot().unwrap(),
        Disposition::OutcomeUnknown,
        None,
    );
}

#[test]
fn cancellation_after_candidate_prevents_commit_but_keeps_uncertainty() {
    let owner = owner(Arc::new(ManualClock::new()));
    sent(&owner);
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    owner.cancel().unwrap();
    assert_eq!(owner.commit(candidate), Err(Error::Cancelled));
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::OutcomeUnknown,
        Some(Withdrawal::Cancelled),
    );
}

#[test]
fn held_candidate_expires_at_commit_without_timer_delivery() {
    let clock = Arc::new(ManualClock::new());
    let owner = owner(clock.clone());
    sent(&owner);
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    clock.set(UNTIL);
    assert_eq!(owner.commit(candidate), Err(Error::Deadline));
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::OutcomeUnknown,
        Some(Withdrawal::Deadline),
    );
}

#[test]
fn source_loss_prevents_candidate_commit_and_wrong_loss_source_does_not() {
    let owner = owner(Arc::new(ManualClock::new()));
    sent(&owner);
    assert_eq!(owner.source_lost(other_source()), Err(Error::WrongSource));
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    owner.source_lost(source()).unwrap();
    assert_eq!(owner.commit(candidate), Err(Error::SourceLost));
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::OutcomeUnknown,
        Some(Withdrawal::SourceLost),
    );
}

#[test]
fn expiry_during_decode_is_refused_at_the_postdecode_gate() {
    let owner = owner(Arc::new(DecodeExpiryClock(AtomicUsize::new(0))));
    // Do not sample snapshot here: the four planned clock reads are new,
    // dispatch, predecode, and postdecode. No production pause hook is needed.
    let _discarded_packet = owner.take_dispatch().unwrap();
    assert_eq!(
        owner.prepare_response(source(), &response()).err(),
        Some(Error::Deadline)
    );
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::OutcomeUnknown,
        Some(Withdrawal::Deadline),
    );
}

#[test]
fn first_withdrawal_reason_is_sticky_and_snapshot_remains_readable() {
    let clock = Arc::new(ManualClock::new());
    let owner = owner(clock.clone());
    owner.cancel().unwrap();
    owner.source_lost(source()).unwrap();
    clock.set(UNTIL);
    unobserved(
        owner.snapshot().unwrap(),
        Disposition::NotSent,
        Some(Withdrawal::Cancelled),
    );
}

#[test]
fn valid_completion_records_observation_without_settling_dispatch_outcome() {
    let owner = owner(Arc::new(ManualClock::new()));
    sent(&owner);
    let candidate = owner.prepare_response(source(), &response()).unwrap();
    assert_eq!(candidate.report(), &expected_report());
    let snapshot = owner
        .commit(candidate)
        .expect("MISSING_SIT_ATTEMPT_RESPONSE_COMPLETION");
    assert_eq!(snapshot.disposition, Disposition::OutcomeUnknown);
    assert_eq!(snapshot.withdrawal, None);
    let current = snapshot.current.unwrap();
    let history = snapshot.history.unwrap();
    assert!(Arc::ptr_eq(&current, &history));
    assert_eq!(history.source(), source());
    assert_eq!(history.record().as_slice(), response());
    assert_eq!(history.report(), &expected_report());
    assert_eq!(
        owner.snapshot().unwrap().history.as_deref(),
        Some(history.as_ref())
    );
    assert_eq!(
        owner.prepare_response(source(), &response()).err(),
        Some(Error::DuplicateResponse)
    );
    assert_eq!(owner.take_dispatch().err(), Some(Error::AlreadyDispatched));
}

#[test]
fn withdrawal_preserves_the_single_committed_historical_observation() {
    for action in [
        Withdrawal::Cancelled,
        Withdrawal::Deadline,
        Withdrawal::SourceLost,
    ] {
        let clock = Arc::new(ManualClock::new());
        let owner = owner(clock.clone());
        sent(&owner);
        let candidate = owner.prepare_response(source(), &response()).unwrap();
        let history = owner
            .commit(candidate)
            .expect("MISSING_SIT_ATTEMPT_RESPONSE_COMPLETION")
            .history
            .unwrap();
        match action {
            Withdrawal::Cancelled => owner.cancel().unwrap(),
            Withdrawal::Deadline => clock.set(UNTIL),
            Withdrawal::SourceLost => owner.source_lost(source()).unwrap(),
        }
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.disposition, Disposition::OutcomeUnknown);
        assert_eq!(snapshot.withdrawal, Some(action));
        assert!(snapshot.current.is_none());
        let retained = snapshot.history.unwrap();
        assert!(Arc::ptr_eq(&history, &retained));
        assert_eq!(retained.report(), &expected_report());
        assert_eq!(retained.record().as_slice(), response());
    }
}
