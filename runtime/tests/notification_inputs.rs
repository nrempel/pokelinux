#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    notification::{Completion, Notifications},
    notification_inputs::presentation_channel,
    observer::{Lease, Observer},
    watch::decode_snapshot,
};
use std::time::Duration;
fn sample(observer: &mut Observer, t: u64) -> Option<Lease> {
    let pending = observer.begin(Duration::from_secs(t));
    let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":pending.query().id(),"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,"source":"android.telecom.InCallService","source_state":"observed","ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]})).unwrap();
    let snapshot = decode_snapshot(&bytes, pending.query().id()).unwrap();
    observer
        .complete(pending, snapshot, Duration::from_secs(t))
        .unwrap();
    observer.lease(Duration::from_secs(t)).cloned()
}
#[test]
fn coalesced_permission_loss_still_revokes_an_older_request() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    let lease = sample(&mut observer, 11).unwrap();
    let (mut publisher, mut inputs) = presentation_channel();
    let mut guard = Notifications::default();
    publisher.publish(Some(lease.clone()), true).unwrap();
    let first = async_io::block_on(inputs.receive(&mut guard)).unwrap();
    let request = guard
        .begin(
            ":1.7",
            first.lease().unwrap(),
            first.allowed(),
            Duration::from_secs(11),
        )
        .unwrap();
    publisher.publish(Some(lease.clone()), false).unwrap();
    publisher.publish(Some(lease), true).unwrap();
    let latest = async_io::block_on(inputs.receive(&mut guard)).unwrap();
    assert!(latest.allowed());
    assert!(matches!(
        guard.complete(
            request,
            7,
            Some(":1.7"),
            latest.lease(),
            latest.allowed(),
            Duration::from_secs(12)
        ),
        Completion::Close(_)
    ));
}
#[test]
fn many_healthy_renewals_coalesce_without_revoking_the_same_ringing_lifetime() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    let lease = sample(&mut observer, 11).unwrap();
    let (mut publisher, mut inputs) = presentation_channel();
    let mut guard = Notifications::default();
    publisher.publish(Some(lease), true).unwrap();
    let first = async_io::block_on(inputs.receive(&mut guard)).unwrap();
    let request = guard
        .begin(
            ":1.7",
            first.lease().unwrap(),
            true,
            Duration::from_secs(11),
        )
        .unwrap();
    let renewed = sample(&mut observer, 12).unwrap();
    for _ in 0..10000 {
        publisher.publish(Some(renewed.clone()), true).unwrap();
    }
    let latest = async_io::block_on(inputs.receive(&mut guard)).unwrap();
    assert_eq!(latest.lease().unwrap().id(), renewed.id());
    assert!(matches!(
        guard.complete(
            request,
            7,
            Some(":1.7"),
            latest.lease(),
            true,
            Duration::from_secs(12)
        ),
        Completion::Visible(_)
    ));
    drop(publisher);
    assert!(async_io::block_on(inputs.receive(&mut guard)).is_err());
}
#[test]
fn producer_loss_revokes_pending_work_and_stops_new_updates() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    let lease = sample(&mut observer, 11).unwrap();
    let (mut publisher, mut inputs) = presentation_channel();
    let mut guard = Notifications::default();
    publisher.publish(Some(lease), true).unwrap();
    let latest = async_io::block_on(inputs.receive(&mut guard)).unwrap();
    let request = guard
        .begin(
            ":1.7",
            latest.lease().unwrap(),
            true,
            Duration::from_secs(11),
        )
        .unwrap();
    drop(publisher);
    assert!(async_io::block_on(inputs.receive(&mut guard)).is_err());
    assert!(matches!(
        guard.complete(
            request,
            7,
            Some(":1.7"),
            latest.lease(),
            true,
            Duration::from_secs(12)
        ),
        Completion::Close(_)
    ));
}

#[test]
fn queued_state_from_a_closed_producer_is_not_adopted() {
    let mut observer = Observer::default();
    sample(&mut observer, 10);
    let lease = sample(&mut observer, 11).unwrap();
    let (mut publisher, mut inputs) = presentation_channel();
    publisher.publish(Some(lease), true).unwrap();
    drop(publisher);
    assert!(async_io::block_on(inputs.receive(&mut Notifications::default())).is_err());
}

#[test]
fn publishing_after_consumer_loss_fails_without_waiting() {
    let (mut publisher, inputs) = presentation_channel();
    drop(inputs);
    assert!(publisher.publish(None, false).is_err());
}
