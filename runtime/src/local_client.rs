//! One local request per connection. Never reconnect or replay after uncertainty.
use crate::{
    MAX_FRAME, local_protocol,
    local_reply::{self, Reply},
};
use std::{
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    time::Duration,
};
#[derive(Debug)]
pub enum ExchangeError {
    /// No command bytes were offered to the socket.
    NotSent(io::Error),
    /// A write was attempted; recover using the original command ID.
    Unknown(io::Error),
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn now() -> Duration {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(t.tv_sec as u64, t.tv_nsec as u32)
}
fn remaining(deadline: Duration) -> io::Result<Duration> {
    deadline
        .checked_sub(now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "local exchange deadline expired"))
}
pub fn exchange(path: &Path, frame: &[u8], budget: Duration) -> Result<Reply, ExchangeError> {
    let deadline = now() + budget.min(Duration::from_secs(2));
    let request = local_protocol::decode(frame).map_err(|e| ExchangeError::NotSent(invalid(e)))?;
    let mut socket = (|| -> io::Result<UnixStream> {
        remaining(deadline)?;
        if !path.is_absolute() {
            return Err(invalid("absolute service socket path required"));
        }
        let fd = rustix::net::socket_with(
            rustix::net::AddressFamily::UNIX,
            rustix::net::SocketType::STREAM,
            rustix::net::SocketFlags::NONBLOCK | rustix::net::SocketFlags::CLOEXEC,
            None,
        )?;
        // AF_UNIX returns EAGAIN for a full backlog. Fail before writing rather
        // than waiting indefinitely or retrying connection admission.
        rustix::net::connect(&fd, &rustix::net::SocketAddrUnix::new(path)?)?;
        let socket = UnixStream::from(fd);
        if rustix::net::sockopt::socket_peercred(&socket)?.uid != rustix::process::geteuid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "service peer UID mismatch",
            ));
        }
        socket.set_nonblocking(false)?;
        remaining(deadline)?;
        Ok(socket)
    })()
    .map_err(ExchangeError::NotSent)?;
    // Set the initial timeout before classifying any subsequent error as
    // uncertain: from the first write attempt onward, admission is unknown.
    socket
        .set_write_timeout(Some(remaining(deadline).map_err(ExchangeError::NotSent)?))
        .map_err(ExchangeError::NotSent)?;
    (|| -> io::Result<Reply> {
        let mut rest = frame;
        while !rest.is_empty() {
            socket.set_write_timeout(Some(remaining(deadline)?))?;
            match socket.write(rest) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "local request write stopped",
                    ));
                }
                Ok(n) => rest = &rest[n..],
                Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(e),
            }
            remaining(deadline)?;
        }
        let mut data = Vec::with_capacity(4096);
        let mut chunk = [0u8; 4096];
        loop {
            socket.set_read_timeout(Some(remaining(deadline)?))?;
            let limit = chunk.len().min(MAX_FRAME + 1 - data.len());
            let count = match socket.read(&mut chunk[..limit]) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                other => other?,
            };
            remaining(deadline)?;
            if count == 0 {
                return Err(invalid("incomplete local reply"));
            }
            data.extend_from_slice(&chunk[..count]);
            if data.len() > MAX_FRAME {
                return Err(invalid("local reply too large"));
            }
            if chunk[..count].contains(&b'\n') {
                let reply = local_reply::decode(&data, &request).map_err(invalid)?;
                remaining(deadline)?;
                return Ok(reply);
            }
        }
    })()
    .map_err(ExchangeError::Unknown)
}
