//! Independent FIXTURE CAPABILITY prerequisite only; no native provider/worker.
//! Authored fixture probe; runtime evidence is documented separately.
use pokelinux_cellular_boundary::IRadioData;
use pokelinux_cellular_boundary::generated::org::pokelinux::datatest::{
    CaptureSetup::CaptureSetup, IDataFixture::IDataFixture,
};
use rsbinder::{
    FromIBinder, ProcessState, Proxy, StatusCode, Strong,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use serde_json::{Value, json};
use std::time::Duration;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const CONTROL: &str = "org.pokelinux.fixture.cellular-data";
const CONTROL_DESCRIPTOR: &str = "org.pokelinux.datatest.IDataFixture";
const CONTROL_HASH: &str = "b74bae7e2fba9b8fbb574174a24e7431dbb6281b";
const NAME: &str = "android.hardware.radio.data.IRadioData/slot1";
const DESCRIPTOR: &str = "android.hardware.radio.data.IRadioData";
const DATA_HASH: &str = "cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df";
const MAX_RECORD: usize = 4096;
fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(t.tv_nsec).unwrap())
        .unwrap()
}
fn need(value: bool, why: &'static str) -> Result<()> {
    if value { Ok(()) } else { Err(why.into()) }
}
fn before(until: u64) -> Result<()> {
    need(now() < until, "original BOOTTIME cutoff")
}
fn zero(c: &CaptureSetup) -> Result<()> {
    need(
        [
            c.attachmentAttempts,
            c.attachmentsRetained,
            c.setupAttempts,
            c.responseAttempts,
            c.responseSubmissions,
            c.indicationAttempts,
            c.indicationSubmissions,
            c.forbiddenCalls,
            c.rejectedCalls,
        ]
        .into_iter()
        .all(|v| v == 0)
            && !c.setupPresent
            && c.lastResponseInfo.is_none()
            && c.lastResponse.is_none()
            && c.lastDataCallList.is_empty(),
        "all real capture counters zero / no attachment or setup",
    )
}
fn emit(record: &Value, until: u64) -> Result<()> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let mut bytes = serde_json::to_vec(record)?;
    bytes.push(b'\n');
    need(bytes.len() <= MAX_RECORD, "bounded JSON record")?;
    let fd = std::io::stdout();
    need(
        matches!(
            FileType::from_raw_mode(fstat(&fd)?.st_mode),
            FileType::Fifo | FileType::Socket
        ),
        "owned stdout pipe/socket",
    )?;
    fcntl_setfl(&fd, fcntl_getfl(&fd)? | OFlags::NONBLOCK)?;
    before(until)?;
    need(
        rustix::io::write(&fd, &bytes)? == bytes.len(),
        "one whole nonblocking JSON write",
    )?;
    before(until)
}
fn ack(until: u64) -> Result<u64> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let fd = std::io::stdin();
    need(
        FileType::from_raw_mode(fstat(&fd)?.st_mode) == FileType::Fifo,
        "owned stdin ACK pipe",
    )?;
    fcntl_setfl(&fd, fcntl_getfl(&fd)? | OFlags::NONBLOCK)?;
    let mut got = false;
    loop {
        before(until)?;
        let mut bytes = [0_u8; 2];
        match rustix::io::read(&fd, &mut bytes) {
            Ok(0) => {
                need(got, "ACK pipe closed without G")?;
                before(until)?;
                return Ok(now());
            }
            Ok(1) if !got && bytes[0] == b'G' => got = true,
            Ok(_) => return Err("exact single G then EOF required".into()),
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(e) => return Err(e.into()),
        }
        before(until)?;
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn run() -> Result<bool> {
    need(std::env::args_os().len() == 1, "no argv")?;
    let generation = std::env::var("POKELINUX_DATA_INTEROP_GENERATION")?;
    need(
        generation.len() == 32
            && generation
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "explicit32hex generation",
    )?;
    let device = std::env::var("POKELINUX_DATA_INTEROP_DEVICE")?;
    need(
        device.len() <= 128 && device.starts_with('/') && !device.contains(['\0', '\r', '\n']),
        "explicit private Binder device",
    )?;
    let text = std::env::var("POKELINUX_DATA_INTEROP_UNTIL_NS")?;
    need(
        !text.is_empty() && text.len() <= 19 && text.bytes().all(|b| b.is_ascii_digit()),
        "decimal original cutoff",
    )?;
    let until: u64 = text.parse()?;
    let started = now();
    before(until)?;
    need(until - started <= 120_000_000_000, "finite original cutoff")?;
    need(
        rustix::process::getuid().as_raw() == 1000
            && rustix::process::geteuid().as_raw() == 1000
            && rustix::process::getgid().as_raw() == 1000
            && rustix::process::getegid().as_raw() == 1000,
        "actual UID/GID1000",
    )?;
    let status = std::fs::read_to_string("/proc/self/status")?;
    need(
        status.lines().any(|v| v == "CapEff:\t0000000000000000")
            && status
                .lines()
                .any(|v| v.starts_with("Groups:") && v[7..].trim().is_empty()),
        "groups[] caps0",
    )?;
    let process = ProcessState::init(&device, 3)?;
    ProcessState::start_thread_pool(); // process-scoped, no pool/thread-join claim.
    let context = process.context_object()?;
    need(
        context.is_remote() && context.descriptor() == "android.os.IServiceManager",
        "actual context manager",
    )?;
    let manager = BpServiceManager::from_binder(context.clone()).ok_or("manager proxy")?;
    before(until)?;
    let control = manager.checkService(CONTROL)?.ok_or("control missing")?;
    need(
        control.is_remote() && control != context && control.descriptor() == CONTROL_DESCRIPTOR,
        "actual control binder",
    )?;
    let fixture: Strong<dyn IDataFixture> = FromIBinder::try_from(control)?;
    let cv = fixture.getInterfaceVersion()?;
    let ch = fixture.getInterfaceHash()?;
    need(
        cv == 1 && ch == CONTROL_HASH && fixture.generation()? == generation,
        "actual control metadata/generation",
    )?;
    let reference = fixture.endpoint()?; // Independent TEST CONTROL only; never production ingress.
    let reference_binder = reference.as_binder();
    need(
        reference_binder.is_remote() && reference_binder.descriptor() == DESCRIPTOR,
        "reference remote Data",
    )?;
    let rv = reference.getInterfaceVersion()?;
    let rh = reference.getInterfaceHash()?;
    need(
        rv == 3 && rh == DATA_HASH,
        "actual reference Data3 metadata",
    )?;
    zero(&fixture.capture()?)?;
    before(until)?;
    let ready_at = now();
    emit(
        &json!({"schema":1,"event":"named-data-probe-ready","generation":generation,
        "pid":std::process::id(),"uid":1000,"gid":1000,"groups":[],"cap_eff":"0000000000000000",
        "control_version":cv,"control_hash":ch,"reference_remote":true,
        "reference_data_version":rv,"reference_data_hash":rh,"all_counters_zero":true,
        "standard_lookup_started":false,"ready_at_ns":ready_at,"original_until_ns":until}),
        until,
    )?;
    let ack_at = ack(until)?;
    let lookup_started = now();
    before(until)?;
    // The existing narrow manager naturally returns BadValue here. No artificial
    // Unsupported or injected missing-entry result is used by this probe.
    let lookup = (|| -> Result<(bool, Value)> {
        match manager.checkService(NAME) {
            Err(error) if error.transaction_error() == StatusCode::BadValue => Ok((
                false,
                json!({"kind":"bad-value","transaction_error":"BadValue"}),
            )),
            Err(error) => Err(format!("unexpected standard lookup error: {error:?}").into()),
            Ok(None) => Ok((false, json!({"kind":"absent"}))),
            Ok(Some(binder)) => {
                need(
                    binder.is_remote() && binder.descriptor() == DESCRIPTOR,
                    "standard remote Data descriptor",
                )?;
                need(
                    binder.eq(&reference_binder),
                    "same exact remote SIBinder object as test reference",
                )?;
                let endpoint: Strong<dyn IRadioData> = FromIBinder::try_from(binder)?;
                let version = endpoint.getInterfaceVersion()?;
                let hash = endpoint.getInterfaceHash()?;
                need(version == 3 && hash == DATA_HASH, "standard Data3 metadata")?;
                Ok((
                    true,
                    json!({"kind":"same-object","remote":true,"descriptor":DESCRIPTOR,
                    "version":version,"hash":hash,"same_binder_as_control_reference":true}),
                ))
            }
        }
    })();
    let lookup_completed = now();
    before(until)?;
    let counters = (|| -> Result<()> { zero(&fixture.capture()?) })();
    before(until)?;
    let die_started = now();
    let die_result = fixture.die(); // BOTH ordinary missing-entry and same-object paths.
    let die_return = json!({"returned_ok":die_result.is_ok(),
        "transaction_error":die_result.as_ref().err().map(|e|format!("{:?}",e.transaction_error()))});
    let die_completed = now();
    before(until)?;
    counters?;
    let (present, actual) = lookup?;
    drop(reference);
    drop(reference_binder);
    drop(fixture);
    drop(manager);
    drop(context);
    let closed = now();
    before(until)?;
    emit(
        &json!({"schema":1,"event":"named-data-probe-complete","generation":generation,
        "pid":std::process::id(),"lookup":actual,"passed":present,"missing_named_entry":!present,
        "fixture_control_qualified":true,"reference_Data3_qualified":true,
        "all_counters_zero_before_after":true,"fixture_die_invoked":true,"fixture_die_return":die_return,
        "ready_at_ns":ready_at,"ack_at_ns":ack_at,"lookup_started_at_ns":lookup_started,
        "lookup_completed_at_ns":lookup_completed,"die_started_at_ns":die_started,
        "die_completed_at_ns":die_completed,"closed_at_ns":closed,"started_at_ns":started,
        "original_until_ns":until,"native_provider_executed":false,"callbacks_attached":false,
        "setup_dispatched":false,"kernel_observer_executed":false,"Binder_pool_join_claimed":false}),
        until,
    )?;
    Ok(present)
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(101),
        Err(error) => {
            if let Ok(raw) = std::env::var("POKELINUX_DATA_INTEROP_UNTIL_NS")
                && let Ok(until) = raw.parse::<u64>()
            {
                let _ = emit(
                    &json!({"schema":1,"event":"named-data-probe-failed",
                    "classification":"prerequisite-or-closure-failure-not-RED",
                    "error":error.to_string().chars().take(256).collect::<String>()}),
                    until,
                );
            }
            std::process::ExitCode::FAILURE
        }
    }
}
