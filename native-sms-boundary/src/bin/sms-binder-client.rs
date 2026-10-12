//! Exact public V3 NDK endpoint over real Binder; original Rust outbox ownership.
use pokelinux_messaging::{
    outbox::{Outbox, PartState},
    transport::{AttemptId, EndpointId, Outcome, Request, Signal, Source, SubmissionOwner},
};
use pokelinux_sms_boundary::*;
use rsbinder::{
    Binder, FromIBinder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status,
    StatusCode, Strong, hub,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SyncSender},
    },
    time::Duration,
};
const PACKAGE: &str = "org.pokelinux.smspeer";
const COMPONENT: &str = "org.pokelinux.smspeer.Peer";
const DESCRIPTOR: &str = "org.pokelinux.smstest.ISmsBoundaryProbe";
struct Watcher(SyncSender<Result<SIBinder>>);
fn peer_from_status(data: &mut Parcel) -> Result<SIBinder> {
    if data.read_i32()? != 1
        || data.read::<String>()? != PACKAGE
        || data.read::<String>()? != COMPONENT
        || data.read_i32()? != 37
        || data.read_i32()? != 1
    {
        return Err(StatusCode::BadValue);
    }
    let length = usize::try_from(data.read_i32()?).map_err(|_| StatusCode::BadValue)?;
    if length > data.data_avail() || data.read_i32()? != 0x4c444e42 {
        return Err(StatusCode::BadValue);
    }
    let begin = data.data_position();
    if data.read_i32()? != 1 || data.read::<String>()? != "peer" || data.read_i32()? != 15 {
        return Err(StatusCode::BadValue);
    }
    let peer: SIBinder = data.read()?;
    if data.data_position() - begin != length || data.read_i32()? != 0 || data.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    if !peer.is_remote() || peer.descriptor() != DESCRIPTOR {
        return Err(StatusCode::BadType);
    }
    Ok(peer)
}
impl Remotable for Watcher {
    fn descriptor() -> &'static str {
        "android.app.IInstrumentationWatcher"
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        if code != 1 {
            return Err(StatusCode::UnknownTransaction);
        }
        self.0
            .try_send(peer_from_status(data))
            .map_err(|_| StatusCode::FailedTransaction)?;
        reply.write_i32(0)
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn bootstrap() -> std::result::Result<SIBinder, Box<dyn std::error::Error>> {
    ProcessState::init("/dev/binder", 2)?;
    ProcessState::start_thread_pool();
    let activity = hub::try_get_service("activity")?.ok_or("activity absent")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let watcher = Binder::new(Watcher(sender));
    let mut data = activity
        .as_remote()
        .ok_or("activity is local")?
        .prepare_transact(true)?;
    data.write_i32(1)?;
    data.write(PACKAGE)?;
    data.write(COMPONENT)?;
    data.write(&None::<String>)?;
    data.write_i32(0)?;
    data.write_i32(0)?;
    data.write(&watcher.as_binder())?;
    data.write(&None::<SIBinder>)?;
    data.write_i32(0)?;
    data.write(&None::<String>)?;
    let mut reply = activity
        .as_remote()
        .ok_or("activity is local")?
        .submit_transact(51, &data, 0)?
        .ok_or("no start reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    if reply.read_i32()? != 1 || reply.data_avail() != 0 {
        return Err("instrumentation refused".into());
    }
    let peer = receiver.recv_timeout(Duration::from_secs(5))??;
    peer.ping_binder()?;

    Ok(peer)
}

#[derive(Clone, Copy)]
struct Event {
    source: Source,
    attempt: Option<AttemptId>,
    signal: Signal,
}
struct State {
    owner: Mutex<SubmissionOwner>,
    events: SyncSender<Event>,
}
struct Response {
    source: Source,
    mapping: Arc<Mutex<HashMap<i32, AttemptId>>>,
    state: Arc<State>,
}
impl Interface for Response {}
impl Response {
    fn queue(&self, serial: Option<i32>, signal: Signal) -> rsbinder::BinderResult<()> {
        let attempt = serial.and_then(|n| self.mapping.lock().unwrap().get(&n).copied());
        if self
            .state
            .events
            .try_send(Event {
                source: self.source,
                attempt,
                signal,
            })
            .is_err()
        {
            self.state.owner.lock().unwrap().revoke(self.source);
            return Err(StatusCode::FailedTransaction.into());
        }
        Ok(())
    }
}
impl IRadioMessagingResponse for Response {
    fn sendSmsResponse(
        &self,
        info: &RadioResponseInfo,
        sms: &SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        if info.r#type != RadioResponseType::SOLICITED
            || !(0..=255).contains(&sms.messageRef)
            || sms.ackPDU.len() > 488
            || !sms.ackPDU.len().is_multiple_of(2)
            || !sms.ackPDU.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return self.queue(None, Signal::RequestAcknowledged);
        }
        let signal = if info.error == RadioError::NONE && sms.errorCode == -1 {
            Signal::SendAccepted
        } else {
            Signal::Uncertain
        };
        self.queue(Some(info.serial), signal)
    }
    fn acknowledgeRequest(&self, serial: i32) -> rsbinder::BinderResult<()> {
        self.queue(Some(serial), Signal::RequestAcknowledged)
    }
    fn r#acknowledgeIncomingGsmSmsWithPduResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#acknowledgeLastIncomingCdmaSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#acknowledgeLastIncomingGsmSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#deleteSmsOnRuimResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#deleteSmsOnSimResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#getCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[pokelinux_sms_boundary::generated::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo::CdmaBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#getGsmBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[pokelinux_sms_boundary::generated::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo::GsmBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#getSmscAddressResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_smsc: &str,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#reportSmsMemoryStatusResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#sendCdmaSmsExpectMoreResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#sendCdmaSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#sendImsSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#sendSmsExpectMoreResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#setCdmaBroadcastActivationResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#setCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#setGsmBroadcastActivationResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#setGsmBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#setSmscAddressResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#writeSmsToRuimResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
    fn r#writeSmsToSimResponse(
        &self,
        _arg_info: &pokelinux_sms_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, Signal::RequestAcknowledged)
    }
}
struct Indication;
impl Interface for Indication {}
impl IRadioMessagingIndication for Indication {
    fn r#cdmaNewSms(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_msg: &pokelinux_sms_boundary::generated::android::hardware::radio::messaging::CdmaSmsMessage::CdmaSmsMessage,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#cdmaRuimSmsStorageFull(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newBroadcastSms(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_data: &[u8],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSms(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSmsOnSim(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_record_number: i32,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSmsStatusReport(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#simSmsStorageFull(
        &self,
        _arg_type: pokelinux_sms_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
}
struct Death {
    source: Source,
    state: Arc<State>,
    notice: SyncSender<()>,
}
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.state.owner.lock().unwrap().revoke(self.source);
        let _ = self.notice.try_send(());
    }
}
struct Session {
    source: Source,
    endpoint: Strong<dyn IRadioMessaging>,
    mapping: Arc<Mutex<HashMap<i32, AttemptId>>>,
    state: Arc<State>,
    death: Arc<Death>,
    _callback: Strong<dyn IRadioMessagingResponse>,
}
impl Session {
    fn new(
        endpoint: Strong<dyn IRadioMessaging>,
        state: Arc<State>,
        number: u64,
        notice: SyncSender<()>,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        assert!(endpoint.as_binder().is_remote());
        assert_eq!(endpoint.getInterfaceVersion()?, 3);
        assert_eq!(
            endpoint.getInterfaceHash()?,
            "30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb"
        );
        let source = state
            .owner
            .lock()
            .unwrap()
            .attach(EndpointId(number))
            .map_err(|e| format!("{e:?}"))?;
        let mapping = Arc::new(Mutex::new(HashMap::new()));
        let death = Arc::new(Death {
            source,
            state: state.clone(),
            notice,
        });
        endpoint.as_binder().link_to_death_arc(&death)?;
        let callback = BnRadioMessagingResponse::new_binder(Response {
            source,
            mapping: mapping.clone(),
            state: state.clone(),
        });
        let indication = BnRadioMessagingIndication::new_binder(Indication);
        endpoint.setResponseFunctions(&callback, &indication)?;
        Ok(Self {
            source,
            endpoint,
            mapping,
            state,
            death,
            _callback: callback,
        })
    }
    fn send(
        &self,
        message: &str,
        serial: i32,
    ) -> std::result::Result<Request, Box<dyn std::error::Error>> {
        {
            let map = self.mapping.lock().unwrap();
            if map.contains_key(&serial) || map.len() >= 32 {
                return Err("serial occupied or binding capacity exhausted".into());
            }
        }
        let request = self
            .state
            .owner
            .lock()
            .unwrap()
            .begin(self.source, message, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            self.state
                .owner
                .lock()
                .unwrap()
                .receipt(message)
                .unwrap()
                .parts[0],
            PartState::Uncertain,
            "uncertainty must commit before IPC"
        );
        self.mapping.lock().unwrap().insert(serial, request.attempt);
        let wire = GsmSmsMessage {
            smscPdu: String::new(),
            pdu: hex(request.tpdu()),
        };
        match self.endpoint.sendSms(serial, &wire) {
            Ok(()) => {
                self.state
                    .owner
                    .lock()
                    .unwrap()
                    .observe(self.source, request.attempt, Signal::IpcReturned)
                    .unwrap();
            }
            Err(error) => {
                self.state
                    .owner
                    .lock()
                    .unwrap()
                    .observe(self.source, request.attempt, Signal::Uncertain)
                    .unwrap();
                return Err(error.into());
            }
        }
        Ok(request)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.state.owner.lock().unwrap().revoke(self.source);
        let _ = self.endpoint.as_binder().unlink_to_death_arc(&self.death);
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn apply(state: &State, event: Event) -> Outcome {
    event.attempt.map_or(Outcome::Ignored, |attempt| {
        state
            .owner
            .lock()
            .unwrap()
            .observe(event.source, attempt, event.signal)
            .unwrap()
    })
}
fn event(receiver: &Receiver<Event>) -> Event {
    receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("bounded callback observation")
}
fn receipt(state: &State, id: &str) -> pokelinux_messaging::outbox::Receipt {
    state.owner.lock().unwrap().receipt(id).unwrap()
}
fn prepare(state: &State, n: u32) -> String {
    let id = format!("{n:032x}");
    state
        .owner
        .lock()
        .unwrap()
        .prepare(&id, "+15550100", &"x".repeat(161), true)
        .unwrap();
    id
}
fn assert_uncertain(state: &State, source: Source, id: &str) {
    assert_eq!(
        receipt(state, id).parts[0],
        PartState::Uncertain,
        "one-way send return cannot qualify transport acceptance"
    );
    assert!(
        state.owner.lock().unwrap().begin(source, id, 0).is_err(),
        "original part must not retry"
    );
    assert!(
        state.owner.lock().unwrap().begin(source, id, 1).is_err(),
        "uncertain part blocks multipart continuation"
    );
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let binder = bootstrap()?;
    let peer: Strong<dyn ISmsBoundaryProbe> = FromIBinder::try_from(binder)?;
    let state_path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("private test state directory absent")?,
    );
    let outbox = Outbox::open(&state_path).map_err(|e| format!("{e:?}"))?;
    let (sender, receiver) = mpsc::sync_channel(64);
    let state = Arc::new(State {
        owner: Mutex::new(SubmissionOwner::new(outbox)),
        events: sender,
    });
    let (notice, death) = mpsc::sync_channel(8);
    let first = Session::new(peer.endpoint(0)?, state.clone(), 1, notice.clone())?;
    let second = Session::new(peer.endpoint(1)?, state.clone(), 2, notice.clone())?;
    peer.barrier(0, 1, 0)?;
    peer.barrier(1, 1, 0)?;
    println!("sms-binder-frozen-bootstrap\tPASS");
    let id = prepare(&state, 1);
    let request = first.send(&id, 7)?;
    peer.barrier(0, 1, 1)?;
    assert_eq!(peer.capturedSerial(0)?, 7);
    assert_eq!(peer.capturedPdu(0)?, hex(request.tpdu()));
    assert_eq!(peer.capturedSmsc(0)?, "");
    assert_uncertain(&state, first.source, &id);
    println!("sms-binder-durable-before-ipc-oneway\tPASS");
    peer.acknowledgeRequest(0, 7)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Pending);
    assert_uncertain(&state, first.source, &id);
    println!("sms-binder-intermediate-ack\tPASS");
    for (slot, serial, mode) in [(0, 999, 0), (0, 7, 1), (1, 7, 0), (0, 7, 3), (0, 7, 4)] {
        peer.finalResponse(slot, serial, mode, false)?;
        assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
        assert_uncertain(&state, first.source, &id);
    }
    peer.finalResponse(0, 7, 5, false)?;
    assert_eq!(
        apply(&state, event(&receiver)),
        Outcome::Ignored,
        "odd-length acknowledgement must not consume a permit"
    );
    assert_uncertain(&state, first.source, &id);
    println!("sms-binder-malformed-ackpdu\tPASS");
    peer.finalResponse(0, 7, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Accepted);
    assert_eq!(receipt(&state, &id).parts[0], PartState::TransportAccepted);
    peer.finalResponse(0, 7, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    assert!(
        state
            .owner
            .lock()
            .unwrap()
            .begin(first.source, &id, 0)
            .is_err()
    );
    println!("sms-binder-final-matching-wrong-duplicate\tPASS");
    let cancelled = prepare(&state, 2);
    first.send(&cancelled, 8)?;
    peer.barrier(0, 1, 2)?;
    peer.finalResponse(0, 8, 0, false)?;
    let held = event(&receiver);
    state.owner.lock().unwrap().cancel(&cancelled).unwrap();
    assert_eq!(apply(&state, held), Outcome::Ignored);
    peer.finalResponse(0, 8, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    assert!(receipt(&state, &cancelled).cancelled);
    assert_uncertain(&state, first.source, &cancelled);
    println!("sms-binder-cancel-consumed-late\tPASS");
    let retired = prepare(&state, 3);
    first.send(&retired, 9)?;
    peer.barrier(0, 1, 3)?;
    let replacement = Session::new(peer.endpoint(0)?, state.clone(), 1, notice.clone())?;
    peer.barrier(0, 2, 3)?;
    let successor = prepare(&state, 4);
    replacement.send(&successor, 9)?;
    peer.barrier(0, 2, 4)?;
    peer.finalResponse(0, 9, 0, true)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    assert_uncertain(&state, replacement.source, &successor);
    assert_eq!(receipt(&state, &retired).parts[0], PartState::Uncertain);
    drop(first);
    peer.finalResponse(0, 9, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Accepted);
    println!("sms-binder-replaced-generation-same-serial\tPASS");
    let failed = prepare(&state, 5);
    replacement.send(&failed, 10)?;
    peer.barrier(0, 2, 5)?;
    peer.finalResponse(0, 10, 2, false)?;
    let _ = apply(&state, event(&receiver));
    peer.finalResponse(0, 10, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    assert_uncertain(&state, replacement.source, &failed);
    println!("sms-binder-error-no-negative-proof-retry\tPASS");
    let dying = prepare(&state, 6);
    replacement.send(&dying, 11)?;
    peer.barrier(0, 2, 6)?;
    let other = prepare(&state, 7);
    second.send(&other, 12)?;
    peer.barrier(1, 1, 1)?;
    peer.finalResponse(0, 11, 0, false)?;
    let held = event(&receiver);
    assert_eq!(
        peer.die().unwrap_err().transaction_error(),
        StatusCode::DeadObject
    );
    death.recv_timeout(Duration::from_secs(3))?;
    death.recv_timeout(Duration::from_secs(3))?;
    assert_eq!(apply(&state, held), Outcome::Ignored);
    assert_uncertain(&state, replacement.source, &dying);
    assert_uncertain(&state, second.source, &other);
    println!("sms-binder-death-consumed-callback\tPASS");
    drop(replacement);
    drop(second);
    drop(state);
    let reopened = Outbox::open(&state_path).map_err(|e| format!("{e:?}"))?;
    for id in [&cancelled, &retired, &failed, &dying, &other] {
        assert_eq!(reopened.lookup(id).unwrap().parts[0], PartState::Uncertain);
    }
    assert!(reopened.lookup(&cancelled).unwrap().cancelled);
    println!("sms-binder-restart-preserves-uncertainty\tPASS");
    Ok(())
}
