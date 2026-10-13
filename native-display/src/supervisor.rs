//! One owned setup/context process. The supervising actor never calls the SDK.
//! Caller must protect procfs/namespaces and listener/executable provisioning,
//! retain sole child-reaping ownership and use normal SIGCHLD semantics.
use crate::{Context, MAX_INITIALIZATION_NS, Metadata, Phase, SourceIdentity, boottime_ns, living};
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags, SocketFlags, UCred,
    },
    process::{self, Pid, PidfdFlags, Resource, Rlimit, Signal},
};
use std::{
    fs,
    io::{self, IoSlice, IoSliceMut, Read},
    mem::MaybeUninit,
    os::unix::{fs::MetadataExt, net::UnixStream},
    path::Path,
    process::{Child, Command, Stdio},
};

pub const WORKER_MEMORY_BYTES: u64 = 128 * 1024 * 1024;
pub const WORKER_FD_LIMIT: u64 = 64;
const MAX_PACKET: usize = 512;
const VERSION: u8 = 1;
const READY: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Initializing,
    Ready,
    Failed,
    Stopping,
    Stopped,
}

struct Bootstrap {
    bytes: Vec<u8>,
    // Sole parent copies, dropped immediately following successful transfer.
    handles: [OwnedFd; 4],
}
struct Packet {
    bytes: Vec<u8>,
    credentials: UCred,
    handles: Vec<OwnedFd>,
}

