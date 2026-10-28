fn main() -> std::io::Result<()> {
    use std::path::Path;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--messages-read-worker") {
        return pokelinux_messages::worker::reader_entry(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--messages-worker") {
        return pokelinux_messages::worker::entry(&args[1..]);
    }
    if args.len() != 3 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    pokelinux_messages::read_service::run(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
    )
}
