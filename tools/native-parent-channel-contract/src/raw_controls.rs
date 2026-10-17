//! Deliberate hostile fixture entry only. It duplicates its inherited fd0 BEFORE
//! BoundParent consumes it. This is not a public handle escape. Actual sender
//! credentials and kernel rights/truncation are never mocked.
use crate::common::*;
use pokelinux_process_owner::supervisor::ProcessIdentity;
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags, SocketFlags, SocketType,
    },
    process::{PidfdFlags, getpid},
};
use std::{
    fs::File,
    io::{self, IoSlice, IoSliceMut, Read},
    mem::MaybeUninit,
    process::{Child, Command, ExitStatus, Stdio},
};

pub fn send(fd: impl AsFd, bytes: &[u8], handles: &[BorrowedFd<'_>]) -> io::Result<()> {
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(64))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    require(
        handles.is_empty() || ancillary.push(SendAncillaryMessage::ScmRights(handles)),
        "hostile fixture ancillary capacity",
    )?;
    let count = net::sendmsg(
        fd,
        &[IoSlice::new(bytes)],
        &mut ancillary,
        SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
    )?;
    require(
        count == bytes.len(),
        "hostile fixture packet not submitted atomically",
    )
}

// Exact existing bootstrap wire, used solely to reach actual BoundParent in the
// reciprocal negative controls. Normal/expiry/Stop use production Supervisor.
pub fn packet(kind: u8, identity: ProcessIdentity, until: u64) -> Vec<u8> {
    let mut bytes = vec![1, kind];
    bytes.extend_from_slice(identity.as_bytes());
    bytes.extend_from_slice(&until.to_le_bytes());
    bytes
}
pub fn receive_ready(fd: &OwnedFd, expected: &[u8], child_pid: u32) -> io::Result<bool> {
    let mut bytes = [0; 27];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(false),
        result => result?,
    };
    let mut credentials = None;
    let mut valid = !message
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if credentials.is_none() => {
                credentials = Some(value)
            }
            RecvAncillaryMessage::ScmRights(handles) => {
                for handle in handles {
                    drop(handle);
                }
                valid = false;
            }
            _ => valid = false,
        }
    }
    valid &= credentials.is_some_and(|value| {
        value.pid.as_raw_pid() as u32 == child_pid
            && value.uid == rustix::process::getuid()
            && value.gid == rustix::process::getgid()
    });
    require(
        valid && message.bytes == expected.len() && bytes[..message.bytes] == *expected,
        "actual reciprocal READY was malformed or from another sender",
    )?;
    Ok(true)
}

