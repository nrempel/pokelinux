//! Real parent/peer pipes and an owned unprivileged peer process only.
//! All worker waits/cgroup fields in peer records are synthetic wire facts.
//! No actual authentication worker, PAM, scope cleanup or unlock is exercised.
use super::{PromptQueue, ScopeChild, Snapshot, WaitJournal};
use crate::{owner, record};
use rustix::{
    event::{self, PollFd, PollFlags},
    fd::{AsFd, OwnedFd},
    fs::{self, OFlags},
    process::{self, Pid, PidfdFlags, Signal},
};
use std::{
    fs::{DirBuilder, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const PEER_FILTER: &str = "parent::parent_process_tests::private_parent_pipe_peer";
const PEER_ENV: &str = "POKELINUX_PARENT_PIPE_PEER";
const OUTPUT_ENV: &str = "POKELINUX_PARENT_PIPE_PUBLIC_OUTPUT";
const READY: &[u8] = b"POKELINUX-PARENT-PIPE-PEER-READY\n";
const PUBLIC_VALUE: &[u8] = b"public-test-input";
const BOUND: Duration = Duration::from_secs(3);

// Own setup failures before transferring the exact child into ScopeChild.
struct PreparingPeer {
    child: Option<Child>,
    alive: Option<OwnedFd>,
}
impl Drop for PreparingPeer {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            if let Some(alive) = &self.alive {
                let _ = process::pidfd_send_signal(alive, Signal::KILL);
            } else {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

fn readable(fd: &impl AsFd, until: Instant) -> io::Result<bool> {
    loop {
        let Some(left) = until.checked_duration_since(Instant::now()) else {
            return Ok(false);
        };
        let timeout = event::Timespec {
            tv_sec: left.as_secs() as _,
            tv_nsec: left.subsec_nanos() as _,
        };
        let mut fds = [PollFd::new(fd, PollFlags::IN)];
        match event::poll(&mut fds, Some(&timeout)) {
            Ok(0) => return Ok(false),
            Ok(_) if fds[0].revents().intersects(PollFlags::IN | PollFlags::HUP) => {
                return Ok(true);
            }
            Ok(_) => return Err(io::ErrorKind::Other.into()),
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn construct_parent(output_file: &Path) -> io::Result<ScopeChild> {
    let child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            PEER_FILTER,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env(PEER_ENV, "1")
        .env(OUTPUT_ENV, output_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut preparing = PreparingPeer {
        child: Some(child),
        alive: None,
    };
    let child = preparing.child.as_ref().ok_or(io::ErrorKind::Other)?;
    let pid = child.id();
    let child_pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;
    preparing.alive = Some(process::pidfd_open(child_pid, PidfdFlags::NONBLOCK)?);
    let input = child.stdin.as_ref().ok_or(io::ErrorKind::Other)?;
    let output = child.stdout.as_ref().ok_or(io::ErrorKind::Other)?;
    fs::fcntl_setfl(input, fs::fcntl_getfl(input)? | OFlags::NONBLOCK)?;
    fs::fcntl_setfl(output, fs::fcntl_getfl(output)? | OFlags::NONBLOCK)?;
    let mut matched = 0;
    let mut total = 0;
    let bound = Instant::now() + BOUND;
    while matched < READY.len() {
        if !readable(output, bound)? {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Single-byte reads cannot consume any subsequent binary record.
        let mut byte = [0];
        match rustix::io::read(output, &mut byte) {
            Ok(1) => {
                total += 1;
                if total > 4096 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                matched = if byte[0] == READY[matched] {
                    matched + 1
                } else {
                    usize::from(byte[0] == READY[0])
                };
            }
            Ok(_) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
    if rustix::io::write(input, b"?")? != 1 {
        return Err(io::ErrorKind::WriteZero.into());
    }
    let mut child = preparing.child.take().ok_or(io::ErrorKind::Other)?;
    let input = child.stdin.take();
    let output = child.stdout.take();
    Ok(ScopeChild {
        child,
        _alive: preparing.alive.take(),
        input,
        output,
        until: crate::boottime_ns() + 15_000_000_000,
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
        prompts: PromptQueue::default(),
        commands: crate::command::Decoder::new(),
    })
}

fn queue_public_response(parent: &mut ScopeChild) -> io::Result<()> {
    let bound = Instant::now() + BOUND;
    loop {
        parent.poll();
        let state = parent.snapshot();
        if state.retired.is_some() || state.cleanup_fault {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if parent.pending_hidden_prompt() == Some(1) {
            let scope = parent.scope_snapshot().ok_or(io::ErrorKind::InvalidData)?;
            if scope.worker_pid != 100 || !scope.worker_ready || !scope.start_released {
                return Err(io::ErrorKind::InvalidData.into());
            }
            return parent.respond_hidden_prompt(1, zeroize::Zeroizing::new(PUBLIC_VALUE.to_vec()));
        }
        if Instant::now() >= bound {
            return Err(io::ErrorKind::TimedOut.into());
        }
        std::thread::yield_now();
    }
}

fn pump_actual_closure(parent: &mut ScopeChild, bound: Instant) -> bool {
    loop {
        parent.poll();
        let state = parent.snapshot();
        if state.supervisor_wait.is_some() && state.stdout_eof {
            return true;
        }
        if Instant::now() >= bound {
            return false;
        }
        std::thread::yield_now();
    }
}

#[test]
fn exact_parent_prompt_response_crosses_owned_peer_pipes_and_closes() {
    let directory = std::env::temp_dir().join(format!(
        "pokelinux-parent-pipe-{}-{}",
        std::process::id(),
        crate::boottime_ns()
    ));
    DirBuilder::new().mode(0o700).create(&directory).unwrap();
    let output_file = directory.join("public-response.bin");
    let mut parent = construct_parent(&output_file).expect("construct exact private peer owner");
    let admitted = queue_public_response(&mut parent);
    if admitted.is_err() {
        // EOF releases a peer waiting for the public response on failure.
        parent.input = None;
    }
    let natural = pump_actual_closure(&mut parent, Instant::now() + BOUND);
    if !natural {
        parent.input = None;
        if let Some(alive) = &parent._alive {
            let _ = process::pidfd_send_signal(alive, Signal::KILL);
        } else {
            let _ = parent.child.kill();
        }
        // Failure-only exact cleanup. The single Rust peer never forks, so
        // after its actual wait no other process can retain its stdout writer.
        let _ = parent.child.wait();
        let _ = pump_actual_closure(&mut parent, Instant::now() + BOUND);
    }
    // A final actual wait also verifies Child's cached natural wait on GREEN.
    let actual_wait = parent.child.wait();
    // After an exact wait the non-forking peer has no remaining stdout writer.
    // Let the production poll observe cached wait status and actual pipe EOF;
    // do not synthesize either observation or any clean protocol flags.
    let final_actual_closure = pump_actual_closure(&mut parent, Instant::now() + BOUND);
    let state = parent.snapshot();
    let received = std::fs::read(&output_file);
    let pidfd_dead = parent
        ._alive
        .as_ref()
        .is_some_and(|alive| owner::pidfd_living(alive).is_ok_and(|living| !living));
    let proc_absent = std::fs::metadata(format!("/proc/{}", parent.child.id()))
        .is_err_and(|error| error.kind() == io::ErrorKind::NotFound);
    let _ = std::fs::remove_file(&output_file);
    let _ = std::fs::remove_dir(&directory);

    // Preserve production quarantine: never overwrite a fault/occupied bit.
    // All ordinary peer paths above finish actual child wait/EOF first. If
    // protocol validation quarantined this owner, fail the aggregate process
    // after actual closure rather than invoking a forbidden occupied Drop.
    if state.occupied && actual_wait.is_ok() && final_actual_closure {
        eprintln!("PARENT_PIPE_QUARANTINE actual_wait=true stdout_EOF=true");
        std::process::abort();
    }
    assert!(admitted.is_ok(), "exact parent response admission failed");
    assert!(
        natural,
        "failure-only cleanup cannot count as a successful exchange"
    );
    assert!(actual_wait.is_ok_and(|status| status.code() == Some(0)));
    assert!(state.supervisor_wait == Some(0) && state.stdout_eof);
    assert!(pidfd_dead && proc_absent);
    assert!(!state.occupied && !state.cleanup_fault && state.retired.is_none());
    assert!(state.scope_reported_closed);
    let mut expected = vec![0x20, 1, 0];
    expected.extend_from_slice(&(PUBLIC_VALUE.len() as u16).to_le_bytes());
    expected.extend_from_slice(PUBLIC_VALUE);
    assert!(
        received.is_ok_and(|actual| actual == expected),
        "actual private response frame differs"
    );
}

fn synthetic_live_scope() -> owner::Snapshot {
    owner::Snapshot {
        worker_pid: 100,
        occupied: true,
        retired: None,
        candidate_buffered: false,
        worker_ready: true,
        worker_stdout_eof: false,
        worker_wait: None,
        adopted_waits: 0,
        final_echild: false,
        populated: true,
        cleanup_fault: false,
        start_released: true,
        direct_kill_attempted: false,
        direct_kill_returned: false,
        scope_kill_attempted: false,
        scope_kill_returned: false,
    }
}

fn peer() -> io::Result<()> {
    let output_file = std::env::var_os(OUTPUT_ENV).ok_or(io::ErrorKind::InvalidInput)?;
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    output.write_all(READY)?;
    output.flush()?;
    let mut barrier = [0];
    input.read_exact(&mut barrier)?;
    if barrier != *b"?" {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let live = synthetic_live_scope();
    output.write_all(&record::state(live, record::STATE))?;
    output.write_all(&record::hidden_prompt(live.worker_pid, 1))?;
    output.flush()?;
    let mut received = vec![0; 5 + PUBLIC_VALUE.len()];
    let response = input.read_exact(&mut received);
    if response.is_ok() {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(PathBuf::from(output_file))?;
        file.write_all(&received)?;
    }
    // These are public synthetic protocol facts, not actual worker/cgroup waits.
    let closed = owner::Snapshot {
        occupied: false,
        worker_stdout_eof: true,
        worker_wait: Some(0),
        final_echild: true,
        populated: false,
        ..live
    };
    output.write_all(&record::wait(owner::WaitReceipt {
        pid: live.worker_pid,
        raw_status: 0,
        worker: true,
    }))?;
    output.write_all(&record::state(closed, record::STATE))?;
    output.write_all(&record::state(closed, record::CLOSED))?;
    output.flush()?;
    response
}

#[test]
#[ignore = "exact private subprocess of the parent process-boundary test"]
fn private_parent_pipe_peer() {
    if std::env::var_os(PEER_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        std::process::exit(2);
    }
    let code = if peer().is_ok() { 0 } else { 2 };
    // Suppress libtest's trailing text on the binary observation pipe.
    std::process::exit(code);
}
