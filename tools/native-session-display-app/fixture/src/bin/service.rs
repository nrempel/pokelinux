//! Single service-epoch display context. Metadata is not an app generation.
//! Actor Stop fences and reaps the source's SDK child under its cleanup deadline.
//! Fixture-owned final cleanup also covers actor preflight failures.
use pokelinux_display::{
    MAX_INITIALIZATION_NS, Metadata, SourceIdentity,
    supervisor::{State, Supervisor, worker_entry},
};
use pokelinux_lifecycle::{
    boottime_ns,
    messages::Provider as MessagesProvider,
    session::{Provider as SessionProvider, Snapshot},
};
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
    net,
    process::{Gid, Pid, Uid},
};
use std::{
    io,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::Path,
};

fn descriptor(value: &std::ffi::OsStr) -> io::Result<i32> {
    value
        .to_str()
        .and_then(|s| s.parse().ok())
        .filter(|v| *v >= 3)
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}
fn adopt(raw: i32) -> OwnedFd {
    // SAFETY: the isolated Root launcher exclusively transfers these two FDs;
    // parsed numerical equality is refused before either adoption.
    unsafe { OwnedFd::from_raw_fd(raw) }
}
struct Combined {
    observer: Observer,
    display: Supervisor,
    failed: bool,
    traced_display: Option<(State, bool)>,
    revision: u64,
    sampled_at: u64,
    maximum_sample_gap: u64,
    source_failed: bool,
}
impl Combined {
    fn trace_display(&mut self) {
        let status = (self.display.snapshot(), self.display.occupied());
        if self.traced_display != Some(status) {
            self.traced_display = Some(status);
            println!(
                "{}",
                serde_json::json!({"event":"display", "state":format!("{:?}", status.0),
                "occupied":status.1,"admission":self.display.admission(),"worker_pid":self.display.worker_pid(),
                "until_ns":self.display.until_ns(),"metadata_scope":"service_epoch"})
            );
        }
    }
    fn finish(&mut self, until: u64) {
        self.failed = true;
        self.display.stop();
        while self.display.occupied() {
            if boottime_ns() >= until {
                std::process::abort();
            }
            let _ = self.display.poll();
            self.trace_display();
            if self.display.occupied() {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }
}
impl SessionProvider for Combined {
    fn stop(&mut self) {
        self.failed = true;
        self.display.stop();
        SessionProvider::stop(&mut self.observer);
        self.trace_display();
    }
    fn occupied(&self) -> bool {
        self.display.occupied()
    }
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        // Always drive ownership/reaping, even after either input is terminal.
        let display = self.display.poll();
        // Drain/check actual source throughout setup; never defer its deadline.
        let observed = SessionProvider::poll(&mut self.observer);
        match &observed {
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
                        serde_json::json!({"event":"session", "revision":s.revision,"sequence":s.sequence,
                        "sampled_at_ns":s.sampled_at_ns,"until":s.until,"locked":s.locked,"focus":s.focus.as_ref().map(|v| &v.metadata)})
                    );
                }
            }
            Err(e) if !self.source_failed => {
                self.source_failed = true;
                println!(
                    "{}",
                    serde_json::json!({"event":"source_failed","kind":format!("{:?}",e.kind()),
                    "observed_at_ns":boottime_ns(),"last_sampled_at_ns":self.sampled_at,"maximum_sample_gap_ns":self.maximum_sample_gap})
                );
            }
            _ => {}
        }
        if display.is_err()
            || observed.is_err()
            || matches!(
                self.display.snapshot(),
                State::Failed | State::Stopping | State::Stopped
            )
        {
            self.failed = true;
            self.display.stop();
        }
        self.trace_display();
        if self.failed {
            return Err(io::ErrorKind::NotConnected.into());
        }
        if self.display.admission() {
            observed
        } else {
            Ok(None)
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args
        .first()
        .is_some_and(|s| s == "--display-context-worker")
    {
        return worker_entry();
    }
    if args.first().is_some_and(|s| s == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "--messages-worker") {
        return pokelinux_messages::worker::entry(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "--messages-read-worker") {
        return pokelinux_messages::worker::reader_entry(&args[1..]);
    }
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || args.len() != 13
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let socket_raw = descriptor(&args[6])?;
    let watch_raw = descriptor(&args[7])?;
    if socket_raw == watch_raw {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pid = args[8]
        .to_str()
        .and_then(|s| s.parse().ok())
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let uid = args[9]
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let source = args[10]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .to_owned();
    let watch = adopt(watch_raw);
    let expected = rustix::io::fcntl_dupfd_cloexec(&watch, 3)?;
    let observer = Observer::new(adopt(socket_raw), watch, pid, uid, source)?;
    let profiles = pokelinux_lifecycle::service::profile(Path::new(&args[2]))?;
    let profile = profiles
        .apps
        .iter()
        .find(|p| p.app == pokelinux_lifecycle::messages::APP)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let path = Path::new(&args[12]);
    let parent = std::fs::symlink_metadata(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    if !parent.is_dir() || parent.uid() != 0 || parent.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let listener = UnixListener::bind(path)?;
    rustix::fs::chown(path, Some(Uid::ROOT), Some(Gid::from_raw(profile.uid)))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
    let socket = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::STREAM,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )?;
    net::connect(&socket, &net::SocketAddrUnix::new(Path::new(&args[11]))?)?;
    let until = boottime_ns()
        .checked_add(MAX_INITIALIZATION_NS)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let metadata = Metadata {
        app: profile.app.clone(),
        instance: pokelinux_lifecycle::fresh_id()?,
    };
    let display = Supervisor::spawn(
        &std::env::current_exe()?,
        UnixStream::from(socket),
        expected,
        SourceIdentity { pid, uid, gid: uid },
        listener.into(),
        metadata,
        until,
    )?;
    let mut combined = Combined {
        observer,
        display,
        failed: false,
        traced_display: None,
        revision: 0,
        sampled_at: 0,
        maximum_sample_gap: 0,
        source_failed: false,
    };
    println!(
        "{}",
        serde_json::json!({"event":"display_spawned","worker_pid":combined.display.worker_pid(),
        "state":format!("{:?}",combined.display.snapshot()),"occupied":combined.display.occupied(),
        "until_ns":combined.display.until_ns(),"metadata_scope":"service_epoch"})
    );
    // No `?` can drop occupied setup ownership after successful spawn.
    let mut fallback_until = None;
    let result = (|| {
        let mut messages = pokelinux_messages::worker::Projector::spawn(
            Path::new(&args[0]),
            Path::new(&args[4]),
            Path::new(&args[5]),
            &std::env::current_exe()?,
        )?;
        let outcome = pokelinux_lifecycle::service::run_with_messages_and_session(
            Path::new(&args[0]),
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
            None,
            Some(&mut messages),
            Some(&mut combined),
        );
        // Actor preflight can fail before it assumes these owners. Fence the
        // display before any other drain and retain one caller fallback budget.
        SessionProvider::stop(&mut combined);
        let until =
            *fallback_until.get_or_insert_with(|| boottime_ns().saturating_add(2_000_000_000));
        if !messages.drain_until(until) {
            std::process::abort();
        }
        outcome
    })();
    // Preflight failures also stop and actually reap the retained setup child.
    // Normal actor Stop has already reaped the child under its original budget.
    SessionProvider::stop(&mut combined);
    let until = fallback_until.unwrap_or_else(|| boottime_ns().saturating_add(2_000_000_000));
    combined.finish(until);
    result
}
