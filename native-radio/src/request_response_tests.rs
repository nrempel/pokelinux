//! Independent frozen local response-attribution contract. The unit fixture uses
//! real decoded parcels, CallbackQueue, dispatch admission and RadioOwner commit.
//! It supplies authored metadata; it is not a Binder/modem integration result.
use super::*;
use crate::{
    RadioExternalResponseInfo,
    owner::{Observation, RadioOwner, Registration},
    receiver::{RadioEvent, ReceiverFailure},
    response::decode_response_arguments,
};
use pokelinux_process_owner::deadline::Clock;
use rsbinder::Parcel;
use std::{
    num::NonZeroUsize,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

struct Controlled(AtomicU64);
impl Clock for Controlled {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::Acquire))
    }
}
fn n(v: usize) -> NonZeroUsize {
    NonZeroUsize::new(v).unwrap()
}
fn setup() -> (RadioOwner, Registration, Arc<Controlled>) {
    let clock = Arc::new(Controlled(AtomicU64::new(10)));
    let mut owner = RadioOwner::with_clock(n(4), 1, clock.clone());
    let reg = owner
        .begin_until(None, n(16), Deadline::at(Duration::from_nanos(100)))
        .unwrap();
    owner.activate(&reg.key, 42).unwrap();
    (owner, reg, clock)
}
fn original_add_pdn() -> Vec<u8> {
    let line = include_str!("../../native-ims/tests/fixtures/shannon-add-pdn.tsv")
        .lines()
        .find(|s| s.starts_with("empty-pcscf-wire2-fixed-time\t"))
        .unwrap();
    let hex = line.split('\t').nth(5).unwrap();
    let (pairs, tail) = hex.as_bytes().as_chunks::<2>();
    assert!(tail.is_empty());
    let bytes: Vec<_> = pairs
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), 663);
    bytes
}
fn prepare(owner: &mut RadioOwner, reg: &Registration, code: i32) -> PreparedRequest {
    owner
        .prepare_request(
            &reg.key,
            code,
            1,
            original_add_pdn(),
            Deadline::at(Duration::from_nanos(20)),
        )
        .unwrap()
}
fn event(
    serial: i32,
    code: i32,
    slot: i32,
    length: i32,
    error: i32,
    payload: &[u8],
    segmented: bool,
) -> RadioEvent {
    let mut parcel = Parcel::new();
    parcel
        .write(&RadioExternalResponseInfo {
            slotId: slot,
            serial,
            rilcMsgId: code,
            length,
            error,
        })
        .unwrap();
    parcel.write(&payload.to_vec()).unwrap();
    if segmented {
        parcel.write_i32(0).unwrap();
        parcel.write_i32(1).unwrap();
    }
    parcel.set_data_position(0);
    RadioEvent::Response(
        decode_response_arguments(if segmented { 2 } else { 1 }, &mut parcel).unwrap(),
    )
}
fn raw() -> RadioEvent {
    event(1, 105, 1, 3, -17, &[9, 8, 7], false)
}
fn enqueue(reg: &Registration, event: RadioEvent) {
    reg.endpoints.control.queue().deliver(event).unwrap();
}
fn consume(owner: &mut RadioOwner) -> crate::owner::PreparedEvent {
    owner
        .prepare_until(Instant::now() + Duration::from_millis(100))
        .unwrap()
        .unwrap()
}
fn publish(owner: &mut RadioOwner, reg: &Registration, event: RadioEvent) -> u64 {
    enqueue(reg, event);
    let prepared = consume(owner);
    owner.publish(prepared).unwrap()
}
fn no_response(owner: &RadioOwner) {
    let receipt = owner.snapshot().request.unwrap();
    assert_eq!(receipt.outcome, Outcome::OutcomeUnknown);
    assert!(receipt.response.is_none());
}

