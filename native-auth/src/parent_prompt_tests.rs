//! Ordinary host pipe -> production record decoder runtime RED.
//! Worker/cgroup fields below are synthetic wire facts, not scope cleanup.
//! No ScopeChild/Attempt, subprocess, PAM, cgroup or privileged fixture runs.
use crate::{owner, record};
use std::{
    fs::File,
    io::{Read, Write},
};

#[test]
fn credential_free_prompt_crosses_real_pipe_after_an_accepted_state() {
    let (reader, writer) = rustix::pipe::pipe().unwrap();
    let mut reader = File::from(reader);
    let mut writer = File::from(writer);
    let synthetic_scope = owner::Snapshot {
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
    };
    writer
        .write_all(&record::state(synthetic_scope, record::STATE))
        .unwrap();
    let mut baseline = [0; record::SIZE];
    reader.read_exact(&mut baseline).unwrap();
    assert!(matches!(
        record::decode(&baseline),
        Ok(record::Decoded::State(value, false)) if value == synthetic_scope
    ));

    // Emit the next event only after the baseline was actually decoded. Use
    // current MAGIC so the same test follows the planned PKSCOPE3 revision.
    let mut prompt = [0; record::SIZE];
    prompt[..8].copy_from_slice(record::MAGIC);
    prompt[8..12].copy_from_slice(&6_u32.to_le_bytes());
    prompt[16..20].copy_from_slice(&synthetic_scope.worker_pid.to_le_bytes());
    prompt[20..24].copy_from_slice(&1_u32.to_le_bytes());
    prompt[32..40].copy_from_slice(&crate::boottime_ns().to_le_bytes());
    writer.write_all(&prompt).unwrap();
    let mut observed = [0; record::SIZE];
    reader.read_exact(&mut observed).unwrap();
    let decoded = record::decode(&observed);
    // Complete actual private-pipe EOF before asserting the intended RED.
    drop(writer);
    let eof = reader.read(&mut [0; 1]).unwrap();
    drop(reader);

    assert!(eof == 0);
    assert!(
        observed == prompt,
        "the actual pipe must preserve the whole frame"
    );
    assert!(
        decoded.is_ok(),
        "credential-free Prompt event 6 was refused after an accepted state"
    );
}

fn scope() -> owner::Snapshot {
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
fn value() -> zeroize::Zeroizing<Vec<u8>> {
    zeroize::Zeroizing::new(b"public-parent-pipe-input".to_vec())
}
fn pipes() -> (rustix::fd::OwnedFd, rustix::fd::OwnedFd) {
    rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC | rustix::pipe::PipeFlags::NONBLOCK)
        .unwrap()
}
fn empty(reader: &rustix::fd::OwnedFd) {
    assert!(matches!(
        rustix::io::read(reader, &mut [0; 1]),
        Err(rustix::io::Errno::AGAIN)
    ));
}

#[test]
fn parent_queue_two_prompts_cross_real_pipe_into_actual_command_decoder() {
    let (reader, writer) = pipes();
    let until = crate::boottime_ns() + 20_000_000_000;
    let mut queue = super::parent_prompt::PromptQueue::default();
    let mut decoder = crate::command::Decoder::new();
    for expected in 1..=2 {
        let encoded = record::hidden_prompt(scope().worker_pid, expected);
        let (worker_pid, sequence) = match record::decode(&encoded).unwrap() {
            record::Decoded::HiddenPrompt {
                worker_pid,
                sequence,
            } => (worker_pid, sequence),
            _ => panic!("exact prompt record required"),
        };
        queue
            .observe(Some(scope()), worker_pid, sequence, false)
            .unwrap();
        assert!(queue.pending(until) == Some(expected));
        queue.respond(until, expected, value()).unwrap();
        assert!(queue.pending(until).is_none());
        queue.flush(until, &writer).unwrap();
        let mut bytes = zeroize::Zeroizing::new([0; 517]);
        let count = rustix::io::read(&reader, &mut bytes[..]).unwrap();
        let batch = decoder.batch(&bytes[..count]);
        assert!(batch.retired.is_none() && !batch.snapshot);
        let (sequence, observed) = batch.response.expect("whole decoded response");
        assert!(sequence == expected && observed.as_slice() == value().as_slice());
        queue.flush(until, &writer).unwrap();
        empty(&reader);
    }
    drop(writer);
    assert!(rustix::io::read(&reader, &mut [0; 1]).unwrap() == 0);
}

