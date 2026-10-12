//! Read-only kernel Binder qualification. No modem or radio request is sent.
use rsbinder::{ProcessState, hub};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = ProcessState::init("/dev/binder", 0)?;
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
    Ok(())
}
