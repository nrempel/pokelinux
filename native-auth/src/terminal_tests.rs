//! Actual fixture Terminal/private-pipe writes with an authored outcome source.
//! No actual Attempt, PAM, cgroup or authentication authority is exercised.
use super::{Terminal, TerminalSource, record};
use pokelinux_auth_scope::{boottime_ns, owner::Snapshot};
use rustix::{
    fd::{AsFd, OwnedFd},
    io::Errno,
    pipe::{PipeFlags, pipe_with},
};
use std::time::Duration;

struct Source {
    until: u64,
    revoked: bool,
    decisions: usize,
}
impl Source {
    fn new(until: u64) -> Self {
        Self {
            until,
            revoked: false,
            decisions: 0,
        }
    }
}
impl TerminalSource for Source {
    fn take_record(&mut self) -> Option<[u8; record::SIZE]> {
        self.decisions += 1;
        self.current().then(|| {
            let mut bytes = [0; record::SIZE];
            bytes[..8].copy_from_slice(record::MAGIC);
            bytes[8..12].copy_from_slice(&record::OUTCOME.to_le_bytes());
            bytes[16..20].copy_from_slice(&100_u32.to_le_bytes());
            bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
            bytes[24..32].copy_from_slice(&self.until.to_le_bytes());
            bytes
        })
    }
    fn current(&self) -> bool {
        !self.revoked && boottime_ns() < self.until
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        worker_pid: 100,
        occupied: false,
        retired: None,
        candidate_buffered: true,
        worker_ready: true,
        worker_stdout_eof: true,
        worker_wait: Some(0),
        adopted_waits: 0,
        final_echild: true,
        populated: false,
        cleanup_fault: false,
        start_released: true,
        direct_kill_attempted: false,
        direct_kill_returned: false,
        scope_kill_attempted: false,
        scope_kill_returned: false,
    }
}
fn pipe() -> (OwnedFd, OwnedFd) {
    pipe_with(PipeFlags::NONBLOCK | PipeFlags::CLOEXEC).unwrap()
}
fn fill(output: &impl AsFd) -> usize {
    let mut count = 0;
    let filler = vec![0x55; rustix::param::page_size()];
    loop {
        match rustix::io::write(output, &filler) {
            Ok(amount) if amount == filler.len() => count += amount,
            Err(Errno::AGAIN) => return count,
            other => panic!("unexpected atomic filler write: {other:?}"),
        }
        assert!(count <= 1_048_576);
    }
}
fn drain(input: &impl AsFd, count: usize) {
    let mut left = count;
    let mut bytes = [0; 4096];
    while left > 0 {
        let amount = left.min(bytes.len());
        let read = rustix::io::read(input, &mut bytes[..amount]).unwrap();
        assert!(read > 0 && bytes[..read].iter().all(|byte| *byte == 0x55));
        left -= read;
    }
}
fn records(input: &impl AsFd) -> Vec<[u8; record::SIZE]> {
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0; record::SIZE * 3];
        match rustix::io::read(input, &mut buffer) {
            Ok(count) if count > 0 => bytes.extend_from_slice(&buffer[..count]),
            Err(Errno::AGAIN) => break,
            other => panic!("unexpected record read: {other:?}"),
        }
        assert!(bytes.len() <= record::SIZE * 3);
    }
    assert!(bytes.len().is_multiple_of(record::SIZE));
    bytes.as_chunks::<{ record::SIZE }>().0.to_vec()
}
fn assert_closed(frame: &[u8; record::SIZE]) {
    assert!(
        matches!(record::decode(frame), Ok(record::Decoded::State(value, true)) if value == snapshot())
    );
}

#[test]
fn terminal_full_pipe_retries_one_decision_then_outcome_and_identical_closed() {
    let (input, output) = pipe();
    let filled = fill(&output);
    assert!(filled >= rustix::param::page_size());
    let mut source = Source::new(boottime_ns() + 15_000_000_000);
    let mut terminal = Terminal::new(snapshot());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    assert!(source.decisions == 1 && terminal.pending_outcome.is_some());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    assert!(source.decisions == 1 && terminal.pending_outcome.is_some());
    drain(&input, filled);
    assert!(terminal.write_once(&mut source, &output).unwrap());
    let frames = records(&input);
    assert!(frames.len() == 2 && source.decisions == 1);
    assert!(
        matches!(record::decode(&frames[0]), Ok(record::Decoded::Outcome { worker_pid: 100, until, .. }) if until == source.until)
    );
    assert_closed(&frames[1]);
}

#[test]
fn terminal_outcome_sent_before_closed_backpressure_is_never_repeated() {
    let (input, output) = pipe();
    let filled = fill(&output);
    let page = rustix::param::page_size();
    assert!(filled >= page);
    drain(&input, page);
    // Leave exactly one atomic record in the tail page, not enough for CLOSED.
    assert!(
        rustix::io::write(&output, &vec![0x55; page - record::SIZE]).unwrap()
            == page - record::SIZE
    );
    let mut source = Source::new(boottime_ns() + 15_000_000_000);
    let mut terminal = Terminal::new(snapshot());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    assert!(source.decisions == 1 && terminal.pending_outcome.is_none());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    drain(&input, filled - record::SIZE);
    let frames = records(&input);
    assert!(frames.len() == 1);
    assert!(matches!(
        record::decode(&frames[0]),
        Ok(record::Decoded::Outcome { .. })
    ));
    // Already delivered observations cannot be retracted. Subsequent revocation
    // prevents new results, but CLOSED still uses the original captured state.
    source.revoked = true;
    assert!(terminal.write_once(&mut source, &output).unwrap());
    let frames = records(&input);
    assert!(frames.len() == 1 && source.decisions == 1);
    assert_closed(&frames[0]);
}

#[test]
fn terminal_revocation_discards_pending_outcome_before_retry_without_new_decision() {
    let (input, output) = pipe();
    let filled = fill(&output);
    let mut source = Source::new(boottime_ns() + 15_000_000_000);
    let mut terminal = Terminal::new(snapshot());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    assert!(terminal.pending_outcome.is_some());
    source.revoked = true;
    drain(&input, filled);
    assert!(terminal.write_once(&mut source, &output).unwrap());
    let frames = records(&input);
    assert!(frames.len() == 1 && source.decisions == 1);
    assert_closed(&frames[0]);
}

#[test]
fn terminal_original_clock_expiry_discards_an_already_pending_outcome() {
    let (input, output) = pipe();
    let filled = fill(&output);
    let until = boottime_ns() + 100_000_000;
    let mut source = Source::new(until);
    let mut terminal = Terminal::new(snapshot());
    assert!(!terminal.write_once(&mut source, &output).unwrap());
    assert!(terminal.pending_outcome.is_some() && source.current());
    std::thread::sleep(
        Duration::from_nanos(until.saturating_sub(boottime_ns())) + Duration::from_millis(2),
    );
    assert!(!source.current() && source.until == until);
    drain(&input, filled);
    assert!(terminal.write_once(&mut source, &output).unwrap());
    let frames = records(&input);
    assert!(frames.len() == 1 && source.decisions == 1);
    assert_closed(&frames[0]);
}
