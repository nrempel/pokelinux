//! Private actual-descriptor bootstrap control; include as a module in client.rs.
//! The fixed worker selector must dispatch CANARY_ENV before any ProcessState.
//! No credentials, pidfd identity or production bootstrap result is mocked.
#![forbid(unsafe_code)]
use super::{TestResult, marker, report};
use pokelinux_process_owner::bootstrap::BoundParent;
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
const TIMED_OUT: &[u8] = b"BOOTSTRAP-REFUSED TimedOut\n";
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
                    require(
                        now() < self.cleanup_until,
                        "error disposal actual wait crossed original cutoff",
                    )?;
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
            // Private parent-policy body: its first deliberate action is an
            // unmistakable successful-entry status, with no fallible output IO.
            if std::env::args().nth(2).as_deref() == Some("bootstrap-canary") {
                std::process::exit(42);
            }
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
            } else if error.kind() == io::ErrorKind::TimedOut {
                io::stderr().write_all(TIMED_OUT)?;
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
        .arg(pokelinux_process_owner::bootstrap::WORKER_SELECTOR)
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
                require(
                    now() < until,
                    "wrong-parent actual wait crossed original setup cutoff",
                )?;
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

fn stopped(pid: u32) -> io::Result<bool> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let (_, fields) = stat
        .rsplit_once(") ")
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
    Ok(fields.split_whitespace().next() == Some("T"))
}
fn bounded_output(mut input: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input.by_ref().take(4097).read_to_end(&mut bytes)?;
    require(bytes.len() <= 4096, "bounded worker output exceeded")?;
    Ok(bytes)
}

/// Correct actual parent handoff; GO is queued while the child is kernel-stopped.
/// This qualifies child-side absolute expiry separately from parent READY policy.
pub fn late_go(peer: &SIBinder) -> TestResult {
    require(
        report(peer)?[0] == 0,
        "late-GO needs a fresh unregistered peer",
    )?;
    let actual_parent =
        rustix::process::pidfd_open(rustix::process::getpid(), PidfdFlags::NONBLOCK)?;
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
    let resume_at = until
        .checked_add(25_000_000)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    // Reserve the final second of the original cleanup window for error disposal.
    let refusal_until = until
        .checked_add(1_000_000_000)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut boot = vec![1, 1];
    boot.extend(identity);
    boot.extend(until.to_le_bytes());
    let mut ready = boot.clone();
    ready[1] = 2;
    let mut go = boot.clone();
    go[1] = 3;
    let child = Command::new(std::env::current_exe()?)
        .arg(pokelinux_process_owner::bootstrap::WORKER_SELECTOR)
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
        let pid = i32::try_from(child_pid)
            .ok()
            .and_then(Pid::from_raw)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let child_watch = rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK)?;
        send(&parent, &boot, Some(&actual_parent))?;
        let ready_at = loop {
            require(
                now() < until,
                "late-GO setup missed original READY deadline",
            )?;
            require(owned.poll()?.is_none(), "child exited before actual READY")?;
            match receive(&parent, child_pid)? {
                Receive::Packet(bytes) => {
                    require(bytes == ready, "late-GO actual READY differs")?;
                    let at = now();
                    require(at < until, "actual READY consumed after original deadline")?;
                    break at;
                }
                Receive::Closed => return Err(io::Error::other("child closed before READY").into()),
                Receive::Idle => std::thread::sleep(Duration::from_millis(1)),
            }
        };
        rustix::process::pidfd_send_signal(&child_watch, rustix::process::Signal::STOP)?;
        let stopped_at = loop {
            require(
                now() < until,
                "actual stop not observed before original deadline",
            )?;
            require(owned.poll()?.is_none(), "child exited before actual stop")?;
            if stopped(child_pid)? {
                break now();
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        require(now() < until, "GO was not queued before original deadline")?;
        send(&parent, &go, None)?;
        let queued_at = now();
        require(queued_at < until, "GO submission crossed original deadline")?;
        while now() < resume_at {
            require(
                owned.poll()?.is_none(),
                "stopped child exited before resume",
            )?;
            std::thread::sleep(Duration::from_millis(1));
        }
        require(
            stopped(child_pid)?,
            "child did not remain kernel-stopped across expiry",
        )?;
        let resumed_at = now();
        require(
            resumed_at >= resume_at && resumed_at < refusal_until,
            "resume missed the fixed observation window",
        )?;
        rustix::process::pidfd_send_signal(&child_watch, rustix::process::Signal::CONT)?;
        let (status, waited_at) = loop {
            if let Some(status) = owned.poll()? {
                let observed = now();
                require(
                    observed < refusal_until,
                    "late-GO actual wait crossed fixed observation cutoff",
                )?;
                break (status, observed);
            }
            require(
                now() < refusal_until,
                "late GO did not refuse before fixed disposal reserve",
            )?;
            std::thread::sleep(Duration::from_millis(1));
        };
        let stdout = bounded_output(
            owned
                .child
                .stdout
                .take()
                .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?,
        )?;
        let stderr = bounded_output(
            owned
                .child
                .stderr
                .take()
                .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?,
        )?;
        require(
            status.code() == Some(1),
            "late GO needs actual refusal exit1",
        )?;
        require(
            !stdout.windows(CANARY.len()).any(|part| part == CANARY),
            "late GO entered the canary body",
        )?;
        require(
            stderr
                .windows(TIMED_OUT.len())
                .any(|part| part == TIMED_OUT),
            "late GO lacked typed TimedOut; setup/refusal cause differs",
        )?;
        require(
            report(peer)?[0] == 0,
            "late-GO worker registered with actual peer",
        )?;
        println!(
            "LATE-GO child_pid={child_pid} actual_wait={status} until_ns={until} ready_at_ns={ready_at} stopped_at_ns={stopped_at} queued_at_ns={queued_at} resumed_at_ns={resumed_at} waited_at_ns={waited_at} body=false peer_registers=0"
        );
        Ok(())
    })();
    let cleanup = owned.close();
    cleanup?;
    body?;
    marker("bootstrap-refuses-queued-go-after-original-deadline");
    Ok(())
}

