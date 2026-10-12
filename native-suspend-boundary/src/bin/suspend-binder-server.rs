use pokelinux_suspend::{Coordinator, DescriptorBackend};
use pokelinux_suspend_boundary::{
    BnSuspendControl, BnSystemSuspend, ISuspendBoundaryProbe, bootstrap,
    frontend::{Control, State, Suspend, features},
};
use rsbinder::{FromIBinder, Interface, ProcessState, Strong};
use std::fs::{self, OpenOptions};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("fresh isolated FD directory required")?;
    let no_restart = match std::env::args().nth(2).as_deref() {
        Some("fresh") => false,
        Some("attach") => true,
        _ => return Err("fresh/attach broker mode required".into()),
    };
    let root = std::path::PathBuf::from(root);
    fs::create_dir(&root)?;
    let file = |name: &str| {
        OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(root.join(name))
    };
    fs::write(root.join("count"), b"7\n")?;
    let count = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("count"))?;
    let backend = DescriptorBackend::new(
        count.into(),
        file("state")?.into(),
        file("abort")?.into(),
        file("unlock")?.into(),
    )?;
    let coordinator = Coordinator::new(backend, file("writer")?.into())
        .map_err(|e| format!("coordinator: {e:?}"))?;
    let peer: Strong<dyn ISuspendBoundaryProbe> =
        FromIBinder::try_from(bootstrap::bootstrap(no_restart)?)?;
    let state = State::new(coordinator);
    let service = BnSystemSuspend::new_binder_with_features(Suspend(state.clone()), features());
    let control = BnSuspendControl::new_binder_with_features(Control(state), features());
    let name = "android.system.suspend.ISystemSuspend/pokelinux_test";
    let registration = rsbinder::hub::add_service(name, service.as_binder());
    println!("custom-registration\t{registration:?}");
    peer.install(&service, &control)?;
    println!("suspend-rust-server\t{}\tREADY", std::process::id());
    ProcessState::join_thread_pool()?;
    Ok(())
}
