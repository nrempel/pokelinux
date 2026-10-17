//! Independent fixture-control-only consumer. No radio owner or encoder imports.
//! Never registers callbacks, sends request105, resets capture, clears or retries.
use rsbinder::{
    Parcel, ProcessState, Proxy, SIBinder, Status,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use serde_json::{Value, json};
use std::{error::Error, io};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why).into())
    }
}
fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(t.tv_nsec).unwrap())
        .unwrap()
}
fn before(until: u64) -> Result<()> {
    require(now() < until, "original BOOTTIME cutoff")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn call(peer: &SIBinder, code: u32) -> Result<Parcel> {
    let remote = peer.as_remote().ok_or("local fixture endpoint")?;
    let input = remote.prepare_transact(true)?;
    let mut output = remote
        .submit_transact(code, &input, 0)?
        .ok_or("missing fixture reply")?;
    let status: Status = output.read()?;
    require(status.is_ok(), &format!("fixture returned {status:?}"))?;
    Ok(output)
}
fn output(value: Value, until: u64) -> Result<()> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let fd = std::io::stdout();
    require(
        FileType::from_raw_mode(fstat(&fd)?.st_mode) == FileType::Fifo,
        "owned output PIPE",
    )?;
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    require(bytes.len() <= 4096, "bounded controller output")?;
    fcntl_setfl(&fd, fcntl_getfl(&fd)? | OFlags::NONBLOCK)?;
    before(until)?;
    require(
        rustix::io::write(&fd, &bytes)? == bytes.len(),
        "whole bounded output",
    )?;
    before(until)
}
fn run() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    require(a.len() == 2, "action plus original absolute cutoff")?;
    let action = &a[0];
    let until: u64 = a[1].parse()?;
    before(until)?;
    require(
        until - now() <= 60_000_000_000,
        "bounded fixture action budget",
    )?;
    require(
        rustix::process::getuid().as_raw() == 1000 && rustix::process::geteuid().as_raw() == 1000,
        "independent fixture controller UID1000",
    )?;
    require(
        matches!(
            action.as_str(),
            "report" | "capture" | "140" | "141" | "142" | "143" | "144" | "145" | "146"
        ),
        "fixture control-only allowlist",
    )?;
    let process = ProcessState::init("/dev/binderfs/radio", 2)?;
    ProcessState::start_thread_pool();
    let manager =
        BpServiceManager::from_binder(process.context_object()?).ok_or("manager interface")?;
    let peer = manager
        .checkService("org.pokelinux.fixture.arm64-radio")?
        .ok_or("named fixture absent")?;
    require(
        peer.is_remote()
            && peer.descriptor()
                == "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal",
        "exact fixture descriptor",
    )?;
    before(until)?;
    let value = if action == "capture" {
        let mut p = call(&peer, 127)?;
        let version = p.read_i32()?;
        let epoch = p.read_i32()?;
        let count = p.read_i32()?;
        let total = p.read_i32()?;
        let serial = p.read_i32()?;
        let client = p.read_i32()?;
        let request = p.read_i32()?;
        let slot = p.read_i32()?;
        let owner_pid = p.read_i32()?;
        let owner_uid = p.read_i32()?;
        let calling_pid = p.read_i32()?;
        let calling_uid = p.read_i32()?;
        let bytes: Vec<u8> = p.read()?;
        require(
            p.data_avail() == 0 && version == 1 && bytes.len() <= 2048,
            "complete bounded capture reply",
        )?;
        json!({"kind":"capture","controller_uid":1000,"ok":true,"version":version,
            "epoch":epoch,"count":count,"total":total,"serial":serial,"client_id":client,
            "request":request,"slot":slot,"owner_pid":owner_pid,"owner_uid":owner_uid,
            "calling_pid":calling_pid,"calling_uid":calling_uid,"payload_hex":hex(&bytes)})
    } else if action == "report" {
        let mut p = call(&peer, 100)?;
        let mut v = [0; 6];
        for x in &mut v {
            *x = p.read_i32()?;
        }
        require(p.data_avail() == 0, "complete fixture counter reply")?;
        json!({"kind":"report","controller_uid":1000,"ok":true,"registers":v[0],
            "raw_other":v[1],"segments":v[2],"owner_pid":v[3],"owner_uid":v[4],"clears":v[5]})
    } else {
        let code: u32 = action.parse()?;
        match call(&peer, code) {
            Ok(mut p) => {
                require(
                    p.read_i32()? == 1 && p.data_avail() == 0,
                    "actual receiver-return ACK",
                )?;
                json!({"kind":action,"controller_uid":1000,"ok":true,"receiver_returned":true})
            }
            Err(error) => json!({"kind":action,"controller_uid":1000,"ok":false,
                "receiver_returned":false,"actual_control_error":error.to_string()}),
        }
    };
    before(until)?;
    output(value, until)
}
fn main() {
    if let Err(error) = run() {
        // No unbounded stderr diagnostic after a timeout; owning harness records70.
        let _ = error;
        std::process::exit(70);
    }
}