#[test]
fn parent_prompt_refuses_wrong_worker_order_duplicate_and_terminal_scope() {
    use super::parent_prompt::PromptQueue;
    let ready = scope();
    let invalid = [
        None,
        Some(owner::Snapshot {
            worker_pid: 101,
            ..ready
        }),
        Some(owner::Snapshot {
            retired: Some(owner::Retired::Cancel),
            ..ready
        }),
        Some(owner::Snapshot {
            occupied: false,
            ..ready
        }),
        Some(owner::Snapshot {
            worker_ready: false,
            ..ready
        }),
        Some(owner::Snapshot {
            start_released: false,
            ..ready
        }),
        Some(owner::Snapshot {
            candidate_buffered: true,
            ..ready
        }),
        Some(owner::Snapshot {
            worker_stdout_eof: true,
            ..ready
        }),
        Some(owner::Snapshot {
            worker_wait: Some(0),
            ..ready
        }),
        Some(owner::Snapshot {
            cleanup_fault: true,
            ..ready
        }),
    ];
    for scope in invalid {
        let mut queue = PromptQueue::default();
        assert!(queue.observe(scope, 100, 1, false).is_err());
        assert!(queue.pending(u64::MAX).is_none());
    }
    for sequence in [0, 2, 32, 33, u16::MAX] {
        assert!(
            PromptQueue::default()
                .observe(Some(ready), 100, sequence, false)
                .is_err()
        );
    }
    let mut queue = PromptQueue::default();
    queue.observe(Some(ready), 100, 1, false).unwrap();
    assert!(queue.observe(Some(ready), 100, 1, false).is_err());
    assert!(queue.pending(u64::MAX).is_none());
    // Local retirement never accepts a new prompt, even a valid late record.
    queue.observe(Some(ready), 100, 2, true).unwrap();
    assert!(queue.pending(u64::MAX).is_none());
}

#[test]
fn parent_queued_response_clears_under_backpressure_and_original_deadline() {
    let (reader, writer) = pipes();
    let mut total = 0;
    loop {
        match rustix::io::write(&writer, &[0x55; 4096]) {
            Ok(count) => total += count,
            Err(rustix::io::Errno::AGAIN) => break,
            other => panic!("unexpected pipe fill: {other:?}"),
        }
        assert!(total <= 1_048_576);
    }
    assert!(total > 0);
    let mut queue = super::parent_prompt::PromptQueue::default();
    let until = crate::boottime_ns() + 20_000_000_000;
    queue.observe(Some(scope()), 100, 1, false).unwrap();
    queue.respond(until, 1, value()).unwrap();
    queue.flush(until, &writer).unwrap();
    queue.clear();
    let mut drained = 0;
    loop {
        let mut bytes = [0; 4096];
        match rustix::io::read(&reader, &mut bytes) {
            Ok(count) => {
                assert!(bytes[..count].iter().all(|byte| *byte == 0x55));
                drained += count;
            }
            Err(rustix::io::Errno::AGAIN) => break,
            other => panic!("unexpected pipe drain: {other:?}"),
        }
    }
    assert!(drained == total);
    queue.flush(until, &writer).unwrap();
    empty(&reader);
    queue.observe(Some(scope()), 100, 2, false).unwrap();
    let original_until = crate::boottime_ns() + 100_000_000;
    queue.respond(original_until, 2, value()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(120));
    assert!(
        queue.flush(original_until, &writer).unwrap_err().kind() == std::io::ErrorKind::TimedOut
    );
    assert!(queue.pending(u64::MAX).is_none());
    empty(&reader);
}

#[test]
fn parent_response_refusal_clears_queued_plaintext_without_output() {
    let (reader, writer) = pipes();
    let until = crate::boottime_ns() + 20_000_000_000;
    for case in 0..5 {
        let mut queue = super::parent_prompt::PromptQueue::default();
        queue.observe(Some(scope()), 100, 1, false).unwrap();
        if case == 1 {
            queue.respond(until, 1, value()).unwrap();
        }
        let (deadline, sequence, bytes) = match case {
            0 => (until, 2, value()),
            1 => (until, 1, value()),
            2 => (0, 1, value()),
            3 => (until, 1, zeroize::Zeroizing::new(b"public\0input".to_vec())),
            _ => (until, 1, zeroize::Zeroizing::new(vec![b'x'; 513])),
        };
        assert!(queue.respond(deadline, sequence, bytes).is_err());
        assert!(queue.pending(u64::MAX).is_none());
        queue.flush(until, &writer).unwrap();
        empty(&reader);
    }
}

#[test]
fn hidden_prompt_record_checks_every_non_timestamp_field_and_rejects_old_magic() {
    let valid = record::hidden_prompt(100, 1);
    assert!(matches!(
        record::decode(&valid),
        Ok(record::Decoded::HiddenPrompt {
            worker_pid: 100,
            sequence: 1
        })
    ));
    for (at, value) in [
        (12, 1_u32),
        (16, 0),
        (16, 1),
        (16, u32::MAX),
        (20, 0),
        (20, 33),
        (20, 65_537),
        (24, 1),
        (28, 1),
        (40, 1),
        (44, 1),
    ] {
        let mut bytes = valid;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(record::decode(&bytes).is_err());
    }
    let mut historical = valid;
    historical[..8].copy_from_slice(b"PKSCOPE2");
    assert!(record::decode(&historical).is_err());
}
