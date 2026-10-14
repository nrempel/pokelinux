//! Local nonblocking parent-side ownership. No cgroup or worker syscall is
//! performed here; a stuck supervisor never turns Stop into a blocking wait.
use crate::{boottime_ns, owner::Retired, record};
use rustix::{
    fd::OwnedFd,
    fs::{self, OFlags},
    process::{self, Pid, PidfdFlags},
};
use std::{
    collections::BTreeMap,
    io,
    os::unix::{fs::MetadataExt, process::ExitStatusExt},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

const SUPERVISOR: &str = "/run/pokelinux-auth-scope-test/auth-scope-fixture";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub supervisor_pid: u32,
    pub occupied: bool,
    pub retired: Option<Retired>,
    pub supervisor_wait: Option<i32>,
    pub scope_reported_closed: bool,
    pub stdout_eof: bool,
    pub cleanup_fault: bool,
}

impl Snapshot {
    fn reconcile(&mut self) {
        self.occupied = self.supervisor_wait != Some(0)
            || !self.scope_reported_closed
            || !self.stdout_eof
            || self.cleanup_fault;
    }
}

fn scope_follows(previous: crate::owner::Snapshot, next: crate::owner::Snapshot) -> bool {
    previous.worker_pid == next.worker_pid
        && (!previous.worker_ready || next.worker_ready)
        && (!previous.worker_stdout_eof || next.worker_stdout_eof)
        && (!previous.start_released || next.start_released)
        && (!previous.direct_kill_attempted || next.direct_kill_attempted)
        && (!previous.direct_kill_returned || next.direct_kill_returned)
        && (!previous.scope_kill_attempted || next.scope_kill_attempted)
        && (!previous.scope_kill_returned || next.scope_kill_returned)
        && (!previous.cleanup_fault || next.cleanup_fault)
        && (previous.worker_wait.is_none() || previous.worker_wait == next.worker_wait)
        && (previous.retired.is_none() || previous.retired == next.retired)
        && next.adopted_waits >= previous.adopted_waits
        && (previous.occupied || previous == next)
}

#[derive(Default)]
struct WaitJournal {
    worker: Option<(u32, i32)>,
    adopted: BTreeMap<u32, i32>,
}

impl WaitJournal {
    fn observe(&mut self, value: crate::owner::WaitReceipt) -> bool {
        if self.worker.is_some_and(|(pid, _)| pid == value.pid)
            || self.adopted.contains_key(&value.pid)
        {
            return false;
        }
        if value.worker {
            if self.worker.is_some() {
                return false;
            }
            self.worker = Some((value.pid, value.raw_status));
        } else {
            if self.worker.is_none() || self.adopted.len() >= 64 {
                return false;
            }
            self.adopted.insert(value.pid, value.raw_status);
        }
        true
    }

    fn matches(&self, value: crate::owner::Snapshot) -> bool {
        self.worker == value.worker_wait.map(|status| (value.worker_pid, status))
            && self.adopted.len() == value.adopted_waits as usize
    }
}

pub struct ScopeChild {
    child: Child,
    _alive: Option<OwnedFd>,
    input: Option<ChildStdin>,
    output: Option<ChildStdout>,
    until: u64,
    state: Snapshot,
    last_scope: Option<crate::owner::Snapshot>,
    pending_cancel: Option<u8>,
    incoming: Vec<u8>,
    records: Vec<[u8; record::SIZE]>,
    waits: WaitJournal,
}