// Direct owned-Child pattern from bootstrap_controls.rs, not a task/thread pool.
// Its original cleanup bound is fixed before spawn. No blocking Child::wait.
pub struct OwnedChild {
    pub child: Child,
    status: Option<ExitStatus>,
    cleanup_until: u64,
    stdout_eof: bool,
    remaining_output: usize,
}
impl OwnedChild {
    pub fn new(child: Child, cleanup_until: u64) -> Self {
        let stdout_eof = child.stdout.is_none();
        Self {
            child,
            status: None,
            cleanup_until,
            stdout_eof,
            remaining_output: 0,
        }
    }
    fn stdout_complete(&mut self) -> io::Result<bool> {
        if self.stdout_eof {
            return Ok(true);
        }
        require(
            self.remaining_output <= 4096,
            "raw stdout closure cap already exceeded",
        )?;
        let output = self
            .child
            .stdout
            .as_mut()
            .ok_or(io::ErrorKind::BrokenPipe)?;
        // Also covers error disposal before manual_child configured this pipe.
        rustix::fs::fcntl_setfl(
            output.as_fd(),
            rustix::fs::fcntl_getfl(output.as_fd())? | rustix::fs::OFlags::NONBLOCK,
        )?;
        let mut bytes = [0; 256];
        let capacity = bytes.len().min(4097 - self.remaining_output);
        match output.read(&mut bytes[..capacity]) {
            Ok(0) => {
                self.stdout_eof = true;
                eprintln!(
                    "PIPE-EOF raw child_pid={} remaining_bytes={}",
                    self.child.id(),
                    self.remaining_output
                );
                Ok(true)
            }
            Ok(count) => {
                self.remaining_output += count;
                require(
                    self.remaining_output <= 4096,
                    "remaining raw child stdout exceeded 4096 bytes",
                )?;
                Ok(false)
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(error) => Err(error),
        }
    }
    pub fn poll(&mut self) -> io::Result<Option<ExitStatus>> {
        // No try_wait can reap/drop the child before actual stdout EOF. Forged
        // senders have stdout=None, so they have no PIPE EOF fact to claim.
        before(self.cleanup_until)?;
        if self.status.is_none() && self.stdout_complete()? {
            self.status = self.child.try_wait()?;
            if let Some(status) = self.status {
                eprintln!(
                    "CHILD-WAIT raw child_pid={} stdout_pipe={} status={status}",
                    self.child.id(),
                    self.child.stdout.is_some()
                );
            }
        }
        Ok(self.status)
    }
    pub fn wait_before(&mut self, until: u64) -> io::Result<ExitStatus> {
        loop {
            before(until)?;
            if let Some(status) = self.poll()? {
                before(until)?;
                return Ok(status);
            }
            pause();
        }
    }
    pub fn close(&mut self) -> io::Result<()> {
        if self.status.is_none() {
            let _ = self.child.kill();
            self.wait_before(self.cleanup_until)?;
        }
        Ok(())
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.close().is_err() {
            std::process::abort();
        }
    }
}

pub fn different_sender(channel: &OwnedFd, cleanup: u64, until: u64) -> io::Result<()> {
    let copy = rustix::io::fcntl_dupfd_cloexec(channel, 3)?;
    let child = Command::new(std::env::current_exe()?)
        .arg("--raw-sender")
        .stdin(Stdio::from(copy))
        .stdout(Stdio::null())
        .spawn()?;
    let mut owned = OwnedChild::new(child, cleanup);
    let result = (|| -> io::Result<()> {
        let status = owned.wait_before(until)?;
        require(
            status.code() == Some(0),
            "distinct-PID send did not complete",
        )
    })();
    owned.close()?;
    result
}
pub fn sender_entry() -> io::Result<()> {
    send(io::stdin(), b"forged-from-distinct-pid", &[])
}

pub fn inject(
    case: &str,
    channel: &OwnedFd,
    fifo: &std::path::Path,
    until: u64,
    cleanup: u64,
    max_message: usize,
) -> io::Result<()> {
    before(until)?;
    match case {
        "wrong-sender" => different_sender(channel, cleanup, until),
        "rights" | "many-rights" => {
            let writer = rustix::fs::open(
                fifo,
                rustix::fs::OFlags::WRONLY
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )?;
            let count = if case == "rights" { 1 } else { 64 };
            let handles: Vec<_> = (0..count).map(|_| writer.as_fd()).collect();
            send(channel, b"forbidden-rights", &handles)?;
            // All sender-side copies are gone before INJECTED is reported.
            drop(handles);
            drop(writer);
            Ok(())
        }
        "oversize" => send(channel, &vec![0xa7; max_message + 1], &[]),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
pub fn fifo_reader(path: &std::path::Path) -> io::Result<OwnedFd> {
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        path,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )?;
    Ok(rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?)
}
pub fn rights_queued(reader: &OwnedFd) -> io::Result<()> {
    let mut byte = [0];
    match rustix::io::read(reader, &mut byte) {
        Err(rustix::io::Errno::AGAIN) => Ok(()),
        other => Err(io::Error::other(format!(
            "queued SCM_RIGHTS must retain FIFO writer, got {other:?}"
        ))),
    }
}
pub fn rights_closed(reader: &OwnedFd) -> io::Result<()> {
    let mut byte = [0];
    require(
        rustix::io::read(reader, &mut byte)? == 0,
        "rejected SCM_RIGHTS kept FIFO writer alive",
    )
}
pub fn fd_count() -> io::Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir("/proc/self/fd")? {
        entry?;
        count += 1;
    }
    Ok(count)
}

pub fn manual_child(
    case: &str,
    until: u64,
    cleanup: u64,
) -> io::Result<(OwnedFd, OwnedChild, ProcessIdentity)> {
    let (channel, child_channel) = net::socketpair(
        net::AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&channel, true)?;
    net::sockopt::set_socket_passcred(&child_channel, true)?;
    let parent = rustix::process::pidfd_open(getpid(), PidfdFlags::empty())?;
    let mut identity = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut identity)?;
    let identity = ProcessIdentity::from_bytes(identity);
    let setup_until = plus(now(), 1_000_000_000)?.min(until);
    let child = Command::new(std::env::current_exe()?)
        .args([
            pokelinux_process_owner::bootstrap::WORKER_SELECTOR,
            "victim",
            case,
        ])
        .arg(until.to_string())
        .arg(cleanup.to_string())
        .env("POKELINUX_OWNER_INCARNATION", identity.hex())
        .stdin(Stdio::from(child_channel))
        .stdout(Stdio::piped())
        .spawn()?;
    let mut owned = OwnedChild::new(child, cleanup);
    let result = (|| -> io::Result<()> {
        let output = owned
            .child
            .stdout
            .as_ref()
            .ok_or(io::ErrorKind::BrokenPipe)?;
        rustix::fs::fcntl_setfl(
            output.as_fd(),
            rustix::fs::fcntl_getfl(output.as_fd())? | rustix::fs::OFlags::NONBLOCK,
        )?;
        // This explicit startup cutoff was fixed before spawn; no renewal.
        send(
            &channel,
            &packet(1, identity, setup_until),
            &[parent.as_fd()],
        )?;
        loop {
            before(setup_until)?;
            require(
                owned.poll()?.is_none(),
                "reciprocal child exited before READY",
            )?;
            if receive_ready(
                &channel,
                &packet(2, identity, setup_until),
                owned.child.id(),
            )? {
                break;
            }
            pause();
        }
        send(&channel, &packet(3, identity, setup_until), &[])?;
        Ok(())
    })();
    if let Err(error) = result {
        owned.close()?;
        return Err(error);
    }
    Ok((channel, owned, identity))
}
pub fn child_line(
    owned: &mut OwnedChild,
    buffered: &mut Vec<u8>,
    until: u64,
) -> io::Result<String> {
    loop {
        before(until)?;
        if let Some(end) = buffered.iter().position(|&byte| byte == b'\n') {
            let bytes: Vec<_> = buffered.drain(..=end).collect();
            return String::from_utf8(bytes).map_err(|_| io::ErrorKind::InvalidData.into());
        }
        let mut bytes = [0; 256];
        match owned
            .child
            .stdout
            .as_mut()
            .ok_or(io::ErrorKind::BrokenPipe)?
            .read(&mut bytes)
        {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => buffered.extend_from_slice(&bytes[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
            Err(error) => return Err(error),
        }
        require(buffered.len() <= 4096, "bounded reciprocal output exceeded")?;
        pause();
    }
}
