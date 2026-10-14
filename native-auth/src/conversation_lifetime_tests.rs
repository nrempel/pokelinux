//! Exact conversation methods used by Attempt, with real private host pipes.
//! No Child/Leaf/PAM or application authority is constructed by these tests.
use super::{
    Barrier, Candidate, Retired, conversation::Conversation,
    conversation_tests::original_wire as wire,
};
use rustix::{
    fd::OwnedFd,
    fs::{self, OFlags},
    io::Errno,
};
use std::{
    fs::File,
    io::{Read, Write},
    time::{Duration, Instant},
};

struct Pipes {
    input: OwnedFd,
    output: OwnedFd,
    peer_input: File,
    peer_output: File,
}
impl Pipes {
    fn new() -> Self {
        let (peer_input, input) = rustix::pipe::pipe().unwrap();
        let (output, peer_output) = rustix::pipe::pipe().unwrap();
        Self {
            input,
            output,
            peer_input: File::from(peer_input),
            peer_output: File::from(peer_output),
        }
    }
    fn started(&mut self, until: u64) -> (Conversation, Barrier) {
        let mut conversation = Conversation::new(until);
        let mut barrier = Barrier::new(100);
        conversation
            .initialize(Some(&self.input), Some(&self.output))
            .unwrap();
        conversation
            .release_start(Some(&self.input), &mut barrier)
            .unwrap();
        assert!(wire::start(&mut self.peer_input).unwrap() == until);
        assert!(barrier.start_released);
        (conversation, barrier)
    }
    fn prompt(&mut self, conversation: &mut Conversation, barrier: &mut Barrier) {
        self.peer_output.write_all(&[0x10, 0x11, 1, 0]).unwrap();
        conversation
            .read_protocol(Some(&self.output), barrier)
            .unwrap();
        assert!(conversation.pending_hidden_prompt(barrier) == Some(1));
    }
    fn drainable(&self) {
        fs::fcntl_setfl(
            &self.peer_input,
            fs::fcntl_getfl(&self.peer_input).unwrap() | OFlags::NONBLOCK,
        )
        .unwrap();
    }
}
fn value() -> zeroize::Zeroizing<Vec<u8>> {
    zeroize::Zeroizing::new(b"public-test-input".to_vec())
}

#[test]
fn exact_conversation_methods_complete_two_prompts_terminal_ack_and_eof() {
    std::thread::scope(|threads| {
        // All parent endpoints are local to this scope closure. Panic drops
        // them before the scope waits, unblocking a failed peer's private read.
        let Pipes {
            input,
            output,
            peer_input,
            peer_output,
        } = Pipes::new();
        let until = crate::boottime_ns() + 20_000_000_000;
        let mut conversation = Conversation::new(until);
        let mut barrier = Barrier::new(100);
        conversation
            .initialize(Some(&input), Some(&output))
            .unwrap();
        let peer = threads.spawn(move || {
            let mut reader = peer_input;
            let mut writer = peer_output;
            let original = wire::start(&mut reader).unwrap();
            assert!(original == until);
            wire::ready(&mut writer, original).unwrap();
            for sequence in [1, 2] {
                let response =
                    wire::response(&mut reader, &mut writer, sequence, original).unwrap();
                assert!(response.as_slice() == b"public-test-input");
            }
            wire::finish(&mut reader, &mut writer, wire::Outcome::Denied, original).unwrap();
        });
        conversation
            .release_start(Some(&input), &mut barrier)
            .unwrap();
        let bound = Instant::now() + Duration::from_secs(2);
        for sequence in [1, 2] {
            while conversation.pending_hidden_prompt(&barrier) != Some(sequence) {
                assert!(Instant::now() < bound && crate::boottime_ns() < until);
                conversation
                    .read_protocol(Some(&output), &mut barrier)
                    .unwrap();
                std::thread::yield_now();
            }
            conversation
                .respond_hidden_prompt(&mut barrier, sequence, value())
                .unwrap();
            conversation
                .write_pending(Some(&input), &mut barrier)
                .unwrap();
            assert!(conversation.pending_hidden_prompt(&barrier).is_none());
        }
        while barrier.candidate.is_none() {
            assert!(Instant::now() < bound && crate::boottime_ns() < until);
            conversation
                .read_protocol(Some(&output), &mut barrier)
                .unwrap();
            std::thread::yield_now();
        }
        assert!(barrier.candidate == Some(Candidate::Denied) && barrier.snapshot().occupied);
        conversation
            .write_pending(Some(&input), &mut barrier)
            .unwrap();
        peer.join().unwrap();
        conversation
            .read_protocol(Some(&output), &mut barrier)
            .unwrap();
        assert!(barrier.worker_stdout_eof && barrier.snapshot().occupied);
        assert!(conversation.deadline() == until);
    });
}

