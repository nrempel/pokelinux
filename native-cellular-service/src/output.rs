//! Whole bounded nonblocking Linux pipe publication; no queued stale bytes.
use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
use std::io;
pub fn configure() -> io::Result<()> {
    let fd = std::io::stdout();
    if FileType::from_raw_mode(fstat(&fd)?.st_mode) != FileType::Fifo {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    fcntl_setfl(&fd, fcntl_getfl(&fd)? | OFlags::NONBLOCK)?;
    Ok(())
}
pub fn publish(bytes: &[u8]) -> io::Result<bool> {
    publish_to(std::io::stdout(), bytes)
}
fn publish_to(fd: impl rustix::fd::AsFd, bytes: &[u8]) -> io::Result<bool> {
    if bytes.is_empty() || bytes.len() > 4096 || bytes.len() > rustix::pipe::PIPE_BUF {
        return Err(io::ErrorKind::InvalidData.into());
    }
    match rustix::io::write(fd, bytes) {
        Ok(size) if size == bytes.len() => Ok(true),
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => Ok(false),
        Ok(_) => Err(io::ErrorKind::WriteZero.into()),
        Err(error) => Err(error.into()),
    }
}
pub fn event(mut value: serde_json::Value) {
    value["schema"] = 1.into();
    if let Ok(mut bytes) = serde_json::to_vec(&value) {
        bytes.push(b'\n');
        // Dropped/partial diagnostics never qualify as lifecycle evidence.
        let _ = publish(&bytes);
    }
}

/// Require complete Pending publication before attempting the mutating setup call.
pub fn publish_required(bytes: &[u8]) -> io::Result<()> {
    publish_required_to(std::io::stdout(), bytes)
}
fn publish_required_to(fd: impl rustix::fd::AsFd, bytes: &[u8]) -> io::Result<()> {
    if publish_to(fd, bytes)? {
        Ok(())
    } else {
        Err(io::ErrorKind::WouldBlock.into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_pipe_required_publication_preserves_complete_record() {
        let (read, write) = rustix::pipe::pipe_with(
            rustix::pipe::PipeFlags::NONBLOCK | rustix::pipe::PipeFlags::CLOEXEC,
        )
        .unwrap();
        let bytes = b"{\"kind\":\"Pending\"}\n";
        publish_required_to(&write, bytes).unwrap();
        let mut buffer = [0u8; 64];
        let n = rustix::io::read(&read, &mut buffer).unwrap();
        assert_eq!(&buffer[..n], bytes);
    }
    #[test]
    fn actual_full_pipe_refuses_required_pending_publication() {
        let (_read, write) = rustix::pipe::pipe_with(
            rustix::pipe::PipeFlags::NONBLOCK | rustix::pipe::PipeFlags::CLOEXEC,
        )
        .unwrap();
        let padding = [b'x'; 4096];
        let mut filled = 0;
        for _ in 0..16384 {
            if !publish_to(&write, &padding).unwrap() {
                break;
            }
            filled += padding.len();
        }
        assert!(filled > 0);
        assert!(!publish_to(&write, b"{}\n").unwrap());
        println!("PENDING-CONTROL actual nonblocking pipe full; whole record was not delivered");
        assert_eq!(
            publish_required_to(&write, b"{}\n").unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
}
