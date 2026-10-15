//! Publication fences for the original-qualified complete registration route.
use super::*;
use crate::indication::{IndicationKind, RadioIndication};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::SeqCst))
    }
}
fn setup() -> (RadioOwner, Arc<TestClock>, SessionKey, Arc<CallbackQueue>) {
    let clock = Arc::new(TestClock(AtomicU64::new(10)));
    let mut owner = RadioOwner::with_clock(NonZeroUsize::new(4).unwrap(), 1, clock.clone());
    let (key, queue) = owner
        .begin_queue(
            None,
            NonZeroUsize::new(1).unwrap(),
            Deadline::at(Duration::from_nanos(30)),
        )
        .unwrap();
    owner.activate(&key, 42).unwrap();
    (owner, clock, key, queue)
}
fn payload() -> Vec<u8> {
    let mut bytes = vec![255, 128];
    bytes.extend(i32::MIN.to_le_bytes());
    bytes.extend(u16::MAX.to_le_bytes());
    bytes.extend(3u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend([0xff, 0, 65, 0xc0, 0]);
    bytes
}
fn indication(slot: i32, id: i32, kind: IndicationKind, bytes: Vec<u8>) -> RadioEvent {
    RadioEvent::Indication(RadioIndication {
        message_id: id,
        slot_id: slot,
        payload: bytes,
        kind,
    })
}
fn prepare(owner: &mut RadioOwner, queue: &CallbackQueue) -> PreparedEvent {
    queue
        .deliver(indication(1, 3012, IndicationKind::Raw, payload()))
        .unwrap();
    owner.prepare_until(Instant::now()).unwrap().unwrap()
}
fn seed(owner: &mut RadioOwner, queue: &CallbackQueue) -> Arc<CommittedObservation> {
    queue
        .deliver(RadioEvent::Response(RadioResponse {
            info: crate::RadioExternalResponseInfo::default(),
            payload: vec![9],
            kind: ResponseKind::Raw,
        }))
        .unwrap();
    let event = owner.prepare_until(Instant::now()).unwrap().unwrap();
    owner.publish(event).unwrap();
    owner.snapshot().observation.unwrap()
}
#[test]
fn full_wire_observation_preserves_unknown_values_and_opaque_bytes_at_selected_slot() {
    let (mut owner, _, _, queue) = setup();
    let event = prepare(&mut owner, &queue);
    let revision = owner.publish(event).unwrap();
    let snapshot = owner.snapshot();
    assert_eq!(snapshot.phase, Phase::Active { client_id: 42 });
    let observed = snapshot.observation.unwrap();
    assert_eq!(observed.revision, revision);
    let Observation::RegistrationWire {
        slot_id,
        message_id,
        wire,
    } = &observed.kind
    else {
        panic!("complete registration indication was not committed");
    };
    assert_eq!((*slot_id, *message_id), (1, 3012));
    assert_eq!(
        (
            wire.registration_state_wire,
            wire.feature_wire,
            wire.feature_mask_wire,
            wire.status_code_wire
        ),
        (255, 128, i32::MIN, u16::MAX)
    );
    assert_eq!(wire.uri_bytes(), [0xff, 0, 65]);
    assert_eq!(wire.reason_bytes(), [0xc0, 0]);
    assert_eq!(
        (
            wire.feature_mask_ext_wire,
            wire.sip_delegate_mask_wire,
            wire.feature_ext_wire
        ),
        (None, None, None)
    );
}
#[test]
fn unselected_raw_and_segmented_indications_cannot_replace_response_history() {
    let (mut owner, _, _, queue) = setup();
    let previous = seed(&mut owner, &queue);
    for event in [
        indication(0, 3012, IndicationKind::Raw, payload()),
        indication(1, 3013, IndicationKind::Raw, payload()),
        indication(
            1,
            3012,
            IndicationKind::Segment {
                index: 0,
                total_length: 17,
            },
            payload(),
        ),
    ] {
        queue.deliver(event).unwrap();
        let event = owner.prepare_until(Instant::now()).unwrap().unwrap();
        assert_eq!(owner.publish(event), Err(OwnerError::Unsupported));
        let snapshot = owner.snapshot();
        assert_eq!(snapshot.revision, previous.revision);
        assert!(Arc::ptr_eq(&snapshot.observation.unwrap(), &previous));
    }
}
#[test]
fn incomplete_or_trailing_selected_payload_cannot_publish_partial_fields() {
    let (mut owner, _, _, queue) = setup();
    let previous = seed(&mut owner, &queue);
    let bytes = payload();
    let trailing = [bytes.as_slice(), &[0x7e]].concat();
    for bytes in [vec![], bytes[..11].to_vec(), bytes[..16].to_vec(), trailing] {
        queue
            .deliver(indication(1, 3012, IndicationKind::Raw, bytes))
            .unwrap();
        let event = owner.prepare_until(Instant::now()).unwrap().unwrap();
        assert!(owner.publish(event).is_err());
        let snapshot = owner.snapshot();
        assert_eq!(snapshot.revision, previous.revision);
        assert_eq!(snapshot.phase, Phase::Active { client_id: 42 });
        assert!(Arc::ptr_eq(&snapshot.observation.unwrap(), &previous));
    }
}
#[test]
fn prepared_registration_cannot_publish_after_deadline_without_timer_delivery() {
    let (mut owner, clock, _, queue) = setup();
    let event = prepare(&mut owner, &queue);
    clock.0.store(30, Ordering::SeqCst);
    assert_eq!(
        owner.publish(event),
        Err(OwnerError::Failed(ReceiverFailure::Expired))
    );
    assert!(owner.snapshot().observation.is_none());
}
#[test]
fn prepared_registration_cannot_publish_after_cancel_or_peer_loss() {
    for reason in [ReceiverFailure::Cancelled, ReceiverFailure::ServiceDied] {
        let (mut owner, _, _, queue) = setup();
        let event = prepare(&mut owner, &queue);
        queue.fail(reason);
        let revision = owner.snapshot().revision;
        assert_eq!(owner.publish(event), Err(OwnerError::Failed(reason)));
        assert!(owner.snapshot().observation.is_none());
        assert_eq!(owner.snapshot().revision, revision);
    }
}
#[test]
fn prepared_registration_cannot_cross_owner_or_replacement() {
    let (mut owner, _, old, queue) = setup();
    let event = prepare(&mut owner, &queue);
    let (mut other, _, _, _) = setup();
    assert_eq!(other.publish(event), Err(OwnerError::StaleSession));
    assert!(other.snapshot().observation.is_none());
    let event = prepare(&mut owner, &queue);
    let (fresh, _fresh_queue) = owner
        .begin_queue(
            Some(&old),
            NonZeroUsize::new(1).unwrap(),
            Deadline::at(Duration::from_nanos(30)),
        )
        .unwrap();
    owner.activate(&fresh, 42).unwrap();
    assert_eq!(owner.publish(event), Err(OwnerError::StaleSession));
    assert!(owner.snapshot().observation.is_none());
}
#[test]
fn prepared_registration_cannot_publish_after_callback_queue_loss() {
    let (mut owner, _, _, queue) = setup();
    let event = prepare(&mut owner, &queue);
    queue
        .deliver(indication(1, 3012, IndicationKind::Raw, payload()))
        .unwrap();
    assert!(
        queue
            .deliver(indication(1, 3012, IndicationKind::Raw, payload()))
            .is_err()
    );
    assert_eq!(
        owner.publish(event),
        Err(OwnerError::Failed(ReceiverFailure::QueueFull))
    );
    assert!(owner.snapshot().observation.is_none());
}
