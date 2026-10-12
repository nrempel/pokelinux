//! Native response Binder endpoint. Queue failure is terminal for this endpoint;
//! its owner must fail the associated session rather than continue after loss.
use crate::response::{RadioResponse, decode_response_arguments};
use rsbinder::{Binder, Parcel, Remotable, Result, Stability, StatusCode};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
};

pub const RESPONSE_DESCRIPTOR: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalRes";
const INTERFACE_HASH: &str = "62c86cf5f8f15456f451d2241b4f21b10627e514";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ReceiverFailure {
    MalformedResponse = 1,
    QueueFull = 2,
    ConsumerGone = 3,
    InternalFailure = 4,
    PublisherGone = 5,
}
fn failure(state: &AtomicU8) -> Option<ReceiverFailure> {
    match state.load(Ordering::Acquire) {
        0 => None,
        1 => Some(ReceiverFailure::MalformedResponse),
        2 => Some(ReceiverFailure::QueueFull),
        3 => Some(ReceiverFailure::ConsumerGone),
        5 => Some(ReceiverFailure::PublisherGone),
        _ => Some(ReceiverFailure::InternalFailure),
    }
}

pub struct RadioResponseReceiver {
    sender: Mutex<Option<SyncSender<RadioResponse>>>,
    failure: Arc<AtomicU8>,
}
pub struct ResponseInbox {
    receiver: Receiver<RadioResponse>,
    failure: Arc<AtomicU8>,
}
impl ResponseInbox {
    /// Never returns buffered responses once endpoint failure is observed.
    pub fn try_recv(&self) -> std::result::Result<Option<RadioResponse>, ReceiverFailure> {
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
impl RadioResponseReceiver {
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
    fn deliver(&self, event: RadioResponse) -> Result<()> {
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
impl Remotable for RadioResponseReceiver {
    fn descriptor() -> &'static str {
        RESPONSE_DESCRIPTOR
    }
    fn on_transact(&self, code: u32, input: &mut Parcel, reply: &mut Parcel) -> Result<()> {
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
                match decode_response_arguments(code, input) {
                    Ok(event) => self.deliver(event),
                    Err(error) => {
                        self.fail(ReceiverFailure::MalformedResponse);
                        Err(error)
                    }
                }
            }
            _ => Err(StatusCode::UnknownTransaction),
        }
    }
    fn on_dump(&self, _writer: &mut dyn std::io::Write, _args: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}

pub fn new_response_receiver(
    capacity: NonZeroUsize,
) -> (Binder<RadioResponseReceiver>, ResponseInbox) {
    let (sender, receiver) = mpsc::sync_channel(capacity.get());
    let failure = Arc::new(AtomicU8::new(0));
    let binder = Binder::new_with_stability(
        RadioResponseReceiver {
            sender: Mutex::new(Some(sender)),
            failure: failure.clone(),
        },
        Stability::Vintf,
    );
    (binder, ResponseInbox { receiver, failure })
}
