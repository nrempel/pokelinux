//! Fixed Permissions launch request; no grant, capability or app label input.
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 || !std::path::Path::new(&args[0]).is_absolute() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let fd = pokelinux_lifecycle::ipc::connect(std::path::Path::new(&args[0]))?;
    let reply: pokelinux_lifecycle::Reply =
        pokelinux_lifecycle::ipc::call(&fd, &serde_json::json!({"op":"launch_permissions"}))?;
    if reply == pokelinux_lifecycle::Reply::Ok {
        Ok(())
    } else {
        Err(std::io::ErrorKind::PermissionDenied.into())
    }
}
