//! Two fixed Data fixture transfers, not a general/installed service manager.
use rsbinder::{
    Binder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status, StatusCode,
};
use std::{io::Write, sync::Mutex};
const NAME: &str = "org.pokelinux.fixture.cellular-data";
const DESCRIPTOR: &str = "org.pokelinux.datatest.IDataFixture";
const DATA_NAME: &str = "android.hardware.radio.data.IRadioData/slot1";
const DATA_DESCRIPTOR: &str = "android.hardware.radio.data.IRadioData";
fn service_slot(name: &str) -> Result<usize> {
    match name {
        NAME => Ok(0),
        DATA_NAME => Ok(1),
        _ => Err(StatusCode::BadValue),
    }
}
struct Manager([Mutex<Option<SIBinder>>; 2]);
impl Remotable for Manager {
    fn descriptor() -> &'static str {
        "android.os.IServiceManager"
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        match code {
            5 => {
                let slot = service_slot(&data.read::<String>()?)?;
                let service: SIBinder = data.read()?;
                let allow_isolated = data.read::<bool>()?;
                let priority = data.read_i32()?;
                if !service.is_remote() || allow_isolated || priority != 8 || data.data_avail() != 0
                {
                    return Err(StatusCode::BadValue);
                }
                let descriptor = if slot == 0 {
                    DESCRIPTOR
                } else {
                    DATA_DESCRIPTOR
                };
                if service.descriptor() != descriptor {
                    return Err(StatusCode::BadType);
                }
                let mut current = self.0[slot]
                    .lock()
                    .map_err(|_| StatusCode::FailedTransaction)?;
                if current.is_some() {
                    return Err(StatusCode::InvalidOperation);
                }
                *current = Some(service);
                println!(
                    "{}-TRANSFERRED pid={} uid={}",
                    if slot == 0 {
                        "DATA-FIXTURE"
                    } else {
                        "DATA-STANDARD"
                    },
                    rsbinder::thread_state::get_calling_pid(),
                    rsbinder::thread_state::get_calling_uid()
                );
                std::io::stdout()
                    .flush()
                    .map_err(|_| StatusCode::FailedTransaction)?;
                reply.write(&Status::from(StatusCode::Ok))
            }
            3 => {
                let slot = service_slot(&data.read::<String>()?)?;
                if data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                let service = self.0[slot]
                    .lock()
                    .map_err(|_| StatusCode::FailedTransaction)?
                    .clone();
                println!(
                    "{}-LOOKUP pid={} uid={} found={}",
                    if slot == 0 {
                        "DATA-FIXTURE"
                    } else {
                        "DATA-STANDARD"
                    },
                    rsbinder::thread_state::get_calling_pid(),
                    rsbinder::thread_state::get_calling_uid(),
                    service.is_some()
                );
                std::io::stdout()
                    .flush()
                    .map_err(|_| StatusCode::FailedTransaction)?;
                reply.write(&Status::from(StatusCode::Ok))?;
                reply.write(&service)
            }
            _ => Err(StatusCode::UnknownTransaction),
        }
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() != 1 {
        return Err("no manager arguments".into());
    }
    let device = std::env::var("POKELINUX_DATA_INTEROP_DEVICE")?;
    if device.len() > 128 || !device.starts_with('/') || device.contains(['\0', '\n', '\r']) {
        return Err("explicit absolute private Binder device required".into());
    }
    let process = ProcessState::init(&device, 3)?;
    let manager = Manager([Mutex::new(None), Mutex::new(None)]);
    process.become_context_manager(Binder::new(manager).as_binder())?;
    println!(
        "READY-DATA-MANAGER pid={} device={}",
        std::process::id(),
        device
    );
    std::io::stdout().flush()?;
    ProcessState::start_thread_pool();
    ProcessState::join_thread_pool()?;
    Ok(())
}
