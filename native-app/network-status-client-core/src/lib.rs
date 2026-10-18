//! Coarse local network counts. Rust owns read-only transport, validation and expiry.
use serde::{Deserialize, Deserializer};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

mod ffi;
pub use ffi::*;

const ENDPOINT: &str = "/run/pokelinux-network/snapshot.sock";
const POLL_NS: u64 = 250_000_000;
const REQUEST_NS: u64 = 500_000_000;
const MAX_FUTURE_NS: u64 = 2_000_000_000;
const MAX_WIRE: usize = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub available: u32,
    pub local_interfaces: i32,
    pub address_records: i32,
    pub revision: u64,
}
impl Default for View {
    fn default() -> Self {
        Self {
            available: 0,
            local_interfaces: -1,
            address_records: -1,
            revision: 0,
        }
    }
}
const _: () = assert!(std::mem::size_of::<View>() == 24);

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: u32,
    revision: u64,
    until_ns: String,
    available: bool,
    #[serde(deserialize_with = "nullable")]
    local_interfaces: Option<u16>,
    #[serde(deserialize_with = "nullable")]
    address_records: Option<u16>,
}
struct Snapshot {
    view: View,
    until: u64,
}
fn decode(bytes: &[u8], now: u64) -> Option<Option<Snapshot>> {
    let line = bytes.strip_suffix(b"\n")?;
    if line.contains(&b'\n') {
        return None;
    }
    let wire: Wire = serde_json::from_slice(line).ok()?;
    if wire.schema != 1
        || wire.until_ns.is_empty()
        || !wire.until_ns.bytes().all(|b| b.is_ascii_digit())
        || (wire.until_ns.len() > 1 && wire.until_ns.starts_with('0'))
    {
        return None;
    }
    let until = wire.until_ns.parse::<u64>().ok()?;
    if !wire.available {
        return (until == 0 && wire.local_interfaces.is_none() && wire.address_records.is_none())
            .then_some(None);
    }
    if until <= now || until.checked_sub(now)? > MAX_FUTURE_NS {
        return None;
    }
    let interfaces = wire.local_interfaces?;
    let addresses = wire.address_records?;
    if wire.revision == 0 || interfaces > 64 || addresses > interfaces * 16 {
        return None;
    }
    Some(Some(Snapshot {
        until,
        view: View {
            available: 1,
            local_interfaces: i32::from(interfaces),
            address_records: i32::from(addresses),
            revision: wire.revision,
        },
    }))
}

