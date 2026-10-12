//! Synthetic frozen standard messaging V3 incoming/ACK ordering. No modem dispatch.
use pokelinux_messaging::receipt::{
    AckAttempt, AckPermit, AckState, BackendScope, IncomingKind, Outcome, ReceiptId, ReceiptOwner,
    Slot, Source, TestAckRequest, TestAckSink, TestDispatchError, TestSignal,
};
use pokelinux_sms_receipt_boundary::*;
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
const PACKAGE: &str = "org.pokelinux.smsreceiptpeer";
const COMPONENT: &str = "org.pokelinux.smsreceiptpeer.Peer";
const DESCRIPTOR: &str = "org.pokelinux.smsreceipttest.ISmsReceiptProbe";
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
enum Event {
    Arrival {
        source: Source,
        receipt: ReceiptId,
        kind: IncomingKind,
    },
    Ack {
        source: Source,
        attempt: Option<AckAttempt>,
        signal: TestSignal,
    },
    Rejected,
}
struct State {
    owner: Mutex<ReceiptOwner>,
    events: SyncSender<Event>,
    path: PathBuf,
}
impl State {
    fn queue(&self, source: Source, event: Event) -> rsbinder::BinderResult<()> {
        if self.events.try_send(event).is_err() {
            self.owner.lock().unwrap().revoke(source);
            return Err(StatusCode::FailedTransaction.into());
        }
        Ok(())
    }
}
struct Response {
    source: Source,
    mapping: Arc<Mutex<HashMap<i32, AckAttempt>>>,
    state: Arc<State>,
}
impl Interface for Response {}
impl Response {
    fn queue(&self, serial: Option<i32>, signal: TestSignal) -> rsbinder::BinderResult<()> {
        let attempt = serial.and_then(|n| self.mapping.lock().unwrap().get(&n).copied());
        self.state.queue(
            self.source,
            Event::Ack {
                source: self.source,
                attempt,
                signal,
            },
        )
    }
}
impl IRadioMessagingResponse for Response {
    fn sendSmsResponse(
        &self,
        _info: &RadioResponseInfo,
        _sms: &SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn acknowledgeRequest(&self, serial: i32) -> rsbinder::BinderResult<()> {
        self.queue(Some(serial), TestSignal::RequestAcknowledged)
    }
    fn r#acknowledgeIncomingGsmSmsWithPduResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#acknowledgeLastIncomingCdmaSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#acknowledgeLastIncomingGsmSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        if _arg_info.r#type != RadioResponseType::SOLICITED {
            return self.queue(None, TestSignal::RequestAcknowledged);
        }
        self.queue(
            Some(_arg_info.serial),
            if _arg_info.error == RadioError::NONE {
                TestSignal::FinalAccepted
            } else {
                TestSignal::Uncertain
            },
        )
    }
    fn r#deleteSmsOnRuimResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#deleteSmsOnSimResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#getCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo::CdmaBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#getGsmBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo::GsmBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#getSmscAddressResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_smsc: &str,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#reportSmsMemoryStatusResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#sendCdmaSmsExpectMoreResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#sendCdmaSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#sendImsSmsResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#sendSmsExpectMoreResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#setCdmaBroadcastActivationResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#setCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#setGsmBroadcastActivationResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#setGsmBroadcastConfigResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#setSmscAddressResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#writeSmsToRuimResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
    fn r#writeSmsToSimResponse(
        &self,
        _arg_info: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.queue(None, TestSignal::RequestAcknowledged)
    }
}

