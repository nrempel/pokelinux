//! Cooperative process custody before any Binder initialization.
//! This is not app authentication, containment or remote registration cleanup.
use crate::{
    deadline::{BootClock, Clock, Deadline},
    supervisor::ProcessIdentity,
};
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags, SocketFlags, SocketType, UCred,
    },
    process::{self, Pid, PidfdFlags, Signal},
};
use std::{
    fs::File,
    io::{self, IoSlice, IoSliceMut},
    marker::PhantomData,
    mem::MaybeUninit,
    path::Path,
    process::{Command, Stdio},
    rc::Rc,
    time::Duration,
};

pub const WORKER_SELECTOR: &str = "--pokelinux-process-worker";
/// Maximum opaque post-GO record. No application authority is encoded here.
pub const MAX_MESSAGE_BYTES: usize = 8192;
const SETUP_NS: u64 = 3_000_000_000;
const SIZE: usize = 26;
const BOOT: u8 = 1;
const READY: u8 = 2;
const GO: u8 = 3;

/// A selected, trusted cooperative executable. It must enter `BoundParent` on
/// its main thread, after final credentials and before Binder or other threads.
/// This first profile inherits the supervisor's equal real/effective UID/GID.
/// The caller protects executable/procfs provisioning and the common PID view.
/// No arbitrary Command, credential transition, or production worker body is supplied.
pub struct WorkerCommand {
    pub(crate) command: Command,
}
impl WorkerCommand {
    pub fn new(executable: &Path) -> io::Result<Self> {
        if !executable.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut command = Command::new(executable);
        command.arg(WORKER_SELECTOR);
        Ok(Self { command })
    }
    /// Arguments follow the fixed cooperative entry selector.
    pub fn arg(&mut self, argument: impl AsRef<std::ffi::OsStr>) -> &mut Self {
        self.command.arg(argument);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Initializing,
    Ready,
    Terminal,
}

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn now() -> io::Result<u64> {
    BootClock.now().as_nanos().try_into().map_err(|_| invalid())
}
fn cutoff() -> io::Result<u64> {
    now()?.checked_add(SETUP_NS).ok_or_else(invalid)
}
fn before(until: u64) -> io::Result<()> {
    if now()? >= until {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
fn packet(kind: u8, identity: ProcessIdentity, until: u64) -> [u8; SIZE] {
    let mut bytes = [0; SIZE];
    bytes[0] = 1;
    bytes[1] = kind;
    bytes[2..18].copy_from_slice(identity.as_bytes());
    bytes[18..].copy_from_slice(&until.to_le_bytes());
    bytes
}
struct Packet {
    bytes: [u8; SIZE],
    credentials: UCred,
    handles: Vec<OwnedFd>,
}
fn receive(fd: impl AsFd) -> io::Result<Option<Packet>> {
    let mut bytes = [0; SIZE];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        value => value?,
    };
    let mut malformed = message.bytes != SIZE
        || message
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut credentials = None;
    let mut handles = Vec::new();
    let mut rights = 0;
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if credentials.is_none() => {
                credentials = Some(value)
            }
            RecvAncillaryMessage::ScmRights(values) => {
                rights += 1;
                handles.extend(values);
            }
            _ => malformed = true,
        }
    }
    if malformed || rights > 1 || handles.len() > 1 {
        return Err(invalid());
    }
    Ok(Some(Packet {
        bytes,
        credentials: credentials.ok_or_else(invalid)?,
        handles,
    }))
}
fn send(fd: impl AsFd, bytes: &[u8; SIZE], handles: &[BorrowedFd<'_>]) -> io::Result<()> {
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    if handles.len() > 1
        || (!handles.is_empty() && !ancillary.push(SendAncillaryMessage::ScmRights(handles)))
    {
        return Err(invalid());
    }
    let count = net::sendmsg(
        fd,
        &[IoSlice::new(bytes)],
        &mut ancillary,
        SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
    )?;
    if count != SIZE {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
fn message_before(until: Deadline) -> io::Result<()> {
    if until.elapsed(BootClock.now()) {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
fn message_length(bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_MESSAGE_BYTES {
        Err(io::ErrorKind::InvalidInput.into())
    } else {
        Ok(())
    }
}
fn send_payload(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    let count = net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)?;
    if count != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
fn receive_payload(fd: impl AsFd, expected: UCred) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = [0; MAX_MESSAGE_BYTES];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        value => value?,
    };
    let mut malformed = message.bytes == 0
        || message
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut credentials = None;
    // Drain and close every received right even when another field is malformed.
    // Linux closes rights omitted by ancillary truncation; received ones are ours.
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if credentials.is_none() => {
                credentials = Some(value);
            }
            RecvAncillaryMessage::ScmRights(values) => {
                for handle in values {
                    drop(handle);
                }
                malformed = true;
            }
            _ => malformed = true,
        }
    }
    if malformed || credentials != Some(expected) {
        return Err(invalid());
    }
    Ok(Some(bytes[..message.bytes].to_vec()))
}

fn wait_packet(fd: impl AsFd, until: u64) -> io::Result<Packet> {
    loop {
        before(until)?;
        if let Some(value) = receive(fd.as_fd())? {
            before(until)?;
            return Ok(value);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub(crate) struct Prepared {
    channel: OwnedFd,
    child_channel: OwnedFd,
    parent: OwnedFd,
    until: u64,
}
impl Prepared {
    pub(crate) fn new() -> io::Result<Self> {
        if rustix::thread::gettid() != process::getpid()
            || process::getuid() != process::geteuid()
            || process::getgid() != process::getegid()
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let until = cutoff()?;
        let (channel, child_channel) = net::socketpair(
            net::AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        net::sockopt::set_socket_passcred(&channel, true)?;
        net::sockopt::set_socket_passcred(&child_channel, true)?;
        let parent = process::pidfd_open(process::getpid(), PidfdFlags::empty())?;
        Ok(Self {
            channel,
            child_channel,
            parent,
            until,
        })
    }
    pub(crate) fn install(self, worker: &mut WorkerCommand) -> io::Result<Pending> {
        before(self.until)?;
        worker.command.stdin(Stdio::from(self.child_channel));
        Ok(Pending {
            channel: self.channel,
            parent: self.parent,
            until: self.until,
        })
    }
}
pub(crate) struct Pending {
    channel: OwnedFd,
    parent: OwnedFd,
    until: u64,
}
impl Pending {
    pub(crate) fn bind(
        self,
        child_pid: u32,
        identity: ProcessIdentity,
    ) -> io::Result<ParentBootstrap> {
        let pid =
            Pid::from_raw(child_pid.try_into().map_err(|_| invalid())?).ok_or_else(invalid)?;
        let child = process::pidfd_open(pid, PidfdFlags::empty())?;
        let view = ProcessView::open()?;
        before(self.until)?;
        if !view.matches(&child, &child, child_pid)? {
            return Err(invalid());
        }
        send(
            &self.channel,
            &packet(BOOT, identity, self.until),
            &[self.parent.as_fd()],
        )?;
        Ok(ParentBootstrap {
            channel: self.channel,
            child,
            view,
            child_pid,
            identity,
            until: self.until,
            ready: false,
            uid: process::getuid(),
            gid: process::getgid(),
        })
    }
}
pub(crate) struct ParentBootstrap {
    channel: OwnedFd,
    child: OwnedFd,
    view: ProcessView,
    child_pid: u32,
    pub(crate) identity: ProcessIdentity,
    until: u64,
    ready: bool,
    uid: process::Uid,
    gid: process::Gid,
}
impl ParentBootstrap {
    fn check_message(&self, until: Deadline) -> io::Result<()> {
        if !self.ready {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        message_before(until)?;
        if rustix::thread::gettid() != process::getpid()
            || process::getuid() != self.uid
            || process::geteuid() != self.uid
            || process::getgid() != self.gid
            || process::getegid() != self.gid
            || !self
                .view
                .matches(&self.child, &self.child, self.child_pid)?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        message_before(until)
    }
    pub(crate) fn send_message(&mut self, until: Deadline, bytes: &[u8]) -> io::Result<()> {
        message_length(bytes)?;
        self.check_message(until)?;
        let result = send_payload(&self.channel, bytes);
        self.check_message(until)?;
        result
    }
    pub(crate) fn receive_message(&mut self, until: Deadline) -> io::Result<Option<Vec<u8>>> {
        self.check_message(until)?;
        let expected = UCred {
            pid: Pid::from_raw(self.child_pid.try_into().map_err(|_| invalid())?)
                .ok_or_else(invalid)?,
            uid: self.uid,
            gid: self.gid,
        };
        let result = receive_payload(&self.channel, expected);
        self.check_message(until)?;
        result
    }
    pub(crate) fn poll(&mut self) -> io::Result<WorkerState> {
        if self.ready {
            return Ok(WorkerState::Ready);
        }
        before(self.until)?;
        if !self
            .view
            .matches(&self.child, &self.child, self.child_pid)?
        {
            return Err(invalid());
        }
        let Some(reply) = receive(&self.channel)? else {
            return Ok(WorkerState::Initializing);
        };
        if reply.bytes != packet(READY, self.identity, self.until)
            || !reply.handles.is_empty()
            || reply.credentials.pid.as_raw_pid() as u32 != self.child_pid
            || reply.credentials.uid != self.uid
            || reply.credentials.gid != self.gid
        {
            return Err(invalid());
        }
        before(self.until)?;
        send(&self.channel, &packet(GO, self.identity, self.until), &[])?;
        // Admission follows successful GO, exact live child, and original cutoff.
        if !self
            .view
            .matches(&self.child, &self.child, self.child_pid)?
        {
            return Err(invalid());
        }
        before(self.until)?;
        self.ready = true;
        Ok(WorkerState::Ready)
    }
}

/// Retains exact parent handles and the bootstrap channel throughout a trusted
/// worker body. The kernel parent-death signal persists; do not change credentials,
/// replace this entry contract, or clear that signal after bootstrap.
/// This guard gives no radio permission or vendor cleanup/retry authority.
#[must_use]
pub struct BoundParent {
    identity: ProcessIdentity,
    origin: OwnedFd,
    transferred: OwnedFd,
    channel: OwnedFd,
    creator: UCred,
    view: ProcessView,
    _same_thread: PhantomData<Rc<()>>,
}
impl BoundParent {
    /// Execute on the final-credential main thread, before Binder initialization.
    /// Every failure must terminate the worker entry rather than enter its body.
    pub fn from_stdin() -> io::Result<Self> {
        let local_until = cutoff()?;
        if rustix::thread::gettid() != process::getpid() {
            return Err(invalid());
        }
        let channel = rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3)?;
        if net::sockopt::socket_type(&channel)? != SocketType::SEQPACKET
            || !net::sockopt::socket_passcred(&channel)?
        {
            return Err(invalid());
        }
        // CLOEXEC on the duplicate alone would leave the original fd0 inherited.
        rustix::stdio::dup2_stdin(File::open("/dev/null")?)?;
        let creator = net::sockopt::socket_peercred(&channel)?;
        let origin = socket_peer_pidfd(&channel)?;
        let boot = wait_packet(&channel, local_until)?;
        if boot.bytes[0..2] != [1, BOOT] || boot.handles.len() != 1 || boot.credentials != creator {
            return Err(invalid());
        }
        let identity =
            ProcessIdentity::from_bytes(boot.bytes[2..18].try_into().map_err(|_| invalid())?);
        let until = u64::from_le_bytes(boot.bytes[18..].try_into().map_err(|_| invalid())?);
        if until > local_until
            || std::env::var("POKELINUX_OWNER_INCARNATION").ok().as_deref()
                != Some(identity.hex().as_str())
        {
            return Err(invalid());
        }
        let transferred = boot.handles.into_iter().next().ok_or_else(invalid)?;
        process::set_parent_process_death_signal(Some(Signal::KILL))?;
        if process::parent_process_death_signal()? != Some(Signal::KILL) {
            return Err(invalid());
        }
        let view = ProcessView::open()?;
        let check = || -> io::Result<()> {
            before(until)?;
            if process::getppid() != Some(creator.pid)
                || !view.matches(&origin, &transferred, creator.pid.as_raw_pid() as u32)?
                || process::parent_process_death_signal()? != Some(Signal::KILL)
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(())
        };
        check()?;
        send(&channel, &packet(READY, identity, until), &[])?;
        let go = wait_packet(&channel, until)?;
        if go.bytes != packet(GO, identity, until)
            || !go.handles.is_empty()
            || go.credentials != creator
        {
            return Err(invalid());
        }
        check()?;
        Ok(Self {
            identity,
            origin,
            transferred,
            channel,
            creator,
            view,
            _same_thread: PhantomData,
        })
    }
    pub fn identity(&self) -> ProcessIdentity {
        self.identity
    }
    fn check_message(&self, until: Deadline) -> io::Result<()> {
        message_before(until)?;
        if rustix::thread::gettid() != process::getpid()
            || process::getuid() != self.creator.uid
            || process::geteuid() != self.creator.uid
            || process::getgid() != self.creator.gid
            || process::getegid() != self.creator.gid
            || process::getppid() != Some(self.creator.pid)
            || process::parent_process_death_signal()? != Some(Signal::KILL)
            || !self.view.matches(
                &self.origin,
                &self.transferred,
                self.creator.pid.as_raw_pid() as u32,
            )?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        message_before(until)
    }
    /// Copy one opaque bounded record over the retained post-GO channel.
    /// The caller supplies its original absolute application/session deadline;
    /// the bootstrap cutoff is never reused. No wait, queue or resend is added.
    /// A post-I/O error may follow delivery; it proves neither nondelivery nor
    /// permission to retry. The guard stays on the worker's main thread.
    pub fn send_message(&mut self, until: Deadline, bytes: &[u8]) -> io::Result<()> {
        message_length(bytes)?;
        self.check_message(until)?;
        let result = send_payload(&self.channel, bytes);
        self.check_message(until)?;
        result
    }
    /// Receive at most one copied record, or None for actual backpressure.
    /// Exact live parent credentials/pidfds and fresh BOOTTIME bracket the I/O.
    /// Rights, malformed/truncated packets and closed peers are refused. An
    /// error may consume bytes; caller policy owns uncertainty and disposal.
    pub fn receive_message(&mut self, until: Deadline) -> io::Result<Option<Vec<u8>>> {
        self.check_message(until)?;
        let result = receive_payload(&self.channel, self.creator);
        self.check_message(until)?;
        result
    }
}

#[cfg(test)]
impl ParentBootstrap {
    // Authored ready-state component fixture; not evidence of a child handshake.
    pub(crate) fn ready_fixture(identity: ProcessIdentity) -> Self {
        let (channel, _peer) = net::socketpair(
            net::AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .unwrap();
        Self {
            channel,
            child: process::pidfd_open(process::getpid(), PidfdFlags::empty()).unwrap(),
            view: ProcessView::open().unwrap(),
            child_pid: std::process::id(),
            identity,
            until: cutoff().unwrap(),
            ready: true,
            uid: process::getuid(),
            gid: process::getgid(),
        }
    }
}
