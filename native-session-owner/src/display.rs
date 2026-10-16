//! Sounds-only per-app listener and owned SDK supervisor adaptation.
//! Source process custody remains with the installed session owner.
use pokelinux_display::{
    Metadata, SourceIdentity,
    listener::Listener,
    supervisor::{State as SdkState, Supervisor},
};
use pokelinux_kernel_identity::ProcessView;
use pokelinux_lifecycle::{
    AppProfile, boottime_ns,
    display::{self, Context, State},
};
use rustix::{fd::OwnedFd, net};
use std::{
    fs, io,
    os::unix::{fs::MetadataExt, net::UnixStream},
    path::{Path, PathBuf},
};

fn protected_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for ancestor in path.ancestors() {
        let value = fs::symlink_metadata(ancestor)?;
        if !value.is_dir() || value.uid() != 0 || value.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}

pub struct Factory {
    parent: PathBuf,
    original: PathBuf,
    source: OwnedFd,
    identity: SourceIdentity,
    executable: PathBuf,
}
impl Factory {
    /// Trusted fixed-role configuration; no app supplies paths or source handles.
    pub fn new(
        parent: PathBuf,
        original: PathBuf,
        source: OwnedFd,
        identity: SourceIdentity,
        executable: PathBuf,
    ) -> io::Result<Self> {
        if rustix::process::getuid().as_raw() != 0
            || rustix::process::geteuid().as_raw() != 0
            || identity.uid != 1000
            || identity.gid != 1000
            || !original.is_absolute()
            || !ProcessView::open()?.matches(&source, &source, identity.pid.as_raw_pid() as u32)?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        protected_directory(&parent)?;
        pokelinux_lifecycle::service::trusted_executable(&executable)?;
        rustix::io::fcntl_setfd(&source, rustix::io::FdFlags::CLOEXEC)?;
        Ok(Self {
            parent,
            original,
            source,
            identity,
            executable,
        })
    }
}
struct AppContext {
    listener: Listener,
    sdk: Supervisor,
    arguments: Vec<String>,
    terminal: bool,
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
}
impl Context for AppContext {
    fn poll(&mut self) -> io::Result<State> {
        let result = self.sdk.poll();
        if result.is_err() {
            self.terminal = true;
        }
        result.map(|_| self.state())
    }
    fn arguments(&self) -> &[String] {
        &self.arguments
    }
    fn stop(&mut self) -> io::Result<()> {
        self.terminal = true;
        let withdrawal = self.listener.withdraw();
        // Withdrawal failure never skips signalling the owned SDK.
        self.sdk.stop();
        withdrawal
    }
    fn occupied(&self) -> bool {
        self.sdk.occupied()
    }
}
impl display::Factory for Factory {
    fn start(
        &mut self,
        profile: &AppProfile,
        instance: &str,
        until: u64,
    ) -> io::Result<Box<dyn Context>> {
        // Fixed installed role; no application supplies another profile or UID.
        if profile.app != pokelinux_lifecycle::audio::APP
            || profile.uid != 11042
            || profile.args.len() != 6
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let now = boottime_ns();
        if until <= now || until - now > display::INITIALIZATION_BUDGET_NS {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // All fallible local setup precedes helper creation. Listener Drop
        // withdraws only this instance when pre-transfer setup fails.
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
            self.identity,
            fd,
            Metadata {
                app: profile.app.clone(),
                instance: instance.to_owned(),
            },
            until,
        )?;
        // Spawn returns all post-spawn failures as retained occupied ownership.
        Ok(Box::new(AppContext {
            listener,
            sdk,
            arguments,
            terminal: false,
        }))
    }
}
