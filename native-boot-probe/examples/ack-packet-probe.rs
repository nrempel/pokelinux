//! Test infrastructure: call the actual ACK producer with two fixed inputs.
#[path = "../src/ack.rs"]
mod ack;

use std::{env, io};

fn main() -> io::Result<()> {
    let mut args = env::args_os();
    let _ = args.next();
    let case = args
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    if args.next().is_some() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let (event, sequence, message) = match case.to_str() {
        Some("kernel") => (
            2,
            313,
            "POKELINUX_PROBE_KERNEL release=7.1.6-1-aarch64-ARCH",
        ),
        Some("complete") => (
            4,
            315,
            "POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false",
        ),
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    };
    ack::emit("0123456789abcdef0123456789abcdef", event, sequence, message)
}