impl ScopeChild {
    #[cfg(feature = "isolated-fixture")]
    pub fn spawn_isolated(until: u64) -> io::Result<Self> {
        let now = boottime_ns();
        if process::getuid().as_raw() != 0
            || process::geteuid().as_raw() != 0
            || until <= now
            || until - now > crate::owner::MAX_ATTEMPT_NS
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        for path in ["/run", "/run/pokelinux-auth-scope-test", SUPERVISOR] {
            let metadata = std::fs::symlink_metadata(path)?;
            if metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
                || metadata.file_type().is_symlink()
                || (path == SUPERVISOR && (!metadata.is_file() || metadata.mode() & 0o111 == 0))
                || (path != SUPERVISOR && !metadata.is_dir())
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
        let mut child = Command::new(SUPERVISOR)
            .arg("--isolated-supervisor")
            .arg("--parent")
            .arg(std::process::id().to_string())
            .arg("--until")
            .arg(until.to_string())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let pid = child.id();
        let alive = Pid::from_raw(pid as i32)
            .ok_or(rustix::io::Errno::INVAL)
            .and_then(|pid| process::pidfd_open(pid, PidfdFlags::NONBLOCK));
        let input = child.stdin.take();
        let output = child.stdout.take();
        let mut owner = Self {
            child,
            _alive: alive.ok(),
            input,
            output,
            until,
            state: Snapshot {
                supervisor_pid: pid,
                occupied: true,
                retired: None,
                supervisor_wait: None,
                scope_reported_closed: false,
                stdout_eof: false,
                cleanup_fault: false,
            },
            last_scope: None,
            pending_cancel: None,
            incoming: Vec::new(),
            records: Vec::new(),
            waits: WaitJournal::default(),
        };
        let setup = || -> io::Result<()> {
            if owner._alive.is_none() {
                return Err(io::ErrorKind::Other.into());
            }
            let input = owner.input.as_ref().ok_or(io::ErrorKind::Other)?;
            let output = owner.output.as_ref().ok_or(io::ErrorKind::Other)?;
            fs::fcntl_setfl(input, fs::fcntl_getfl(input)? | OFlags::NONBLOCK)?;
            fs::fcntl_setfl(output, fs::fcntl_getfl(output)? | OFlags::NONBLOCK)?;
            Ok(())
        };
        if setup().is_err() {
            owner.state.cleanup_fault = true;
            owner.retire(Retired::Setup);
        }
        Ok(owner)
    }

    /// Pure local reads; no pipe write, cgroup access, child wait or pidfd poll.
    pub fn snapshot(&self) -> Snapshot {
        self.state
    }

    pub fn scope_snapshot(&self) -> Option<crate::owner::Snapshot> {
        self.last_scope
    }

    /// Retire before any future IPC. A stopped/uninterruptible supervisor never
    /// delays this operation and never releases the occupied parent slot.
    pub fn retire(&mut self, reason: Retired) {
        if !self.state.occupied || self.state.retired.is_some() {
            return;
        }
        self.state.retired = Some(reason);
        self.pending_cancel = Some(if reason == Retired::Stop { b'S' } else { b'C' });
    }

    pub fn poll(&mut self) {
        if boottime_ns() >= self.until {
            self.retire(Retired::Deadline);
        }
        if let (Some(command), Some(input)) = (self.pending_cancel, &self.input) {
            match rustix::io::write(input, &[command]) {
                Ok(1) => self.pending_cancel = None,
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                _ => {
                    self.retire(Retired::ParentGone);
                }
            }
        }
        if let Some(output) = &self.output {
            let mut bytes = [0; record::SIZE * 16];
            match rustix::io::read(output, &mut bytes) {
                Ok(0) => self.state.stdout_eof = true,
                Ok(count) => {
                    if self.state.stdout_eof || self.state.scope_reported_closed {
                        self.state.cleanup_fault = true;
                    }
                    self.incoming.extend_from_slice(&bytes[..count]);
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(_) => self.state.cleanup_fault = true,
            }
        }
        for _ in 0..16 {
            if self.incoming.len() < record::SIZE {
                break;
            }
            let bytes: [u8; record::SIZE] = self.incoming[..record::SIZE].try_into().unwrap();
            self.incoming.drain(..record::SIZE);
            if self.state.scope_reported_closed {
                self.state.cleanup_fault = true;
            }
            match record::decode(&bytes) {
                Ok(record::Decoded::State(snapshot, closed)) => {
                    if !self.waits.matches(snapshot) {
                        self.state.cleanup_fault = true;
                    }
                    if let Some(previous) = self.last_scope {
                        if !scope_follows(previous, snapshot)
                            || closed && (previous != snapshot || previous.occupied)
                        {
                            self.state.cleanup_fault = true;
                        }
                    } else if closed {
                        self.state.cleanup_fault = true;
                    }
                    self.last_scope = Some(snapshot);
                    self.state.scope_reported_closed |= closed;
                    if snapshot.cleanup_fault {
                        self.state.cleanup_fault = true;
                    }
                }
                Ok(record::Decoded::Wait(receipt)) => {
                    if !self.waits.observe(receipt) {
                        self.state.cleanup_fault = true;
                    }
                }
                Err(_) => {
                    self.state.cleanup_fault = true;
                    self.retire(Retired::Protocol);
                }
            }
            if self.records.len() < 64 {
                self.records.push(bytes);
            } else {
                self.state.cleanup_fault = true;
            }
        }
        if self.state.stdout_eof && !self.incoming.is_empty() {
            self.state.cleanup_fault = true;
        }
        if self.state.supervisor_wait.is_none() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.state.supervisor_wait = Some(status.into_raw());
                    if !status.success() {
                        self.state.cleanup_fault = true;
                    }
                }
                Ok(None) => {}
                Err(_) => self.state.cleanup_fault = true,
            }
        }
        self.state.reconcile();
    }

    pub fn take_records(&mut self) -> Vec<[u8; record::SIZE]> {
        std::mem::take(&mut self.records)
    }
}

impl Drop for ScopeChild {
    fn drop(&mut self) {
        if self.state.occupied {
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Snapshot, WaitJournal};
    use crate::owner::{Retired, WaitReceipt};

    #[test]
    fn wait_journal_rejects_duplicates_and_adopted_receipt_before_worker_wait() {
        let worker = WaitReceipt {
            pid: 100,
            raw_status: 9,
            worker: true,
        };
        let adopted = WaitReceipt {
            pid: 101,
            raw_status: 9,
            worker: false,
        };
        let mut journal = WaitJournal::default();
        assert!(!journal.observe(adopted));
        assert!(journal.observe(worker));
        assert!(!journal.observe(worker));
        assert!(journal.observe(adopted));
        assert!(!journal.observe(adopted));
        assert!(!journal.observe(WaitReceipt {
            pid: 100,
            worker: false,
            ..worker
        }));
    }

    #[test]
    fn supervisor_death_and_reported_closure_each_require_the_other_actual_barrier() {
        let base = Snapshot {
            supervisor_pid: 123,
            occupied: true,
            retired: Some(Retired::Stop),
            supervisor_wait: None,
            scope_reported_closed: false,
            stdout_eof: false,
            cleanup_fault: false,
        };
        let mut waited = base;
        waited.supervisor_wait = Some(0);
        waited.reconcile();
        assert!(waited.occupied);
        let mut reported = base;
        reported.scope_reported_closed = true;
        reported.reconcile();
        assert!(reported.occupied);
        waited.scope_reported_closed = true;
        waited.reconcile();
        assert!(waited.occupied);
        waited.stdout_eof = true;
        waited.reconcile();
        assert!(!waited.occupied);
        waited.cleanup_fault = true;
        waited.reconcile();
        assert!(waited.occupied);
    }
}
