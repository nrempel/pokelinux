//! Minimal synthetic IServiceManager fixture: only addService and direct checkService.
//! This is not the installed Android service manager or a general framework replacement.
use rsbinder::{
    Binder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status, StatusCode,
};
use std::{io::Write, sync::Mutex};
const NAME: &str = "org.pokelinux.fixture.arm64-radio";
struct Manager(Mutex<Option<SIBinder>>);
impl Remotable for Manager {
    fn descriptor() -> &'static str {
        "android.os.IServiceManager"
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        match code {
            5 => {
                if data.read::<String>()? != NAME {
                    return Err(StatusCode::BadValue);
                }
                let service: SIBinder = data.read()?;
                let allow_isolated = data.read::<bool>()?;
                let priority = data.read_i32()?;
                if !service.is_remote() || allow_isolated || priority != 8 || data.data_avail() != 0
                {
                    return Err(StatusCode::BadValue);
                }
                if service.descriptor()
                    != "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal"
                {
                    return Err(StatusCode::BadType);
                }
                let mut current = self.0.lock().unwrap();
                if current.is_some() {
                    return Err(StatusCode::InvalidOperation);
                }
                *current = Some(service);
                println!("SERVICE-TRANSFERRED");
                std::io::stdout().flush().unwrap();
                reply.write(&Status::from(StatusCode::Ok))
            }
            3 => {
                if data.read::<String>()? != NAME || data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                reply.write(&Status::from(StatusCode::Ok))?;
                let service = self.0.lock().unwrap().clone();
                println!(
                    "SERVICE-LOOKUP pid={} uid={} found={}",
                    rsbinder::thread_state::get_calling_pid(),
                    rsbinder::thread_state::get_calling_uid(),
                    service.is_some()
                );
                std::io::stdout().flush().unwrap();
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
    let process = ProcessState::init("/dev/binderfs/radio", 3)?;
    process.become_context_manager(Binder::new(Manager(Mutex::new(None))).as_binder())?;
    println!("READY-MANAGER pid={}", std::process::id());
    std::io::stdout().flush()?;
    ProcessState::start_thread_pool();
    ProcessState::join_thread_pool()?;
    Ok(())
}