fn receive(fd: impl AsFd) -> io::Result<Option<Packet>> {
    let mut bytes = [0; MAX_PACKET];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(5))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        result => result?,
    };
    let mut invalid = message.bytes == 0
        || message
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut credentials = None;
    let mut handles = Vec::new();
    let mut rights_messages = 0;
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if credentials.is_none() => {
                credentials = Some(c)
            }
            RecvAncillaryMessage::ScmRights(fds) => {
                rights_messages += 1;
                handles.extend(fds);
            }
            _ => invalid = true,
        }
    }
    if invalid || rights_messages > 1 || handles.len() > 4 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(Packet {
        bytes: bytes[..message.bytes].to_vec(),
        credentials: credentials.ok_or(io::ErrorKind::PermissionDenied)?,
        handles,
    }))
}
fn send(fd: impl AsFd, bytes: &[u8], handles: &[BorrowedFd<'_>]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET || handles.len() > 4 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(4))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    if !handles.is_empty() && !ancillary.push(SendAncillaryMessage::ScmRights(handles)) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let written = net::sendmsg(
        fd,
        &[IoSlice::new(bytes)],
        &mut ancillary,
        SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
    )?;
    if written != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
fn ready(nonce: &[u8; 16], instance: &str, until: u64) -> Vec<u8> {
    let mut bytes = vec![VERSION, READY];
    bytes.extend(nonce);
    bytes.extend(instance.as_bytes());
    bytes.extend(until.to_le_bytes());
    bytes
}
fn trusted_executable(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let file = fs::symlink_metadata(path)?;
    if !file.is_file() || file.uid() != 0 || file.mode() & 0o022 != 0 || file.mode() & 0o111 == 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for ancestor in path.ancestors().skip(1) {
        let meta = fs::symlink_metadata(ancestor)?;
        if !meta.is_dir() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}

/// An initializing or ready instance retains exactly one direct child.
/// `stop` and `poll` perform bounded local work. `occupied` clears only after
/// actual `Child::try_wait`, including after kill or a pidfd death observation.
/// A caller must keep polling a terminal occupied instance. Dropping it before
/// reap aborts the owner rather than detaching an unresolved process.
#[must_use]
pub struct Supervisor {
    child: Option<Child>,
    worker_watch: Option<OwnedFd>,
    source: OwnedFd,
    channel: Option<OwnedFd>,
    bootstrap: Option<Bootstrap>,
    expected_ready: Vec<u8>,
    until: u64,
    worker_pid: u32,
    state: State,
}
impl Supervisor {
    /// All errors returned occur before a child exists. Failures following
    /// successful spawn return a terminal but still owned supervisor.
    pub fn spawn(
        executable: &Path,
        stream: UnixStream,
        expected: OwnedFd,
        identity: SourceIdentity,
        listener: OwnedFd,
        metadata: Metadata,
        until: u64,
    ) -> io::Result<Self> {
        let now = boottime_ns();
        if process::getuid().as_raw() != 0
            || !metadata.valid()
            || until <= now
            || until - now > MAX_INITIALIZATION_NS
            || net::sockopt::socket_domain(&listener)? != net::AddressFamily::UNIX
            || net::sockopt::socket_type(&listener)? != net::SocketType::STREAM
            || !net::sockopt::socket_acceptconn(&listener)?
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        trusted_executable(executable)?;
        for fd in [&expected, &listener] {
            rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::CLOEXEC)?;
        }
        rustix::io::fcntl_setfd(&stream, rustix::io::FdFlags::CLOEXEC)?;
        stream.set_nonblocking(true)?;
        let credentials = net::sockopt::socket_peercred(&stream)?;
        let source = socket_peer_pidfd(&stream)?;
        if credentials.pid != identity.pid
            || credentials.uid.as_raw() != identity.uid
            || credentials.gid.as_raw() != identity.gid
            || !ProcessView::open()?.matches(
                &expected,
                &source,
                identity.pid.as_raw_pid() as u32,
            )?
            || boottime_ns() >= until
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let parent = process::pidfd_open(process::getpid(), PidfdFlags::NONBLOCK)?;
        let (channel, worker_channel) = net::socketpair(
            net::AddressFamily::UNIX,
            net::SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        // Credentials must be enabled before any queued bootstrap or reply.
        for fd in [&channel, &worker_channel] {
            net::sockopt::set_socket_passcred(fd, true)?;
        }
        let mut nonce = [0; 16];
        fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
        let expected_ready = ready(&nonce, &metadata.instance, until);
        let mut bytes = vec![VERSION];
        bytes.extend(until.to_le_bytes());
        bytes.extend(identity.pid.as_raw_pid().to_le_bytes());
        bytes.extend(identity.uid.to_le_bytes());
        bytes.extend(identity.gid.to_le_bytes());
        bytes.extend(nonce);
        bytes.extend(metadata.instance.as_bytes());
        bytes.push(metadata.app.len() as u8);
        bytes.extend(metadata.app.as_bytes());
        let bootstrap = Bootstrap {
            bytes,
            handles: [stream.into(), expected, listener, parent],
        };
        let child = Command::new(executable)
            .arg("--display-context-worker")
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::from(worker_channel))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let worker_pid = child.id();
        let worker_watch = Pid::from_raw(worker_pid as i32)
            .and_then(|pid| process::pidfd_open(pid, PidfdFlags::NONBLOCK).ok());
        let mut owner = Self {
            child: Some(child),
            worker_watch,
            source,
            channel: Some(channel),
            bootstrap: Some(bootstrap),
            expected_ready,
            until,
            worker_pid,
            state: State::Initializing,
        };
        if owner.worker_watch.is_none() || boottime_ns() >= until {
            owner.fence(State::Failed);
        }
        Ok(owner)
    }
    pub fn worker_pid(&self) -> u32 {
        self.worker_pid
    }
    pub fn until_ns(&self) -> u64 {
        self.until
    }
    pub fn occupied(&self) -> bool {
        self.child.is_some()
    }
    pub fn snapshot(&self) -> State {
        if matches!(self.state, State::Initializing | State::Ready)
            && (!living(&self.source).unwrap_or(false)
                || !self
                    .worker_watch
                    .as_ref()
                    .is_some_and(|fd| living(fd).unwrap_or(false))
                || (self.state == State::Initializing && boottime_ns() >= self.until))
        {
            State::Failed
        } else {
            self.state
        }
    }
    pub fn admission(&self) -> bool {
        self.snapshot() == State::Ready
    }
    fn fence(&mut self, terminal: State) {
        self.state = terminal;
        self.channel = None;
        self.bootstrap = None;
        if let Some(child) = &mut self.child {
            let signalled = self
                .worker_watch
                .as_ref()
                .is_some_and(|fd| process::pidfd_send_signal(fd, Signal::KILL).is_ok());
            if !signalled {
                let _ = child.kill();
            }
        }
    }
    pub fn stop(&mut self) {
        self.fence(if self.occupied() {
            State::Stopping
        } else {
            State::Stopped
        });
    }
    pub fn poll(&mut self) -> io::Result<State> {
        let result = self.advance();
        if result.is_err() {
            self.fence(State::Failed);
        }
        result
    }
    fn advance(&mut self) -> io::Result<State> {
        // A dead pidfd is not reaping. Keep ownership on any wait error.
        if let Some(child) = &mut self.child
            && child.try_wait()?.is_some()
        {
            self.child = None;
            self.worker_watch = None;
            self.channel = None;
            self.bootstrap = None;
            self.state = if self.state == State::Stopping {
                State::Stopped
            } else {
                State::Failed
            };
        }
        if !matches!(self.state, State::Initializing | State::Ready) {
            return Ok(self.state);
        }
        if self.snapshot() == State::Failed {
            self.fence(State::Failed);
            return Ok(self.state);
        }
        if let Some(bootstrap) = &self.bootstrap {
            let handles = bootstrap
                .handles
                .iter()
                .map(AsFd::as_fd)
                .collect::<Vec<_>>();
            match send(
                self.channel.as_ref().ok_or(io::ErrorKind::NotConnected)?,
                &bootstrap.bytes,
                &handles,
            ) {
                Ok(()) => self.bootstrap = None,
                Err(e) if e.raw_os_error() == Some(11) => return Ok(self.snapshot()),
                Err(e) => return Err(e),
            }
        }
        // At most one bounded packet per turn, never an SDK drain loop.
        if let Some(packet) = receive(self.channel.as_ref().ok_or(io::ErrorKind::NotConnected)?)? {
            if self.state != State::Initializing
                || !packet.handles.is_empty()
                || packet.credentials.pid.as_raw_pid() as u32 != self.worker_pid
                || packet.credentials.uid.as_raw() != 0
                || packet.credentials.gid.as_raw() != 0
                || packet.bytes != self.expected_ready
                || self.snapshot() != State::Initializing
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            self.state = State::Ready;
            if !living(&self.source)?
                || !self
                    .worker_watch
                    .as_ref()
                    .is_some_and(|fd| living(fd).unwrap_or(false))
                || boottime_ns() >= self.until
            {
                self.fence(State::Failed);
            }
        }
        Ok(self.snapshot())
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        if self.child.is_some() {
            self.fence(State::Failed);
            // Never silently detach, including after a failed wait operation.
            if self
                .child
                .as_mut()
                .is_none_or(|child| !matches!(child.try_wait(), Ok(Some(_))))
            {
                std::process::abort();
            }
        }
    }
}

struct Start {
    until: u64,
    identity: SourceIdentity,
    nonce: [u8; 16],
    metadata: Metadata,
}
fn decode(bytes: &[u8]) -> io::Result<Start> {
    // Fixed header plus the bounded canonical app name; no caller paths.
    if bytes.len() < 70 || bytes[0] != VERSION || bytes.len() != 70 + usize::from(bytes[69]) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let u32_at = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let pid = i32::try_from(u32_at(9))
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidData)?;
    let metadata = Metadata {
        instance: String::from_utf8(bytes[37..69].to_vec()).map_err(io::Error::other)?,
        app: String::from_utf8(bytes[70..].to_vec()).map_err(io::Error::other)?,
    };
    if !metadata.valid() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Start {
        until: u64::from_le_bytes(bytes[1..9].try_into().unwrap()),
        identity: SourceIdentity {
            pid,
            uid: u32_at(13),
            gid: u32_at(17),
        },
        nonce: bytes[21..37].try_into().unwrap(),
        metadata,
    })
}

