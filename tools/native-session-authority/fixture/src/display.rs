//! Private per-app display ownership without lifecycle trait or DTO identities.
//! The connected compositor peer supplies display identity, never session facts
//! or authentication authority.
#![forbid(unsafe_code)]
use pokelinux_display::{
    Metadata, SourceIdentity,
    listener::Listener,
    supervisor::{State as SdkState, Supervisor},
};
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use rustix::{fd::OwnedFd, net};
use std::{
    io,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

const MESSAGES_APP: &str = "org.omarchy.Messages";
const MESSAGES_UID: u32 = 11010;
const MAX_ARGUMENT_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Preparing,
    Ready,
    Terminal,
}

fn connect_peer(path: &Path) -> io::Result<(UnixStream, SourceIdentity, OwnedFd)> {
    let socket = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::STREAM,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )?;
    net::connect(&socket, &net::SocketAddrUnix::new(path)?)?;
    let credentials = net::sockopt::socket_peercred(&socket)?;
    let source = socket_peer_pidfd(&socket)?;
    Ok((
        UnixStream::from(socket),
        SourceIdentity {
            pid: credentials.pid,
            uid: credentials.uid.as_raw(),
            gid: credentials.gid.as_raw(),
        },
        source,
    ))
}

/// Keeps one exact compositor peer across all per-app launch instances.
/// Construction creates no helper. Before calling, the root fixture must apply
/// the normal lifecycle trusted_executable check to its selected helper path.
/// Protected procfs/common namespaces and sole child-wait ownership are caller
/// requirements, as for the underlying SDK supervisor.
pub struct Factory {
    parent: PathBuf,
    original: PathBuf,
    helper: PathBuf,
    source: OwnedFd,
    identity: SourceIdentity,
}

impl Factory {
    pub fn new(
        parent: &Path,
        original: &Path,
        helper: &Path,
        expected_desktop_uid: u32,
    ) -> io::Result<Self> {
        if rustix::process::getuid().as_raw() != 0
            || rustix::process::geteuid().as_raw() != 0
            || expected_desktop_uid == 0
            || !parent.is_absolute()
            || !original.is_absolute()
            || !helper.is_absolute()
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let (_stream, identity, source) = connect_peer(original)?;
        if identity.uid != expected_desktop_uid
            || !ProcessView::open()?.matches(&source, &source, identity.pid.as_raw_pid() as u32)?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self {
            parent: parent.to_owned(),
            original: original.to_owned(),
            helper: helper.to_owned(),
            source,
            identity,
        })
    }

    pub fn compositor_pid(&self) -> u32 {
        self.identity.pid.as_raw_pid() as u32
    }

    /// Return all helper ownership to the actor's exact reserved app slot.
    /// The caller's original absolute initialization deadline is not renewed.
    /// Every error precedes helper creation; post-spawn failure remains an owned
    /// terminal Context which the actor must stop and poll through actual reap.
    pub fn start(
        &mut self,
        app: &str,
        uid: u32,
        args: &[String],
        instance: &str,
        until: u64,
    ) -> io::Result<Context> {
        if app != MESSAGES_APP
            || uid != MESSAGES_UID
            || args.len() != 6
            || args
                .iter()
                .any(|arg| arg.len() > MAX_ARGUMENT_BYTES || arg.contains('\0'))
            || instance.len() != 32
            || !instance
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let (stream, identity, source) = connect_peer(&self.original)?;
        if identity.pid != self.identity.pid
            || identity.uid != self.identity.uid
            || identity.gid != self.identity.gid
            || !ProcessView::open()?.matches(&self.source, &source, self.compositor_pid())?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let expected = rustix::io::fcntl_dupfd_cloexec(&self.source, 3)?;
        // Listener Drop withdraws only its own inode on any pre-spawn failure.
        let mut listener = Listener::bind(&self.parent, instance, uid)?;
        let listener_fd = listener.take_fd()?;
        let mut arguments = args.to_vec();
        arguments[2] = listener
            .path()
            .to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .to_owned();
        if arguments[2].len() > MAX_ARGUMENT_BYTES {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let metadata = Metadata {
            app: app.to_owned(),
            instance: instance.to_owned(),
        };
        let sdk = Supervisor::spawn(
            &self.helper,
            stream,
            expected,
            self.identity,
            listener_fd,
            metadata,
            until,
        )?;
        // No fallible operation follows spawn before custody transfer.
        let terminal = !matches!(sdk.snapshot(), SdkState::Initializing | SdkState::Ready);
        Ok(Context {
            listener,
            sdk,
            arguments,
            terminal,
        })
    }
}

/// One per-app pathname and its sole SDK child owner. The lifecycle actor retains
/// this object until both the app and this context finish. No independent cleanup
/// deadline or fallback reaper is introduced by the facade.
#[must_use]
pub struct Context {
    listener: Listener,
    sdk: Supervisor,
    arguments: Vec<String>,
    terminal: bool,
}

impl Context {
    pub fn state(&self) -> State {
        if self.terminal {
            return State::Terminal;
        }
        match self.sdk.snapshot() {
            SdkState::Initializing => State::Preparing,
            SdkState::Ready => State::Ready,
            SdkState::Failed | SdkState::Stopping | SdkState::Stopped => State::Terminal,
        }
    }

    /// One bounded supervisor turn, including after Terminal while occupied.
    pub fn poll(&mut self) -> io::Result<State> {
        let result = self.sdk.poll();
        if result.is_err() || self.state() == State::Terminal {
            self.terminal = true;
        }
        result.map(|_| self.state())
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn listener_path(&self) -> &Path {
        self.listener.path()
    }

    pub fn worker_pid(&self) -> u32 {
        self.sdk.worker_pid()
    }

    pub fn until_ns(&self) -> u64 {
        self.sdk.until_ns()
    }

    /// Withdraw before signalling even when withdrawal itself reports an error.
    pub fn stop(&mut self) -> io::Result<()> {
        self.terminal = true;
        let withdrawal = self.listener.withdraw();
        self.sdk.stop();
        withdrawal
    }

    /// Clears only after the SDK supervisor's actual direct-child wait.
    pub fn occupied(&self) -> bool {
        self.sdk.occupied()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.occupied() {
            // Fence, then abort rather than granting a new wait budget or letting
            // Rust Child Drop detach an unresolved SDK worker.
            let _ = self.stop();
            std::process::abort();
        }
    }
}