/// Actual production parent policy with a cooperative canary child. Withholding
/// polling does not assert that a READY packet was already queued; late-GO's
/// separate control is the exact queued-frame observation.
pub fn policy_control(peer: &SIBinder, withhold: bool) -> TestResult {
    use pokelinux_process_owner::{
        bootstrap::{WorkerCommand, WorkerState},
        supervisor::{StartError, Supervisor},
    };
    use std::os::unix::{
        fs::{DirBuilderExt, PermissionsExt},
        process::ExitStatusExt,
    };
    require(
        report(peer)?[0] == 0,
        "parent policy needs unregistered actual peer",
    )?;
    let mode = if withhold {
        "withheld-poll"
    } else {
        "stop-initializing"
    };
    let home = std::env::temp_dir().join(format!(
        "radio-bootstrap-policy-{}-{mode}",
        std::process::id()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&home)?;
    let journal = home.join("admission");
    let mut owner = Supervisor::new(journal.clone());
    let mut command = WorkerCommand::new(&std::env::current_exe()?)?;
    command.arg("bootstrap-canary");
    // Fix the complete test disposal cutoff before creation; never renew it.
    let cleanup_until = now()
        .checked_add(if withhold {
            6_000_000_000
        } else {
            2_000_000_000
        })
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let attempt = owner
        .start_worker(&mut command)
        .map_err(|e| io::Error::other(format!("bootstrap start {e:?}")))?;
    let body = (|| -> TestResult {
        require(
            owner.occupied() && !owner.admits(attempt),
            "initialization already admitted",
        )?;
        if withhold {
            // An after-start cutoff necessarily exceeds the parent's earlier
            // immutable three-second cutoff; this test does not supply its clock.
            let observe_at = now()
                .checked_add(3_050_000_000)
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
            require(
                observe_at + 1_000_000_000 < cleanup_until,
                "setup exhausted fixed disposal reserve",
            )?;
            while now() < observe_at {
                require(
                    owner.occupied() && !owner.admits(attempt),
                    "withheld bootstrap readmitted",
                )?;
                std::thread::sleep(Duration::from_millis(1));
            }
            let error = owner
                .poll_worker()
                .expect_err("expired original parent deadline admitted worker");
            require(
                error.kind() == io::ErrorKind::TimedOut,
                "parent expiry failed for another cause",
            )?;
        } else {
            owner.stop()?;
        }
        require(
            owner.poll_worker()? == WorkerState::Terminal && !owner.admits(attempt),
            "terminal bootstrap readmitted",
        )?;
        let (status, waited_at) = loop {
            if let Some(status) = owner.poll_reap()? {
                let observed = now();
                require(
                    observed < cleanup_until,
                    "parent policy actual wait crossed original disposal cutoff",
                )?;
                break (status, observed);
            }
            require(
                owner.occupied() && !owner.admits(attempt),
                "unreaped child lost or readmitted",
            )?;
            require(
                now() < cleanup_until,
                "parent policy actual wait missed original cutoff",
            )?;
            std::thread::sleep(Duration::from_millis(1));
        };
        require(
            status.code() == Some(1) || status.signal() == Some(9),
            "unexpected terminal child status",
        )?;
        require(
            !owner.occupied() && !owner.admits(attempt),
            "actual wait failed to close child custody",
        )?;
        require(
            std::fs::read_to_string(&journal)? == format!("admission-v1\n{}\n1\n", attempt.hex()),
            "journal changed",
        )?;
        require(
            std::fs::metadata(&journal)?.permissions().mode() & 0o777 == 0o600,
            "journal mode changed",
        )?;
        require(
            owner.start_worker(&mut command) == Err(StartError::AdmissionUncertain),
            "same parent retried",
        )?;
        let mut restarted = Supervisor::new(journal);
        require(
            restarted.start_worker(&mut command) == Err(StartError::AdmissionUncertain),
            "new parent retried",
        )?;
        require(!restarted.occupied(), "restart launched child")?;
        require(report(peer)?[0] == 0, "parent policy child registered")?;
        // Success never sends GO; actual child can only refuse or be killed.
        println!(
            "POLICY-CONTROL mode={mode} actual_wait={status} waited_at_ns={waited_at} cleanup_until_ns={cleanup_until} admitted=false body=false peer_registers=0 journal_retained=true"
        );
        Ok(())
    })();
    if owner.occupied() {
        let _ = owner.stop();
        while owner.occupied() && now() < cleanup_until {
            let _ = owner.poll_reap();
            if owner.occupied() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if owner.occupied() {
            std::process::abort();
        }
    }
    body?;
    marker(if withhold {
        "bootstrap-parent-original-deadline-with-poll-withheld"
    } else {
        "bootstrap-stop-initializing-never-admits"
    });
    Ok(())
}
