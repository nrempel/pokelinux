//! Root-only, fixed-profile descendant ownership. Not a production broker.
#![forbid(unsafe_code)]
use pokelinux_auth_scope::{
    owner::{DedicatedSupervisor, Retired},
    record,
};
use rustix::{fd::OwnedFd, fs, io::Errno};
use std::{collections::VecDeque, io};

fn duplicate_nonblocking(fd: impl rustix::fd::AsFd) -> io::Result<OwnedFd> {
    let result = rustix::io::fcntl_dupfd_cloexec(fd, 3)?;
    fs::fcntl_setfl(&result, fs::fcntl_getfl(&result)? | fs::OFlags::NONBLOCK)?;
    Ok(result)
}

// Return local intent only. The zeroizing read buffer drops before the caller
// applies any command that can reach a potentially blocked cgroup operation.
fn read_commands(
    input: &impl rustix::fd::AsFd,
    mut decode: impl FnMut(&[u8]) -> pokelinux_auth_scope::command::Batch,
) -> io::Result<Option<pokelinux_auth_scope::command::Batch>> {
    let mut bytes = zeroize::Zeroizing::new([0; 64]);
    match rustix::io::read(input, &mut bytes[..]) {
        Ok(0) => Err(io::ErrorKind::UnexpectedEof.into()),
        Ok(count) => Ok(Some(decode(&bytes[..count]))),
        Err(Errno::AGAIN | Errno::INTR) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn run(parent: u32, until: u64) -> io::Result<()> {
    let supervisor = DedicatedSupervisor::enter(parent)?;
    let input = duplicate_nonblocking(std::io::stdin())?;
    let output = duplicate_nonblocking(std::io::stdout())?;
    let mut attempt = supervisor.start_isolated(until)?;
    let mut queue = VecDeque::new();
    let mut previous = None;
    let mut parent_gone = false;
    let mut last_prompt = None;
    loop {
        // This child may block in a cgroup syscall. Its parent never performs
        // those syscalls: it owns this Child/pidfd and polls private pipes only.
        if !parent_gone {
            match read_commands(&input, |bytes| attempt.decode_commands(bytes)) {
                Ok(Some(batch)) => {
                    if batch.snapshot {
                        previous = None;
                    }
                    if let Some(reason) = batch.retired {
                        attempt.retire(reason);
                    } else if let Some((sequence, value)) = batch.response {
                        let _ = attempt.respond_hidden_prompt(sequence, value);
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    parent_gone = true;
                    attempt.retire(Retired::ParentGone);
                }
            }
        }
        attempt.poll();
        for receipt in attempt.take_wait_receipts() {
            if queue.len() >= 64 {
                attempt.retire(Retired::Protocol);
            } else {
                queue.push_back(record::wait(receipt));
            }
        }
        let state = attempt.snapshot();
        if previous != Some(state) && queue.len() < 64 {
            queue.push_back(record::state(state, record::STATE));
            previous = Some(state);
        }
        if state.retired.is_some() {
            queue.retain(|bytes| {
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != record::HIDDEN_PROMPT
            });
        } else if let Some(sequence) = attempt.pending_hidden_prompt()
            && last_prompt != Some(sequence)
        {
            if queue.len() >= 64 {
                attempt.retire(Retired::Protocol);
            } else {
                queue.push_back(record::hidden_prompt(state.worker_pid, sequence));
                last_prompt = Some(sequence);
            }
        }
        // Each write is a complete <= PIPE_BUF record to the private pipe.
        // Full output queues cannot make the parent wait or release occupancy.
        for _ in 0..8 {
            let Some(bytes) = queue.front() else { break };
            match rustix::io::write(&output, bytes) {
                Ok(record::SIZE) => {
                    queue.pop_front();
                }
                Err(Errno::AGAIN | Errno::INTR) => break,
                _ => {
                    attempt.retire(Retired::ParentGone);
                    queue.clear();
                    parent_gone = true;
                    break;
                }
            }
        }
        if !state.occupied {
            if !parent_gone {
                // The driver still independently waits this supervisor. A
                // CLOSED record or its pidfd death alone is never that wait.
                if queue.is_empty() {
                    match rustix::io::write(&output, &record::state(state, record::CLOSED)) {
                        Ok(record::SIZE) => return Ok(()),
                        Err(Errno::AGAIN | Errno::INTR) => {}
                        _ => return Ok(()),
                    }
                }
            } else {
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn run_parent(parent: u32, until: u64, missing_pidfd_fixture: bool) -> io::Result<()> {
    // The outer fixture also has an exact parent guard. It never touches the
    // workload cgroup; all such operations occur in its separately owned child.
    let _guard = DedicatedSupervisor::enter(parent)?;
    let input = duplicate_nonblocking(std::io::stdin())?;
    let output = duplicate_nonblocking(std::io::stdout())?;
    let mut owner = if missing_pidfd_fixture {
        #[cfg(feature = "setup-fault-fixture")]
        {
            pokelinux_auth_scope::parent::ScopeChild::spawn_isolated_missing_pidfd_fixture(until)?
        }
        #[cfg(not(feature = "setup-fault-fixture"))]
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    } else {
        pokelinux_auth_scope::parent::ScopeChild::spawn_isolated(until)?
    };
    let mut queue = VecDeque::new();
    let mut previous = None;
    let mut parent_gone = false;
    loop {
        if !parent_gone {
            match read_commands(&input, |bytes| owner.decode_commands(bytes)) {
                Ok(Some(batch)) => {
                    if batch.snapshot {
                        previous = None;
                    }
                    if let Some(reason) = batch.retired {
                        owner.retire(reason);
                    } else if let Some((sequence, value)) = batch.response {
                        let _ = owner.respond_hidden_prompt(sequence, value);
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    parent_gone = true;
                    owner.retire(Retired::ParentGone);
                }
            }
        }
        owner.poll();
        for record in owner.take_records() {
            if queue.len() < 128 {
                queue.push_back(record);
            } else {
                owner.retire(Retired::Protocol);
            }
        }
        let state = owner.snapshot();
        if state.retired.is_some() {
            queue.retain(|bytes| {
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != record::HIDDEN_PROMPT
            });
        }
        if previous != Some(state) && queue.len() < 128 {
            queue.push_back(record::parent(state));
            previous = Some(state);
        }
        for _ in 0..8 {
            let Some(bytes) = queue.front() else { break };
            match rustix::io::write(&output, bytes) {
                Ok(record::SIZE) => {
                    queue.pop_front();
                }
                Err(Errno::AGAIN | Errno::INTR) => break,
                _ => {
                    owner.retire(Retired::ParentGone);
                    queue.clear();
                    parent_gone = true;
                    break;
                }
            }
        }
        if !state.occupied && (queue.is_empty() || parent_gone) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// The injected setup failure owns a child that can never launch PAM, even if
/// accidentally resumed. Its stdout pipe remains open until actual termination.
#[cfg(feature = "setup-fault-fixture")]
fn hold_supervisor_no_workload(parent: u32, until: u64) -> io::Result<()> {
    let _guard = DedicatedSupervisor::enter(parent)?;
    let now = pokelinux_auth_scope::boottime_ns();
    if until <= now || until - now > pokelinux_auth_scope::owner::MAX_ATTEMPT_NS {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    rustix::process::kill_process(rustix::process::getpid(), rustix::process::Signal::STOP)?;
    loop {
        std::thread::park();
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let parsed = || {
        if args.len() != 6
            || !(matches!(
                args[1].as_str(),
                "--isolated-supervisor" | "--isolated-parent"
            ) || cfg!(feature = "setup-fault-fixture")
                && matches!(
                    args[1].as_str(),
                    "--missing-pidfd-parent" | "--held-supervisor-no-workload"
                ))
            || args[2] != "--parent"
            || args[4] != "--until"
        {
            return None;
        }
        let parent = args[3].parse::<u32>().ok()?;
        let until = args[5].parse::<u64>().ok()?;
        (parent >= 2 && parent <= i32::MAX as u32).then_some((parent, until))
    };
    if parsed().is_none_or(|(parent, until)| {
        let result = match args[1].as_str() {
            "--isolated-parent" => run_parent(parent, until, false),
            "--isolated-supervisor" => run(parent, until),
            #[cfg(feature = "setup-fault-fixture")]
            "--missing-pidfd-parent" => run_parent(parent, until, true),
            #[cfg(feature = "setup-fault-fixture")]
            "--held-supervisor-no-workload" => hold_supervisor_no_workload(parent, until),
            _ => Err(io::ErrorKind::PermissionDenied.into()),
        };
        result.is_err()
    }) {
        std::process::exit(1);
    }
}
