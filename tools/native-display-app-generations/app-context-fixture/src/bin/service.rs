//! Actual per-app SDK provisioner. Source observation stays independently owned.
use pokelinux_display::{
    Metadata, SourceIdentity,
    listener::Listener,
    supervisor::{State as SdkState, Supervisor, worker_entry},
};
use pokelinux_lifecycle::{
    AppProfile, boottime_ns,
    display::{Context, Factory, State},
    messages::Provider as MessagesProvider,
    session::{Provider as SessionProvider, Snapshot},
};
use pokelinux_session::Observer;
use rustix::{
    fd::{FromRawFd, OwnedFd},
    net,
    process::Pid,
};
use std::{
    io,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

fn descriptor(value: &std::ffi::OsStr) -> io::Result<i32> {
    value
        .to_str()
        .and_then(|s| s.parse().ok())
        .filter(|v| *v >= 3)
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}
fn adopt(raw: i32) -> OwnedFd {
    // SAFETY: this isolated Root parent exclusively transfers distinct descriptors.
    unsafe { OwnedFd::from_raw_fd(raw) }
}
struct TracedObserver {
    observer: Observer,
    revision: u64,
    failed: bool,
}
impl SessionProvider for TracedObserver {
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        let result = SessionProvider::poll(&mut self.observer);
        match &result {
            Ok(Some(s)) if s.revision != self.revision => {
                self.revision = s.revision;
                println!(
                    "{}",
                    serde_json::json!({"event":"session","revision":s.revision,
                    "sequence":s.sequence,"sampled_at_ns":s.sampled_at_ns,"until":s.until,
                    "locked":s.locked,"focus":s.focus.as_ref().map(|f| &f.metadata)})
                );
            }
            Err(e) if !self.failed => {
                self.failed = true;
                println!(
                    "{}",
                    serde_json::json!({"event":"source_failed","kind":format!("{:?}",e.kind()),"observed_at_ns":boottime_ns()})
                );
            }
            _ => {}
        }
        result
    }
    fn stop(&mut self) {
        SessionProvider::stop(&mut self.observer);
    }
    fn occupied(&self) -> bool {
        SessionProvider::occupied(&self.observer)
    }
}
struct AppContext {
    listener: Listener,
    sdk: Supervisor,
    arguments: Vec<String>,
    app: String,
    instance: String,
    terminal: bool,
    traced: Option<(State, bool)>,
}
impl AppContext {
    fn state(&self) -> State {
        if self.terminal {
            return State::Terminal;
        }
        match self.sdk.snapshot() {
            SdkState::Initializing => State::Preparing,
            SdkState::Ready => State::Ready,
            SdkState::Failed | SdkState::Stopping | SdkState::Stopped => State::Terminal,
        }
    }
    fn trace(&mut self) {
        let value = (self.state(), self.sdk.occupied());
        if self.traced != Some(value) {
            self.traced = Some(value);
            println!(
                "{}",
                serde_json::json!({"event":"app_context","app":self.app,
                "instance":self.instance,"listener":self.listener.path(),
                "worker_pid":self.sdk.worker_pid(),"state":format!("{:?}",value.0),
                "sdk_state":format!("{:?}",self.sdk.snapshot()),"occupied":value.1,
                "admission":self.sdk.admission() && !self.terminal,"until_ns":self.sdk.until_ns(),
                "metadata_scope":"app_generation","observed_at_ns":boottime_ns()})
            );
        }
    }
}
impl Context for AppContext {
    fn poll(&mut self) -> io::Result<State> {
        let result = self.sdk.poll();
        if result.is_err() {
            self.terminal = true;
        }
        self.trace();
        result.map(|_| self.state())
    }
    fn arguments(&self) -> &[String] {
        &self.arguments
    }
    fn stop(&mut self) -> io::Result<()> {
        self.terminal = true;
        // Even a withdrawal error must still signal the retained owned SDK.
        let withdrawal = self.listener.withdraw();
        println!(
            "{}",
            serde_json::json!({"event":"app_listener_withdrawal","instance":self.instance,
            "listener":self.listener.path(),"successful":withdrawal.is_ok(),
            "before_sdk_stop":true,"observed_at_ns":boottime_ns()})
        );
        self.sdk.stop();
        self.trace();
        withdrawal
    }
    fn occupied(&self) -> bool {
        self.sdk.occupied()
    }
}
struct DisplayFactory {
    parent: PathBuf,
    original: PathBuf,
    source: OwnedFd,
    identity: SourceIdentity,
    executable: PathBuf,
}
impl Factory for DisplayFactory {
    fn start(
        &mut self,
        profile: &AppProfile,
        instance: &str,
        until: u64,
    ) -> io::Result<Box<dyn Context>> {
        if profile.app != pokelinux_lifecycle::messages::APP || profile.args.len() != 6 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // Every fallible local setup precedes helper creation. Listener Drop
        // identity-checks withdrawal for failures before ownership transfer.
        let socket = net::socket_with(
            net::AddressFamily::UNIX,
            net::SocketType::STREAM,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
            None,
        )?;
        net::connect(&socket, &net::SocketAddrUnix::new(&self.original)?)?;
        let expected = rustix::io::fcntl_dupfd_cloexec(&self.source, 3)?;
        let mut listener = Listener::bind(&self.parent, instance, profile.uid)?;
        let fd = listener.take_fd()?;
        let mut arguments = profile.args.clone();
        arguments[2] = listener
            .path()
            .to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .to_owned();
        let sdk = Supervisor::spawn(
            &self.executable,
            UnixStream::from(socket),
            expected,
            SourceIdentity {
                pid: self.identity.pid,
                uid: self.identity.uid,
                gid: self.identity.gid,
            },
            fd,
            Metadata {
                app: profile.app.clone(),
                instance: instance.to_owned(),
            },
            until,
        )?;
        // Supervisor guarantees post-spawn failure returns retained ownership.
        let mut context = AppContext {
            listener,
            sdk,
            arguments,
            app: profile.app.clone(),
            instance: instance.to_owned(),
            terminal: false,
            traced: None,
        };
        context.trace();
        Ok(Box::new(context))
    }
}
fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.first().is_some_and(|s| s == "--display-context-worker") {
        return worker_entry();
    }
    if a.first().is_some_and(|s| s == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&a[1..]);
    }
    if a.first().is_some_and(|s| s == "--messages-worker") {
        return pokelinux_messages::worker::entry(&a[1..]);
    }
    if a.first().is_some_and(|s| s == "--messages-read-worker") {
        return pokelinux_messages::worker::reader_entry(&a[1..]);
    }
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || a.len() != 13
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let socket_raw = descriptor(&a[6])?;
    let watch_raw = descriptor(&a[7])?;
    if socket_raw == watch_raw {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pid = a[8]
        .to_str()
        .and_then(|s| s.parse().ok())
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
    let watch = adopt(watch_raw);
    let expected = rustix::io::fcntl_dupfd_cloexec(&watch, 3)?;
    let observer = Observer::new(adopt(socket_raw), watch, pid, uid, source)?;
    let mut session = TracedObserver {
        observer,
        revision: 0,
        failed: false,
    };
    let mut factory = DisplayFactory {
        parent: PathBuf::from(&a[12]),
        original: PathBuf::from(&a[11]),
        source: expected,
        identity: SourceIdentity { pid, uid, gid: uid },
        executable: std::env::current_exe()?,
    };
    let mut messages = pokelinux_messages::worker::Projector::spawn(
        Path::new(&a[0]),
        Path::new(&a[4]),
        Path::new(&a[5]),
        &std::env::current_exe()?,
    )?;
    let outcome = pokelinux_lifecycle::service::run_with_services(
        Path::new(&a[0]),
        Path::new(&a[1]),
        Path::new(&a[2]),
        Path::new(&a[3]),
        None,
        Some(&mut messages),
        Some(&mut session),
        Some(&mut factory),
        None,
        None,
    );
    // No app Context exists before the actor adopts the factory. The actor is
    // its sole actual-wait owner; its original cleanup budget retires all slots.
    SessionProvider::stop(&mut session);
    let until = boottime_ns().saturating_add(2_000_000_000);
    if !messages.drain_until(until) {
        std::process::abort();
    }
    outcome
}
