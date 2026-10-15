//! Private actual-descriptor bootstrap control; include as a module in client.rs.
//! The fixed worker selector must dispatch CANARY_ENV before any ProcessState.
//! No credentials, pidfd identity or production bootstrap result is mocked.
#![forbid(unsafe_code)]
use super::{TestResult, marker, report};
use pokelinux_radio::bootstrap::BoundParent;
use rsbinder::SIBinder;
use rustix::{
    fd::{AsFd, OwnedFd},
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags, SocketFlags,
    },
    process::{Pid, PidfdFlags},
};
use std::{
    fs::File,
    io::{self, IoSlice, IoSliceMut, Read, Write},
    mem::MaybeUninit,
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};

pub const CANARY_ENV: &str = "POKELINUX_BOOTSTRAP_CANARY";
const REFUSED: &[u8] = b"BOOTSTRAP-REFUSED PermissionDenied\n";
const CANARY: &[u8] = b"BOOTSTRAP-BODY-ENTERED";
const SETUP_NS: u64 = 3_000_000_000;
const CLEANUP_NS: u64 = 2_000_000_000;

fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (t.tv_sec as u64) * 1_000_000_000 + t.tv_nsec as u64
}
fn require(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
fn send(fd: &OwnedFd, bytes: &[u8], handle: Option<&OwnedFd>) -> io::Result<()> {
    let handles: Vec<_> = handle.into_iter().map(AsFd::as_fd).collect();
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    if !handles.is_empty() && !ancillary.push(SendAncillaryMessage::ScmRights(&handles)) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let count = net::sendmsg(
        fd,
        &[IoSlice::new(bytes)],
        &mut ancillary,
        SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
    )?;
    require(count == bytes.len(), "partial bootstrap packet")
}

enum Receive {
    Idle,
    Closed,
    Packet(Vec<u8>),
}
fn receive(fd: &OwnedFd, child: u32) -> io::Result<Receive> {
    let mut bytes = [0; 27];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let packet = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(Receive::Idle),
        result => result?,
    };
    let mut credentials = None;
    let mut valid = !packet
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if credentials.is_none() => {
                credentials = Some(c);
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
    if packet.bytes == 0 {
        require(valid, "malformed bootstrap EOF")?;
        return Ok(Receive::Closed);
    }
    valid &= credentials.is_some_and(|c| {
        c.pid.as_raw_pid() as u32 == child
            && c.uid == rustix::process::getuid()
            && c.gid == rustix::process::getgid()
    });
    require(
        valid,
        "unexpected actual bootstrap sender or ancillary data",
    )?;
    Ok(Receive::Packet(bytes[..packet.bytes].to_vec()))
}

struct OwnedChild {
    child: Child,
    status: Option<ExitStatus>,
    cleanup_until: u64,
}
impl OwnedChild {
    fn poll(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }
    fn close(&mut self) -> io::Result<()> {
        if self.status.is_none() {
            // This is error disposal, never the control's refusal oracle.
            let _ = self.child.kill();
            while now() < self.cleanup_until {
                if self.poll()?.is_some() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            return Err(io::ErrorKind::TimedOut.into());
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

/// Called only for the private canary branch of the fixed worker selector,
/// before peer(), Binder initialization, pool startup or any registration.
pub fn child_canary() -> TestResult {
    match BoundParent::from_stdin() {
        Ok(bound) => {
            writeln!(
                io::stdout(),
                "BOOTSTRAP-BODY-ENTERED {}",
                bound.identity().hex()
            )?;
            io::stdout().flush()?;
            Ok(())
        }
        Err(error) => {
            if error.kind() == io::ErrorKind::PermissionDenied {
                io::stderr().write_all(REFUSED)?;
                io::stderr().flush()?;
            }
            Err(error.into())
        }
    }
}

/// The guest supplies the actual independent live peer PID from its owned Popen.
/// Requires a fresh peer with zero registrations. It never signals that peer.
pub fn wrong_parent(peer: &SIBinder, peer_pid: u32) -> TestResult {
    require(
        report(peer)?[0] == 0,
        "fresh peer registration count must start at zero",
    )?;
    require(
        peer_pid != std::process::id(),
        "peer must be independent of sender",
    )?;
    let pid = i32::try_from(peer_pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let wrong_parent = rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK)?;
    let mut watch = [rustix::event::PollFd::new(
        &wrong_parent,
        rustix::event::PollFlags::IN,
    )];
    require(
        rustix::event::poll(
            &mut watch,
            Some(&rustix::event::Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )? == 0,
        "independent peer pidfd must be live",
    )?;
    let (parent, child) = net::socketpair(
        net::AddressFamily::UNIX,
        net::SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    for fd in [&parent, &child] {
        net::sockopt::set_socket_passcred(fd, true)?;
    }
    let mut identity = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut identity)?;
    let incarnation: String = identity.iter().map(|byte| format!("{byte:02x}")).collect();
    let until = now()
        .checked_add(SETUP_NS)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let cleanup_until = until
        .checked_add(CLEANUP_NS)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut boot = vec![1, 1];
    boot.extend(identity);
    boot.extend(until.to_le_bytes());
    let mut ready = boot.clone();
    ready[1] = 2;
    let mut go = boot.clone();
    go[1] = 3;
    let child = Command::new(std::env::current_exe()?)
        .arg("--pokelinux-radio-worker")
        .env_clear()
        .env("POKELINUX_OWNER_INCARNATION", incarnation)
        .env(CANARY_ENV, "1")
        .stdin(Stdio::from(child))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let child_pid = child.id();
    let mut owned = OwnedChild {
        child,
        status: None,
        cleanup_until,
    };
    let body = (|| -> TestResult {
        send(&parent, &boot, Some(&wrong_parent))?;
        let mut saw_ready = false;
        let mut closed = false;
        let status = loop {
            if !closed {
                match receive(&parent, child_pid)? {
                    Receive::Idle => {}
                    Receive::Closed => closed = true,
                    Receive::Packet(bytes) => {
                        require(bytes == ready && !saw_ready, "unexpected bootstrap reply")?;
                        saw_ready = true;
                        // If the wrong parent was accepted, complete the actual
                        // protocol so the forbidden canary can exit, then fail.
                        send(&parent, &go, None)?;
                    }
                }
            }
            if let Some(status) = owned.poll()? {
                if !closed && let Receive::Packet(bytes) = receive(&parent, child_pid)? {
                    require(bytes == ready, "unexpected final bootstrap reply")?;
                    saw_ready = true;
                }
                break status;
            }
            require(
                now() < until,
                "worker did not refuse within original bootstrap deadline",
            )?;
            std::thread::sleep(Duration::from_millis(1));
        };
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        owned
            .child
            .stdout
            .take()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?
            .take(4097)
            .read_to_end(&mut stdout)?;
        owned
            .child
            .stderr
            .take()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?
            .take(4097)
            .read_to_end(&mut stderr)?;
        require(
            stdout.len() <= 4096 && stderr.len() <= 4096,
            "bounded worker output exceeded",
        )?;
        require(
            status.code() == Some(1),
            "wrong-parent refusal needs actual child exit1",
        )?;
        require(!saw_ready, "wrong parent produced READY")?;
        require(
            !stdout.windows(CANARY.len()).any(|part| part == CANARY),
            "canary body entered",
        )?;
        require(
            stderr.windows(REFUSED.len()).any(|part| part == REFUSED),
            "missing typed refusal; setup failure",
        )?;
        require(
            report(peer)?[0] == 0,
            "refused worker registered with actual peer",
        )?;
        println!(
            "WRONG-PARENT child_pid={child_pid} actual_wait={status} ready=false body=false peer_registers=0"
        );
        Ok(())
    })();
    let cleanup = owned.close();
    cleanup?;
    body?;
    marker("bootstrap-rejects-live-wrong-parent-before-binder");
    Ok(())
}
