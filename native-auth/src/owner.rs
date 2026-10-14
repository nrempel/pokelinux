//! Dedicated worker-tree ownership. Recursive termination never substitutes
//! for actual worker/adopted waits, EOF and an empty anchored workload.
use crate::{boottime_ns, cgroup::Leaf};
use rustix::{
    event::{self, PollFd, PollFlags},
    fd::{AsFd, OwnedFd},
    fs::{self, OFlags},
    process::{self, Pid, PidfdFlags, Signal, WaitOptions},
};
use std::{
    fs as stdfs, io,
    os::unix::{fs::MetadataExt, process::ExitStatusExt},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub const MAX_ATTEMPT_NS: u64 = 30_000_000_000;
const MAX_WAITS_PER_POLL: usize = 16;
const WORKER: &str = "/run/pokelinux-auth-scope-test/pam-worker";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Retired {
    Cancel = 1,
    Stop = 2,
    Deadline = 3,
    ParentGone = 4,
    Protocol = 5,
    Setup = 6,
    Observation = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Candidate {
    Authenticated = 0,
    Denied = 1,
    Unavailable = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub worker_pid: u32,
    pub occupied: bool,
    pub retired: Option<Retired>,
    pub candidate_buffered: bool,
    pub worker_ready: bool,
    pub worker_stdout_eof: bool,
    pub worker_wait: Option<i32>,
    pub adopted_waits: u32,
    pub final_echild: bool,
    pub populated: bool,
    pub cleanup_fault: bool,
    pub start_released: bool,
    pub direct_kill_attempted: bool,
    pub direct_kill_returned: bool,
    pub scope_kill_attempted: bool,
    pub scope_kill_returned: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WaitReceipt {
    pub pid: u32,
    pub raw_status: i32,
    pub worker: bool,
}

/// Construct only in the dedicated, single-threaded process, before spawning
/// any child. It intentionally has no application request or policy API.
pub struct DedicatedSupervisor {
    _private: (),
}

impl DedicatedSupervisor {
    pub fn enter(expected_parent: u32) -> io::Result<Self> {
        if process::getuid().as_raw() != 0
            || process::geteuid().as_raw() != 0
            || process::getppid().map(|p| p.as_raw_nonzero().get() as u32) != Some(expected_parent)
            || stdfs::read_dir("/proc/self/task")?
                .collect::<Result<Vec<_>, _>>()?
                .len()
                != 1
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        process::set_dumpable_behavior(process::DumpableBehavior::NotDumpable)?;
        // Linux PR_SET_CHILD_SUBREAPER takes a boolean. rustix exposes that
        // argument as Option<Pid>; its nonzero value 1 is the required true.
        process::set_child_subreaper(Pid::from_raw(1))?;
        if process::child_subreaper()?.is_none()
            || process::dumpable_behavior()? != process::DumpableBehavior::NotDumpable
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self { _private: () })
    }

    /// This profile is root-only qualification infrastructure. No production
    /// scope constructor is provided before broker/provisioning qualification.
    #[cfg(feature = "isolated-fixture")]
    pub fn start_isolated(&self, until: u64) -> io::Result<Attempt> {
        let now = boottime_ns();
        if until <= now || until - now > MAX_ATTEMPT_NS {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        trusted_fixture_worker()?;
        let leaf = Leaf::open_isolated()?;
        let mut child = Command::new(WORKER)
            .arg("--parent")
            .arg(std::process::id().to_string())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // The synthetic module writes its credential-free 40-byte records
            // to retained FD 2; no extra FD survives worker initial setup.
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take();
        let output = child.stdout.take();
        let pid = child.id();
        let alive = Pid::from_raw(pid as i32)
            .ok_or(rustix::io::Errno::INVAL)
            .and_then(|pid| process::pidfd_open(pid, PidfdFlags::NONBLOCK));
        // From this point onward, even setup failure returns an occupied owner.
        // No error path drops a live Child or substitutes a detached cleanup.
        let mut attempt = Attempt {
            child,
            alive: alive.ok(),
            input,
            output,
            leaf,
            until,
            barrier: Barrier::new(pid),
            pending: Vec::new(),
            response: None,
            protocol: Vec::new(),
            receipts: Vec::new(),
        };
        if attempt.initialize().is_err() {
            attempt.barrier.cleanup_fault = true;
            attempt.retire(Retired::Setup);
        }
        Ok(attempt)
    }
}

#[cfg(feature = "isolated-fixture")]
fn trusted_fixture_worker() -> io::Result<()> {
    for name in ["/run", "/run/pokelinux-auth-scope-test", WORKER] {
        let path = Path::new(name);
        let metadata = stdfs::symlink_metadata(path)?;
        if metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || metadata.file_type().is_symlink()
            || (name == WORKER && (!metadata.is_file() || metadata.mode() & 0o111 == 0))
            || (name != WORKER && !metadata.is_dir())
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}

fn nonblocking(fd: &impl AsFd) -> io::Result<()> {
    fs::fcntl_setfl(fd, fs::fcntl_getfl(fd)? | OFlags::NONBLOCK)?;
    Ok(())
}

pub fn pidfd_living(fd: &impl AsFd) -> io::Result<bool> {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    event::poll(
        &mut fds,
        Some(&event::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    Ok(fds[0].revents().is_empty())
}

#[derive(Clone, Copy)]
struct Barrier {
    worker_pid: u32,
    retired: Option<Retired>,
    candidate: Option<Candidate>,
    worker_ready: bool,
    hidden_prompt: Option<u16>,
    prompts: u16,
    worker_stdout_eof: bool,
    worker_wait: Option<i32>,
    adopted_waits: u32,
    echild: bool,
    populated: bool,
    cleanup_fault: bool,
    start_released: bool,
    direct_kill_attempted: bool,
    direct_kill_returned: bool,
    scope_kill_attempted: bool,
    scope_kill_returned: bool,
}

impl Barrier {
    fn new(worker_pid: u32) -> Self {
        Self {
            worker_pid,
            retired: None,
            candidate: None,
            worker_ready: false,
            hidden_prompt: None,
            prompts: 0,
            worker_stdout_eof: false,
            worker_wait: None,
            adopted_waits: 0,
            echild: false,
            populated: true,
            cleanup_fault: false,
            start_released: false,
            direct_kill_attempted: false,
            direct_kill_returned: false,
            scope_kill_attempted: false,
            scope_kill_returned: false,
        }
    }

    fn retire(&mut self, reason: Retired) {
        if self.retired.is_none() {
            self.retired = Some(reason);
        }
        self.candidate = None;
        self.hidden_prompt = None;
    }

    fn closed(&self) -> bool {
        self.worker_wait.is_some()
            && self.worker_stdout_eof
            && self.echild
            && !self.populated
            && !self.cleanup_fault
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            worker_pid: self.worker_pid,
            occupied: !self.closed(),
            retired: self.retired,
            candidate_buffered: self.candidate.is_some(),
            worker_ready: self.worker_ready,
            worker_stdout_eof: self.worker_stdout_eof,
            worker_wait: self.worker_wait,
            adopted_waits: self.adopted_waits,
            final_echild: self.echild,
            populated: self.populated,
            cleanup_fault: self.cleanup_fault,
            start_released: self.start_released,
            direct_kill_attempted: self.direct_kill_attempted,
            direct_kill_returned: self.direct_kill_returned,
            scope_kill_attempted: self.scope_kill_attempted,
            scope_kill_returned: self.scope_kill_returned,
        }
    }
}

const MAX_PROMPTS: u16 = 32;
const MAX_RESPONSE: usize = 512;

fn parse_worker_protocol(
    protocol: &mut Vec<u8>,
    barrier: &mut Barrier,
    pending: &mut Vec<u8>,
) -> io::Result<()> {
    if barrier.retired.is_some() {
        return Ok(());
    }
    if protocol.is_empty() {
        return Ok(());
    }
    if !barrier.start_released {
        return Err(io::ErrorKind::InvalidData.into());
    }
    if !barrier.worker_ready && protocol.first() == Some(&0x10) {
        protocol.remove(0);
        barrier.worker_ready = true;
    }
    if protocol.is_empty() {
        return Ok(());
    }
    if !barrier.worker_ready || barrier.candidate.is_some() || barrier.hidden_prompt.is_some() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    match protocol[0] {
        0x11 => {
            if protocol.len() < 3 {
                return Ok(());
            }
            let sequence = u16::from_le_bytes([protocol[1], protocol[2]]);
            if protocol.len() != 3
                || sequence == 0
                || sequence > MAX_PROMPTS
                || sequence != barrier.prompts + 1
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            barrier.prompts = sequence;
            barrier.hidden_prompt = Some(sequence);
            protocol.clear();
        }
        0x30 => {
            if protocol.len() == 1 {
                return Ok(());
            }
            if protocol.len() != 2 {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let candidate = match protocol[1] {
                0 => Candidate::Authenticated,
                1 => Candidate::Denied,
                2 => Candidate::Unavailable,
                _ => return Err(io::ErrorKind::InvalidData.into()),
            };
            protocol.clear();
            barrier.candidate = Some(candidate);
            pending.push(0x40);
        }
        _ => return Err(io::ErrorKind::InvalidData.into()),
    }
    Ok(())
}

// No formatting/Debug/Clone. Keep the complete frame until release so prefix
// writes never leave forgotten response bytes in a drained Vec's spare memory.
struct ResponseFrame {
    bytes: zeroize::Zeroizing<Vec<u8>>,
    offset: usize,
}
impl ResponseFrame {
    fn new(sequence: u16, value: zeroize::Zeroizing<Vec<u8>>) -> io::Result<Self> {
        if !(1..=MAX_PROMPTS).contains(&sequence)
            || value.is_empty()
            || value.len() > MAX_RESPONSE
            || value.contains(&0)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity(5 + value.len()));
        bytes.push(0x20);
        bytes.extend_from_slice(&sequence.to_le_bytes());
        bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&value);
        Ok(Self { bytes, offset: 0 })
    }
    fn write_once(&mut self, fd: &impl AsFd) -> io::Result<bool> {
        if self.offset == self.bytes.len() {
            return Ok(true);
        }
        match rustix::io::write(fd, &self.bytes[self.offset..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => self.offset += count,
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(self.offset == self.bytes.len())
    }
}

/// Owns every worker handle until actual direct/adopted waits and cgroup-empty
/// closure. The parent must own this dedicated supervisor process separately:
/// filesystem/kill syscalls here never run in the lifecycle actor's poll path.
pub struct Attempt {
    child: Child,
    alive: Option<OwnedFd>,
    input: Option<ChildStdin>,
    output: Option<ChildStdout>,
    leaf: Leaf,
    until: u64,
    barrier: Barrier,
    pending: Vec<u8>,
    response: Option<ResponseFrame>,
    protocol: Vec<u8>,
    receipts: Vec<WaitReceipt>,
}

impl Attempt {
    fn initialize(&mut self) -> io::Result<()> {
        let alive = self.alive.as_ref().ok_or(io::ErrorKind::Other)?;
        nonblocking(self.input.as_ref().ok_or(io::ErrorKind::Other)?)?;
        nonblocking(self.output.as_ref().ok_or(io::ErrorKind::Other)?)?;
        if !pidfd_living(alive)? {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        // The exact numeric PID remains held unreaped by Child through both
        // checks. Root worker setup waits for START before NSS/PAM/forks.
        self.leaf.admit(self.child.id())?;
        if !pidfd_living(alive)? {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.pending.extend_from_slice(b"PKPA");
        self.pending.extend_from_slice(&self.until.to_le_bytes());
        self.write_pending()?;
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        self.barrier.snapshot()
    }

    pub fn deadline(&self) -> u64 {
        self.until
    }

    /// Retire synchronously before touching the workload. A successful kill
    /// write or direct signal grants no cleanup/result authority.
    pub fn retire(&mut self, reason: Retired) {
        self.barrier.retire(reason);
        // Release plaintext before any potentially blocked cgroup operation.
        self.response = None;
        self.pending.clear();
        self.protocol.clear();
        if self.barrier.scope_kill_attempted {
            return;
        }
        self.barrier.scope_kill_attempted = true;
        match self.leaf.kill_tree() {
            Ok(()) => self.barrier.scope_kill_returned = true,
            Err(_) => self.barrier.cleanup_fault = true,
        }
        // Admission can fail before the gated worker enters the workload.
        // Retain exact-Child termination too; it never stands in for tree kill.
        self.barrier.direct_kill_attempted = true;
        let result = match &self.alive {
            Some(fd) => process::pidfd_send_signal(fd, Signal::KILL).map_err(io::Error::from),
            None => self.child.kill(),
        };
        match result {
            Ok(()) => self.barrier.direct_kill_returned = true,
            Err(error) if error.raw_os_error() == Some(3) => {
                // ESRCH proves no closure. Actual waits and populated0 remain.
                self.barrier.direct_kill_returned = true;
            }
            Err(_) => self.barrier.cleanup_fault = true,
        }
    }

    /// Observation only. The original deadline is never replaced or renewed.
    pub fn pending_hidden_prompt(&self) -> Option<u16> {
        (self.barrier.retired.is_none() && boottime_ns() < self.until)
            .then_some(self.barrier.hidden_prompt)
            .flatten()
    }

    /// Queue one matching response; all pipe writes remain nonblocking.
    /// Called only in the dedicated supervisor. This isolated API is not wired
    /// into a broker or app ingress; its outside parent retains responsiveness.
    pub fn respond_hidden_prompt(
        &mut self,
        sequence: u16,
        value: zeroize::Zeroizing<Vec<u8>>,
    ) -> io::Result<()> {
        if boottime_ns() >= self.until {
            drop(value);
            self.retire(Retired::Deadline);
            return Err(io::ErrorKind::TimedOut.into());
        }
        if self.barrier.retired.is_some()
            || self.barrier.hidden_prompt != Some(sequence)
            || self.response.is_some()
            || self.barrier.candidate.is_some()
            || !self.pending.is_empty()
        {
            drop(value);
            self.retire(Retired::Protocol);
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let frame = match ResponseFrame::new(sequence, value) {
            Ok(frame) => frame,
            Err(error) => {
                self.retire(Retired::Protocol);
                return Err(error);
            }
        };
        if boottime_ns() >= self.until {
            drop(frame);
            self.retire(Retired::Deadline);
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.response = Some(frame);
        Ok(())
    }

    fn write_pending(&mut self) -> io::Result<()> {
        if self.pending.is_empty() && self.response.is_none() {
            return Ok(());
        }
        let input = self.input.as_ref().ok_or(io::ErrorKind::BrokenPipe)?;
        if !self.pending.is_empty() {
            match rustix::io::write(input, &self.pending) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => {
                    self.pending.drain(..count);
                    if self.pending.is_empty() && !self.barrier.start_released {
                        self.barrier.start_released = true;
                    }
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
            return Ok(());
        }
        if let Some(frame) = &mut self.response
            && frame.write_once(input)?
        {
            self.response = None;
            self.barrier.hidden_prompt = None;
        }
        Ok(())
    }

    fn read_protocol(&mut self) -> io::Result<()> {
        let output = self.output.as_ref().ok_or(io::ErrorKind::BrokenPipe)?;
        let mut bytes = [0; 16];
        match rustix::io::read(output, &mut bytes) {
            Ok(0) => {
                self.barrier.worker_stdout_eof = true;
                if self.barrier.retired.is_none() && self.barrier.candidate.is_none() {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
            }
            Ok(count) => {
                if self.barrier.retired.is_none() {
                    self.protocol.extend_from_slice(&bytes[..count]);
                }
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        parse_worker_protocol(&mut self.protocol, &mut self.barrier, &mut self.pending)
    }

    /// At most one bounded protocol read/write, one direct wait and sixteen
    /// adopted WNOHANG waits. No wait-for-exit loop runs on an actor thread.
    pub fn poll(&mut self) {
        if boottime_ns() >= self.until {
            self.retire(Retired::Deadline);
        }
        if self
            .write_pending()
            .and_then(|()| self.read_protocol())
            .is_err()
        {
            self.retire(Retired::Protocol);
        }
        if self.barrier.worker_wait.is_none() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.barrier.worker_wait = Some(status.into_raw());
                    self.receipts.push(WaitReceipt {
                        pid: self.child.id(),
                        raw_status: status.into_raw(),
                        worker: true,
                    });
                    if !status.success() || self.barrier.candidate.is_none() {
                        self.retire(Retired::Protocol);
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    self.barrier.cleanup_fault = true;
                    self.retire(Retired::Observation);
                }
            }
        }
        // Never let the all-child wait consume the worker behind Child.
        // rustix waitpid(None) selects only our process group (PID 0); wait()
        // selects every adopted child, including setsid-detached descendants.
        if self.barrier.worker_wait.is_some() {
            self.barrier.echild = false;
            for _ in 0..MAX_WAITS_PER_POLL {
                match process::wait(WaitOptions::NOHANG) {
                    Ok(Some((pid, status))) => {
                        self.barrier.adopted_waits = self.barrier.adopted_waits.saturating_add(1);
                        if self.receipts.len() < 64 {
                            self.receipts.push(WaitReceipt {
                                pid: pid.as_raw_nonzero().get() as u32,
                                raw_status: status.as_raw(),
                                worker: false,
                            });
                        } else {
                            self.barrier.cleanup_fault = true;
                        }
                    }
                    Ok(None) | Err(rustix::io::Errno::INTR) => break,
                    Err(rustix::io::Errno::CHILD) => {
                        self.barrier.echild = true;
                        break;
                    }
                    Err(_) => {
                        self.barrier.cleanup_fault = true;
                        self.retire(Retired::Observation);
                        break;
                    }
                }
            }
        }
        match self.leaf.populated() {
            Ok(value) => self.barrier.populated = value,
            Err(_) => {
                self.barrier.cleanup_fault = true;
                self.retire(Retired::Observation);
            }
        }
    }

    pub fn take_wait_receipts(&mut self) -> Vec<WaitReceipt> {
        std::mem::take(&mut self.receipts)
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if !self.barrier.closed() {
            // Never detach an occupied tree merely because its owner returned.
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Barrier, Candidate, Retired};

    #[test]
    fn success_candidate_cannot_release_a_live_or_unreaped_tree() {
        let mut barrier = Barrier::new(100);
        barrier.candidate = Some(Candidate::Authenticated);
        assert!(barrier.snapshot().occupied);
        barrier.populated = false;
        assert!(barrier.snapshot().occupied);
        barrier.worker_wait = Some(0);
        assert!(barrier.snapshot().occupied);
        barrier.echild = true;
        assert!(barrier.snapshot().occupied);
        barrier.worker_stdout_eof = true;
        assert!(!barrier.snapshot().occupied);
    }

    #[test]
    fn direct_worker_wait_and_deadline_do_not_close_detached_descendants() {
        let mut barrier = Barrier::new(100);
        barrier.candidate = Some(Candidate::Authenticated);
        barrier.worker_wait = Some(9);
        barrier.retire(Retired::Deadline);
        assert!(!barrier.snapshot().candidate_buffered);
        assert!(barrier.snapshot().occupied);
        barrier.echild = true;
        assert!(barrier.snapshot().occupied);
        barrier.populated = false;
        barrier.worker_stdout_eof = true;
        assert!(!barrier.snapshot().occupied);
    }

    #[test]
    fn cancel_then_late_success_and_empty_observation_cannot_clear_uncertainty() {
        let mut barrier = Barrier::new(100);
        barrier.candidate = Some(Candidate::Authenticated);
        barrier.retire(Retired::Cancel);
        barrier.retire(Retired::Stop);
        assert_eq!(barrier.retired, Some(Retired::Cancel));
        assert!(barrier.candidate.is_none());
        barrier.worker_wait = Some(9);
        barrier.echild = true;
        barrier.populated = false;
        barrier.cleanup_fault = true;
        assert!(barrier.snapshot().occupied);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;
