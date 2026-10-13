//! Responsive parent ownership; blocking backend and uncertainty IO stay in one child.
use pokelinux_camera::{CaptureId, Generation};
use pokelinux_lifecycle::{
    boottime_ns,
    camera::{Event, Intent, Provider, State},
    fresh_id, ipc,
};
use rustix::{
    event::{PollFd, PollFlags, Timespec},
    fd::OwnedFd,
    net::{self, AddressFamily, SocketFlags, SocketType},
    process::{Pid, PidfdFlags},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const BOOTSTRAP_BUDGET_NS: u64 = 1_000_000_000;
pub const ATTEMPT_MARKER: &str = "camera-backend-attempt";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireIntent {
    owner: String,
    session: u64,
    configuration: u64,
    capture: u64,
    until_ns: u64,
}
impl WireIntent {
    fn new(owner: &str, intent: &Intent) -> Self {
        Self {
            owner: owner.into(),
            session: intent.generation.session,
            configuration: intent.generation.configuration,
            capture: intent.capture.0,
            until_ns: intent.until_ns,
        }
    }
    fn intent(&self) -> Intent {
        Intent {
            capture: CaptureId(self.capture),
            generation: Generation {
                session: self.session,
                configuration: self.configuration,
            },
            until_ns: self.until_ns,
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Message {
    Ready {
        owner: String,
        session: u64,
        configuration: u64,
    },
    State {
        intent: WireIntent,
        state: State,
    },
    Failed {
        intent: WireIntent,
    },
}
fn living(fd: &OwnedFd) -> io::Result<bool> {
    let mut polls = [PollFd::new(fd, PollFlags::IN)];
    rustix::event::poll(
        &mut polls,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    Ok(polls[0].revents().is_empty())
}

/// No pending output queue exists: only the actor may retry submit after EAGAIN.
pub struct Supervisor {
    child: Option<Child>,
    alive: Option<OwnedFd>,
    fd: OwnedFd,
    owner: String,
    generation: Generation,
    uid: u32,
    ready_until: u64,
    ready: bool,
    terminal: bool,
    spent: bool,
    active: Option<Intent>,
    busy: bool,
    failed: Option<CaptureId>,
}
impl Supervisor {
    /// Installation creates the root-owned private state directory. The parent
    /// performs no marker IO. Extra arguments belong only to the selected adapter.
    pub fn spawn(
        state: &Path,
        executable: &Path,
        backend_uid: u32,
        generation: Generation,
        adapter_args: &[OsString],
    ) -> io::Result<Self> {
        if rustix::process::geteuid().as_raw() != 0
            || !(10000..=60000).contains(&backend_uid)
            || generation.session == 0
            || generation.configuration == 0
            || adapter_args.len() > 8
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        pokelinux_lifecycle::service::trusted_executable(executable)?;
        let owner = fresh_id()?;
        let ready_until = boottime_ns().saturating_add(BOOTSTRAP_BUDGET_NS);
        let (fd, incoming) = net::socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        net::sockopt::set_socket_passcred(&fd, true)?;
        net::sockopt::set_socket_passcred(&incoming, true)?;
        let mut command = Command::new(executable);
        command
            .arg("--camera-worker")
            .arg(state)
            .arg(std::process::id().to_string())
            .arg(backend_uid.to_string())
            .arg(&owner)
            .arg(generation.session.to_string())
            .arg(generation.configuration.to_string())
            .arg(ready_until.to_string())
            .args(adapter_args)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::from(incoming))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn()?;
        // Retain actual Child before the first fallible operation after spawn.
        let mut supervisor = Self {
            child: Some(child),
            alive: None,
            fd,
            owner,
            generation,
            uid: backend_uid,
            ready_until,
            ready: false,
            terminal: false,
            spent: false,
            active: None,
            busy: false,
            failed: None,
        };
        let pid = Pid::from_raw(supervisor.child.as_ref().unwrap().id() as i32).unwrap();
        match rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK) {
            Ok(fd) => supervisor.alive = Some(fd),
            Err(error) => {
                if !supervisor.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
                    std::process::abort();
                }
                return Err(error.into());
            }
        }
        Ok(supervisor)
    }
    fn retire(&mut self) {
        // Provenance/publication fencing always precedes signal delivery.
        self.terminal = true;
        self.ready = false;
        self.busy = false;
        if let Some(intent) = self.active {
            self.failed.get_or_insert(intent.capture);
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
    }
    fn take_failure(&mut self) -> Option<Event> {
        self.failed.take().map(|capture| Event::Failed { capture })
    }
}
impl Provider for Supervisor {
    fn incarnation(&self) -> &str {
        &self.owner
    }
    fn generation(&self) -> Generation {
        self.generation
    }
    fn occupied(&self) -> bool {
        self.child.is_some()
    }
    fn busy(&self) -> bool {
        self.busy && !self.terminal
    }
    fn available(&self) -> bool {
        self.ready
            && !self.terminal
            && !self.spent
            && self.child.is_some()
            && self
                .alive
                .as_ref()
                .is_some_and(|fd| living(fd).unwrap_or(false))
    }
    fn current(&self, capture: CaptureId) -> bool {
        !self.terminal
            && self.child.is_some()
            && self
                .active
                .is_some_and(|intent| intent.capture == capture && boottime_ns() < intent.until_ns)
            && self
                .alive
                .as_ref()
                .is_some_and(|fd| living(fd).unwrap_or(false))
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        if !self.available()
            || intent.generation != self.generation
            || intent.capture.0 == 0
            || boottime_ns() >= intent.until_ns
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let packet = serde_json::to_vec(&WireIntent::new(&self.owner, intent))?;
        match ipc::send(&self.fd, &packet) {
            Ok(()) => {
                self.spent = true;
                self.active = Some(*intent);
                self.busy = true;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(false),
            Err(error) => {
                self.retire();
                Err(error)
            }
        }
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.retire();
                    self.child = None;
                    self.alive = None;
                }
                Ok(None) => {}
                Err(error) => {
                    self.retire();
                    return Err(error);
                }
            }
        }
        if self.child.is_some()
            && !self.terminal
            && ((!self.ready && boottime_ns() >= self.ready_until)
                || self
                    .active
                    .is_some_and(|intent| boottime_ns() >= intent.until_ns)
                || self
                    .alive
                    .as_ref()
                    .is_none_or(|fd| !living(fd).unwrap_or(false)))
        {
            self.retire();
        }
        if self.terminal {
            return Ok(self.take_failure());
        }
        let received = match ipc::receive(&self.fd) {
            Ok(Some(value)) => value,
            Ok(None) => return Ok(None),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => return Ok(None),
            Err(error) => {
                self.retire();
                return Err(error);
            }
        };
        if received.1.uid.as_raw() != self.uid
            || Some(received.1.pid.as_raw_nonzero().get() as u32)
                != self.child.as_ref().map(Child::id)
        {
            self.retire();
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let message: Message = match serde_json::from_slice(&received.0) {
            Ok(message) => message,
            Err(error) => {
                self.retire();
                return Err(error.into());
            }
        };
        match message {
            Message::Ready {
                owner,
                session,
                configuration,
            } if !self.ready
                && !self.spent
                && owner == self.owner
                && session == self.generation.session
                && configuration == self.generation.configuration
                && boottime_ns() < self.ready_until =>
            {
                self.ready = true;
                Ok(None)
            }
            Message::State { intent, state }
                if intent.owner == self.owner
                    && self.active == Some(intent.intent())
                    && self.busy
                    && self.current(CaptureId(intent.capture))
                    && (!state.fence_ready || state.returned)
                    && (!state.capture_complete || state.returned) =>
            {
                self.busy = false;
                Ok(Some(Event::Complete {
                    capture: CaptureId(intent.capture),
                    state,
                }))
            }
            Message::Failed { intent }
                if intent.owner == self.owner && self.active == Some(intent.intent()) =>
            {
                self.retire();
                Ok(self.take_failure())
            }
            _ => {
                self.retire();
                Err(io::ErrorKind::InvalidData.into())
            }
        }
    }
    fn revoke(&mut self, capture: CaptureId) {
        if self.active.is_some_and(|intent| intent.capture == capture) {
            self.retire();
        }
    }
    fn stop(&mut self) {
        self.retire();
    }
    fn drain_until(&mut self, until_ns: u64) -> bool {
        self.stop();
        while self.occupied() {
            if boottime_ns() >= until_ns {
                return false;
            }
            let _ = self.poll();
            if self.occupied() {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        true
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        if self.child.is_some() {
            std::process::abort();
        }
    }
}

/// Device adapters implement only the current child-side capture boundary.
/// Constructing this value must not perform Binder lookup/registration/submission.
pub trait Backend {
    fn capture(&mut self, intent: &Intent, control: &Control) -> io::Result<()>;
}

#[derive(Clone)]
pub struct Control {
    fd: Arc<OwnedFd>,
    parent: Arc<OwnedFd>,
    wire: WireIntent,
    reported: Arc<AtomicBool>,
}
impl Control {
    pub fn check(&self) -> io::Result<()> {
        if boottime_ns() >= self.wire.until_ns || !living(&self.parent)? {
            Err(io::ErrorKind::TimedOut.into())
        } else {
            Ok(())
        }
    }
    pub fn report(&self, state: State) -> io::Result<()> {
        self.check()?;
        if self.reported.swap(true, Ordering::SeqCst) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        ipc::send(
            &self.fd,
            &serde_json::to_vec(&Message::State {
                intent: self.wire.clone(),
                state,
            })?,
        )
    }
    /// A terminal adapter fact may follow the single command observation.
    /// Failure carries no state/publication and is never queued for retry.
    pub fn fail(&self) -> io::Result<()> {
        ipc::send(
            &self.fd,
            &serde_json::to_vec(&Message::Failed {
                intent: self.wire.clone(),
            })?,
        )
    }
}

fn number(value: &std::ffi::OsStr) -> io::Result<u64> {
    value
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput.into())
}
fn protect_attempt(directory: &Path, owner: &str) -> io::Result<(File, File)> {
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o077 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for parent in directory.ancestors().skip(1) {
        let metadata = fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(directory.join("camera-backend.lock"))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
    let directory_fd = File::open(directory)?;
    {
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(directory.join(ATTEMPT_MARKER))?;
        writeln!(marker, "camera-attempt\n{owner}")?;
        marker.sync_all()?;
        directory_fd.sync_all()?;
        // Never retain a writable marker descriptor across the UID drop.
    }
    Ok((lock, directory_fd))
}

/// Trusted root entry protects uncertainty, drops credentials, then receives one
/// authorized parent command. No adapter method runs during bootstrap.
pub fn entry(args: &[OsString], backend: &mut dyn Backend) -> io::Result<()> {
    if args.len() != 7
        || rustix::process::getuid().as_raw() != 0
        || rustix::process::geteuid().as_raw() != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let directory = Path::new(&args[0]);
    let parent = u32::try_from(number(&args[1])?).map_err(|_| io::ErrorKind::InvalidInput)?;
    let uid = u32::try_from(number(&args[2])?).map_err(|_| io::ErrorKind::InvalidInput)?;
    let owner = args[3].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let generation = Generation {
        session: number(&args[4])?,
        configuration: number(&args[5])?,
    };
    let ready_until = number(&args[6])?;
    if !(10000..=60000).contains(&uid)
        || owner.len() != 32
        || !owner.bytes().all(|byte| byte.is_ascii_hexdigit())
        || generation.session == 0
        || generation.configuration == 0
        || boottime_ns() >= ready_until
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    let expected_parent = Pid::from_raw(parent as i32).ok_or(io::ErrorKind::InvalidInput)?;
    if rustix::process::getppid() != Some(expected_parent) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent_fd = rustix::process::pidfd_open(expected_parent, PidfdFlags::NONBLOCK)?;
    let (_lock, _directory_fd) = protect_attempt(directory, owner)?;
    let fd = Arc::new(rustix::io::dup(std::io::stdin())?);
    rustix::thread::set_no_new_privs(true)?;
    rustix::thread::set_thread_groups(&[])?;
    let gid = rustix::process::Gid::from_raw(uid);
    rustix::thread::set_thread_res_gid(gid, gid, gid)?;
    let uid = rustix::process::Uid::from_raw(uid);
    rustix::thread::set_thread_res_uid(uid, uid, uid)?;
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    if rustix::process::getppid() != Some(expected_parent)
        || !living(&parent_fd)?
        || boottime_ns() >= ready_until
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    ipc::send(
        &fd,
        &serde_json::to_vec(&Message::Ready {
            owner: owner.into(),
            session: generation.session,
            configuration: generation.configuration,
        })?,
    )?;
    let parent_fd = Arc::new(parent_fd);
    let mut active: Option<Control> = None;
    loop {
        if !living(&parent_fd)?
            || active
                .as_ref()
                .is_some_and(|control| control.check().is_err())
        {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match ipc::receive(&fd) {
            Ok(Some((wire, cred))) => {
                if active.is_some()
                    || cred.uid.as_raw() != 0
                    || cred.pid.as_raw_nonzero().get() as u32 != parent
                {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                let wire: WireIntent = serde_json::from_slice(&wire)?;
                let intent = wire.intent();
                if wire.owner != owner
                    || intent.generation != generation
                    || intent.capture.0 == 0
                    || boottime_ns() >= intent.until_ns
                {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let control = Control {
                    fd: fd.clone(),
                    parent: parent_fd.clone(),
                    wire,
                    reported: Arc::new(AtomicBool::new(false)),
                };
                control.check()?;
                if let Err(error) = backend.capture(&intent, &control) {
                    let _ = ipc::send(
                        &fd,
                        &serde_json::to_vec(&Message::Failed {
                            intent: control.wire.clone(),
                        })?,
                    );
                    return Err(error);
                }
                active = Some(control);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}