/// Called by the installed same-build executable's fixed worker selector.
/// stdin is the privately transferred seqpacket; no external socket pathname.
pub fn worker_entry() -> io::Result<()> {
    // Install before SDK, filesystem setup or bootstrap parsing. Checking the
    // original parent afterward closes the died-before-prctl reparenting race.
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    for (resource, limit) in [
        (Resource::As, WORKER_MEMORY_BYTES),
        (Resource::Nofile, WORKER_FD_LIMIT),
        (Resource::Core, 0),
    ] {
        process::setrlimit(
            resource,
            Rlimit {
                current: Some(limit),
                maximum: Some(limit),
            },
        )?;
    }
    rustix::thread::set_no_new_privs(true)?;
    if process::getuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let input = io::stdin();
    rustix::io::fcntl_setfd(input.as_fd(), rustix::io::FdFlags::CLOEXEC)?;
    let channel = rustix::io::fcntl_dupfd_cloexec(input.as_fd(), 3)?;
    if net::sockopt::socket_domain(&channel)? != net::AddressFamily::UNIX
        || net::sockopt::socket_type(&channel)? != net::SocketType::SEQPACKET
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent_credentials = net::sockopt::socket_peercred(&channel)?;
    if parent_credentials.uid.as_raw() != 0
        || parent_credentials.gid.as_raw() != 0
        || process::getppid() != Some(parent_credentials.pid)
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent_peer = socket_peer_pidfd(&channel)?;
    let view = ProcessView::open()?;
    let bootstrap_until = boottime_ns().saturating_add(MAX_INITIALIZATION_NS);
    let packet = loop {
        if !living(&parent_peer)? || boottime_ns() >= bootstrap_until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(packet) = receive(&channel)? {
            break packet;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    if packet.credentials != parent_credentials || packet.handles.len() != 4 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let start = decode(&packet.bytes)?;
    let mut handles = packet.handles.into_iter();
    let stream = UnixStream::from(handles.next().unwrap());
    let expected = handles.next().unwrap();
    let listener = handles.next().unwrap();
    let parent = handles.next().unwrap();
    if !view.matches(
        &parent,
        &parent_peer,
        parent_credentials.pid.as_raw_pid() as u32,
    )? || process::getppid() != Some(parent_credentials.pid)
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let response = ready(&start.nonce, &start.metadata.instance, start.until);
    let mut context = Context::new(
        stream,
        &expected,
        start.identity,
        listener,
        start.metadata,
        start.until,
    )?;
    drop(expected);
    drop(view);
    drop(parent_peer);
    let mut notified = false;
    loop {
        if !living(&parent)? {
            context.stop();
            return Err(io::ErrorKind::NotConnected.into());
        }
        match context.poll()? {
            Phase::Ready if !notified => {
                send(&channel, &response, &[])?;
                notified = true;
            }
            Phase::Failed | Phase::Stopped => return Err(io::ErrorKind::NotConnected.into()),
            _ => {}
        }
        // Parent sends exactly one bootstrap. Extra input/closure terminates.
        if receive(&channel)?.is_some() {
            context.stop();
            return Err(io::ErrorKind::InvalidData.into());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
