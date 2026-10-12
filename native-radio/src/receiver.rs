//! Native response and indication endpoints share one terminal event stream;
//! its owner must fail the associated session rather than continue after loss.
use crate::{
    indication::{RadioIndication, decode_indication_arguments},
    response::{RadioResponse, decode_response_arguments},
    transport::INDICATION_DESCRIPTOR,
};
use rsbinder::{Binder, Parcel, Remotable, Result, Stability, StatusCode};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    time::Instant,
};

pub const RESPONSE_DESCRIPTOR: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalRes";
const INTERFACE_HASH: &str = "62c86cf5f8f15456f451d2241b4f21b10627e514";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ReceiverFailure {
    MalformedCallback = 1,
    QueueFull = 2,
    ConsumerGone = 3,
    InternalFailure = 4,
    PublisherGone = 5,
    Cancelled = 6,
    ServiceDied = 7,
    MonitoringFailed = 8,
}
fn failure(state: &AtomicU8) -> Option<ReceiverFailure> {
    match state.load(Ordering::Acquire) {
        0 => None,
        1 => Some(ReceiverFailure::MalformedCallback),
        2 => Some(ReceiverFailure::QueueFull),
        3 => Some(ReceiverFailure::ConsumerGone),
        5 => Some(ReceiverFailure::PublisherGone),
        6 => Some(ReceiverFailure::Cancelled),
        7 => Some(ReceiverFailure::ServiceDied),
        8 => Some(ReceiverFailure::MonitoringFailed),
        _ => Some(ReceiverFailure::InternalFailure),
    }
}

struct CallbackQueue {
    sender: Mutex<Option<SyncSender<RadioEvent>>>,
    failure: Arc<AtomicU8>,
}
pub struct CallbackInbox {
    receiver: Receiver<RadioEvent>,
    failure: Arc<AtomicU8>,
}
impl CallbackInbox {
    /// Wait without polling. Terminal failure closes the shared sender, waking
    /// this wait even when no event can be enqueued. The deadline bounds waiting,
    /// not event age or downstream publication; those remain owner policy.
    pub fn recv_until(
        &self,
        deadline: Instant,
    ) -> std::result::Result<Option<RadioEvent>, ReceiverFailure> {
        if let Some(error) = failure(&self.failure) {
            return Err(error);
        }
        let result = match self
            .receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(event) => Ok(Some(event)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(ReceiverFailure::PublisherGone),
        };
        if let Some(error) = failure(&self.failure) {
            return Err(error);
        }
        result
    }
    /// Never returns buffered responses once endpoint failure is observed.
    pub fn try_recv(&self) -> std::result::Result<Option<RadioEvent>, ReceiverFailure> {
        if let Some(error) = failure(&self.failure) {
            return Err(error);
        }
        let result = match self.receiver.try_recv() {
            Ok(response) => Ok(Some(response)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ReceiverFailure::PublisherGone),
        };
        if let Some(error) = failure(&self.failure) {
            return Err(error);
        }
        result
    }
}
impl CallbackQueue {
    pub fn failure(&self) -> Option<ReceiverFailure> {
        failure(&self.failure)
    }
    fn fail(&self, reason: ReceiverFailure) {
        let mut sender = self
            .sender
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.failure
            .compare_exchange(0, reason as u8, Ordering::AcqRel, Ordering::Acquire)
            .ok();
        sender.take();
    }
    fn deliver(&self, event: RadioEvent) -> Result<()> {
        let mut sender = match self.sender.lock() {
            Ok(sender) => sender,
            Err(poison) => {
                self.failure
                    .store(ReceiverFailure::InternalFailure as u8, Ordering::Release);
                poison.into_inner().take();
                return Err(StatusCode::FailedTransaction);
            }
        };
        if self.failure().is_some() {
            return Err(StatusCode::FailedTransaction);
        }
        let Some(active) = sender.as_ref() else {
            return Err(StatusCode::DeadObject);
        };
        match active.try_send(event) {
            Ok(()) => Ok(()),
            Err(error) => {
                let reason = match error {
                    TrySendError::Full(_) => ReceiverFailure::QueueFull,
                    TrySendError::Disconnected(_) => ReceiverFailure::ConsumerGone,
                };
                self.failure.store(reason as u8, Ordering::Release);
                sender.take();
                Err(StatusCode::FailedTransaction)
            }
        }
    }
}
impl CallbackQueue {
    fn dispatch(
        &self,
        code: u32,
        input: &mut Parcel,
        reply: &mut Parcel,
        decode: fn(u32, &mut Parcel) -> Result<RadioEvent>,
    ) -> Result<()> {
        match code {
            16_777_215 | 16_777_214 => {
                if input.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                reply.write_i32(0)?; // Android's successful exception header.
                if code == 16_777_215 {
                    reply.write_i32(1)
                } else {
                    reply.write(INTERFACE_HASH)
                }
            }
            1 | 2 => {
                if self.failure().is_some() {
                    return Err(StatusCode::FailedTransaction);
                }
                match decode(code, input) {
                    Ok(event) => self.deliver(event),
                    Err(error) => {
                        self.fail(ReceiverFailure::MalformedCallback);
                        Err(error)
                    }
                }
            }
            _ => Err(StatusCode::UnknownTransaction),
        }
    }
}

