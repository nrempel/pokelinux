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
const OUTCOME_ENV: &str = "POKELINUX_PARENT_PIPE_OUTCOME";
const UNTIL_ENV: &str = "POKELINUX_PARENT_PIPE_ORIGINAL_UNTIL";
const NEGATIVE_FILTER: &str = "parent::parent_process_tests::private_terminal_quarantine_owner";
const NEGATIVE_ENV: &str = "POKELINUX_PARENT_PIPE_TERMINAL_NEGATIVE";
const QUARANTINE_CLOSED: &[u8] =
    b"PARENT-NEGATIVE-INNER-NATURAL-WAIT0-EOF-PIDFD-DEAD-PROC-ABSENT-QUARANTINED\n";
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

fn construct_parent(output_file: &Path, outcome: bool, budget: u64) -> io::Result<ScopeChild> {
    construct_parent_variant(output_file, if outcome { "denied" } else { "none" }, budget)
}

fn construct_parent_variant(
    output_file: &Path,
    variant: &str,
    budget: u64,
) -> io::Result<ScopeChild> {
    let until = crate::boottime_ns() + budget;
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
        .env(OUTCOME_ENV, variant)
        .env(UNTIL_ENV, until.to_string())
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
        prompts: PromptQueue::default(),
        commands: crate::command::Decoder::new(),
        outcome_issuer: crate::outcome::Issuer::new(until),
        outcome_taken: false,
        outcome: None,
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

#[derive(Clone, Copy, PartialEq)]
enum Consumption {
    Current,
    Stop,
    Expired,
}

fn exercise_parent_pipe(outcome: bool, consumption: Consumption) {
    let directory = std::env::temp_dir().join(format!(
        "pokelinux-parent-pipe-{}-{}",
        std::process::id(),
        crate::boottime_ns()
    ));
    DirBuilder::new().mode(0o700).create(&directory).unwrap();
    let output_file = directory.join("public-response.bin");
    let budget = if consumption == Consumption::Expired {
        2_000_000_000
    } else {
        15_000_000_000
    };
    let mut parent = construct_parent(&output_file, outcome, budget)
        .expect("construct exact private peer owner");
    let admitted = queue_public_response(&mut parent);
    if admitted.is_err() {
        // EOF releases a peer waiting for the public response on failure.
        parent.input = None;
    }
    // A synthetic CLOSED record cannot substitute for this actual live peer's
    // wait and EOF. The peer withholds both until this explicit release.
    let mut held_gate = !outcome;
    if outcome && admitted.is_ok() {
        let bound = Instant::now() + BOUND;
        while !parent.state.scope_reported_closed
            && !parent.state.cleanup_fault
            && Instant::now() < bound
        {
            parent.poll();
            std::thread::yield_now();
        }
        held_gate = parent.state.scope_reported_closed
            && !parent.state.cleanup_fault
            && parent.state.supervisor_wait.is_none()
            && !parent.state.stdout_eof
            && parent
                ._alive
                .as_ref()
                .is_some_and(|fd| owner::pidfd_living(fd).is_ok_and(|live| live))
            && parent.take_closed_outcome().is_none();
        if held_gate {
            let released = parent
                .input
                .as_ref()
                .is_some_and(|input| rustix::io::write(input, b"?").is_ok_and(|count| count == 1));
            if !released {
                parent.input = None;
            }
        } else {
            parent.input = None;
        }
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
    let before_extraction = parent.snapshot();
    let token = parent.take_closed_outcome();
    let expected_token_issued = token.is_some() == outcome;
    let original_deadline_matches = token
        .as_ref()
        .is_none_or(|token| token.deadline() == parent.until);
    let second_extraction_refused = parent.take_closed_outcome().is_none();
    if consumption == Consumption::Stop {
        parent.retire(owner::Retired::Stop);
    } else if consumption == Consumption::Expired {
        let remaining = parent.until.saturating_sub(crate::boottime_ns());
        std::thread::sleep(Duration::from_nanos(remaining) + Duration::from_millis(5));
        parent.poll();
    }
    let classified = token.and_then(|token| parent.consume_closed_outcome(token));
    let terminal_unchanged = parent.snapshot() == before_extraction;
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
        eprintln!(
            "PARENT_PIPE_QUARANTINE outcome={} natural={} wait_code={:?} stdout_EOF={} pidfd_dead={} proc_absent={} cleanup_fault={} retired={:?}",
            outcome,
            natural,
            actual_wait.as_ref().ok().and_then(|status| status.code()),
            state.stdout_eof,
            pidfd_dead,
            proc_absent,
            state.cleanup_fault,
            state.retired
        );
        std::process::abort();
    }
    assert!(admitted.is_ok(), "exact parent response admission failed");
    assert!(
        held_gate,
        "a live peer or unobserved EOF must prevent a closed outcome"
    );
    assert!(
        expected_token_issued
            && original_deadline_matches
            && second_extraction_refused
            && terminal_unchanged
    );
    if outcome && consumption == Consumption::Current {
        assert!(classified == Some(owner::Candidate::Denied));
    } else {
        assert!(classified.is_none());
    }
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

#[test]
fn exact_parent_prompt_response_crosses_owned_peer_pipes_and_closes() {
    exercise_parent_pipe(false, Consumption::Current);
}

#[test]
fn denied_outcome_record_requires_owned_peer_closure_without_quarantine() {
    exercise_parent_pipe(true, Consumption::Current);
}

#[test]
fn closed_outcome_stop_revokes_token_without_reoccupying_closed_capacity() {
    exercise_parent_pipe(true, Consumption::Stop);
}

#[test]
fn closed_outcome_consumption_rechecks_original_deadline_after_cleanup() {
    exercise_parent_pipe(true, Consumption::Expired);
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
    let outcome = std::env::var(OUTCOME_ENV).map_err(|_| io::ErrorKind::InvalidInput)?;
    let denied = match outcome.as_str() {
        "denied" | "omitted" | "duplicate" | "early" | "wrong-pid" | "wrong-deadline" => true,
        "none" => false,
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    };
    let until = std::env::var(UNTIL_ENV)
        .map_err(|_| io::ErrorKind::InvalidInput)?
        .parse::<u64>()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
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
        candidate_buffered: denied,
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
    // Negative variants alter only the terminal observation stream. Prompt and
    // actual public response exchange stay identical to the positive control.
    if outcome != "early" {
        output.write_all(&record::state(closed, record::STATE))?;
    }
    if denied {
        // Authored raw record for the intended private outcome boundary. These
        // scope facts remain synthetic; only the ordinary peer is really owned.
        let mut frame = [0; record::SIZE];
        frame[..8].copy_from_slice(record::MAGIC);
        frame[8..12].copy_from_slice(&7_u32.to_le_bytes());
        let worker_pid = if outcome == "wrong-pid" {
            live.worker_pid + 1
        } else {
            live.worker_pid
        };
        let original_until = if outcome == "wrong-deadline" {
            until + 1
        } else {
            until
        };
        frame[16..20].copy_from_slice(&worker_pid.to_le_bytes());
        frame[20..24].copy_from_slice(&1_u32.to_le_bytes());
        frame[24..32].copy_from_slice(&original_until.to_le_bytes());
        frame[32..40].copy_from_slice(&crate::boottime_ns().to_le_bytes());
        if outcome != "omitted" {
            output.write_all(&frame)?;
        }
        if outcome == "duplicate" {
            output.write_all(&frame)?;
        }
    }
    if outcome == "early" {
        output.write_all(&record::state(closed, record::STATE))?;
    }
    output.write_all(&record::state(closed, record::CLOSED))?;
    output.flush()?;
    if outcome == "denied" && response.is_ok() {
        let mut release = [0];
        input.read_exact(&mut release)?;
        if release != *b"?" {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
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

// Each negative uses the same actual peer pipe. The expected occupied Drop is
// isolated from libtest, and a constant marker proves inner cleanup happened
// before SIGABRT. An abort without that marker cannot pass an outer test.
fn await_quarantine_marker(output: &impl AsFd, bound: Instant) -> io::Result<bool> {
    let mut matched = 0;
    let mut total = 0;
    loop {
        if !readable(output, bound)? {
            return Ok(false);
        }
        let mut bytes = [0; 128];
        match rustix::io::read(output, &mut bytes) {
            Ok(0) => return Ok(false),
            Ok(count) => {
                total += count;
                if total > 4096 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                for byte in bytes[..count].iter().copied() {
                    matched = if byte == QUARANTINE_CLOSED[matched] {
                        matched + 1
                    } else {
                        usize::from(byte == QUARANTINE_CLOSED[0])
                    };
                    if matched == QUARANTINE_CLOSED.len() {
                        return Ok(true);
                    }
                }
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn expect_terminal_quarantine(variant: &str) {
    use std::os::unix::process::ExitStatusExt;
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            NEGATIVE_FILTER,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env(NEGATIVE_ENV, variant)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut preparing = PreparingPeer {
        child: Some(child),
        alive: None,
    };
    let child = preparing.child.as_ref().unwrap();
    let pid = child.id();
    let process_id = Pid::from_raw(pid as i32).unwrap();
    preparing.alive = Some(process::pidfd_open(process_id, PidfdFlags::NONBLOCK).unwrap());
    let output = child.stdout.as_ref().unwrap();
    fs::fcntl_setfl(output, fs::fcntl_getfl(output).unwrap() | OFlags::NONBLOCK).unwrap();
    // Inner ScopeChild cannot exist until all outer ownership setup succeeds.
    let started = rustix::io::write(child.stdin.as_ref().unwrap(), b"?");
    let bound = Instant::now() + Duration::from_secs(30);
    let marker = await_quarantine_marker(output, bound);
    let natural = readable(preparing.alive.as_ref().unwrap(), bound).unwrap_or(false);
    if !natural {
        let _ = process::pidfd_send_signal(preparing.alive.as_ref().unwrap(), Signal::KILL);
    }
    let status = preparing.child.as_mut().unwrap().wait();
    let output = preparing.child.as_ref().unwrap().stdout.as_ref().unwrap();
    let eof_bound = Instant::now() + BOUND;
    let mut eof = false;
    let mut total = 0;
    while Instant::now() < eof_bound && total <= 4096 {
        let mut bytes = [0; 128];
        match rustix::io::read(output, &mut bytes) {
            Ok(0) => {
                eof = true;
                break;
            }
            Ok(count) => total += count,
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                if !readable(output, eof_bound).unwrap_or(false) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let dead = owner::pidfd_living(preparing.alive.as_ref().unwrap()).is_ok_and(|living| !living);
    let absent = std::fs::metadata(format!("/proc/{pid}"))
        .is_err_and(|error| error.kind() == io::ErrorKind::NotFound);
    // The outer exact subprocess is already actually waited here.
    if status.is_ok() {
        preparing.child = None;
    }
    assert!(started.is_ok_and(|count| count == 1));
    assert!(
        marker.is_ok_and(|seen| seen),
        "inner natural closure/quarantine marker missing"
    );
    assert!(natural && eof && dead && absent);
    assert!(
        status.is_ok_and(|status| status.signal() == Some(6)),
        "malformed terminal stream must trigger occupied production Drop SIGABRT"
    );
}

#[test]
fn candidate_closed_without_outcome_quarantines_after_actual_peer_closure() {
    expect_terminal_quarantine("omitted");
}
#[test]
fn duplicate_outcome_quarantines_after_actual_peer_closure() {
    expect_terminal_quarantine("duplicate");
}
#[test]
fn outcome_before_closed_state_quarantines_after_actual_peer_closure() {
    expect_terminal_quarantine("early");
}
#[test]
fn wrong_worker_outcome_quarantines_after_actual_peer_closure() {
    expect_terminal_quarantine("wrong-pid");
}
#[test]
fn wrong_original_deadline_outcome_quarantines_after_actual_peer_closure() {
    expect_terminal_quarantine("wrong-deadline");
}

#[test]
#[ignore = "exact private owner for expected occupied Drop after real inner closure"]
fn private_terminal_quarantine_owner() {
    let variant = std::env::var(NEGATIVE_ENV).unwrap_or_default();
    if !matches!(
        variant.as_str(),
        "omitted" | "duplicate" | "early" | "wrong-pid" | "wrong-deadline"
    ) {
        std::process::exit(2);
    }
    let mut start = [0];
    if io::stdin().read_exact(&mut start).is_err() || start != *b"?" {
        std::process::exit(2);
    }
    let directory = std::env::temp_dir().join(format!(
        "pokelinux-terminal-negative-{}-{}",
        std::process::id(),
        crate::boottime_ns()
    ));
    DirBuilder::new().mode(0o700).create(&directory).unwrap();
    let output_file = directory.join("public-response.bin");
    let mut parent = construct_parent_variant(&output_file, &variant, 15_000_000_000)
        .expect("construct exact inner peer owner");
    // No assertion or unwind after construction until actual peer closure.
    let admitted = queue_public_response(&mut parent);
    if admitted.is_err() {
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
        let _ = parent.child.wait();
    }
    let actual_wait = parent.child.wait();
    let actual_closure = pump_actual_closure(&mut parent, Instant::now() + BOUND);
    let state = parent.snapshot();
    let dead = parent
        ._alive
        .as_ref()
        .is_some_and(|alive| owner::pidfd_living(alive).is_ok_and(|living| !living));
    let absent = std::fs::metadata(format!("/proc/{}", parent.child.id()))
        .is_err_and(|error| error.kind() == io::ErrorKind::NotFound);
    let no_outcome = parent.take_closed_outcome().is_none();
    let events: Vec<u32> = parent
        .take_records()
        .iter()
        .map(|frame| u32::from_le_bytes(frame[8..12].try_into().unwrap()))
        .collect();
    let expected_events: &[u32] = match variant.as_str() {
        "omitted" => &[1, 6, 2, 1, 4],
        "duplicate" => &[1, 6, 2, 1, 7, 7, 4],
        "early" => &[1, 6, 2, 7, 1, 4],
        _ => &[1, 6, 2, 1, 7, 4],
    };
    let expected_retired = if variant == "omitted" {
        owner::Retired::Observation
    } else {
        owner::Retired::Protocol
    };
    let mut expected_response = vec![0x20, 1, 0];
    expected_response.extend_from_slice(&(PUBLIC_VALUE.len() as u16).to_le_bytes());
    expected_response.extend_from_slice(PUBLIC_VALUE);
    let response_matches =
        std::fs::read(&output_file).is_ok_and(|actual| actual == expected_response);
    let _ = std::fs::remove_file(&output_file);
    let _ = std::fs::remove_dir(&directory);
    let qualifies = admitted.is_ok()
        && natural
        && actual_closure
        && actual_wait.is_ok_and(|status| status.code() == Some(0))
        && state.supervisor_wait == Some(0)
        && state.stdout_eof
        && dead
        && absent
        && response_matches
        && events.as_slice() == expected_events
        && state.occupied
        && state.cleanup_fault
        && state.scope_reported_closed
        && state.retired == Some(expected_retired)
        && no_outcome;
    if qualifies {
        let mut output = io::stdout().lock();
        if output
            .write_all(QUARANTINE_CLOSED)
            .and_then(|()| output.flush())
            .is_err()
        {
            // Actual inner wait/EOF were already observed; no marker can pass.
            drop(parent);
            std::process::exit(3);
        }
        drop(output);
        // Exercise unchanged production Drop; its occupied quarantine aborts.
        drop(parent);
        std::process::exit(4);
    }
    // Any acceptance, wrong retirement, nonzero wait or fallback cleanup omits
    // the marker. Even an unrelated abort then fails the outer test.
    drop(parent);
    std::process::exit(3);
}
