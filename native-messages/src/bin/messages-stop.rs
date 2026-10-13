//! Installed service-manager stop uses the authenticated root control boundary.
fn main() -> std::io::Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let fd = pokelinux_lifecycle::ipc::connect(std::path::Path::new(&args[0]))?;
    let reply: pokelinux_lifecycle::Reply =
        pokelinux_lifecycle::ipc::call(&fd, &serde_json::json!({"op":"stop"}))?;
    if reply == pokelinux_lifecycle::Reply::Ok {
        Ok(())
    } else {
        Err(std::io::ErrorKind::PermissionDenied.into())
    }
}
