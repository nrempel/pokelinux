//! Disposable SDK37 test bootstrap. No framework ABI belongs in the radio library.
use pokelinux_radio::{
    indication::IndicationKind,
    receiver::{CallbackInbox, RadioEvent, ReceiverFailure, new_radio_callbacks},
    response::ResponseKind,
    transport::{RADIO_DESCRIPTOR, RadioTransport},
};
use rsbinder::{
    Binder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status, StatusCode, hub,
};
use std::{
    num::NonZeroUsize,
    sync::mpsc::{self, SyncSender},
    time::{Duration, Instant},
};
const PACKAGE: &str = "org.pokelinux.radiopeer";
const COMPONENT: &str = "org.pokelinux.radiopeer.Peer";
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
    if !peer.is_remote() || peer.descriptor() != RADIO_DESCRIPTOR {
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
fn callback(events: &CallbackInbox) -> std::result::Result<RadioEvent, Box<dyn std::error::Error>> {
    let end = Instant::now() + Duration::from_secs(5);
    events
        .recv_until(end)
        .map_err(|e| format!("receiver failure: {e:?}"))?
        .ok_or_else(|| "callback timeout".into())
}
fn report(peer: &SIBinder) -> Result<(i32, i32, i32, i32, i32, String)> {
    let data = peer
        .as_remote()
        .ok_or(StatusCode::BadType)?
        .prepare_transact(true)?;
    let mut reply = peer
        .as_remote()
        .ok_or(StatusCode::BadType)?
        .submit_transact(100, &data, 0)?
        .ok_or(StatusCode::UnexpectedNull)?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(StatusCode::from(status));
    }
    let value = (
        reply.read_i32()?,
        reply.read_i32()?,
        reply.read_i32()?,
        reply.read_i32()?,
        reply.read_i32()?,
        reply.read::<String>()?,
    );
    if reply.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(value)
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(2).as_deref() == Some("supervised-blocked-registration") {
        return supervised_registration();
    }
    // Captured and checked against the installed framework by FrameworkBootstrapOracle.
    let code: u32 = std::env::args()
        .nth(1)
        .ok_or("missing captured transaction ID")?
        .parse()?;
    if code != 51 {
        return Err("unqualified framework transaction ID".into());
    }
    let scenario = std::env::args().nth(2);
    let die_register = match scenario.as_deref() {
        None
        | Some("session-owner")
        | Some("expire-before-request")
        | Some("cancel-before-request")
        | Some("stop-before-request")
        | Some("death-before-request")
        | Some("send-owned-segments")
        | Some("cancel-before-response")
        | Some("cancel-during-registration")
        | Some("expire-during-registration")
        | Some("owned-service-death")
        | Some("owned-registration-death")
        | Some("permanent-registration-child") => false,
        Some("death-during-registration") => true,
        _ => return Err("unknown test scenario".into()),
    };
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
        .submit_transact(code, &data, 0)?
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
    println!("radio-bootstrap\tPASS");
    let transport = RadioTransport::from_binder(peer.clone())?;
    if scenario.as_deref() == Some("permanent-registration-child") {
        return permanent_registration(&peer);
    }
    if matches!(
        scenario.as_deref(),
        Some("owned-service-death") | Some("owned-registration-death")
    ) {
        return owned_death(
            &peer,
            scenario.as_deref() == Some("owned-registration-death"),
        );
    }
    if matches!(
        scenario.as_deref(),
        Some(
            "cancel-before-request"
                | "stop-before-request"
                | "death-before-request"
                | "send-owned-segments"
                | "cancel-before-response"
        )
    ) {
        return owned_request_scenario(&peer, scenario.as_deref().unwrap());
    }
    if scenario.as_deref() == Some("expire-before-request") {
        return expire_before_request(&peer);
    }
    if scenario.as_deref() == Some("session-owner") {
        return owner_ipc(&transport, &peer);
    }
    if scenario.as_deref() == Some("expire-during-registration") {
        return cancelled_registration(&transport, &peer, true);
    }
    if scenario.as_deref() == Some("cancel-during-registration") {
        return cancelled_registration(&transport, &peer, false);
    }
    let callbacks = new_radio_callbacks(NonZeroUsize::new(4).unwrap());
    let inbox = &callbacks.events;
    if die_register {
        let _watch = transport.watch_death(callbacks.control.clone())?;
        let error = transport
            .register(
                &callbacks.response.as_binder(),
                &callbacks.indication.as_binder(),
                "pokelinux-die-register-test",
            )
            .expect_err("peer unexpectedly returned a registration ID");
        if error.transaction_error() != StatusCode::DeadObject {
            return Err(format!("unexpected registration failure: {error:?}").into());
        }
        if !matches!(
            inbox.recv_until(Instant::now() + Duration::from_secs(5)),
            Err(ReceiverFailure::ServiceDied)
        ) {
            return Err("registration death did not invalidate callbacks".into());
        }
        println!("radio-registration-death\tDeadObject\tPASS");
        return Ok(());
    }
    let id = transport.register(
        &callbacks.response.as_binder(),
        &callbacks.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    if id != 42 {
        return Err("client ID mismatch".into());
    }
    let mut seen = [false; 4];
    let mut assembly = pokelinux_radio::assembly::Assembly::new(
        34,
        0,
        5,
        pokelinux_radio::assembly::Limits {
            bytes: 8,
            fragments: 3,
        },
    )
    .map_err(|e| format!("assembly setup: {e:?}"))?;
    let mut assembled = None;
    for _ in 0..4 {
        let index = match callback(inbox)? {
            RadioEvent::Response(event) if event.info.serial == 7 && event.payload == [7] => 0,
            RadioEvent::Indication(event) if event.slot_id == 0 => {
                if event.message_id == 34
                    && let Some(bytes) = assembly
                        .push(&event)
                        .map_err(|e| format!("assembly: {e:?}"))?
                {
                    assembled = Some(bytes);
                }
                match (event.message_id, event.kind, event.payload.as_slice()) {
                    (33, IndicationKind::Raw, [1, 2, 3]) => 1,
                    (
                        34,
                        IndicationKind::Segment {
                            index: 0,
                            total_length: 5,
                        },
                        [4, 5],
                    ) => 2,
                    (
                        34,
                        IndicationKind::Segment {
                            index: 1,
                            total_length: 5,
                        },
                        [6, 7, 8],
                    ) => 3,
                    _ => return Err("indication content mismatch".into()),
                }
            }
            _ => return Err("registration event mismatch".into()),
        };
        if seen[index] {
            return Err("duplicate registration event".into());
        }
        seen[index] = true;
    }
    println!("radio-registration\tPASS");
    println!("radio-indications\tPASS");
    if assembled != Some(vec![4, 5, 6, 7, 8]) {
        return Err("assembled indication mismatch".into());
    }
    println!("radio-assembly\tPASS");
    for (offset, length) in [0, 2048, 2049, 4096].into_iter().enumerate() {
        let serial = 100 + i32::try_from(offset)?;
        let payload: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
        transport
            .send(serial, id, 137, 0, &payload)
            .map_err(|e| format!("submission failed: {e:?}"))?;
        let RadioEvent::Response(event) = callback(inbox)? else {
            return Err("unexpected indication".into());
        };
        if event.info.serial != serial
            || event.info.slotId != 0
            || event.info.rilcMsgId != 137
            || event.info.error != 0
            || event.info.length != length
            || event.kind != ResponseKind::Raw
            || event.payload != payload
        {
            return Err("response content or correlation mismatch".into());
        }
        println!("radio-roundtrip\t{length}\tPASS");
    }
    let data = peer
        .as_remote()
        .ok_or("peer is local")?
        .prepare_transact(true)?;
    let mut reply = peer
        .as_remote()
        .ok_or("peer is local")?
        .submit_transact(105, &data, 0)?
        .ok_or("no overflow reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    if reply.data_avail() != 0 {
        return Err("extra overflow reply fields".into());
    }
    let end = Instant::now() + Duration::from_secs(5);
    while callbacks.indication.failure().is_none() {
        if Instant::now() >= end {
            return Err("overflow failure timeout".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if callbacks.indication.failure() != Some(ReceiverFailure::QueueFull)
        || callbacks.response.failure() != Some(ReceiverFailure::QueueFull)
        || !matches!(inbox.try_recv(), Err(ReceiverFailure::QueueFull))
    {
        return Err("overflow did not invalidate the shared stream".into());
    }
    println!("radio-callback-overflow\tPASS");
    transport.clear(id)?;
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let (raw, segmented, cleared, pid, uid, domain) = report(&peer)?;
        if raw != 2
            || segmented != 4
            || pid <= 0
            || uid < 10000
            || !domain.starts_with("u:r:untrusted_app")
        {
            return Err(
                format!("peer evidence mismatch: {raw} {segmented} {pid} {uid} {domain}").into(),
            );
        }
        if cleared == 1 {
            println!("radio-peer\t{pid}\t{uid}\t{domain}");
            println!("radio-clear-observed\tPASS");
            break;
        }
        if Instant::now() >= end {
            return Err("clear observation timeout".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // New endpoints after observed teardown; a reused ID does not reuse local state.
    let fresh = new_radio_callbacks(NonZeroUsize::new(4).unwrap());
    let fresh_id = transport.register(
        &fresh.response.as_binder(),
        &fresh.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    if fresh_id != 42 {
        return Err("replacement ID mismatch".into());
    }
    for _ in 0..4 {
        callback(&fresh.events)?;
    }
    let remote = peer.as_remote().ok_or("peer is local")?;
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(108, &data, 0)?
        .ok_or("missing late-callback reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    let rejection: String = reply.read()?;
    if rejection.is_empty() || reply.data_avail() != 0 {
        return Err("invalid late callback evidence".into());
    }
    let RadioEvent::Response(current) = callback(&fresh.events)? else {
        return Err("replacement event type".into());
    };
    if current.info.serial != 777
        || current.payload != [7]
        || fresh
            .events
            .try_recv()
            .map_err(|e| format!("new stream failed: {e:?}"))?
            .is_some()
        || !matches!(inbox.try_recv(), Err(ReceiverFailure::QueueFull))
    {
        return Err("late callback escaped its old stream".into());
    }
    println!("radio-late-callback\t{rejection}\tPASS");
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(106, &data, 0)?
        .ok_or("missing malformed-trigger reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    if reply.data_avail() != 0 {
        return Err("extra malformed-trigger reply fields".into());
    }
    if !matches!(
        fresh
            .events
            .recv_until(Instant::now() + Duration::from_secs(5)),
        Err(ReceiverFailure::MalformedCallback)
    ) {
        return Err("remote malformed callback did not wake failure wait".into());
    }
    if fresh.response.failure() != Some(ReceiverFailure::MalformedCallback)
        || callbacks.response.failure() != Some(ReceiverFailure::QueueFull)
    {
        return Err("replacement callback state not isolated".into());
    }
    println!("radio-malformed-wakeup\tPASS");
    let local = new_radio_callbacks(NonZeroUsize::new(1).unwrap());
    let control = local.control.clone();
    let waiter = std::thread::spawn(move || {
        local
            .events
            .recv_until(Instant::now() + Duration::from_secs(5))
    });
    control.cancel();
    if !matches!(
        waiter.join().map_err(|_| "waiter panicked")?,
        Err(ReceiverFailure::Cancelled)
    ) {
        return Err("owner cancellation did not wake wait".into());
    }
    println!("radio-owner-cancel\tPASS");
    transport.clear(fresh_id)?;
    let end = Instant::now() + Duration::from_secs(5);
    while report(&peer)?.2 != 1 {
        if Instant::now() >= end {
            return Err("replacement clear timeout".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let dying = new_radio_callbacks(NonZeroUsize::new(4).unwrap());
    let abandoned = new_radio_callbacks(NonZeroUsize::new(1).unwrap());
    drop(transport.watch_death(abandoned.control.clone())?);
    if !matches!(abandoned.events.try_recv(), Err(ReceiverFailure::Cancelled)) {
        return Err("dropping death watch did not invalidate local stream".into());
    }
    let _watch = transport.watch_death(dying.control.clone())?;
    transport.register(
        &dying.response.as_binder(),
        &dying.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    for _ in 0..4 {
        callback(&dying.events)?;
    }
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(107, &data, 0)?
        .ok_or("missing exit-trigger reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    if reply.data_avail() != 0 {
        return Err("extra exit reply".into());
    }
    if !matches!(
        dying
            .events
            .recv_until(Instant::now() + Duration::from_secs(5)),
        Err(ReceiverFailure::ServiceDied)
    ) {
        return Err("peer death did not invalidate callback stream".into());
    }
    println!("radio-service-death\tPASS");
    let dead = new_radio_callbacks(NonZeroUsize::new(1).unwrap());
    if !matches!(
        transport.watch_death(dead.control.clone()),
        Err(StatusCode::DeadObject)
    ) || !matches!(dead.events.try_recv(), Err(ReceiverFailure::ServiceDied))
    {
        return Err("already-dead service watch did not fail closed".into());
    }
    println!("radio-already-dead\tPASS");
    Ok(())
}

fn owner_next(
    owner: &mut pokelinux_radio::owner::RadioOwner,
) -> std::result::Result<pokelinux_radio::owner::PreparedEvent, Box<dyn std::error::Error>> {
    owner
        .prepare_until(Instant::now() + Duration::from_secs(5))
        .map_err(|e| format!("owner receive: {e:?}"))?
        .ok_or_else(|| "owner response timeout".into())
}
fn owner_initial(
    owner: &mut pokelinux_radio::owner::RadioOwner,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut responses = 0;
    for _ in 0..4 {
        let event = owner_next(owner)?;
        if matches!(event.event(), RadioEvent::Response(_)) {
            owner
                .publish(event)
                .map_err(|e| format!("owner publish: {e:?}"))?;
            responses += 1;
        }
    }
    if responses != 1
        || owner
            .snapshot()
            .observation
            .as_ref()
            .and_then(|value| value.kind.raw_response())
            .map(|response| response.payload.as_slice())
            != Some(&[7][..])
    {
        return Err("owner initial response mismatch".into());
    }
    Ok(())
}
fn owner_clear(
    transport: &RadioTransport,
    peer: &SIBinder,
    id: i32,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    transport.clear(id)?;
    let end = Instant::now() + Duration::from_secs(5);
    while report(peer)?.2 != 1 {
        if Instant::now() >= end {
            return Err("owner clear timeout".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
fn owner_ipc(
    transport: &RadioTransport,
    peer: &SIBinder,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_radio::owner::{OwnerError, Phase, RadioOwner};
    let mut owner = RadioOwner::new(NonZeroUsize::new(4).unwrap(), 0);
    let first = owner
        .begin_until(None, NonZeroUsize::new(4).unwrap(), owner_deadline())
        .map_err(|e| format!("owner begin: {e:?}"))?;
    let _first_watch = transport.watch_death(first.endpoints.control.clone())?;
    let id = transport.register(
        &first.endpoints.response.as_binder(),
        &first.endpoints.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    if !matches!(
        owner.prepare_until(Instant::now()),
        Err(OwnerError::NotActive)
    ) {
        return Err("callback published before activation".into());
    }
    owner
        .activate(&first.key, id)
        .map_err(|e| format!("owner activate: {e:?}"))?;
    owner_initial(&mut owner)?;
    println!("radio-owner-activation\tPASS");
    transport
        .send(9000, id, 137, 0, &[9, 8])
        .map_err(|e| format!("owner send: {e:?}"))?;
    let prepared = owner_next(&mut owner)?;
    if !matches!(prepared.event(), RadioEvent::Response(r) if r.info.serial == 9000 && r.payload == [9, 8])
    {
        return Err("owner consumed response mismatch".into());
    }
    first.endpoints.control.cancel();
    if owner.publish(prepared) != Err(OwnerError::Failed(ReceiverFailure::Cancelled))
        || owner.snapshot().observation.is_some()
    {
        return Err("consumed event survived cancellation".into());
    }
    println!("radio-owner-consumed-cancel\tPASS");
    owner_clear(transport, peer, id)?;
    let second = owner
        .begin_until(
            Some(&first.key),
            NonZeroUsize::new(4).unwrap(),
            owner_deadline(),
        )
        .map_err(|e| format!("owner replace: {e:?}"))?;
    let _second_watch = transport.watch_death(second.endpoints.control.clone())?;
    let id = transport.register(
        &second.endpoints.response.as_binder(),
        &second.endpoints.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    owner
        .activate(&second.key, id)
        .map_err(|e| format!("owner activate: {e:?}"))?;
    owner_initial(&mut owner)?;
    transport
        .send(9001, id, 137, 0, &[5])
        .map_err(|e| format!("owner send: {e:?}"))?;
    let stale = owner_next(&mut owner)?;
    if !matches!(stale.event(), RadioEvent::Response(r) if r.info.serial == 9001 && r.payload == [5])
    {
        return Err("owner stale response mismatch".into());
    }
    owner_clear(transport, peer, id)?;
    let third = owner
        .begin_until(
            Some(&second.key),
            NonZeroUsize::new(4).unwrap(),
            owner_deadline(),
        )
        .map_err(|e| format!("owner replace: {e:?}"))?;
    let _third_watch = transport.watch_death(third.endpoints.control.clone())?;
    let id = transport.register(
        &third.endpoints.response.as_binder(),
        &third.endpoints.indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    if id != 42 {
        return Err("owner recycled registration id mismatch".into());
    }
    owner
        .activate(&third.key, id)
        .map_err(|e| format!("owner activate: {e:?}"))?;
    if owner.publish(stale) != Err(OwnerError::StaleSession) {
        return Err("consumed event survived replacement".into());
    }
    owner_initial(&mut owner)?;
    println!("radio-owner-consumed-replacement\tPASS");
    let remote = peer.as_remote().ok_or("owner peer is local")?;
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(108, &data, 0)?
        .ok_or("owner late reply absent")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    let rejection = reply.read::<String>()?;
    if !rejection.starts_with("android.os.") || reply.data_avail() != 0 {
        return Err("owner late callback evidence mismatch".into());
    }
    let fresh = owner_next(&mut owner)?;
    if !matches!(fresh.event(), RadioEvent::Response(r) if r.info.serial == 777 && r.payload == [7])
    {
        return Err("owner fresh callback mismatch".into());
    }
    owner
        .publish(fresh)
        .map_err(|e| format!("owner fresh publish: {e:?}"))?;
    if owner.snapshot().phase != (Phase::Active { client_id: 42 }) {
        return Err("old endpoint invalidated replacement".into());
    }
    println!("radio-owner-late-endpoint\tPASS");
    owner
        .stop(&third.key)
        .map_err(|e| format!("owner stop: {e:?}"))?;
    if owner.snapshot().observation.is_some()
        || owner.snapshot().phase != Phase::Failed(ReceiverFailure::Cancelled)
    {
        return Err("owner stop retained current response".into());
    }
    owner_clear(transport, peer, id)?;
    println!("radio-owner-stop\tPASS");
    Ok(())
}

// Test controls below belong only to the independent synthetic peer.
fn registration_latch(peer: &SIBinder, code: u32) -> Result<()> {
    let remote = peer.as_remote().ok_or(StatusCode::BadType)?;
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(code, &data, 0)?
        .ok_or(StatusCode::UnexpectedNull)?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(StatusCode::from(status));
    }
    if reply.read_i32()? != 1 || reply.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(())
}
fn cancelled_registration(
    _transport: &RadioTransport,
    peer: &SIBinder,
    expire: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_process_owner::deadline::{Clock, Deadline};
    use pokelinux_radio::owner::{OwnerError, Phase};
    use pokelinux_radio::worker::{LifecycleError, TransportSession};
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    struct ControlledClock(AtomicU64);
    impl Clock for ControlledClock {
        fn now(&self) -> Duration {
            Duration::from_nanos(self.0.load(Ordering::SeqCst))
        }
    }
    let clock = Arc::new(ControlledClock(AtomicU64::new(10)));
    let mut session = TransportSession::with_clock(NonZeroUsize::new(2).unwrap(), 0, clock.clone());
    let terminal = if expire {
        ReceiverFailure::Expired
    } else {
        ReceiverFailure::Cancelled
    };
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "pokelinux-hold-register-test".into(),
            NonZeroUsize::new(4).unwrap(),
            Deadline::at(Duration::from_nanos(20)),
        )
        .map_err(|e| format!("start: {e:?}"))?;
    registration_latch(peer, 110)?;
    if !session.worker_occupied() || session.snapshot().phase != Phase::Registering {
        return Err("registration not held".into());
    }
    println!("radio-registration-held\tPASS");
    if expire {
        clock.0.store(20, Ordering::SeqCst);
    } else {
        session.stop(&key).map_err(|e| format!("stop: {e:?}"))?;
    }
    if session.snapshot().phase != Phase::Failed(terminal)
        || session.snapshot().observation.is_some()
        || session.poll_registration().is_some()
        || !session.worker_occupied()
    {
        return Err("local terminal state released held worker or retained response".into());
    }
    println!(
        "{}\tPASS",
        if expire {
            "radio-registration-held-expire"
        } else {
            "radio-registration-held-cancel"
        }
    );
    for _ in 0..16 {
        if session.start(
            RadioTransport::from_binder(peer.clone())?,
            "retry".into(),
            NonZeroUsize::new(4).unwrap(),
            Deadline::at(Duration::from_nanos(30)),
        ) != Err(LifecycleError::WorkerOccupied)
        {
            return Err("replacement freed an occupied worker".into());
        }
        session
            .stop(&key)
            .map_err(|e| format!("repeated stop: {e:?}"))?;
        if session.snapshot().phase != Phase::Failed(terminal) {
            return Err("first failure changed".into());
        }
    }
    println!("radio-worker-capacity-retained\tPASS");
    registration_latch(peer, 109)?;
    let end = Instant::now() + Duration::from_secs(5);
    let report = loop {
        if let Some(report) = session.poll_registration() {
            break report;
        }
        if Instant::now() >= end {
            return Err("owned completion timeout".into());
        }
        std::thread::yield_now();
    };
    if !report.dispatched
        || !matches!(report.result, Ok(42))
        || report.activation != Err(OwnerError::Failed(terminal))
        || session.snapshot().observation.is_some()
        || session.worker_occupied()
    {
        return Err("late registration completion violated lifecycle".into());
    }
    println!("radio-registration-late-success-rejected\tPASS");
    if session.start(
        RadioTransport::from_binder(peer.clone())?,
        "retry".into(),
        NonZeroUsize::new(4).unwrap(),
        Deadline::at(Duration::from_nanos(30)),
    ) != Err(LifecycleError::AlreadyAttempted)
    {
        return Err("uncertain registration was retried".into());
    }
    println!("radio-registration-retry-refused\tPASS");
    Ok(())
}

fn owner_deadline() -> pokelinux_process_owner::deadline::Deadline {
    use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
    Deadline::at(BootClock.now() + Duration::from_secs(60))
}

fn owned_death(
    peer: &SIBinder,
    during_registration: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_radio::owner::{OwnerError, Phase};
    use pokelinux_radio::worker::{RegistrationFailure, TransportSession};
    let mut session = TransportSession::new(NonZeroUsize::new(2).unwrap(), 0);
    session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            if during_registration {
                "pokelinux-die-register-test"
            } else {
                "pokelinux-ipc-test"
            }
            .into(),
            NonZeroUsize::new(8).unwrap(),
            owner_deadline(),
        )
        .map_err(|e| format!("start: {e:?}"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(report) = session.poll_registration() {
            break report;
        }
        if Instant::now() >= end {
            return Err("owned death registration timeout".into());
        }
        std::thread::yield_now();
    };
    if during_registration {
        if !matches!(result.result, Err(RegistrationFailure::Binder(status)) if status.transaction_error() == StatusCode::DeadObject)
            || result.activation != Err(OwnerError::Failed(ReceiverFailure::ServiceDied))
        {
            return Err("owned registration death mismatch".into());
        }
    } else {
        if !matches!(result.result, Ok(42))
            || result.activation != Ok(())
            || session.snapshot().phase != (Phase::Active { client_id: 42 })
        {
            return Err("owned registration did not activate".into());
        }
        println!("radio-owned-activation\tPASS");
        let remote = peer.as_remote().ok_or("local peer")?;
        let data = remote.prepare_transact(true)?;
        let mut reply = remote
            .submit_transact(107, &data, 0)?
            .ok_or("missing exit reply")?;
        let status: Status = reply.read()?;
        if !status.is_ok() || reply.data_avail() != 0 {
            return Err("exit control mismatch".into());
        }
    }
    loop {
        if session.snapshot().phase == Phase::Failed(ReceiverFailure::ServiceDied) {
            break;
        }
        if Instant::now() >= end {
            return Err("owned watch failed to invalidate session".into());
        }
        std::thread::yield_now();
    }
    if session.worker_occupied() || session.snapshot().observation.is_some() {
        return Err("owned death retained state or worker".into());
    }
    println!(
        "{}\tPASS",
        if during_registration {
            "radio-owned-registration-death"
        } else {
            "radio-owned-service-death"
        }
    );
    Ok(())
}

fn permanent_registration(peer: &SIBinder) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_process_owner::supervisor::ProcessIdentity;
    use pokelinux_radio::owner::Phase;
    use pokelinux_radio::worker::{LifecycleError, TransportSession};
    use std::io::Write;
    let identity = std::env::var("POKELINUX_OWNER_INCARNATION")?;
    ProcessIdentity::from_hex(&identity).ok_or("missing owner incarnation")?;
    let mut session = TransportSession::new(NonZeroUsize::new(2).unwrap(), 0);
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "pokelinux-permanent-hold-test".into(),
            NonZeroUsize::new(4).unwrap(),
            owner_deadline(),
        )
        .map_err(|e| format!("permanent start: {e:?}"))?;
    registration_latch(peer, 110)?;
    session
        .stop(&key)
        .map_err(|e| format!("permanent stop: {e:?}"))?;
    for _ in 0..16 {
        if session.snapshot().phase != Phase::Failed(ReceiverFailure::Cancelled)
            || !session.worker_occupied()
            || session.poll_registration().is_some()
            || session.start(
                RadioTransport::from_binder(peer.clone())?,
                "retry".into(),
                NonZeroUsize::new(4).unwrap(),
                owner_deadline(),
            ) != Err(LifecycleError::WorkerOccupied)
        {
            return Err("permanent worker failed local responsiveness/capacity".into());
        }
    }
    println!("radio-permanent-held\t{identity}\t1");
    std::io::stdout().flush()?;
    // Do not detach or drop the owned blocked worker. The supervisor terminates
    // this whole process. No release transaction exists for the permanent peer.
    loop {
        std::thread::park();
    }
}
fn supervised_registration() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_process_owner::supervisor::{StartError, Supervisor};
    use std::{os::unix::fs::DirBuilderExt, process::Command};
    let directory = std::path::PathBuf::from(format!(
        "/data/local/tmp/pokelinux-supervisor-{}",
        std::process::id()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let journal = directory.join("registration-admission");
    let mut supervisor = Supervisor::new(journal.clone());
    let binary = std::env::current_exe()?;
    let mut command = Command::new(&binary);
    command.args(["51", "permanent-registration-child"]);
    let id = supervisor
        .start(&mut command)
        .map_err(|e| format!("supervisor start: {e:?}"))?;
    let expected = format!("radio-permanent-held\t{}\t1\n", id.hex());
    let end = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    loop {
        let mut buffer = [0; 1024];
        match supervisor.read_output(&mut buffer) {
            Ok(0) => return Err("permanent child exited before barrier".into()),
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(error) => return Err(error.into()),
        }
        if output.len() > 8192 {
            return Err("bounded child output exceeded".into());
        }
        if output.ends_with(expected.as_bytes()) {
            break;
        }
        if Instant::now() >= end {
            return Err("permanent registration barrier timeout".into());
        }
        std::thread::yield_now();
    }
    if !supervisor.admits_frame(id, id, 1) {
        return Err("live child tag rejected".into());
    }
    println!("radio-supervised-held\tPASS");
    supervisor.stop()?;
    if supervisor.admits_frame(id, id, 1)
        || supervisor.start(&mut command) != Err(StartError::Occupied)
    {
        return Err("termination released publication or process occupancy incorrectly".into());
    }
    println!("radio-supervisor-output-revoked\tPASS");
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = supervisor.poll_reap()? {
            use std::os::unix::process::ExitStatusExt;
            if status.signal() != Some(9) {
                return Err("unexpected worker process exit".into());
            }
            break;
        }
        if !supervisor.occupied() || supervisor.start(&mut command) != Err(StartError::Occupied) {
            return Err("unreaped child capacity was released".into());
        }
        if Instant::now() >= end {
            return Err("worker reaping timeout: registration still unavailable".into());
        }
        std::thread::yield_now();
    }
    println!("radio-supervisor-reaped\tPASS");
    if supervisor.admits_frame(id, id, 1)
        || supervisor.occupied()
        || supervisor.start(&mut command) != Err(StartError::AdmissionUncertain)
    {
        return Err("reaping was confused with vendor cleanup".into());
    }
    println!("radio-supervisor-retry-refused\tPASS");
    let mut restarted = Supervisor::new(journal);
    if restarted.start(&mut command) != Err(StartError::AdmissionUncertain) {
        return Err("supervisor restart forgot uncertain registration".into());
    }
    println!("radio-supervisor-restart-refused\tPASS");
    Ok(())
}

fn expire_before_request(peer: &SIBinder) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_process_owner::deadline::{Clock, Deadline};
    use pokelinux_radio::{request::Outcome, worker::TransportSession};
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    struct Controlled(AtomicU64);
    impl Clock for Controlled {
        fn now(&self) -> Duration {
            Duration::from_nanos(self.0.load(Ordering::Acquire))
        }
    }
    let clock = Arc::new(Controlled(AtomicU64::new(10)));
    let mut session = TransportSession::with_clock(NonZeroUsize::new(2).unwrap(), 0, clock.clone());
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "pokelinux-ipc-test".into(),
            NonZeroUsize::new(8).unwrap(),
            Deadline::at(Duration::from_nanos(100)),
        )
        .map_err(|e| format!("registration admission: {e:?}"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let registration = loop {
        if let Some(report) = session.poll_registration() {
            break report;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    registration
        .activation
        .map_err(|e| format!("activation: {e:?}"))?;
    println!("radio-owned-request-registration\tPASS");
    let prepared = session
        .prepare_request(
            &key,
            137,
            0,
            vec![9, 8],
            Deadline::at(Duration::from_nanos(20)),
        )
        .map_err(|e| format!("request preparation: {e:?}"))?;
    clock.0.store(20, Ordering::Release); // Withhold timers; expiry must govern dispatch.
    session
        .submit_request(prepared)
        .map_err(|e| format!("request worker: {e:?}"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(report) = session.poll_request() {
            break report.map_err(|_| "request worker panicked")?;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    let end = Instant::now() + Duration::from_secs(5);
    let observed = loop {
        let r = report(peer)?;
        if result.submitted_parts == 0 || r.0 > 0 {
            break r;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    println!("radio-owned-request-peer-count\t{}", observed.0);
    assert_eq!(
        observed.0, 0,
        "expired operation crossed actual Binder into peer"
    );
    assert_eq!(result.submitted_parts, 0);
    assert_eq!(result.outcome, Outcome::NotSent);
    assert!(result.submission.is_err(), "expired dispatch was accepted");
    assert_eq!(
        session.snapshot().request.unwrap().outcome,
        Outcome::NotSent
    );
    session.stop(&key).map_err(|e| format!("stop: {e:?}"))?;
    println!("radio-owned-request-expiry-without-timer\tPASS");
    Ok(())
}

fn request_control(peer: &SIBinder, code: u32) -> Result<()> {
    let remote = peer.as_remote().ok_or(StatusCode::BadType)?;
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(code, &data, 0)?
        .ok_or(StatusCode::UnexpectedNull)?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(StatusCode::from(status));
    }
    if code != 107 && reply.read_i32()? != 1 {
        return Err(StatusCode::BadValue);
    }
    if reply.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(())
}
fn owned_request_scenario(
    peer: &SIBinder,
    scenario: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
    use pokelinux_radio::{
        owner::{OwnerError, Phase},
        request::Outcome,
        worker::{LifecycleError, TransportSession},
    };
    let mut session = TransportSession::new(NonZeroUsize::new(2).unwrap(), 0);
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "pokelinux-ipc-test".into(),
            NonZeroUsize::new(8).unwrap(),
            Deadline::at(BootClock.now() + Duration::from_secs(60)),
        )
        .map_err(|e| format!("register admission: {e:?}"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let registration = loop {
        if let Some(r) = session.poll_registration() {
            break r;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    registration
        .activation
        .map_err(|e| format!("activation: {e:?}"))?;
    let payload = if scenario == "send-owned-segments" {
        vec![42; 4096]
    } else {
        vec![9, 8]
    };
    let prepared = session
        .prepare_request(
            &key,
            137,
            0,
            payload,
            Deadline::at(BootClock.now() + Duration::from_secs(30)),
        )
        .map_err(|e| format!("prepare: {e:?}"))?;
    let operation = prepared.key();
    match scenario {
        "cancel-before-request" => session
            .cancel_request(&operation)
            .map_err(|e| format!("cancel: {e:?}"))?,
        "stop-before-request" => session.stop(&key).map_err(|e| format!("stop: {e:?}"))?,
        "death-before-request" => {
            assert_eq!(report(peer)?.0, 0);
            request_control(peer, 107)?;
            let end = Instant::now() + Duration::from_secs(5);
            while session.snapshot().phase != Phase::Failed(ReceiverFailure::ServiceDied) {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
        "cancel-before-response" => request_control(peer, 111)?,
        "send-owned-segments" => {}
        _ => unreachable!(),
    }
    session
        .submit_request(prepared)
        .map_err(|e| format!("worker: {e:?}"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(r) = session.poll_request() {
            break r.map_err(|_| "request panic")?;
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    assert!(!session.worker_occupied());
    if matches!(
        scenario,
        "cancel-before-request" | "stop-before-request" | "death-before-request"
    ) {
        assert_eq!(result.submitted_parts, 0);
        assert_eq!(result.outcome, Outcome::NotSent);
        assert!(result.submission.is_err());
        if scenario != "death-before-request" {
            assert_eq!(report(peer)?.0, 0);
        }
        assert_eq!(
            session.snapshot().request.unwrap().outcome,
            Outcome::NotSent
        );
    } else {
        assert!(result.submission.is_ok());
        assert_eq!(result.outcome, Outcome::OutcomeUnknown);
        if scenario == "send-owned-segments" {
            assert_eq!(result.submitted_parts, 2);
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let counts = report(peer)?;
                if counts.1 == 2 {
                    assert_eq!(counts.0, 0);
                    break;
                }
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            assert!(matches!(
                session.prepare_request(
                    &key,
                    137,
                    0,
                    vec![0],
                    Deadline::at(BootClock.now() + Duration::from_secs(30))
                ),
                Err(LifecycleError::Owner(OwnerError::OperationPending))
            ));
        } else {
            request_control(peer, 112)?; // The original-wire request actually entered the peer.
            assert_eq!(result.submitted_parts, 1);
            session
                .cancel_request(&operation)
                .map_err(|e| format!("cancel: {e:?}"))?;
            assert_eq!(
                session.snapshot().request.unwrap().outcome,
                Outcome::OutcomeUnknown
            );
            let started = Instant::now();
            let _ = session.snapshot();
            assert!(started.elapsed() < Duration::from_millis(350));
            request_control(peer, 113)?;
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let event = session
                    .prepare_callback_until(end)
                    .map_err(|e| format!("callback: {e:?}"))?
                    .ok_or("response deadline")?;
                if matches!(event.event(),RadioEvent::Response(r) if r.info.serial==1) {
                    session
                        .publish_callback(event)
                        .map_err(|e| format!("publish raw: {e:?}"))?;
                    break;
                }
                assert!(Instant::now() < end);
            }
            let snapshot = session.snapshot();
            assert_eq!(
                snapshot
                    .observation
                    .unwrap()
                    .kind
                    .raw_response()
                    .unwrap()
                    .payload,
                [9, 8]
            );
            assert_eq!(snapshot.request.unwrap().outcome, Outcome::OutcomeUnknown);
        }
    }
    session.stop(&key).map_err(|e| format!("stop: {e:?}"))?;
    println!("radio-owned-request-{}\tPASS", scenario);
    Ok(())
}
