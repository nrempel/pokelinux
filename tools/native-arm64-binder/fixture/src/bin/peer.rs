//! Independent synthetic native peer. No radio library/bindings are used here.
//! The manually authored vendor-shaped parcels are fixture data, not original interop.
use rsbinder::{
    Binder, FLAG_ONEWAY, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status,
    StatusCode, thread_state,
};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
const RADIO: &str = "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";
const RESPONSE: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalRes";
const INDICATION: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalInd";
#[derive(Default)]
struct State {
    response: Option<SIBinder>,
    registers: i32,
    raw: i32,
    segments: i32,
    calling_pid: i32,
    calling_uid: i32,
    clears: i32,
    bytes: Vec<u8>,
    serial: i32,
    registration_entered: bool,
    registration_released: bool,
    hold_request: bool,
    request_entered: bool,
    request_released: bool,
}
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
impl Shared {
    fn wait(&self, predicate: impl FnMut(&mut State) -> bool) -> Result<()> {
        let state = self.state.lock().unwrap();
        let (_state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(10), predicate)
            .unwrap();
        if timeout.timed_out() {
            Err(StatusCode::TimedOut)
        } else {
            Ok(())
        }
    }
}
struct Radio(Arc<Shared>);
struct Bootstrap(SIBinder);
fn response(callback: &SIBinder, serial: i32, bytes: &[u8]) -> Result<()> {
    let remote = callback.as_remote().ok_or(StatusCode::BadType)?;
    let mut data = remote.prepare_transact(true)?;
    data.write_i32(1)?; // nullable parcelable presence
    data.write_i32(24)?; // sized five-i32 struct including its size
    for value in [0, serial, 137, bytes.len() as i32, 0] {
        data.write_i32(value)?;
    }
    data.write(bytes)?;
    remote.submit_transact(1, &data, FLAG_ONEWAY)?;
    println!(
        "CALLBACK-SUBMITTED target_pid={} serial={} bytes={}",
        thread_state::get_calling_pid(),
        serial,
        bytes.len()
    );
    std::io::Write::flush(&mut std::io::stdout())?;
    Ok(())
}
fn indication(callback: &SIBinder) -> Result<()> {
    let remote = callback.as_remote().ok_or(StatusCode::BadType)?;
    let mut data = remote.prepare_transact(true)?;
    data.write_i32(33)?;
    data.write_i32(0)?;
    data.write(&[1u8, 2, 3][..])?;
    remote.submit_transact(1, &data, FLAG_ONEWAY)?;
    Ok(())
}
fn ack(reply: &mut Parcel) -> Result<()> {
    reply.write(&Status::from(StatusCode::Ok))?;
    reply.write_i32(1)
}
impl Remotable for Bootstrap {
    fn descriptor() -> &'static str {
        "org.pokelinux.fixture.ArmBinderBootstrap"
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        if code != 1 {
            return Err(StatusCode::UnknownTransaction);
        }
        if data.data_avail() != 0 {
            return Err(StatusCode::BadValue);
        }
        reply.write(&Status::from(StatusCode::Ok))?;
        reply.write(&self.0)
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
impl Remotable for Radio {
    fn descriptor() -> &'static str {
        RADIO
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        match code {
            5 => {
                let res: SIBinder = data.read()?;
                let ind: SIBinder = data.read()?;
                let client: String = data.read()?;
                if data.data_avail() != 0
                    || !res.is_remote()
                    || !ind.is_remote()
                    || res.descriptor() != RESPONSE
                    || ind.descriptor() != INDICATION
                {
                    return Err(StatusCode::BadValue);
                }
                {
                    let mut state = self.0.state.lock().unwrap();
                    state.registers += 1;
                    state.response = Some(res.clone());
                    state.calling_pid = thread_state::get_calling_pid();
                    state.calling_uid = thread_state::get_calling_uid() as i32;
                    println!(
                        "REGISTER pid={} uid={} response_remote={} indication_remote={}",
                        state.calling_pid,
                        state.calling_uid,
                        res.is_remote(),
                        ind.is_remote()
                    );
                }
                if client == "die-register" {
                    std::process::exit(0);
                }
                if client == "hold" || client == "permanent" {
                    let mut state = self.0.state.lock().unwrap();
                    state.registration_entered = true;
                    self.0.changed.notify_all();
                    if client == "permanent" {
                        // No timeout and no release control for this branch. Guest owns peer teardown.
                        loop {
                            state = self.0.changed.wait(state).unwrap();
                        }
                    }
                    drop(state);
                    self.0.wait(|state| !state.registration_released)?;
                    response(&res, 77, &[77])?; // Late callback must not revive cancelled/expired owner.
                }
                if client == "callbacks" {
                    let remote = res.as_remote().unwrap();
                    let input = remote.prepare_transact(true)?;
                    let mut output = remote
                        .submit_transact(16777215, &input, 0)?
                        .ok_or(StatusCode::UnexpectedNull)?;
                    let status: Status = output.read()?;
                    if !status.is_ok() || output.read_i32()? != 1 || output.data_avail() != 0 {
                        return Err(StatusCode::BadValue);
                    }
                    response(&res, 7, &[7])?;
                    indication(&ind)?;
                } else if !client.starts_with("hold") && client != "permanent" {
                    response(&res, 7, &[7])?;
                }
                reply.write(&Status::from(StatusCode::Ok))?;
                reply.write_i32(42)
            }
            2 | 3 => {
                let serial = data.read_i32()?;
                if data.read_i32()? != 42 || data.read_i32()? != 137 || data.read_i32()? != 0 {
                    return Err(StatusCode::BadValue);
                }
                let bytes: Vec<u8> = data.read()?;
                if bytes.len() > 2048 {
                    return Err(StatusCode::BadValue);
                }
                let completed = {
                    let mut state = self.0.state.lock().unwrap();
                    if code == 2 {
                        state.raw += 1;
                        Some(bytes)
                    } else {
                        let index = data.read_i32()?;
                        let total = data.read_i32()?;
                        if !(2049..=4096).contains(&total)
                            || index != state.segments
                            || (index > 0 && serial != state.serial)
                        {
                            return Err(StatusCode::BadValue);
                        }
                        state.serial = serial;
                        state.segments += 1;
                        state.bytes.extend_from_slice(&bytes);
                        if state.bytes.len() > total as usize {
                            return Err(StatusCode::BadValue);
                        }
                        if state.bytes.len() == total as usize {
                            Some(std::mem::take(&mut state.bytes))
                        } else {
                            None
                        }
                    }
                };
                if data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                if let Some(bytes) = completed {
                    let mut state = self.0.state.lock().unwrap();
                    if state.hold_request {
                        state.request_entered = true;
                        self.0.changed.notify_all();
                        drop(state);
                        self.0.wait(|state| !state.request_released)?;
                    } else {
                        drop(state);
                    }
                    let callback = self
                        .0
                        .state
                        .lock()
                        .unwrap()
                        .response
                        .clone()
                        .ok_or(StatusCode::UnexpectedNull)?;
                    response(&callback, serial, &bytes)?;
                }
                Ok(())
            }
            1 => {
                self.0.state.lock().unwrap().clears += 1;
                Err(StatusCode::InvalidOperation)
            }
            100 => {
                if data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                let state = self.0.state.lock().unwrap();
                reply.write(&Status::from(StatusCode::Ok))?;
                for value in [
                    state.registers,
                    state.raw,
                    state.segments,
                    state.calling_pid,
                    state.calling_uid,
                    state.clears,
                ] {
                    reply.write_i32(value)?;
                }
                Ok(())
            }
            109 | 110 | 111 | 112 | 113 | 199 => {
                if data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                match code {
                    109 => {
                        self.0.state.lock().unwrap().registration_released = true;
                        self.0.changed.notify_all();
                    }
                    110 => self.0.wait(|state| !state.registration_entered)?,
                    111 => self.0.state.lock().unwrap().hold_request = true,
                    112 => self.0.wait(|state| !state.request_entered)?,
                    113 => {
                        self.0.state.lock().unwrap().request_released = true;
                        self.0.changed.notify_all();
                    }
                    199 => {}
                    _ => unreachable!(),
                }
                ack(reply)
            }
            107 => std::process::exit(0),
            _ => Err(StatusCode::UnknownTransaction),
        }
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let process = ProcessState::init("/dev/binderfs/radio", 4)?;
    let radio = Binder::new(Radio(Arc::new(Shared::default())));
    if std::env::args().nth(1).as_deref() == Some("named") {
        use rsbinder::{
            Proxy,
            hub::android_16::{BpServiceManager, IServiceManager},
        };
        let manager =
            BpServiceManager::from_binder(process.context_object()?).ok_or(StatusCode::BadType)?;
        ProcessState::start_thread_pool();
        manager.addService(
            "org.pokelinux.fixture.arm64-radio",
            &radio.as_binder(),
            false,
            8,
        )?;
    } else {
        process.become_context_manager(Binder::new(Bootstrap(radio.as_binder())).as_binder())?;
    }
    println!("READY pid={}", std::process::id());
    use std::io::Write;
    std::io::stdout().flush()?;
    ProcessState::start_thread_pool();
    ProcessState::join_thread_pool()?;
    Ok(())
}
