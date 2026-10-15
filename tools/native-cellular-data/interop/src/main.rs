//! ABI prerequisite only. No native Data owner, APN context, or kernel observation.
mod callbacks;
mod typed;
use callbacks::{Death, Event, Indication, Response, Shared};
use pokelinux_cellular_boundary::generated::{
    android::hardware::radio::{
        RadioIndicationType::RadioIndicationType, data::DataRequestReason::DataRequestReason,
    },
    org::pokelinux::datatest::{CaptureSetup::CaptureSetup, IDataFixture::IDataFixture},
};
use pokelinux_cellular_boundary::{
    AccessNetwork, BnRadioDataIndication, BnRadioDataResponse, RadioError, RadioResponseInfo,
    RadioResponseType,
};
use rsbinder::{
    FromIBinder, ProcessState, Proxy, SIBinder, Strong,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::Duration,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const NAME: &str = "org.pokelinux.fixture.cellular-data";
const DESCRIPTOR: &str = "org.pokelinux.datatest.IDataFixture";
const DATA_HASH: &str = "cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df";
const FIXTURE_HASH: &str = "b74bae7e2fba9b8fbb574174a24e7431dbb6281b";
const SERIAL: i32 = 713;
fn boot_ns() -> i64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    t.tv_sec
        .checked_mul(1_000_000_000)
        .and_then(|s| s.checked_add(t.tv_nsec))
        .expect("bounded boot time")
}
fn require(value: bool, message: &'static str) -> Result<()> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn before(until: i64) -> Result<()> {
    require(boot_ns() < until, "original BOOTTIME cutoff")
}
fn barrier(f: &Strong<dyn IDataFixture>, a: i32, s: i32, r: i32, i: i32, until: i64) -> Result<()> {
    before(until)?;
    f.barrier(a, s, r, i, until.min(boot_ns() + 1_000_000_000))?;
    before(until)
}
fn counts(c: &CaptureSetup, a: i32, s: i32, r: i32, i: i32) -> Result<()> {
    require(
        c.attachmentAttempts == a
            && c.attachmentsRetained == a
            && c.setupAttempts == s
            && c.setupPresent == (s == 1)
            && c.responseAttempts == r
            && c.responseSubmissions == r
            && c.indicationAttempts == i
            && c.indicationSubmissions == i
            && c.forbiddenCalls == 0
            && c.rejectedCalls == 0,
        "exact fixture counters",
    )
}
fn receive(rx: &Receiver<Event>, shared: &Shared, until: i64) -> Result<Event> {
    loop {
        before(until)?;
        require(
            !shared.fault.load(Ordering::SeqCst),
            "callback refusal/overflow",
        )?;
        match rx.try_recv() {
            Ok(event) => return Ok(event),
            Err(TryRecvError::Disconnected) => return Err("callback queue disconnected".into()),
            Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}
fn empty(rx: &Receiver<Event>, shared: &Shared) -> Result<()> {
    require(
        !shared.fault.load(Ordering::SeqCst),
        "callback refusal/overflow",
    )?;
    require(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "unexpected callback",
    )
}
struct Watch {
    binder: SIBinder,
    death: Arc<Death>,
    shared: Arc<Shared>,
}
impl Drop for Watch {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        let _ = self.binder.unlink_to_death_arc(&self.death);
    }
}
fn setup_event(event: Event, serial: i32, uid: u32) -> Result<Value> {
    let Event::Setup(packet, caller) = event else {
        return Err("expected setup callback".into());
    };
    let (info, result) = *packet;
    require(
        info.r#type == RadioResponseType::SOLICITED
            && info.serial == serial
            && info.error == RadioError::NONE
            && caller == uid,
        "actual response metadata/UID",
    )?;
    require(typed::bounded_result(&result), "bounded callback payload")?;
    let value = typed::result_json(&result);
    require(
        value == typed::result_json(&typed::result()),
        "full actual setup result",
    )?;
    Ok(value)
}
fn run() -> Result<Value> {
    require(std::env::args_os().len() == 1, "no command line arguments")?;
    let generation = std::env::var("POKELINUX_DATA_INTEROP_GENERATION")?;
    require(
        generation.len() == 32
            && generation
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "explicit generation",
    )?;
    let raw_until = std::env::var("POKELINUX_DATA_INTEROP_UNTIL_NS")?;
    require(
        raw_until.len() <= 19
            && !raw_until.is_empty()
            && raw_until.bytes().all(|b| b.is_ascii_digit()),
        "absolute until required",
    )?;
    let until: i64 = raw_until.parse()?;
    let started = boot_ns();
    require(
        until > started && until - started <= 120_000_000_000,
        "original until range",
    )?;
    let device = std::env::var("POKELINUX_DATA_INTEROP_DEVICE")?;
    require(
        device.len() <= 128 && device.starts_with('/') && !device.contains('\0'),
        "explicit private Binder device",
    )?;
    let uid = rustix::process::geteuid().as_raw();
    require(
        rustix::process::getuid().as_raw() == uid,
        "fixture real/effective UID equality",
    )?;
    let process = ProcessState::init(&device, 4)?;
    ProcessState::start_thread_pool(); // Process-scoped fixture pool, no owned-pool join claim.
    let context = process.context_object()?;
    require(
        context.is_remote() && context.descriptor() == "android.os.IServiceManager",
        "actual context manager",
    )?;
    let manager = BpServiceManager::from_binder(context.clone()).ok_or("manager proxy")?;
    before(until)?;
    let binder = manager.checkService(NAME)?.ok_or("named fixture missing")?;
    require(
        binder.is_remote() && binder != context && binder.descriptor() == DESCRIPTOR,
        "actual transferred fixture",
    )?;
    let fixture: Strong<dyn IDataFixture> = FromIBinder::try_from(binder)?;
    require(
        fixture.getInterfaceVersion()? == 1
            && fixture.getInterfaceHash()? == FIXTURE_HASH
            && fixture.generation()? == generation,
        "fixture metadata/generation",
    )?;
    before(until)?;
    counts(&fixture.capture()?, 0, 0, 0, 0)?;
    let endpoint = fixture.endpoint()?;
    let endpoint_binder = endpoint.as_binder();
    require(
        endpoint_binder.is_remote()
            && endpoint_binder.descriptor() == "android.hardware.radio.data.IRadioData"
            && endpoint.getInterfaceVersion()? == 3
            && endpoint.getInterfaceHash()? == DATA_HASH,
        "actual Data3 metadata",
    )?;
    let (queue, received) = mpsc::sync_channel(8);
    let shared = Arc::new(Shared {
        queue,
        fault: AtomicBool::new(false),
        closed: AtomicBool::new(false),
    });
    let death = Arc::new(Death(shared.clone()));
    endpoint_binder.link_to_death_arc(&death)?;
    let watch = Watch {
        binder: endpoint_binder,
        death,
        shared: shared.clone(),
    };
    let response = BnRadioDataResponse::new_binder(Response(shared.clone()));
    let indication = BnRadioDataIndication::new_binder(Indication(shared.clone()));
    before(until)?;
    endpoint.setResponseFunctions(&response, &indication)?;
    barrier(&fixture, 1, 0, 0, 0, until)?;
    let attached = fixture.capture()?;
    counts(&attached, 1, 0, 0, 0)?;
    require(
        attached.attachmentCallingPid == 0 && attached.attachmentCallingUid == i32::try_from(uid)?,
        "actual one-way attachment caller",
    )?;
    let profile = typed::profile();
    let addresses = typed::addresses();
    let dnses = typed::dnses();
    let slice = typed::slice();
    before(until)?;
    endpoint.setupDataCall(
        SERIAL,
        AccessNetwork::NGRAN,
        &profile,
        false,
        DataRequestReason::NORMAL,
        &addresses,
        &dnses,
        7,
        Some(&slice),
        true,
    )?;
    barrier(&fixture, 1, 1, 0, 0, until)?;
    let captured = fixture.capture()?;
    counts(&captured, 1, 1, 0, 0)?;
    require(
        captured.serial == SERIAL
            && captured.accessNetwork == AccessNetwork::NGRAN
            && typed::profile_json(&captured.dataProfileInfo) == typed::profile_json(&profile)
            && !captured.roamingAllowed
            && captured.reason == DataRequestReason::NORMAL
            && typed::addresses_json(&captured.addresses) == typed::addresses_json(&addresses)
            && captured.dnses == dnses
            && captured.pduSessionId == 7
            && typed::slice_json(captured.sliceInfo.as_ref()) == typed::slice_json(Some(&slice))
            && captured.matchAllRuleAllowed
            && captured.setupCallingPid == 0
            && captured.setupCallingUid == i32::try_from(uid)?,
        "all ten captured setup arguments and actual UID",
    )?;
    let request_json = json!({"serial":captured.serial,"accessNetwork":captured.accessNetwork.0,
        "dataProfileInfo":typed::profile_json(&captured.dataProfileInfo),"roamingAllowed":captured.roamingAllowed,
        "reason":captured.reason.0,"addresses":typed::addresses_json(&captured.addresses),"dnses":captured.dnses,
        "pduSessionId":captured.pduSessionId,"sliceInfo":typed::slice_json(captured.sliceInfo.as_ref()),
        "matchAllRuleAllowed":captured.matchAllRuleAllowed});
    // A finite no-callback observation, not a native pending-operation reducer.
    let hold_started = boot_ns();
    let hold_until = hold_started + 100_000_000;
    require(hold_until < until, "hold fits original budget")?;
    while boot_ns() < hold_until {
        empty(&received, &shared)?;
        before(until)?;
        std::thread::sleep(Duration::from_millis(1));
    }
    empty(&received, &shared)?;
    let hold_observed = boot_ns();
    let wrong = RadioResponseInfo {
        r#type: RadioResponseType::SOLICITED,
        serial: SERIAL + 1,
        error: RadioError::NONE,
    };
    before(until)?;
    fixture.setupResponse(&wrong, &typed::result())?;
    barrier(&fixture, 1, 1, 1, 0, until)?;
    let _ = setup_event(receive(&received, &shared, until)?, SERIAL + 1, uid)?;
    let right = RadioResponseInfo {
        r#type: RadioResponseType::SOLICITED,
        serial: SERIAL,
        error: RadioError::NONE,
    };
    before(until)?;
    fixture.setupResponse(&right, &typed::result())?;
    barrier(&fixture, 1, 1, 2, 0, until)?;
    let response_json = setup_event(receive(&received, &shared, until)?, SERIAL, uid)?;
    before(until)?;
    fixture.listChanged(&[typed::result()])?;
    barrier(&fixture, 1, 1, 2, 1, until)?;
    let Event::List(kind, list, caller) = receive(&received, &shared, until)? else {
        return Err("expected populated list".into());
    };
    require(
        kind == RadioIndicationType::UNSOLICITED
            && caller == uid
            && list.len() == 1
            && typed::result_json(&list[0]) == response_json,
        "full copied populated list",
    )?;
    before(until)?;
    fixture.listChanged(&[])?;
    barrier(&fixture, 1, 1, 2, 2, until)?;
    let Event::List(kind, list, caller) = receive(&received, &shared, until)? else {
        return Err("expected empty list".into());
    };
    require(
        kind == RadioIndicationType::UNSOLICITED && caller == uid && list.is_empty(),
        "full copied empty list",
    )?;
    let final_capture = fixture.capture()?;
    counts(&final_capture, 1, 1, 2, 2)?;
    let info = final_capture
        .lastResponseInfo
        .as_ref()
        .ok_or("captured response info")?;
    let result = final_capture
        .lastResponse
        .as_ref()
        .ok_or("captured response result")?;
    require(
        info.r#type == RadioResponseType::SOLICITED
            && info.serial == SERIAL
            && info.error == RadioError::NONE
            && typed::bounded_result(result)
            && typed::result_json(result) == response_json
            && final_capture.lastDataCallList.is_empty(),
        "complete final peer attempt history",
    )?;
    empty(&received, &shared)?;
    before(until)?;
    let _ = fixture.die();
    require(
        matches!(receive(&received, &shared, until)?, Event::Died),
        "actual endpoint deathwatch",
    )?;
    let death_observed = boot_ns();
    empty(&received, &shared)?;
    drop(watch);
    drop(response);
    drop(indication);
    drop(endpoint);
    drop(fixture);
    let watch_dropped = boot_ns();
    before(until)?;
    let completed = boot_ns();
    require(completed < until, "completed before original cutoff")?;
    Ok(
        json!({"schema":1,"event":"cellular-data-interop-complete","passed":true,"generation":generation,
        "data_version":3,"data_hash":DATA_HASH,"fixture_version":1,"fixture_hash":FIXTURE_HASH,
        "client_pid":std::process::id(),"client_uid":uid,"oneway_setup_calling_pid":0,
        "oneway_setup_calling_uid":uid,"attachments":1,"setups":1,"response_copies":2,"list_copies":2,
        "withheld_no_callback":true,"hold_started_ns":hold_started,"hold_until_ns":hold_until,
        "hold_observed_at_ns":hold_observed,"wrong_serial_copied":SERIAL+1,"right_serial_copied":SERIAL,
        "request":request_json,"copied_response":response_json,"populated_list_full_equal":true,"empty_list_copied":true,
        "deathwatch_observed_at_ns":death_observed,"watch_dropped_at_ns":watch_dropped,
        "started_at_ns":started,"original_until_ns":until,"completed_at_ns":completed,
        "native_owner_executed":false,"kernel_observer_executed":false,"APN_acquisition_qualified":false,
        "ACK_EXP_qualified":false,"Binder_pool_join_claimed":false,"native_transport_worker_join_claimed":false}),
    )
}
fn output(value: &Value) -> Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    require(line.len() <= 16384, "bounded JSON receipt")?;
    let until = if value.get("passed").and_then(Value::as_bool) == Some(true) {
        Some(
            value
                .get("original_until_ns")
                .and_then(Value::as_i64)
                .ok_or("receipt until")?,
        )
    } else {
        None
    };
    if let Some(until) = until {
        before(until)?;
    }
    let out = std::io::stdout();
    let flags = rustix::fs::fcntl_getfl(&out)?;
    rustix::fs::fcntl_setfl(&out, flags | rustix::fs::OFlags::NONBLOCK)?;
    require(
        rustix::io::write(&out, &line)? == line.len(),
        "one complete nonblocking receipt write",
    )?;
    if let Some(until) = until {
        before(until)?;
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(value) => {
            if output(&value).is_ok() {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::from(74)
            }
        }
        Err(error) => {
            eprintln!(
                "DATA-INTEROP-FAIL {}",
                error.to_string().chars().take(512).collect::<String>()
            );
            let _ = output(
                &json!({"schema":1,"event":"cellular-data-interop-failed","passed":false,
                "classification":"ABI/setup prerequisite failure; no native behavioral RED"}),
            );
            std::process::ExitCode::FAILURE
        }
    }
}
