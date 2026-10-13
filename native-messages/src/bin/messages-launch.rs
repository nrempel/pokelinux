//! Fixed Messages launch request. No grant, data query or arbitrary app selection.
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let fd = pokelinux_lifecycle::ipc::connect(std::path::Path::new(&args[0]))?;
    let reply: pokelinux_lifecycle::Reply =
        pokelinux_lifecycle::ipc::call(&fd, &serde_json::json!({"op":"launch_messages"}))?;
    if reply == pokelinux_lifecycle::Reply::Ok {
        Ok(())
    } else {
        Err(std::io::ErrorKind::PermissionDenied.into())
    }
}
