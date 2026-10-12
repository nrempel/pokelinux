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
    let callbacks = new_radio_callbacks(NonZeroUsize::new(4).unwrap());
    let inbox = &callbacks.events;
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
    Ok(())
}