struct Request {
    stream: UnixStream,
    connecting: bool,
    until: u64,
    bytes: Vec<u8>,
}
enum Progress {
    Pending,
    Complete(Vec<u8>),
}
impl Request {
    fn start(path: &std::path::Path, now: u64) -> io::Result<Self> {
        let path = path.as_os_str().as_bytes();
        // SAFETY: sockaddr_un is a plain C value; all-zero is valid initial storage.
        let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        if path.first() != Some(&b'/') || path.contains(&0) || path.len() >= address.sun_path.len()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "network status socket path",
            ));
        }
        address.sun_family = libc::AF_UNIX as libc::sa_family_t;
        for (out, byte) in address.sun_path.iter_mut().zip(path) {
            *out = *byte as libc::c_char;
        }
        // SAFETY: no borrowed memory; socket returns a new owned descriptor or -1.
        let raw = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
            )
        };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: raw is a newly created descriptor, transferred exactly once.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        // SAFETY: address is initialized sockaddr_un storage for the complete call.
        let result = unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                std::mem::size_of_val(&address) as libc::socklen_t,
            )
        };
        let connecting = if result == 0 {
            false
        } else {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EINPROGRESS) {
                return Err(error);
            }
            true
        };
        let request = Self {
            stream: UnixStream::from(fd),
            connecting,
            until: now
                .checked_add(REQUEST_NS)
                .ok_or_else(|| io::Error::other("network status deadline overflow"))?,
            bytes: Vec::with_capacity(512),
        };
        if !connecting {
            request.check_peer()?;
        }
        Ok(request)
    }
    fn check_peer(&self) -> io::Result<()> {
        // SAFETY: ucred is a plain integer C value with valid zero representation.
        let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of_val(&credentials) as libc::socklen_t;
        // SAFETY: credentials and length are writable complete storage; fd is live.
        let result = unsafe {
            libc::getsockopt(
                self.stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut length,
            )
        };
        if result != 0
            || length as usize != std::mem::size_of_val(&credentials)
            || credentials.uid != 0
            || credentials.gid != 0
            || credentials.pid <= 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "network status peer must be root:root",
            ));
        }
        Ok(())
    }
    fn progress(&mut self) -> io::Result<Progress> {
        if self.connecting {
            let mut poll = libc::pollfd {
                fd: self.stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            // SAFETY: one initialized pollfd; timeout zero never waits.
            let result = unsafe { libc::poll(&mut poll, 1, 0) };
            if result == 0 {
                return Ok(Progress::Pending);
            }
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            if let Some(error) = self.stream.take_error()? {
                return Err(error);
            }
            self.check_peer()?;
            self.connecting = false;
        }
        // A tick has a fixed syscall/byte budget even against a live noisy peer.
        for _ in 0..17 {
            let mut bytes = [0u8; 256];
            match self.stream.read(&mut bytes) {
                Ok(0) => return Ok(Progress::Complete(std::mem::take(&mut self.bytes))),
                Ok(count) => {
                    if self.bytes.len() + count > MAX_WIRE {
                        return Err(io::Error::other("network status line too large"));
                    }
                    self.bytes.extend_from_slice(&bytes[..count]);
                }
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        || error.kind() == io::ErrorKind::Interrupted =>
                {
                    return Ok(Progress::Pending)
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Progress::Pending)
    }
}

struct Client {
    path: PathBuf,
    request: Option<Request>,
    snapshot: Option<Snapshot>,
    next_poll: u64,
}
impl Client {
    fn new() -> Self {
        Self::with_path(PathBuf::from(ENDPOINT))
    }
    // Only Rust-private tests select a different endpoint. The C ABI never does.
    fn with_path(path: PathBuf) -> Self {
        Self {
            path,
            request: None,
            snapshot: None,
            next_poll: 0,
        }
    }
    fn clear(&mut self) {
        self.request = None;
        self.snapshot = None;
    }
    fn expire(&mut self, now: u64) {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| now >= snapshot.until)
        {
            self.snapshot = None;
        }
        if self
            .request
            .as_ref()
            .is_some_and(|request| now >= request.until)
        {
            self.clear();
            self.next_poll = now.saturating_add(POLL_NS);
        }
    }
    fn begin(&mut self, now: u64) -> Option<Request> {
        self.expire(now);
        if let Some(request) = self.request.take() {
            return Some(request);
        }
        if now < self.next_poll {
            return None;
        }
        match Request::start(&self.path, now) {
            Ok(request) => Some(request),
            Err(_) => {
                self.clear();
                self.next_poll = now.saturating_add(POLL_NS);
                None
            }
        }
    }
    fn finish(&mut self, request: Request, progress: io::Result<Progress>, now: u64) {
        self.expire(now);
        if now >= request.until {
            self.clear();
            self.next_poll = now.saturating_add(POLL_NS);
            return;
        }
        match progress {
            Ok(Progress::Pending) => self.request = Some(request),
            Ok(Progress::Complete(bytes)) => {
                self.snapshot = decode(&bytes, now).flatten();
                self.next_poll = now.saturating_add(POLL_NS);
            }
            Err(_) => {
                self.clear();
                self.next_poll = now.saturating_add(POLL_NS);
            }
        }
    }
    #[cfg(test)]
    fn tick_at(&mut self, now: u64) {
        if let Some(mut request) = self.begin(now) {
            let progress = request.progress();
            self.finish(request, progress, now);
        }
    }
    fn view_at(&mut self, now: u64) -> View {
        self.expire(now);
        self.snapshot
            .as_ref()
            .map_or_else(View::default, |snapshot| snapshot.view)
    }
    fn tick(&mut self) {
        let Some(started) = boottime() else {
            self.clear();
            return;
        };
        let Some(mut request) = self.begin(started) else {
            return;
        };
        let progress = request.progress();
        // A suspend/preemption during I/O cannot publish using the old sample.
        // Finish rechecks the original request deadline before parsing any reply.
        if let Some(completed) = boottime() {
            self.finish(request, progress, completed);
        } else {
            self.clear();
        }
    }
    fn view(&mut self) -> View {
        if let Some(now) = boottime() {
            self.view_at(now)
        } else {
            self.clear();
            View::default()
        }
    }
}
fn boottime() -> Option<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: complete writable timespec; CLOCK_BOOTTIME is the wire clock.
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) } != 0
        || !(0..1_000_000_000).contains(&time.tv_nsec)
    {
        return None;
    }
    u64::try_from(time.tv_sec)
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(time.tv_nsec as u64)
}
#[cfg(test)]
mod tests;
