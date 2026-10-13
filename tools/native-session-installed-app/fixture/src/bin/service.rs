//! Explicit inherited-FD transfer in an isolated binary. No production selector.
use pokelinux_lifecycle::session::{Provider as SessionProvider, Snapshot};
use pokelinux_lifecycle::{boottime_ns, messages::Provider};
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
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
// Fixture diagnostics forward the original Provider result unchanged. They never
// consume a second receiver, alter a sample deadline or restore a failed source.
struct TracedObserver {
    observer: Observer,
    revision: u64,
    failed: bool,
    sampled_at: u64,
    maximum_sample_gap: u64,
}
impl SessionProvider for TracedObserver {
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        let reply = SessionProvider::poll(&mut self.observer);
        match &reply {
            Ok(Some(s)) => {
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
            Err(e) if !self.failed => {
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
    if a.first().is_some_and(|s| s == "--messages-worker") {
        return pokelinux_messages::worker::entry(&a[1..]);
    }
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || a.len() != 11
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let source_socket = descriptor(&a[6])?;
    let source_alive = descriptor(&a[7])?;
    if source_socket == source_alive {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pid = a[8]
        .to_str()
        .and_then(|s| s.parse::<i32>().ok())
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = a[9]
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let source = a[10]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .to_owned();
    let mut observer = TracedObserver {
        observer: Observer::new(adopt(source_socket), adopt(source_alive), pid, uid, source)?,
        revision: 0,
        failed: false,
        sampled_at: 0,
        maximum_sample_gap: 0,
    };
    let mut messages = pokelinux_messages::worker::Projector::spawn(
        Path::new(&a[0]),
        Path::new(&a[4]),
        Path::new(&a[5]),
        &std::env::current_exe()?,
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
