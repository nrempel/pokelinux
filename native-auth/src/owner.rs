//! Dedicated worker-tree ownership. Recursive termination never substitutes
//! for actual worker/adopted waits, EOF and an empty anchored workload.
use crate::{
    boottime_ns,
    cgroup::Leaf,
    outcome::{Issuer, WorkerOutcome},
};
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
            conversation: Conversation::new(until),
            commands: crate::command::Decoder::new(),
            issuer: Issuer::new(until),
            outcome_taken: false,
            barrier: Barrier::new(pid),
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

#[path = "conversation.rs"]
mod conversation;
use conversation::Conversation;
pub(crate) use conversation::ResponseFrame;
#[cfg(test)]
use conversation::parse_worker_protocol;

// Pure gate shared by the actual owner and ordinary model tests. Synthetic
// test facts do not establish actual process/cgroup closure.
fn extract_worker_outcome(
    barrier: &Barrier,
    conversation_ready: bool,
    ingress_idle: bool,
    receipts_empty: bool,
    issuer: &Issuer,
    taken: &mut bool,
) -> Option<WorkerOutcome> {
    if *taken
        || !barrier.closed()
        || !(2..=i32::MAX as u32).contains(&barrier.worker_pid)
        || barrier.retired.is_some()
        || barrier.worker_wait != Some(0)
        || !barrier.worker_ready
        || !barrier.start_released
        || barrier.direct_kill_attempted
        || barrier.scope_kill_attempted
        || !conversation_ready
        || !ingress_idle
        || !receipts_empty
    {
        return None;
    }
    let candidate = barrier.candidate?;
    // Fresh original clock sample follows every identity/closure validation.
    if !issuer.current() {
        return None;
    }
    *taken = true;
    // A deadline crossing here cannot create a ticket; the memo stays taken.
    issuer.worker(barrier.worker_pid, candidate)
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
    conversation: Conversation,
    commands: crate::command::Decoder,
    issuer: Issuer,
    outcome_taken: bool,
    barrier: Barrier,
    receipts: Vec<WaitReceipt>,
}

