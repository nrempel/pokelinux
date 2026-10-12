use pokelinux_radio::{
    RadioExternalResponseInfo,
    receiver::{
        CallbackInbox, RESPONSE_DESCRIPTOR, RadioEvent, RadioResponseReceiver, ReceiverFailure,
        new_radio_callbacks,
    },
};
use rsbinder::{INTERFACE_HEADER, Interface, Parcel, Result};
use std::num::NonZeroUsize;

fn response_receiver(
    capacity: NonZeroUsize,
) -> (rsbinder::Binder<RadioResponseReceiver>, CallbackInbox) {
    let callbacks = new_radio_callbacks(capacity);
    (callbacks.response, callbacks.events)
}

fn header(descriptor: &str) -> Parcel {
    let mut parcel = Parcel::new();
    parcel.write_i32(0).unwrap();
    parcel.write_i32(-1).unwrap();
    parcel.write_u32(INTERFACE_HEADER).unwrap();
    parcel.write(descriptor).unwrap();
    parcel
}
fn response(descriptor: &str, extra: bool) -> Parcel {
    let mut parcel = header(descriptor);
    parcel
        .write(&RadioExternalResponseInfo {
            serial: 42,
            ..Default::default()
        })
        .unwrap();
    parcel.write(&vec![1_u8, 2, 3]).unwrap();
    if extra {
        parcel.write_i32(123).unwrap();
    }
    parcel
}
fn transact(binder: &impl Interface, code: u32, parcel: &mut Parcel) -> Result<Parcel> {
    rsbinder::ProcessState::init("/dev/binder", 0).unwrap();
    let handle = binder.as_binder();
    let mut reply = Parcel::new();
    handle
        .as_transactable()
        .unwrap()
        .transact(code, parcel, &mut reply)?;
    reply.set_data_position(0);
    Ok(reply)
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn binder_wrapper_checks_token_before_delivery() {
    let (binder, events) = response_receiver(NonZeroUsize::new(1).unwrap());
    assert!(transact(&binder, 1, &mut response("wrong.interface", false)).is_err());
    assert!(events.try_recv().unwrap().is_none());
    assert_eq!(binder.failure(), None);
    transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, false)).unwrap();
    let RadioEvent::Response(event) = events.try_recv().unwrap().unwrap() else {
        panic!("wrong event")
    };
    assert_eq!(event.info.serial, 42);
    assert_eq!(event.payload, [1, 2, 3]);
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn malformed_known_call_latches_failure_without_delivery() {
    let (binder, events) = response_receiver(NonZeroUsize::new(1).unwrap());
    assert!(transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, true)).is_err());
    assert_eq!(binder.failure(), Some(ReceiverFailure::MalformedCallback));
    assert!(events.try_recv().is_err());
    assert!(transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, false)).is_err());
    assert!(events.try_recv().is_err());
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn full_queue_is_terminal_and_does_not_deliver_buffered_events() {
    let (binder, events) = response_receiver(NonZeroUsize::new(1).unwrap());
    transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, false)).unwrap();
    assert!(transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, false)).is_err());
    assert_eq!(binder.failure(), Some(ReceiverFailure::QueueFull));
    assert!(matches!(events.try_recv(), Err(ReceiverFailure::QueueFull)));
    assert!(transact(&binder, 1, &mut response(RESPONSE_DESCRIPTOR, false)).is_err());
    assert!(events.try_recv().is_err());
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn disconnected_consumer_latches_failure() {
    let callbacks = new_radio_callbacks(NonZeroUsize::new(1).unwrap());
    drop(callbacks.events);
    assert_eq!(
        callbacks.response.failure(),
        Some(ReceiverFailure::ConsumerGone)
    );
    assert_eq!(
        callbacks.indication.failure(),
        Some(ReceiverFailure::ConsumerGone)
    );
    callbacks.control.cancel();
    assert!(
        transact(
            &callbacks.response,
            1,
            &mut response(RESPONSE_DESCRIPTOR, false)
        )
        .is_err()
    );
    assert_eq!(
        callbacks.response.failure(),
        Some(ReceiverFailure::ConsumerGone)
    );
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn metadata_calls_match_original_interface_without_queueing_events() {
    let (binder, events) = response_receiver(NonZeroUsize::new(1).unwrap());
    let mut version = transact(&binder, 16_777_215, &mut header(RESPONSE_DESCRIPTOR)).unwrap();
    assert_eq!(version.read_i32().unwrap(), 0);
    assert_eq!(version.read_i32().unwrap(), 1);
    let mut hash = transact(&binder, 16_777_214, &mut header(RESPONSE_DESCRIPTOR)).unwrap();
    assert_eq!(hash.read_i32().unwrap(), 0);
    assert_eq!(
        hash.read::<String>().unwrap(),
        "62c86cf5f8f15456f451d2241b4f21b10627e514"
    );
    assert!(events.try_recv().unwrap().is_none());
    assert!(transact(&binder, 99, &mut header(RESPONSE_DESCRIPTOR)).is_err());
    assert_eq!(binder.failure(), None);
}

#[test]
#[ignore = "Requires the owned Android emulator Binder driver"]
fn indication_overflow_invalidates_the_shared_callback_stream() {
    let callbacks = new_radio_callbacks(NonZeroUsize::new(1).unwrap());
    transact(
        &callbacks.response,
        1,
        &mut response(RESPONSE_DESCRIPTOR, false),
    )
    .unwrap();
    let mut input = header(pokelinux_radio::transport::INDICATION_DESCRIPTOR);
    input.write_i32(33).unwrap();
    input.write_i32(0).unwrap();
    input.write(&vec![1_u8]).unwrap();
    assert!(transact(&callbacks.indication, 1, &mut input).is_err());
    assert_eq!(
        callbacks.response.failure(),
        Some(ReceiverFailure::QueueFull)
    );
    assert_eq!(
        callbacks.indication.failure(),
        Some(ReceiverFailure::QueueFull)
    );
    assert!(matches!(
        callbacks.events.try_recv(),
        Err(ReceiverFailure::QueueFull)
    ));
}