#[test]
fn cancellation_of_queued_response_cannot_write_after_pipe_backpressure_clears() {
    let mut pipes = Pipes::new();
    let until = crate::boottime_ns() + 20_000_000_000;
    let (mut conversation, mut barrier) = pipes.started(until);
    pipes.prompt(&mut conversation, &mut barrier);
    let mut filled = 0;
    let mut blocked = false;
    for _ in 0..256 {
        match rustix::io::write(&pipes.input, &[0x55; 4096]) {
            Ok(count) => filled += count,
            Err(Errno::AGAIN) => {
                blocked = true;
                break;
            }
            _ => panic!("private pipe fill failed"),
        }
    }
    assert!(blocked && filled > 0 && crate::boottime_ns() < until);
    conversation
        .respond_hidden_prompt(&mut barrier, 1, value())
        .unwrap();
    conversation
        .write_pending(Some(&pipes.input), &mut barrier)
        .unwrap();
    assert!(conversation.pending_hidden_prompt(&barrier) == Some(1));
    conversation.retire(&mut barrier, Retired::Cancel);
    conversation.retire(&mut barrier, Retired::Stop);
    pipes.peer_input.read_exact(&mut vec![0; filled]).unwrap();
    pipes.drainable();
    conversation
        .write_pending(Some(&pipes.input), &mut barrier)
        .unwrap();
    assert!(rustix::io::read(&pipes.peer_input, &mut [0; 32]).unwrap_err() == Errno::AGAIN);
    assert!(barrier.retired == Some(Retired::Cancel) && barrier.candidate.is_none());
    assert!(
        conversation.pending_hidden_prompt(&barrier).is_none() && conversation.deadline() == until
    );
}

#[test]
fn mismatched_and_repeated_response_calls_retire_without_plaintext_output() {
    for repeated in [false, true] {
        let mut pipes = Pipes::new();
        let until = crate::boottime_ns() + 20_000_000_000;
        let (mut conversation, mut barrier) = pipes.started(until);
        pipes.prompt(&mut conversation, &mut barrier);
        if repeated {
            conversation
                .respond_hidden_prompt(&mut barrier, 1, value())
                .unwrap();
        }
        assert!(crate::boottime_ns() < until);
        let sequence = if repeated { 1 } else { 2 };
        assert!(
            conversation
                .respond_hidden_prompt(&mut barrier, sequence, value())
                .is_err()
        );
        conversation
            .write_pending(Some(&pipes.input), &mut barrier)
            .unwrap();
        pipes.drainable();
        assert!(rustix::io::read(&pipes.peer_input, &mut [0; 32]).unwrap_err() == Errno::AGAIN);
        assert!(barrier.retired == Some(Retired::Protocol) && barrier.candidate.is_none());
    }
}

#[test]
fn expired_queued_response_is_retired_before_the_next_pipe_write() {
    let mut pipes = Pipes::new();
    let until = crate::boottime_ns() + 100_000_000;
    let (mut conversation, mut barrier) = pipes.started(until);
    pipes.prompt(&mut conversation, &mut barrier);
    conversation
        .respond_hidden_prompt(&mut barrier, 1, value())
        .unwrap();
    assert!(crate::boottime_ns() < until);
    std::thread::sleep(Duration::from_millis(120));
    assert!(crate::boottime_ns() >= until);
    let result = conversation.write_pending(Some(&pipes.input), &mut barrier);
    pipes.drainable();
    let observed = rustix::io::read(&pipes.peer_input, &mut [0; 32]);
    eprintln!(
        "CONVERSATION_DEADLINE_WRITE timed_out={} postdeadline_bytes={}",
        result
            .as_ref()
            .err()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::TimedOut),
        observed.as_ref().copied().unwrap_or(0)
    );
    assert!(result.is_err_and(|e| e.kind() == std::io::ErrorKind::TimedOut));
    assert!(observed.is_err_and(|e| e == Errno::AGAIN));
    assert!(barrier.retired == Some(Retired::Deadline) && barrier.candidate.is_none());
    assert!(conversation.deadline() == until);
}

#[test]
fn late_terminal_frame_cannot_be_buffered_after_original_deadline() {
    let mut pipes = Pipes::new();
    let until = crate::boottime_ns() + 100_000_000;
    let (mut conversation, mut barrier) = pipes.started(until);
    wire::ready(&mut pipes.peer_output, until).unwrap();
    conversation
        .read_protocol(Some(&pipes.output), &mut barrier)
        .unwrap();
    assert!(barrier.worker_ready && crate::boottime_ns() < until);
    std::thread::sleep(Duration::from_millis(120));
    assert!(crate::boottime_ns() >= until);
    pipes.peer_output.write_all(&[0x30, 0]).unwrap();
    let result = conversation.read_protocol(Some(&pipes.output), &mut barrier);
    eprintln!(
        "CONVERSATION_DEADLINE_READ timed_out={} candidate_buffered={}",
        result
            .as_ref()
            .err()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::TimedOut),
        barrier.candidate.is_some()
    );
    assert!(result.is_err_and(|e| e.kind() == std::io::ErrorKind::TimedOut));
    assert!(barrier.retired == Some(Retired::Deadline) && barrier.candidate.is_none());
}
