//! Submit the complete readback ACK in one contiguous write, without replay.
use std::io;

pub(super) fn emit(epoch: &str, event: u64, sequence: u64, message: &str) -> io::Result<()> {
    let record = format!(
        "POKELINUX_DIAGNOSTIC_ACK epoch={epoch} event={event} kernel_seq={sequence} {message}\n"
    );
    let output = io::stdout().lock();
    if rustix::io::write(&output, record.as_bytes())? != record.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
