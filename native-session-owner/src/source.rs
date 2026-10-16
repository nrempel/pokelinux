use pokelinux_lifecycle::{
    boottime_ns,
    session::{Provider, Snapshot},
};
use pokelinux_session::Observer;
use rustix::{fd::OwnedFd, process::Pid};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
};

pub const MAX_SOURCE_STARTUP_NS: u64 = 120_000_000_000;
pub(crate) const MAX_BOOTSTRAP: usize = pokelinux_lifecycle::MAX_PACKET;
/// Installer-owned values only; no application-selected executable or environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compositor {
    pub helper: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub source: String,
}
impl Compositor {
    pub(crate) fn validate(&self, until: u64) -> io::Result<()> {
        if rustix::process::getuid().as_raw() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let now = boottime_ns();
        if now >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if until - now > MAX_SOURCE_STARTUP_NS
            || self.source.len() != 32
            || !self
                .source
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.arguments.len() > 16
            || self
                .arguments
                .iter()
                .any(|v| v.len() > 4096 || v.contains('\0'))
            || self.environment.len() > 16
            || self.environment.iter().any(|(k, v)| {
                k.is_empty()
                    || k.len() > 64
                    || !k
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                    || k.starts_with("POKELINUX_SESSION_")
                    || v.len() > 4096
                    || v.contains('\0')
            })
            || serde_json::to_vec(self)?.len() > MAX_BOOTSTRAP - 128
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        pokelinux_lifecycle::service::trusted_executable(&self.helper)?;
        pokelinux_lifecycle::service::trusted_executable(&self.executable)
    }
}

/// Sole owner of the compositor Child. Source facts never confer policy authority.
pub struct SourceOwner {
    child: Child,
    alive: Option<OwnedFd>,
    observer: Option<Observer>,
    original_until_ns: u64,
    initialized: bool,
    terminal: Option<io::ErrorKind>,
    stopped: bool,
    status: Option<ExitStatus>,
}
impl std::fmt::Debug for SourceOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceOwner")
            .field("pid", &self.child.id())
            .field("terminal", &self.terminal)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
impl SourceOwner {
    /// All Err results precede spawn. Every postspawn failure retains the Child
    /// in a terminal owner; the caller fences authority before invoking stop.
    pub fn spawn(spec: Compositor, original_until_ns: u64) -> io::Result<Self> {
        spec.validate(original_until_ns)?;
        let packet = crate::bootstrap::encode(&spec, original_until_ns)?;
        let (receive, send) = rustix::net::socketpair(
            rustix::net::AddressFamily::UNIX,
            rustix::net::SocketType::SEQPACKET,
            rustix::net::SocketFlags::CLOEXEC | rustix::net::SocketFlags::NONBLOCK,
            None,
        )?;
        // Before spawn: an eager final exec may emit before Observer::new runs.
        rustix::net::sockopt::set_socket_passcred(&receive, true)?;
        rustix::net::sockopt::set_socket_passcred(&send, true)?;
        if boottime_ns() >= original_until_ns {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let child = Command::new(&spec.helper)
            .arg("--compositor-bootstrap")
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::from(send))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        // Custody is installed before pidfd acquisition, send, or Observer setup.
        let mut owner = Self {
            child,
            alive: None,
            observer: None,
            original_until_ns,
            initialized: false,
            terminal: None,
            stopped: false,
            status: None,
        };
        let setup = (|| -> io::Result<()> {
            let alive =
                rustix::process::pidfd_open(owner.pid(), rustix::process::PidfdFlags::NONBLOCK)?;
            let observer_alive = rustix::io::fcntl_dupfd_cloexec(&alive, 3)?;
            owner.alive = Some(alive);
            if boottime_ns() >= original_until_ns {
                return Err(io::ErrorKind::TimedOut.into());
            }
            pokelinux_lifecycle::ipc::send(&receive, &packet)?;
            let observer = Observer::new(receive, observer_alive, owner.pid(), 1000, spec.source)?;
            owner.observer = Some(observer);
            if boottime_ns() >= original_until_ns {
                return Err(io::ErrorKind::TimedOut.into());
            }
            Ok(())
        })();
        if let Err(error) = setup {
            owner.fail(error.kind());
        }
        Ok(owner)
    }
    pub fn pid(&self) -> Pid {
        // std Child's still-owned positive Linux child PID cannot be zero.
        Pid::from_raw(self.child.id() as i32).expect("owned child PID")
    }
    pub fn duplicate_pidfd(&self) -> io::Result<OwnedFd> {
        let alive = self.alive.as_ref().ok_or(io::ErrorKind::NotConnected)?;
        Ok(rustix::io::fcntl_dupfd_cloexec(alive, 3)?)
    }
    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.status
    }
    fn fail(&mut self, kind: io::ErrorKind) {
        self.terminal.get_or_insert(kind);
        if let Some(observer) = &mut self.observer {
            observer.stop();
        }
    }
    fn reap(&mut self) {
        if self.status.is_some() {
            return;
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.status = Some(status);
                self.fail(io::ErrorKind::NotConnected);
            }
            Ok(None) => {}
            Err(error) => self.fail(error.kind()),
        }
    }
}
impl Provider for SourceOwner {
    fn first_rejection(&self) -> Option<&pokelinux_lifecycle::session::SourceRejection> {
        self.observer.as_ref().and_then(Provider::first_rejection)
    }
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        // Terminal state must never bypass actual child reaping.
        self.reap();
        if let Some(kind) = self.terminal {
            return Err(kind.into());
        }
        if !self.initialized && boottime_ns() >= self.original_until_ns {
            self.fail(io::ErrorKind::TimedOut);
            return Err(io::ErrorKind::TimedOut.into());
        }
        let result = Provider::poll(self.observer.as_mut().ok_or(io::ErrorKind::NotConnected)?);
        match result {
            Ok(sample) => {
                if !self.initialized && boottime_ns() >= self.original_until_ns {
                    self.fail(io::ErrorKind::TimedOut);
                    return Err(io::ErrorKind::TimedOut.into());
                }
                self.initialized |= sample.is_some();
                Ok(sample)
            }
            Err(error) => {
                self.fail(error.kind());
                Err(error)
            }
        }
    }
    fn stop(&mut self) {
        // Actor calls this after Source authority has been fenced. No signal in
        // poll's failure path; no TERM timer that depends on later poll ordering.
        self.fail(io::ErrorKind::NotConnected);
        if self.stopped || self.status.is_some() {
            return;
        }
        self.stopped = true;
        if let Some(alive) = &self.alive {
            let _ = rustix::process::pidfd_send_signal(alive, rustix::process::Signal::KILL);
        } else {
            // Exact still-unreaped std Child during partial postspawn setup.
            let _ = self.child.kill();
        }
    }
    fn occupied(&self) -> bool {
        self.status.is_none()
    }
}
impl Drop for SourceOwner {
    fn drop(&mut self) {
        if self.status.is_none() {
            // A failed-stop is not cleanup. Never silently detach a Child.
            std::process::abort();
        }
    }
}