struct Indication {
    source: Source,
    state: Arc<State>,
}
impl Interface for Indication {}
impl Indication {
    fn admit(
        &self,
        indication_type: RadioIndicationType,
        kind: IncomingKind,
        raw: &[u8],
    ) -> rsbinder::BinderResult<()> {
        let result = {
            let mut owner = self.state.owner.lock().unwrap();
            let result = owner.admit(self.source, kind, raw);
            // ACK_EXP has a separate unqualified responseAcknowledgement obligation.
            // Store the arrival but never silently issue either protocol command.
            if indication_type != RadioIndicationType::UNSOLICITED {
                owner.revoke(self.source);
            }
            result
        };
        let event = match result {
            Ok(receipt) => Event::Arrival {
                source: self.source,
                receipt,
                kind,
            },
            Err(_) => Event::Rejected,
        };
        self.state.queue(self.source, event)
    }
}
impl IRadioMessagingIndication for Indication {
    fn r#cdmaNewSms(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_msg: &pokelinux_sms_receipt_boundary::generated::android::hardware::radio::messaging::CdmaSmsMessage::CdmaSmsMessage,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#cdmaRuimSmsStorageFull(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newBroadcastSms(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_data: &[u8],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSms(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        self.admit(_arg_type, IncomingKind::Deliver, _arg_pdu)
    }
    fn r#newSmsOnSim(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_record_number: i32,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSmsStatusReport(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        self.admit(_arg_type, IncomingKind::StatusReport, _arg_pdu)
    }
    fn r#simSmsStorageFull(
        &self,
        _arg_type: pokelinux_sms_receipt_boundary::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
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
    mapping: Arc<Mutex<HashMap<i32, AckAttempt>>>,
    state: Arc<State>,
    death: Arc<Death>,
    _response: Strong<dyn IRadioMessagingResponse>,
    _indication: Strong<dyn IRadioMessagingIndication>,
}
impl Session {
    fn new(
        endpoint: Strong<dyn IRadioMessaging>,
        state: Arc<State>,
        slot: Slot,
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
            .attach(slot, BackendScope::TestBackend)
            .unwrap();
        let mapping = Arc::new(Mutex::new(HashMap::new()));
        let death = Arc::new(Death {
            source,
            state: state.clone(),
            notice,
        });
        endpoint.as_binder().link_to_death_arc(&death)?;
        let response = BnRadioMessagingResponse::new_binder(Response {
            source,
            mapping: mapping.clone(),
            state: state.clone(),
        });
        let indication = BnRadioMessagingIndication::new_binder(Indication {
            source,
            state: state.clone(),
        });
        endpoint.setResponseFunctions(&response, &indication)?;
        Ok(Self {
            source,
            endpoint,
            mapping,
            state,
            death,
            _response: response,
            _indication: indication,
        })
    }
    fn claim(&self, receipt: ReceiptId, serial: i32) -> AckPermit {
        let map = self.mapping.lock().unwrap();
        assert!(
            !map.contains_key(&serial) && map.len() < 32,
            "bounded command serial must not be reused in a callback incarnation"
        );
        self.state
            .owner
            .lock()
            .unwrap()
            .claim_test_ack(self.source, receipt)
            .unwrap()
    }
    fn dispatch(&self, permit: AckPermit, serial: i32, raw: &[u8]) -> Outcome {
        let mut sink = Sink {
            session: self,
            serial,
            raw,
        };
        self.state
            .owner
            .lock()
            .unwrap()
            .dispatch_test_ack(permit, &mut sink)
            .unwrap()
    }
    fn send(&self, receipt: ReceiptId, serial: i32, raw: &[u8]) -> AckAttempt {
        let permit = self.claim(receipt, serial);
        let attempt = permit.attempt();
        assert_eq!(self.dispatch(permit, serial, raw), Outcome::Pending);
        assert_eq!(
            self.state
                .owner
                .lock()
                .unwrap()
                .observe_test_ack(self.source, attempt, TestSignal::IpcReturned)
                .unwrap(),
            Outcome::Pending,
            "one-way ACK return cannot finish the command"
        );
        assert_eq!(ack(&self.state, receipt), AckState::Uncertain);
        attempt
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.state.owner.lock().unwrap().revoke(self.source);
        let _ = self.endpoint.as_binder().unlink_to_death_arc(&self.death);
    }
}
struct Sink<'a> {
    session: &'a Session,
    serial: i32,
    raw: &'a [u8],
}
impl TestAckSink for Sink<'_> {
    fn dispatch(&mut self, request: TestAckRequest) -> std::result::Result<(), TestDispatchError> {
        assert_eq!(request.source, self.session.source);
        // Independent SQLite connection proves both commits across the dispatch boundary.
        // Equality checks storage, never message identity or deduplication.
        let db = rusqlite::Connection::open_with_flags(
            self.session.state.path.join("receipts.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let (raw, state): (Vec<u8>, String) = db
            .query_row(
                "SELECT raw,ack FROM receipts WHERE id=?1",
                [request.receipt.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            raw, self.raw,
            "complete raw occurrence must commit before generated ACK IPC"
        );
        assert_eq!(
            state, "uncertain",
            "uncertainty must commit before generated ACK IPC"
        );
        let mut map = self.session.mapping.lock().unwrap();
        assert!(map.len() < 32 && !map.contains_key(&self.serial));
        map.insert(self.serial, request.attempt);
        drop(map);
        // Explicit synthetic success/cause values only. No production policy is selected.
        self.session
            .endpoint
            .acknowledgeLastIncomingGsmSms(
                self.serial,
                true,
                SmsAcknowledgeFailCause::UNSPECIFIED_ERROR,
            )
            .map_err(|_| TestDispatchError)
    }
}
fn event(receiver: &Receiver<Event>) -> Event {
    receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("bounded callback observation")
}
fn arrival(receiver: &Receiver<Event>, source: Source, kind: IncomingKind) -> ReceiptId {
    match event(receiver) {
        Event::Arrival {
            source: actual,
            receipt,
            kind: actual_kind,
        } => {
            assert_eq!(actual, source);
            assert_eq!(actual_kind, kind);
            receipt
        }
        _ => panic!("expected actual incoming indication"),
    }
}
fn apply(state: &State, event: Event) -> Outcome {
    match event {
        Event::Ack {
            source,
            attempt: Some(attempt),
            signal,
        } => state
            .owner
            .lock()
            .unwrap()
            .observe_test_ack(source, attempt, signal)
            .unwrap(),
        Event::Ack { attempt: None, .. } => Outcome::Ignored,
        _ => panic!("expected actual ACK response"),
    }
}
fn ack(state: &State, id: ReceiptId) -> AckState {
    state.owner.lock().unwrap().lookup(id).unwrap().ack
}
fn no_retry(state: &State, source: Source, id: ReceiptId) {
    assert!(
        state
            .owner
            .lock()
            .unwrap()
            .claim_test_ack(source, id)
            .is_err()
    );
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let peer: Strong<dyn ISmsReceiptProbe> = FromIBinder::try_from(bootstrap()?)?;
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("private receipt test directory absent")?,
    );
    let mut owner = ReceiptOwner::open(&path).map_err(|e| format!("{e:?}"))?;
    owner.set_test_ack_permission(true);
    let (sender, receiver) = mpsc::sync_channel(32);
    let state = Arc::new(State {
        owner: Mutex::new(owner),
        events: sender,
        path: path.clone(),
    });
    let (notice, death) = mpsc::sync_channel(16);
    let mut first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    let second = Session::new(peer.endpoint(1)?, state.clone(), Slot(1), notice.clone())?;
    peer.barrier(0, 1, 0)?;
    peer.barrier(1, 1, 0)?;
    println!("sms-receipt-frozen-bootstrap\tPASS");
    let raw = [0, 1, 255]; // Malformed synthetic PDU is retained without decoder side effects.
    peer.incoming(0, 0, &raw, 0, false)?;
    let original = arrival(&receiver, first.source, IncomingKind::Deliver);
    assert_eq!(
        state.owner.lock().unwrap().lookup(original).unwrap().raw(),
        raw
    );
    assert_eq!(ack(&state, original), AckState::Stored);
    assert_eq!(peer.ackCount(0)?, 0);
    first.send(original, 7, &raw);
    peer.barrier(0, 1, 1)?;
    assert_eq!(peer.capturedSerial(0)?, 7);
    assert!(peer.capturedSuccess(0)?);
    assert_eq!(peer.capturedCause(0)?, 255);
    no_retry(&state, first.source, original);
    println!("sms-receipt-raw-uncertainty-before-ipc-oneway\tPASS");
    peer.intermediate(0, 7)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Pending);
    assert_eq!(ack(&state, original), AckState::Uncertain);
    println!("sms-receipt-intermediate-not-final\tPASS");
    peer.finalResponse(0, 7, 1, false)?;
    assert_eq!(
        apply(&state, event(&receiver)),
        Outcome::Ignored,
        "wrong ACK method must not consume the current last-incoming command"
    );
    for (slot, serial, mode) in [(0, 999, 0), (1, 7, 0), (0, 7, 3), (0, 7, 4)] {
        peer.finalResponse(slot, serial, mode, false)?;
        assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
        assert_eq!(ack(&state, original), AckState::Uncertain);
    }
    peer.finalResponse(0, 7, 0, false)?;
    assert_eq!(
        apply(&state, event(&receiver)),
        Outcome::TestBackendAccepted
    );
    peer.finalResponse(0, 7, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    no_retry(&state, first.source, original);
    println!("sms-receipt-exact-final-wrong-duplicate\tPASS");
    peer.incoming(0, 1, &raw, 0, false)?;
    let equal = arrival(&receiver, first.source, IncomingKind::StatusReport);
    assert_ne!(original, equal);
    assert_eq!(
        state.owner.lock().unwrap().lookup(equal).unwrap().raw(),
        raw
    );
    first.send(equal, 8, &raw);
    peer.barrier(0, 1, 2)?;
    peer.finalResponse(0, 8, 0, false)?;
    assert_eq!(
        apply(&state, event(&receiver)),
        Outcome::TestBackendAccepted
    );
    println!("sms-receipt-equal-bytes-distinct-kinds\tPASS");
    peer.incoming(0, 0, &raw, 0, false)?;
    let uncertain = arrival(&receiver, first.source, IncomingKind::Deliver);
    first.send(uncertain, 9, &raw);
    peer.barrier(0, 1, 3)?;
    peer.incoming(0, 1, &raw, 0, false)?;
    let premature = arrival(&receiver, first.source, IncomingKind::StatusReport);
    assert_ne!(uncertain, premature);
    peer.finalResponse(0, 9, 0, false)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    assert_eq!(ack(&state, uncertain), AckState::Uncertain);
    assert_eq!(ack(&state, premature), AckState::Stored);
    no_retry(&state, first.source, uncertain);
    no_retry(&state, first.source, premature);
    assert_eq!(peer.ackCount(0)?, 3);
    println!("sms-receipt-premature-arrival-quarantines-window\tPASS");
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 2, 3)?;
    peer.incoming(0, 0, &raw, 0, false)?;
    let retired = arrival(&receiver, first.source, IncomingKind::Deliver);
    first.send(retired, 10, &raw);
    peer.barrier(0, 2, 4)?;
    let retired_source = first.source;
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 3, 4)?;
    peer.incoming(0, 1, &raw, 0, false)?;
    let successor = arrival(&receiver, first.source, IncomingKind::StatusReport);
    first.send(successor, 10, &raw);
    peer.barrier(0, 3, 5)?;
    peer.finalResponse(0, 10, 0, true)?;
    assert_eq!(apply(&state, event(&receiver)), Outcome::Ignored);
    peer.incoming(0, 0, &raw, 0, true)?;
    let stale = arrival(&receiver, retired_source, IncomingKind::Deliver);
    no_retry(&state, retired_source, stale);
    peer.finalResponse(0, 10, 0, false)?;
    assert_eq!(
        apply(&state, event(&receiver)),
        Outcome::TestBackendAccepted
    );
    assert_eq!(ack(&state, retired), AckState::Uncertain);
    println!("sms-receipt-old-generation-and-stale-raw\tPASS");
    peer.incoming(0, 0, &raw, 0, false)?;
    let cancelled = arrival(&receiver, first.source, IncomingKind::Deliver);
    first.send(cancelled, 11, &raw);
    peer.barrier(0, 3, 6)?;
    peer.finalResponse(0, 11, 0, false)?;
    let held = event(&receiver);
    state.owner.lock().unwrap().cancel(cancelled).unwrap();
    assert_eq!(apply(&state, held), Outcome::Ignored);
    assert_eq!(ack(&state, cancelled), AckState::Uncertain);
    no_retry(&state, first.source, cancelled);
    println!("sms-receipt-cancel-held-final\tPASS");
    peer.incoming(0, 1, &raw, 0, false)?;
    let after_cancel = arrival(&receiver, first.source, IncomingKind::StatusReport);
    no_retry(&state, first.source, after_cancel);
    assert_eq!(ack(&state, after_cancel), AckState::Stored);
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 4, 6)?;
    peer.incoming(0, 0, &raw, 0, false)?;
    let blocked = arrival(&receiver, first.source, IncomingKind::Deliver);
    let permit = first.claim(blocked, 12);
    state.owner.lock().unwrap().set_test_ack_permission(false);
    state.owner.lock().unwrap().set_test_ack_permission(true);
    assert_eq!(first.dispatch(permit, 12, &raw), Outcome::Ignored);
    assert_eq!(peer.ackCount(0)?, 6);
    assert_eq!(ack(&state, blocked), AckState::Uncertain);
    println!("sms-receipt-permission-held-permit\tPASS");
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 5, 6)?;
    peer.incoming(0, 0, &raw, 1, false)?;
    let ack_exp = arrival(&receiver, first.source, IncomingKind::Deliver);
    no_retry(&state, first.source, ack_exp);
    assert_eq!(ack(&state, ack_exp), AckState::Stored);
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 6, 6)?;
    let before = state.owner.lock().unwrap().snapshot(32).unwrap().total;
    peer.incoming(0, 0, &[0; 513], 0, false)?;
    assert!(matches!(event(&receiver), Event::Rejected));
    no_retry(&state, first.source, ack_exp);
    assert_eq!(
        state.owner.lock().unwrap().snapshot(32).unwrap().total,
        before
    );
    assert_eq!(peer.ackCount(0)?, 6);
    println!("sms-receipt-unqualified-type-and-bounded-raw\tPASS");
    first = Session::new(peer.endpoint(0)?, state.clone(), Slot(0), notice.clone())?;
    peer.barrier(0, 7, 6)?;
    peer.incoming(0, 0, &raw, 0, false)?;
    let dying = arrival(&receiver, first.source, IncomingKind::Deliver);
    first.send(dying, 13, &raw);
    peer.barrier(0, 7, 7)?;
    peer.finalResponse(0, 13, 0, false)?;
    let held = event(&receiver);
    assert_eq!(
        peer.die().unwrap_err().transaction_error(),
        StatusCode::DeadObject
    );
    death.recv_timeout(Duration::from_secs(3))?;
    death.recv_timeout(Duration::from_secs(3))?;
    assert_eq!(apply(&state, held), Outcome::Ignored);
    assert_eq!(ack(&state, dying), AckState::Uncertain);
    no_retry(&state, first.source, dying);
    println!("sms-receipt-death-held-final\tPASS");
    drop(first);
    drop(second);
    drop(peer);
    drop(state);
    let end = std::time::Instant::now() + Duration::from_secs(3);
    let mut reopened = loop {
        match ReceiptOwner::open(&path) {
            Ok(value) => break value,
            Err(e) => {
                assert!(
                    std::time::Instant::now() < end,
                    "receipt reopen after peer death: {e:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };
    for id in [uncertain, retired, cancelled, blocked, dying] {
        assert_eq!(reopened.lookup(id).unwrap().ack, AckState::Uncertain);
    }
    assert_eq!(
        reopened.lookup(original).unwrap().ack,
        AckState::TestBackendAccepted
    );
    assert_eq!(
        reopened.lookup(equal).unwrap().ack,
        AckState::TestBackendAccepted
    );
    assert_eq!(reopened.lookup(stale).unwrap().ack, AckState::Stored);
    reopened.set_test_ack_permission(true);
    let source = reopened.attach(Slot(0), BackendScope::TestBackend).unwrap();
    for id in [
        uncertain, retired, cancelled, blocked, dying, premature, stale,
    ] {
        assert!(reopened.claim_test_ack(source, id).is_err());
    }
    let redelivered = reopened.admit(source, IncomingKind::Deliver, &raw).unwrap();
    assert_ne!(redelivered, original);
    assert_ne!(redelivered, equal);
    println!("sms-receipt-restart-no-replay-or-dedup\tPASS");
    Ok(())
}
