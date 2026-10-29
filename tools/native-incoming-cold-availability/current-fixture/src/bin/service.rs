//! Isolated original-session and authored frozen-V3 receive composition.
//! No production incoming backend or application FD selector.
use pokelinux_lifecycle::session::{Provider as SessionProvider, TimingLossReason, Update};
use pokelinux_lifecycle::{boottime_ns, messages::Provider};
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
    io::{FdFlags, fcntl_setfd},
    net::{SendFlags, send},
    process::Pid,
};
use std::{io, path::Path};
fn descriptor(value: &std::ffi::OsStr) -> io::Result<i32> {
    value
        .to_str()
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|fd| *fd >= 3)
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}
fn adopt(raw: i32) -> OwnedFd {
    // SAFETY: the isolated root runner exclusively transfers two descriptors;
    // their parsed numeric IDs are checked for equality before either adoption.
    // This binary accepts no application FD selector.
    unsafe { OwnedFd::from_raw_fd(raw) }
}
// Fixed-size diagnostic packets use a separate root-provisioned channel. Trace
// congestion never changes source state, frames, deadlines or return values.
struct Timing {
    fd: OwnedFd,
    previous_begin: u64,
    dropped: u64,
}
impl Timing {
    fn record(&mut self, begin: u64, ended: u64, reply: &io::Result<Update>) {
        // kind0 awaits first,1 current,2 terminal error,3 authority loss,4 recovering.
        // For loss, detail1 is a gap and2 expiry; no snapshot sequence/time is authored.
        let (kind, sequence, sampled, detail) = match reply {
            Ok(Update::Current(s)) => (1, s.sequence, s.sampled_at_ns, 0),
            Ok(Update::AwaitingFirst) => (0, 0, 0, 0),
            Ok(Update::AuthorityLost(loss)) => (
                3,
                0,
                0,
                match loss.reason {
                    TimingLossReason::SampleGap => 1,
                    TimingLossReason::SampleExpired => 2,
                },
            ),
            Ok(Update::Recovering) => (4, 0, 0, 0),
            Err(e) => (
                2,
                0,
                0,
                match e.kind() {
                    io::ErrorKind::TimedOut => 1,
                    io::ErrorKind::InvalidData => 2,
                    io::ErrorKind::NotConnected => 3,
                    _ => 4,
                },
            ),
        };
        let fields = [
            1,
            begin,
            ended,
            self.previous_begin,
            kind,
            sequence,
            sampled,
            detail,
            self.dropped,
        ];
        self.previous_begin = begin;
        let mut bytes = [0u8; 72];
        for (chunk, field) in bytes.as_chunks_mut::<8>().0.iter_mut().zip(fields) {
            chunk.copy_from_slice(&field.to_ne_bytes());
        }
        if send(&self.fd, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) != Ok(bytes.len()) {
            self.dropped = self.dropped.saturating_add(1);
        }
    }
}
// Fixture diagnostics forward the original Provider result unchanged. They never
// consume a second receiver, alter a sample deadline or restore a failed source.
struct TracedObserver {
    observer: Observer,
    timing: Timing,
    revision: u64,
    failed: bool,
    stopped: bool,
    sampled_at: u64,
    maximum_sample_gap: u64,
}
impl SessionProvider for TracedObserver {
    fn stop(&mut self) {
        if !self.stopped {
            self.stopped = true;
            println!(
                "{}",
                serde_json::json!({"event":"diagnostic_source_stop","boottime_ns":boottime_ns()})
            );
        }
        SessionProvider::stop(&mut self.observer);
    }
    fn occupied(&self) -> bool {
        SessionProvider::occupied(&self.observer)
    }

    fn poll(&mut self) -> io::Result<Update> {
        let began = boottime_ns();
        let reply = SessionProvider::poll(&mut self.observer);
        let ended = boottime_ns();
        self.timing.record(began, ended, &reply);
        match &reply {
            Ok(Update::Current(s)) => {
                if self.sampled_at != 0 {
                    self.maximum_sample_gap = self
                        .maximum_sample_gap
                        .max(s.sampled_at_ns.saturating_sub(self.sampled_at));
                }
                self.sampled_at = s.sampled_at_ns;
                if self.revision != s.revision {
                    self.revision = s.revision;
                    println!(
                        "{}",
                        serde_json::json!({"event":"session", "revision":s.revision, "sequence":s.sequence, "sampled_at_ns":s.sampled_at_ns, "until":s.until, "locked":s.locked, "focus":s.focus.as_ref().map(|f| &f.metadata)})
                    );
                }
            }
            Err(e) if !self.failed && !self.stopped => {
                self.failed = true;
                println!(
                    "{}",
                    serde_json::json!({"event":"source_failed", "kind":format!("{:?}", e.kind()), "observed_at_ns":boottime_ns(), "last_sampled_at_ns":self.sampled_at, "maximum_sample_gap_ns":self.maximum_sample_gap})
                );
            }
            _ => {}
        }
        reply
    }
}
fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.first().is_some_and(|s| s == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&a[1..]);
    }
    if a.first().is_some_and(|s| s == "--messages-read-worker") {
        return pokelinux_messages::worker::reader_entry(&a[1..]);
    }
    if a.first().is_some_and(|s| s == "--messages-worker") {
        return pokelinux_messages::worker::entry(&a[1..]);
    }
    if a.first().is_some_and(|a| a == "--messages-incoming-worker") {
        if a.len() != 8 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let generation = a[6].to_str().ok_or(io::ErrorKind::InvalidInput)?.to_owned();
        let fault = a[7].to_str().ok_or(io::ErrorKind::InvalidInput)?.to_owned();
        let backend = native_messages_incoming_binder_fixture::receiver::Synthetic::new(
            generation.clone(),
            fault,
        )?;
        return pokelinux_messages::worker::entry_with_backend(
            &a[1..6],
            Some((generation, Box::new(backend))),
        );
    }
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || a.len() != 13
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let source_socket = descriptor(&a[8])?;
    let source_alive = descriptor(&a[9])?;
    if source_socket == source_alive {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pid = a[10]
        .to_str()
        .and_then(|s| s.parse::<i32>().ok())
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = a[11]
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let source = a[12]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .to_owned();
    let timing_raw =
        descriptor(&std::env::var_os("NATIVE_COLD_TIMING_FD").ok_or(io::ErrorKind::InvalidInput)?)?;
    if timing_raw == source_socket || timing_raw == source_alive {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let timing_fd = adopt(timing_raw);
    fcntl_setfd(&timing_fd, FdFlags::CLOEXEC)?;
    let mut observer = TracedObserver {
        observer: Observer::new(adopt(source_socket), adopt(source_alive), pid, uid, source)?,
        timing: Timing {
            fd: timing_fd,
            previous_begin: 0,
            dropped: 0,
        },
        revision: 0,
        failed: false,
        stopped: false,
        sampled_at: 0,
        maximum_sample_gap: 0,
    };
    let mut messages = pokelinux_messages::worker::Projector::spawn_receiving(
        Path::new(&a[0]),
        Path::new(&a[4]),
        Path::new(&a[5]),
        &std::env::current_exe()?,
        &a[6..8],
    )?;
    let result = pokelinux_lifecycle::service::run_with_messages_and_session(
        Path::new(&a[0]),
        Path::new(&a[1]),
        Path::new(&a[2]),
        Path::new(&a[3]),
        None,
        Some(&mut messages),
        Some(&mut observer),
    );
    if !messages.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
        std::process::abort();
    }
    result
}
