//! Kernel Binder and temporary callback qualification. No modem or radio request is sent.
use rsbinder::{Interface, ProcessState, SIBinder, hub};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::time::Duration;

struct RegistrationCallback(SyncSender<(String, SIBinder)>);
impl Interface for RegistrationCallback {}
impl hub::IServiceCallback for RegistrationCallback {
    fn onRegistration(&self, name: &str, service: &SIBinder) -> rsbinder::BinderResult<()> {
        // The probe needs only the first notification. Never block a Binder worker.
        let _ = self.0.try_send((name.to_owned(), service.clone()));
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = ProcessState::init("/dev/binder", 1)?;
    let manager = state.context_object()?;
    if !manager.is_remote() || manager.descriptor() != "android.os.IServiceManager" {
        return Err("unexpected Binder context manager".into());
    }
    manager.ping_binder()?;
    let activity = hub::try_get_service("activity")?.ok_or("activity service is absent")?;
    if !activity.is_remote() || activity.descriptor() != "android.app.IActivityManager" {
        return Err("unexpected activity service interface".into());
    }
    activity.ping_binder()?;
    println!(
        "kernel-binder\t{}\t{}\tPASS",
        manager.descriptor(),
        activity.descriptor()
    );
    ProcessState::start_thread_pool();
    let (sender, receiver) = sync_channel(1);
    let callback = hub::BnServiceCallback::new_binder(RegistrationCallback(sender));
    hub::register_for_notifications("activity", &callback)?;
    let received = receiver.recv_timeout(Duration::from_secs(5));
    // Unsubscribe even when receipt times out; then report any failure.
    hub::unregister_for_notifications("activity", &callback)?;
    let (name, service) = received?;
    if name != "activity"
        || !service.is_remote()
        || service.descriptor() != "android.app.IActivityManager"
    {
        return Err("unexpected service registration callback".into());
    }
    service.ping_binder()?;
    println!("kernel-callback\t{}\t{}\tPASS", name, service.descriptor());
    Ok(())
}
