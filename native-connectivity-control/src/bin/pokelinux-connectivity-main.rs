//! Trusted virtual-backend Main, separate from the app-launching lifecycle actor.
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 5 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let until = args[4]
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or(std::io::ErrorKind::InvalidInput)?;
    pokelinux_connectivity_control::main_service::run(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
        std::path::Path::new(&args[3]),
        until,
    )
}
