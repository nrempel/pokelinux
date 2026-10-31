//! Trusted selected-backend Main, separate from the app-launching lifecycle actor.
fn main() -> std::io::Result<()> {
    let until = pokelinux_network_state::boot_time_ns()
        .checked_add(3_000_000_000)
        .ok_or(std::io::ErrorKind::InvalidInput)?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    pokelinux_connectivity_control::main_service::run(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
        std::path::Path::new(&args[3]),
        until,
    )
}