#[derive(Debug)]
pub enum RadioEvent {
    Response(RadioResponse),
    Indication(RadioIndication),
}
pub struct RadioResponseReceiver(Arc<CallbackQueue>);
pub struct RadioIndicationReceiver(Arc<CallbackQueue>);
fn response_event(code: u32, input: &mut Parcel) -> Result<RadioEvent> {
    decode_response_arguments(code, input).map(RadioEvent::Response)
}
fn indication_event(code: u32, input: &mut Parcel) -> Result<RadioEvent> {
    decode_indication_arguments(code, input).map(RadioEvent::Indication)
}
macro_rules! endpoint {
    ($receiver:ty, $descriptor:expr, $decoder:expr) => {
        impl $receiver {
            pub fn failure(&self) -> Option<ReceiverFailure> {
                self.0.failure()
            }
        }
        impl Remotable for $receiver {
            fn descriptor() -> &'static str {
                $descriptor
            }
            fn on_transact(&self, code: u32, input: &mut Parcel, reply: &mut Parcel) -> Result<()> {
                self.0.dispatch(code, input, reply, $decoder)
            }
            fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
                Err(StatusCode::InvalidOperation)
            }
        }
    };
}
endpoint!(RadioResponseReceiver, RESPONSE_DESCRIPTOR, response_event);
endpoint!(
    RadioIndicationReceiver,
    INDICATION_DESCRIPTOR,
    indication_event
);

pub struct RadioCallbacks {
    pub response: Binder<RadioResponseReceiver>,
    pub indication: Binder<RadioIndicationReceiver>,
    pub events: CallbackInbox,
    pub control: CallbackControl,
}
/// Local cancellation only: this does not cancel an in-flight remote Binder call.
/// The owner must cancel explicitly; dropping one clone does not cancel others.
#[derive(Clone)]
pub struct CallbackControl(Arc<CallbackQueue>);
impl CallbackControl {
    pub(crate) fn fail(&self, reason: ReceiverFailure) {
        self.0.fail(reason);
    }
    pub fn cancel(&self) {
        self.0.fail(ReceiverFailure::Cancelled);
    }
}
/// One bounded stream for one registration. On terminal failure the owner must
/// fail that session and create fresh endpoints; neither endpoint can reset it.
pub fn new_radio_callbacks(capacity: NonZeroUsize) -> RadioCallbacks {
    let (sender, receiver) = mpsc::sync_channel(capacity.get());
    let failure = Arc::new(AtomicU8::new(0));
    let shared = Arc::new(CallbackQueue {
        sender: Mutex::new(Some(sender)),
        failure: failure.clone(),
    });
    RadioCallbacks {
        control: CallbackControl(shared.clone()),
        response: Binder::new_with_stability(
            RadioResponseReceiver(shared.clone()),
            Stability::Vintf,
        ),
        indication: Binder::new_with_stability(RadioIndicationReceiver(shared), Stability::Vintf),
        events: CallbackInbox { receiver, failure },
    }
}

#[cfg(test)]
mod wait_tests {
    use super::*;
    use std::{
        thread,
        time::{Duration, Instant},
    };
    fn stream() -> (Arc<CallbackQueue>, CallbackInbox) {
        let (sender, receiver) = mpsc::sync_channel(1);
        let failure = Arc::new(AtomicU8::new(0));
        (
            Arc::new(CallbackQueue {
                sender: Mutex::new(Some(sender)),
                failure: failure.clone(),
            }),
            CallbackInbox { receiver, failure },
        )
    }
    #[test]
    fn cancellation_is_terminal_and_preserves_an_earlier_failure() {
        let (queue, inbox) = stream();
        let control = CallbackControl(queue.clone());
        control.cancel();
        control.cancel();
        assert!(matches!(
            inbox.recv_until(Instant::now() + Duration::from_secs(30)),
            Err(ReceiverFailure::Cancelled)
        ));
        assert_eq!(queue.failure(), Some(ReceiverFailure::Cancelled));
        let (queue, inbox) = stream();
        queue.fail(ReceiverFailure::MalformedCallback);
        CallbackControl(queue).cancel();
        assert!(matches!(
            inbox.recv_until(Instant::now()),
            Err(ReceiverFailure::MalformedCallback)
        ));
    }
    #[test]
    fn idle_wait_expires_without_invalidating_stream() {
        let (queue, inbox) = stream();
        assert!(
            inbox
                .recv_until(Instant::now() + Duration::from_millis(5))
                .unwrap()
                .is_none()
        );
        assert_eq!(queue.failure(), None);
        queue.fail(ReceiverFailure::MalformedCallback);
        assert!(matches!(
            inbox.recv_until(Instant::now()),
            Err(ReceiverFailure::MalformedCallback)
        ));
    }
    #[test]
    fn terminal_failure_wakes_owner_before_its_deadline() {
        let (queue, inbox) = stream();
        let (started, ready) = mpsc::channel();
        let (done, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            started.send(()).unwrap();
            done.send(inbox.recv_until(Instant::now() + Duration::from_secs(30)))
                .unwrap();
        });
        ready.recv().unwrap();
        queue.fail(ReceiverFailure::MalformedCallback);
        assert!(matches!(
            result.recv_timeout(Duration::from_secs(2)).unwrap(),
            Err(ReceiverFailure::MalformedCallback)
        ));
        worker.join().unwrap();
    }
    #[test]
    fn publisher_loss_wakes_owner_and_buffered_events_cannot_hide_failure() {
        let (queue, inbox) = stream();
        drop(queue);
        assert!(matches!(
            inbox.recv_until(Instant::now() + Duration::from_secs(30)),
            Err(ReceiverFailure::PublisherGone)
        ));
        let (queue, inbox) = stream();
        queue
            .deliver(RadioEvent::Indication(RadioIndication {
                message_id: 1,
                slot_id: 0,
                payload: vec![1],
                kind: crate::indication::IndicationKind::Raw,
            }))
            .unwrap();
        queue.fail(ReceiverFailure::QueueFull);
        assert!(matches!(
            inbox.recv_until(Instant::now() + Duration::from_secs(30)),
            Err(ReceiverFailure::QueueFull)
        ));
    }
}
