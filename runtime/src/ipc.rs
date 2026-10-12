//! Bounded local socket reads. The owner closes the socket after one exchange.
use crate::{MAX_FRAME, Request, decode_request};
use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

pub fn receive_request(socket: &mut UnixStream, budget: Duration) -> io::Result<Request> {
    decode_request(&receive_frame(socket, budget)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
pub(crate) fn receive_frame(socket: &mut UnixStream, budget: Duration) -> io::Result<Vec<u8>> {
    let original_timeout = socket.read_timeout()?;
    let deadline = Instant::now() + budget.min(Duration::from_secs(2));
    let result = read_frame(socket, deadline);
    // The caller owns the connection. Restore its configuration on success and
    // failure, but preserve the original read failure if restoration also fails.
    let restored = socket.set_read_timeout(original_timeout);
    match result {
        Ok(request) => restored.map(|()| request),
        Err(error) => Err(error),
    }
}

fn read_frame(socket: &mut UnixStream, deadline: Instant) -> io::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "local frame deadline",
            ));
        }
        socket.set_read_timeout(Some(remaining))?;
        let limit = chunk.len().min(MAX_FRAME + 1 - data.len());
        let count = match socket.read(&mut chunk[..limit]) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            other => other?,
        };
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete local frame",
            ));
        }
        data.extend_from_slice(&chunk[..count]);
        if data.len() > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "local frame too large",
            ));
        }
        if chunk[..count].contains(&b'\n') {
            return Ok(data);
        }
    }
}
