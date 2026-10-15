//! Independent fixture controller; runtime evidence is documented separately.
//! Talks only to the fixture CONTROL object; never calls endpoint(), attaches,
//! dispatches setup, consumes owner inboxes, or calls a private reducer.
#[allow(dead_code)]
mod fixture_values;
use pokelinux_cellular_boundary::generated::org::pokelinux::datatest::{
    CaptureSetup::CaptureSetup, IDataFixture::IDataFixture,
};
use pokelinux_cellular_boundary::{RadioError, RadioResponseInfo, RadioResponseType};
use rsbinder::{
    FromIBinder, ProcessState, Proxy, Strong,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use serde_json::{Value, json};
use std::{fs::File, io::Read, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const CONTROL: &str = "org.pokelinux.fixture.cellular-data";
const CONTROL_DESCRIPTOR: &str = "org.pokelinux.datatest.IDataFixture";
const CONTROL_HASH: &str = "b74bae7e2fba9b8fbb574174a24e7431dbb6281b";
const MAX_EXPECTED: u64 = 16384;
const MAX_RECEIPT: usize = 4096;
fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(t.tv_nsec).unwrap())
        .unwrap()
}
fn require(value: bool, why: &'static str) -> Result<()> {
    if value { Ok(()) } else { Err(why.into()) }
}
fn before(until: u64) -> Result<()> {
    require(now() < until, "original cutoff")
}
fn request(c: &CaptureSetup) -> Value {
    json!({"serial":c.serial,"accessNetwork":c.accessNetwork.0,
      "dataProfileInfo":fixture_values::profile_json(&c.dataProfileInfo),
      "roamingAllowed":c.roamingAllowed,"reason":c.reason.0,
      "addresses":fixture_values::addresses_json(&c.addresses),"dnses":c.dnses,
      "pduSessionId":c.pduSessionId,"sliceInfo":fixture_values::slice_json(c.sliceInfo.as_ref()),
      "matchAllRuleAllowed":c.matchAllRuleAllowed})
}
fn check(c: &CaptureSetup, responses: i32, expected: &Value) -> Result<()> {
    require(
        c.attachmentAttempts == 1
            && c.attachmentsRetained == 1
            && c.setupAttempts == 1
            && c.setupPresent
            && c.responseAttempts == responses
            && c.responseSubmissions == responses
            && c.indicationAttempts == 0
            && c.indicationSubmissions == 0
            && c.forbiddenCalls == 0
            && c.rejectedCalls == 0
            && c.lastDataCallList.is_empty(),
        "exact one-attempt counters; no indication/reset/forbidden/refusal",
    )?;
    let worker = expected["roles"]["worker_uid"]
        .as_i64()
        .ok_or("worker UID expectation")?;
    require(
        c.attachmentCallingPid == 0
            && c.setupCallingPid == 0
            && i64::from(c.attachmentCallingUid) == worker
            && i64::from(c.setupCallingUid) == worker,
        "actual oneway PID0 and configured worker UID",
    )?;
    require(
        request(c) == expected["request"],
        "complete10argument capture /21profile fields",
    )?;
    if responses == 0 {
        require(
            c.lastResponseInfo.is_none() && c.lastResponse.is_none(),
            "no response yet",
        )?;
    } else {
        let i = c
            .lastResponseInfo
            .as_ref()
            .ok_or("response metadata absent")?;
        require(
            i.r#type == RadioResponseType::SOLICITED
                && i.serial == 713
                && i.error == RadioError::NONE,
            "exact ordinary final reply submission metadata",
        )?;
        let r = c.lastResponse.as_ref().ok_or("response absent")?;
        require(
            fixture_values::result_json(r) == expected["response"],
            "complete18field result capture",
        )?;
    }
    Ok(())
}
fn barrier(f: &Strong<dyn IDataFixture>, responses: i32, until: u64) -> Result<()> {
    before(until)?;
    let end = until.min(now().checked_add(1_000_000_000).ok_or("bounded barrier")?);
    f.barrier(1, 1, responses, 0, i64::try_from(end)?)?;
    before(until)
}
fn output(value: &Value, until: u64) -> Result<()> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let mut data = serde_json::to_vec(value)?;
    data.push(b'\n');
    require(data.len() <= MAX_RECEIPT, "bounded controller record")?;
    let fd = std::io::stdout();
    require(
        matches!(
            FileType::from_raw_mode(fstat(&fd)?.st_mode),
            FileType::Fifo | FileType::Socket
        ),
        "owned nonblocking output pipe/socket required",
    )?;
    let flags = fcntl_getfl(&fd)?;
    fcntl_setfl(&fd, flags | OFlags::NONBLOCK)?;
    before(until)?;
    require(
        rustix::io::write(&fd, &data)? == data.len(),
        "whole bounded output",
    )?;
    before(until)
}
fn run() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    require(a.len() == 10, "five exact option/value pairs")?;
    require(
        a[0] == "--device"
            && a[2] == "--generation"
            && a[4] == "--until-ns"
            && a[6] == "--expected"
            && a[8] == "--action",
        "fixed option order",
    )?;
    require(
        a[1].len() <= 128 && a[1].starts_with('/') && !a[1].contains(['\0', '\n', '\r']),
        "explicit private Binder device",
    )?;
    require(
        a[3].len() == 32
            && a[3]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "explicit source-pinned generation",
    )?;
    require(
        !a[5].is_empty() && a[5].len() <= 19 && a[5].bytes().all(|b| b.is_ascii_digit()),
        "absolute decimal BOOTTIME cutoff",
    )?;
    let until: u64 = a[5].parse()?;
    let started = now();
    before(until)?;
    require(
        until - started <= 120_000_000_000,
        "bounded original phase cutoff",
    )?;
    require(
        Path::new(&a[7]).is_absolute(),
        "absolute immutable expected artifact",
    )?;
    let mut bytes = Vec::new();
    File::open(&a[7])?
        .take(MAX_EXPECTED + 1)
        .read_to_end(&mut bytes)?;
    require(
        bytes.len() <= usize::try_from(MAX_EXPECTED)?,
        "bounded expected file",
    )?;
    let expected: Value = serde_json::from_slice(&bytes)?;
    require(
        expected["schema"] == 1 && expected["roles"]["worker_uid"] == 1500,
        "fixed source oracle profile",
    )?;
    require(
        rustix::process::getuid().as_raw() == 1000 && rustix::process::geteuid().as_raw() == 1000,
        "fixed controller UID1000",
    )?;
    require(
        fixture_values::result_json(&fixture_values::result()) == expected["response"],
        "authored response constructor equals frozen expected",
    )?;
    let action = &a[9];
    require(
        matches!(
            action.as_str(),
            "setup-proof" | "respond-once" | "post-restart-proof"
        ),
        "finite action",
    )?;
    before(until)?;
    let process = ProcessState::init(&a[1], 3)?;
    ProcessState::start_thread_pool(); // Process-scoped; actual wait/EOF, no pool-join claim.
    let context = process.context_object()?;
    require(
        context.is_remote() && context.descriptor() == "android.os.IServiceManager",
        "manager",
    )?;
    let manager = BpServiceManager::from_binder(context).ok_or("manager proxy")?;
    let binder = manager.checkService(CONTROL)?.ok_or("control missing")?;
    require(
        binder.is_remote() && binder.descriptor() == CONTROL_DESCRIPTOR,
        "control descriptor",
    )?;
    let f: Strong<dyn IDataFixture> = FromIBinder::try_from(binder)?;
    require(
        f.getInterfaceVersion()? == 1
            && f.getInterfaceHash()? == CONTROL_HASH
            && f.generation()? == a[3],
        "control version/hash/exact retained peer generation",
    )?;
    before(until)?;
    let responses = if action == "post-restart-proof" { 1 } else { 0 };
    barrier(&f, responses, until)?;
    let first = f.capture()?;
    before(until)?;
    check(&first, responses, &expected)?;
    let final_responses = if action == "respond-once" {
        let info = RadioResponseInfo {
            r#type: RadioResponseType::SOLICITED,
            serial: 713,
            error: RadioError::NONE,
        };
        before(until)?;
        f.setupResponse(&info, &fixture_values::result())?;
        barrier(&f, 1, until)?;
        1
    } else {
        responses
    };
    let captured = f.capture()?;
    before(until)?;
    check(&captured, final_responses, &expected)?;
    let completed = now();
    before(until)?;
    let receipt = json!({"schema":1,"event":"cellular-fixture-control","action":action,
      "generation":a[3],"controller_pid":std::process::id(),"controller_uid":1000,
      "attachment_attempts":1,"attachments_retained":1,"setup_attempts":1,
      "attachment_calling_pid":captured.attachmentCallingPid,
      "attachment_calling_uid":captured.attachmentCallingUid,
      "setup_calling_pid":captured.setupCallingPid,"setup_calling_uid":captured.setupCallingUid,
      "response_attempts":final_responses,"response_submissions":final_responses,
      "indication_attempts":0,"indication_submissions":0,"forbidden_calls":0,"rejected_calls":0,
      "request":request(&captured),
      "response":captured.lastResponse.as_ref().map(fixture_values::result_json),
      "response_submission_is_owner_receipt":false,"production_endpoint_obtained":false,
      "started_at_ns":started,"completed_at_ns":completed,"original_until_ns":until,
      "Binder_pool_join_claimed":false});
    drop(f);
    drop(manager);
    output(&receipt, until)
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