impl Attempt {
    fn initialize(&mut self) -> io::Result<()> {
        let alive = self.alive.as_ref().ok_or(io::ErrorKind::Other)?;
        self.conversation
            .initialize(self.input.as_ref(), self.output.as_ref())?;
        if !pidfd_living(alive)? {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        // The exact numeric PID remains held unreaped by Child through both
        // checks. Root worker setup waits for START before NSS/PAM/forks.
        self.leaf.admit(self.child.id())?;
        if !pidfd_living(alive)? {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.conversation
            .release_start(self.input.as_ref(), &mut self.barrier)?;
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        self.barrier.snapshot()
    }

    pub fn deadline(&self) -> u64 {
        self.issuer.deadline()
    }

    /// One sealed classification after actual whole-owner closure. Callers must
    /// first apply/drop every local decoded Batch and drain the wait receipts.
    /// Idle decoding is not an atomic cutoff against unread pipe cancellation.
    pub fn take_worker_outcome(&mut self) -> Option<WorkerOutcome> {
        extract_worker_outcome(
            &self.barrier,
            self.conversation.outcome_ready(&self.barrier),
            self.commands.is_idle(),
            self.receipts.is_empty(),
            &self.issuer,
            &mut self.outcome_taken,
        )
    }

    /// Observation for pending serialized fixture output. It creates no token
    /// and cannot retract bytes already delivered to the independently checking
    /// parent. The original deadline is never replaced or renewed.
    pub fn worker_outcome_current(&self) -> bool {
        self.outcome_taken
            && self.commands.is_idle()
            && self.conversation.outcome_ready(&self.barrier)
            && self.issuer.current()
    }

    /// Retire synchronously before touching the workload. A successful kill
    /// write or direct signal grants no cleanup/result authority.
    pub fn retire(&mut self, reason: Retired) {
        // Revoke even after physical closure, before any other retirement work.
        self.issuer.revoke();
        self.commands.retire();
        if self.barrier.closed() {
            // A terminal Snapshot is immutable. Result revocation never rewrites
            // it or sends a new signal/kill to an already closed workload.
            self.conversation.discard();
            return;
        }
        // Release plaintext before any potentially blocked cgroup operation.
        self.conversation.retire(&mut self.barrier, reason);
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

    /// Parse local intent only; the fixture drops its read buffer before apply.
    pub fn decode_commands(&mut self, bytes: &[u8]) -> crate::command::Batch {
        if self.barrier.retired.is_some() {
            self.commands.retire();
            crate::command::Batch::retired_snapshot()
        } else {
            self.commands.batch(bytes)
        }
    }

    /// Observation only. The original deadline is never replaced or renewed.
    pub fn pending_hidden_prompt(&self) -> Option<u16> {
        if self.barrier.closed() {
            return None;
        }
        self.conversation.pending_hidden_prompt(&self.barrier)
    }

    /// Queue only in the dedicated supervisor; its parent retains responsiveness.
    pub fn respond_hidden_prompt(
        &mut self,
        sequence: u16,
        value: zeroize::Zeroizing<Vec<u8>>,
    ) -> io::Result<()> {
        if self.barrier.closed() {
            drop(value);
            self.retire(Retired::Protocol);
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let result = self
            .conversation
            .respond_hidden_prompt(&mut self.barrier, sequence, value);
        if result.is_err() {
            self.retire(self.barrier.retired.unwrap_or(Retired::Protocol));
        }
        result
    }

    /// At most one bounded protocol read/write, one direct wait and sixteen
    /// adopted WNOHANG waits. No wait-for-exit loop runs on an actor thread.
    pub fn poll(&mut self) {
        if self.barrier.closed() {
            if boottime_ns() >= self.issuer.deadline() {
                self.retire(Retired::Deadline);
            }
            return;
        }
        if boottime_ns() >= self.conversation.deadline() {
            self.retire(Retired::Deadline);
        }
        if self
            .conversation
            .write_pending(self.input.as_ref(), &mut self.barrier)
            .and_then(|()| {
                self.conversation
                    .read_protocol(self.output.as_ref(), &mut self.barrier)
            })
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
        self.issuer.revoke();
        if !self.barrier.closed() {
            // Never detach an occupied tree merely because its owner returned.
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Barrier, Candidate, Issuer, Retired, extract_worker_outcome};

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
    fn healthy_closed(candidate: Candidate) -> Barrier {
        let mut barrier = Barrier::new(100);
        barrier.candidate = Some(candidate);
        barrier.worker_ready = true;
        barrier.start_released = true;
        barrier.worker_wait = Some(0);
        barrier.worker_stdout_eof = true;
        barrier.echild = true;
        barrier.populated = false;
        barrier
    }

    #[test]
    fn closed_outcome_gate_accepts_each_classification_once_without_snapshot_mutation() {
        for candidate in [
            Candidate::Authenticated,
            Candidate::Denied,
            Candidate::Unavailable,
        ] {
            let barrier = healthy_closed(candidate);
            let before = barrier.snapshot();
            let issuer = Issuer::new(crate::boottime_ns() + super::MAX_ATTEMPT_NS);
            let mut taken = false;
            let outcome = extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken)
                .expect("healthy model emits one sealed classification");
            assert!(outcome.into_parts() == Some((100, candidate, issuer.deadline())));
            assert!(taken && barrier.snapshot() == before);
            assert!(
                extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken).is_none()
            );
        }
    }

    #[test]
    fn dropping_first_worker_outcome_never_allows_a_second_extraction() {
        let barrier = healthy_closed(Candidate::Authenticated);
        let issuer = Issuer::new(crate::boottime_ns() + super::MAX_ATTEMPT_NS);
        let mut taken = false;
        drop(extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken).unwrap());
        assert!(taken);
        assert!(extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken).is_none());
    }

    #[test]
    fn worker_outcome_gate_refuses_each_missing_closure_and_health_condition() {
        let baseline = healthy_closed(Candidate::Authenticated);
        let mutations: [fn(&mut Barrier); 12] = [
            |barrier| barrier.worker_wait = None,
            |barrier| barrier.worker_wait = Some(9),
            |barrier| barrier.worker_stdout_eof = false,
            |barrier| barrier.echild = false,
            |barrier| barrier.populated = true,
            |barrier| barrier.cleanup_fault = true,
            |barrier| barrier.worker_ready = false,
            |barrier| barrier.start_released = false,
            |barrier| barrier.retired = Some(Retired::Cancel),
            |barrier| barrier.candidate = None,
            |barrier| barrier.direct_kill_attempted = true,
            |barrier| barrier.scope_kill_attempted = true,
        ];
        for mutate in mutations {
            let mut barrier = baseline;
            mutate(&mut barrier);
            let issuer = Issuer::new(crate::boottime_ns() + super::MAX_ATTEMPT_NS);
            let mut taken = false;
            assert!(
                extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken).is_none()
            );
            assert!(!taken);
        }
    }

    #[test]
    fn worker_outcome_gate_refuses_transport_ingress_or_unforwarded_receipts() {
        let barrier = healthy_closed(Candidate::Authenticated);
        for (conversation, ingress, receipts) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let issuer = Issuer::new(crate::boottime_ns() + super::MAX_ATTEMPT_NS);
            let mut taken = false;
            assert!(
                extract_worker_outcome(
                    &barrier,
                    conversation,
                    ingress,
                    receipts,
                    &issuer,
                    &mut taken
                )
                .is_none()
            );
            assert!(!taken);
        }
    }

    #[test]
    fn worker_outcome_revocation_and_expiry_preserve_closed_snapshot() {
        let barrier = healthy_closed(Candidate::Denied);
        let before = barrier.snapshot();
        let issuer = Issuer::new(crate::boottime_ns() + super::MAX_ATTEMPT_NS);
        let mut taken = false;
        let outcome =
            extract_worker_outcome(&barrier, true, true, true, &issuer, &mut taken).unwrap();
        issuer.revoke();
        assert!(outcome.into_parts().is_none());
        assert!(barrier.snapshot() == before);
        let expired = Issuer::new(crate::boottime_ns());
        let mut expired_taken = false;
        assert!(
            extract_worker_outcome(&barrier, true, true, true, &expired, &mut expired_taken)
                .is_none()
        );
        assert!(!expired_taken && barrier.snapshot() == before);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;

#[cfg(test)]
#[path = "conversation_io_tests.rs"]
mod conversation_io_tests;

#[cfg(test)]
#[path = "conversation_lifetime_tests.rs"]
mod conversation_lifetime_tests;