#[test]
fn exact_raw_add_pdn_response_binds_original_keys_and_owned_bytes_once() {
    let (mut owner, reg, _clock) = setup();
    let request = prepare(&mut owner, &reg, 105);
    let key = request.key();
    request.admit_part().unwrap();
    let mut supplied = vec![9, 8, 7];
    let decoded = event(1, 105, 1, 3, -17, &supplied, false);
    supplied.fill(0xa5);
    drop(supplied);
    let revision = publish(&mut owner, &reg, decoded);
    let receipt = owner.snapshot().request.unwrap();
    assert_eq!(receipt.outcome, Outcome::OutcomeUnknown);
    let observed = receipt
        .response
        .expect("MISSING_ADD_PDN_RESPONSE_ATTRIBUTION");
    assert_eq!(observed.request_key(), &key);
    assert_eq!(observed.session_key(), &reg.key);
    let route = observed.route();
    assert_eq!((route.serial, route.request_id, route.slot_id), (1, 105, 1));
    assert_eq!(observed.vendor_error(), -17);
    assert_eq!(observed.payload(), [9, 8, 7]);
    assert_eq!(observed.received_at(), Duration::from_nanos(10));
    assert!(observed.ingress_sequence() > 0);
    assert_eq!(observed.revision(), revision);
    assert_eq!(receipt.revision, 2);
    assert_eq!(
        owner
            .prepare_request(
                &reg.key,
                105,
                1,
                original_add_pdn(),
                Deadline::at(Duration::from_nanos(20))
            )
            .err(),
        Some(OwnerError::OperationPending)
    );
}
#[test]
fn wrong_serial_route_length_and_oversize_cannot_bind_a_response() {
    let (mut owner, reg, _clock) = setup();
    let request = prepare(&mut owner, &reg, 105);
    request.admit_part().unwrap();
    for e in [
        event(2, 105, 1, 3, 0, &[9, 8, 7], false),
        event(1, 106, 1, 3, 0, &[9, 8, 7], false),
        event(1, 105, 0, 3, 0, &[9, 8, 7], false),
        event(1, 105, 1, 2, 0, &[9, 8, 7], false),
        event(1, 105, 1, -1, 0, &[9, 8, 7], false),
        event(1, 105, 1, 4097, 0, &vec![0x5a; 4097], false),
    ] {
        publish(&mut owner, &reg, e);
        no_response(&owner);
    }
    publish(&mut owner, &reg, raw());
    assert!(owner.snapshot().request.unwrap().response.is_some());
}
#[test]
fn queued_before_dispatch_permission_cannot_become_the_request_response() {
    let (mut owner, reg, _clock) = setup();
    let request = prepare(&mut owner, &reg, 105);
    enqueue(&reg, raw());
    request.admit_part().unwrap();
    let old = consume(&mut owner);
    owner.publish(old).unwrap();
    no_response(&owner);
    publish(&mut owner, &reg, raw());
    assert!(owner.snapshot().request.unwrap().response.is_some());
}
#[test]
fn callback_received_while_queued_is_not_admitted_by_later_dispatch() {
    let (mut owner, reg, _clock) = setup();
    let request = prepare(&mut owner, &reg, 105);
    publish(&mut owner, &reg, raw());
    assert!(owner.snapshot().request.unwrap().response.is_none());
    request.admit_part().unwrap();
    no_response(&owner);
    publish(&mut owner, &reg, raw());
    assert!(owner.snapshot().request.unwrap().response.is_some());
}
#[test]
fn cancellation_expiry_stop_and_death_defeat_an_already_consumed_callback() {
    for action in 0..4 {
        let (mut owner, reg, clock) = setup();
        let request = prepare(&mut owner, &reg, 105);
        request.admit_part().unwrap();
        enqueue(&reg, raw());
        let candidate = consume(&mut owner);
        match action {
            0 => owner.cancel_request(&request.key()).unwrap(),
            1 => clock.0.store(20, Ordering::Release),
            2 => owner.stop(&reg.key).unwrap(),
            _ => reg
                .endpoints
                .control
                .queue()
                .fail(ReceiverFailure::ServiceDied),
        }
        let _ = owner.publish(candidate);
        no_response(&owner);
    }
}
#[test]
fn duplicate_and_later_failure_preserve_the_first_immutable_observation() {
    let (mut owner, reg, _clock) = setup();
    let request = prepare(&mut owner, &reg, 105);
    request.admit_part().unwrap();
    publish(&mut owner, &reg, raw());
    let first = owner.snapshot().request.unwrap();
    let saved = first
        .response
        .clone()
        .expect("MISSING_ADD_PDN_RESPONSE_ATTRIBUTION");
    publish(&mut owner, &reg, event(1, 105, 1, 2, 0, &[0, 0], false));
    let after = owner.snapshot().request.unwrap();
    assert_eq!(after, first);
    assert!(Arc::ptr_eq(after.response.as_ref().unwrap(), &saved));
    owner.cancel_request(&request.key()).unwrap();
    owner.stop(&reg.key).unwrap();
    let stopped = owner.snapshot();
    assert!(stopped.observation.is_none());
    let history = stopped.request.unwrap();
    assert_eq!(history.outcome, Outcome::OutcomeUnknown);
    assert!(Arc::ptr_eq(history.response.as_ref().unwrap(), &saved));
    assert_eq!(saved.payload(), [9, 8, 7]);
    assert_eq!(saved.vendor_error(), -17);
}
#[test]
fn foreign_owner_and_old_dead_session_cannot_bind_another_request() {
    let (mut first, old, _) = setup();
    let old_request = prepare(&mut first, &old, 105);
    old_request.admit_part().unwrap();
    enqueue(&old, raw());
    let old_candidate = consume(&mut first);
    let (mut fresh, new, _) = setup();
    let new_request = prepare(&mut fresh, &new, 105);
    new_request.admit_part().unwrap();
    assert_eq!(fresh.publish(old_candidate), Err(OwnerError::StaleSession));
    no_response(&fresh);
    first.stop(&old.key).unwrap();
    assert!(old.endpoints.control.queue().deliver(raw()).is_err());
    no_response(&fresh);
    publish(&mut fresh, &new, raw());
    let response = fresh.snapshot().request.unwrap().response.unwrap();
    assert_eq!(response.request_key(), &new_request.key());
    assert_ne!(response.request_key(), &old_request.key());
    assert_eq!(response.session_key(), &new.key);
    assert_ne!(response.session_key(), &old.key);
}
#[test]
fn segmented_and_other_request_types_never_create_response_attribution() {
    let (mut owner, reg, _) = setup();
    let request = prepare(&mut owner, &reg, 105);
    request.admit_part().unwrap();
    enqueue(&reg, event(1, 105, 1, 3, 0, &[9, 8, 7], true));
    let prepared = consume(&mut owner);
    assert_eq!(owner.publish(prepared), Err(OwnerError::Unsupported));
    no_response(&owner);
    let (mut owner, reg, _) = setup();
    let request = prepare(&mut owner, &reg, 137);
    request.admit_part().unwrap();
    publish(&mut owner, &reg, event(1, 137, 1, 3, 0, &[9, 8, 7], false));
    no_response(&owner);
}
#[test]
fn matching_wire_error_zero_and_empty_body_are_observations_only() {
    let (mut owner, reg, _) = setup();
    let request = prepare(&mut owner, &reg, 105);
    request.admit_part().unwrap();
    publish(&mut owner, &reg, event(1, 105, 1, 0, 0, &[], false));
    let receipt = owner.snapshot().request.unwrap();
    assert_eq!(receipt.outcome, Outcome::OutcomeUnknown);
    let response = receipt
        .response
        .expect("MISSING_ADD_PDN_RESPONSE_ATTRIBUTION");
    assert_eq!(response.vendor_error(), 0);
    assert!(response.payload().is_empty());
    assert!(matches!(
        owner.snapshot().observation.unwrap().kind,
        Observation::RawResponse(_)
    ));
    assert_eq!(
        owner
            .prepare_request(
                &reg.key,
                105,
                1,
                original_add_pdn(),
                Deadline::at(Duration::from_nanos(20))
            )
            .err(),
        Some(OwnerError::OperationPending)
    );
}
