//! Disposable SDK37 test bootstrap. No framework ABI belongs in the radio library.
use pokelinux_radio::{
    receiver::{ResponseInbox, new_response_receiver},
    response::{RadioResponse, ResponseKind},
    transport::{INDICATION_DESCRIPTOR, RADIO_DESCRIPTOR, RadioTransport},
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
// Only a distinct registration handle for this test. Production indication decoding is not implemented here.
struct IndicationHandle;
impl Remotable for IndicationHandle {
    fn descriptor() -> &'static str {
        INDICATION_DESCRIPTOR
    }
    fn on_transact(&self, _: u32, _: &mut Parcel, _: &mut Parcel) -> Result<()> {
        Err(StatusCode::UnknownTransaction)
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn callback(
    events: &ResponseInbox,
) -> std::result::Result<RadioResponse, Box<dyn std::error::Error>> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = events
            .try_recv()
            .map_err(|e| format!("receiver failure: {e:?}"))?
        {
            return Ok(event);
        }
        if Instant::now() >= end {
            return Err("callback timeout".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
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
    // Captured and checked against the installed framework by FrameworkBootstrapOracle.
    let code: u32 = std::env::args()
        .nth(1)
        .ok_or("missing captured transaction ID")?
        .parse()?;
    if code != 51 {
        return Err("unqualified framework transaction ID".into());
    }
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
    let (response, inbox) = new_response_receiver(NonZeroUsize::new(8).unwrap());
    let indication = Binder::new(IndicationHandle);
    let id = transport.register(
        &response.as_binder(),
        &indication.as_binder(),
        "pokelinux-ipc-test",
    )?;
    if id != 42 {
        return Err("client ID mismatch".into());
    }
    let initial = callback(&inbox)?;
    if initial.info.serial != 7 || initial.payload != [7] {
        return Err("registration callback mismatch".into());
    }
    println!("radio-registration\tPASS");
    for (offset, length) in [0, 2048, 2049, 4096].into_iter().enumerate() {
        let serial = 100 + i32::try_from(offset)?;
        let payload: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
        transport
            .send(serial, id, 137, 0, &payload)
            .map_err(|e| format!("submission failed: {e:?}"))?;
        let event = callback(&inbox)?;
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
    Ok(())
}
