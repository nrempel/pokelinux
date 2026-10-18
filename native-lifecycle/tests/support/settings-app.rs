//! Native fixture only: six absolute inputs, no GUI, backend, socket or grant.
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 6 || args.iter().any(|p| !std::path::Path::new(p).is_absolute()) {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    loop {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
