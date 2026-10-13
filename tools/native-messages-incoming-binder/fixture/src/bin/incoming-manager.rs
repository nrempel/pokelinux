//! Minimal synthetic IServiceManager fixture: only addService and direct checkService.
//! One fixed incoming probe, one add only. Fresh fixture generation requires full peer/manager reap.
use native_messages_incoming_binder_fixture::{DESCRIPTOR, NAME};
use rsbinder::{
    Binder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status, StatusCode,
};
use std::{io::Write, sync::Mutex};
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
                if service.descriptor() != DESCRIPTOR {
                    return Err(StatusCode::BadType);
                }
                let mut current = self.0.lock().unwrap();
                if current.is_some() {
                    return Err(StatusCode::InvalidOperation);
                }
                *current = Some(service);
                println!("INCOMING-PROBE-TRANSFERRED");
                std::io::stdout().flush().unwrap();
                reply.write(&Status::from(StatusCode::Ok))
            }
            3 => {
                if data.read::<String>()? != NAME || data.data_avail() != 0 {
                    return Err(StatusCode::BadValue);
                }
                reply.write(&Status::from(StatusCode::Ok))?;
                reply.write(&self.0.lock().unwrap().clone())
            }
            _ => Err(StatusCode::UnknownTransaction),
        }
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let process = ProcessState::init("/dev/binder", 3)?;
    process.become_context_manager(Binder::new(Manager(Mutex::new(None))).as_binder())?;
    println!("READY-MANAGER pid={}", std::process::id());
    std::io::stdout().flush()?;
    ProcessState::start_thread_pool();
    ProcessState::join_thread_pool()?;
    Ok(())
}
