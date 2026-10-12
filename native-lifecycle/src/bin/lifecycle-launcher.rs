use pokelinux_lifecycle::{AdminRequest, Reply, ipc};
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "app-entry") {
        return pokelinux_lifecycle::service::entry(&args[1..]);
    }
    if args.len() != 2 || rustix::process::getuid().as_raw() != 0 {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let fd = ipc::connect(std::path::Path::new(&args[0]).join("admin.sock").as_path())?;
    let reply: Reply = ipc::call(
        fd,
        &AdminRequest::Launch {
            app: args[1].clone(),
        },
    )?;
    println!("{}", serde_json::to_string(&reply)?);
    if reply != Reply::Ok {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
