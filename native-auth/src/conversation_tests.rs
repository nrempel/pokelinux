//! Ordinary host pipe integration: original Rust wire -> production parser.
//! No worker, PAM, cgroup, namespace, broker, candidate authority or unlock.
#[path = "../../native-pam/src/wire.rs"]
#[allow(dead_code)]
mod original_wire;
use super::{Barrier, Candidate, ResponseFrame, Retired, parse_worker_protocol};
use std::{fs::File, io::Read};

#[test]
fn original_hidden_prompt_crosses_real_private_pipes() {
    let (to_parent, worker_output) = rustix::pipe::pipe().unwrap();
    let (worker_input, to_worker) = rustix::pipe::pipe().unwrap();
    let until = crate::boottime_ns() + 15_000_000_000;
    std::thread::scope(|threads| {
        let worker = threads.spawn(move || {
            let mut input = File::from(worker_input);
            let mut output = File::from(worker_output);
            original_wire::ready(&mut output, until).unwrap();
            let value = original_wire::response(&mut input, &mut output, 1, until).unwrap();
            assert!(value.as_slice() == b"public-test-input");
        });
        let mut reader = File::from(to_parent);
        let writer = File::from(to_worker);
        let mut bytes = [0; 4];
        reader.read_exact(&mut bytes).unwrap();
        assert!(bytes == [0x10, 0x11, 1, 0]);
        let mut barrier = Barrier::new(100);
        barrier.start_released = true;
        let mut protocol = bytes.to_vec();
        let mut output = Vec::new();
        let result = parse_worker_protocol(&mut protocol, &mut barrier, &mut output);
        // Close the real wire thread before asserting the intended RED. This
        // fixture response is public dummy data, never an authentication input.
        let mut frame =
            ResponseFrame::new(1, zeroize::Zeroizing::new(b"public-test-input".to_vec())).unwrap();
        assert!(frame.write_once(&writer).unwrap());
        drop(frame);
        worker.join().unwrap();
        assert!(
            result.is_ok(),
            "original hidden prompt was refused by the owned-scope parser"
        );
        assert!(barrier.worker_ready && barrier.candidate.is_none());
        assert!(barrier.hidden_prompt == Some(1) && barrier.prompts == 1);
        assert!(output.is_empty(), "prompt must not generate terminal ACK");
    });
}

#[test]
fn existing_terminal_wire_still_buffers_without_unlock_authority() {
    let mut barrier = Barrier::new(100);
    barrier.start_released = true;
    let mut protocol = vec![0x10, 0x30, 1];
    let mut output = Vec::new();
    parse_worker_protocol(&mut protocol, &mut barrier, &mut output).unwrap();
    assert!(barrier.worker_ready);
    assert!(barrier.candidate == Some(Candidate::Denied));
    assert!(barrier.snapshot().occupied);
    assert!(output == [0x40]);
}

#[test]
fn fragmented_prompt_requires_complete_sequence_and_retirement_never_revives_it() {
    let mut barrier = Barrier::new(100);
    barrier.start_released = true;
    let mut protocol = Vec::new();
    let mut output = Vec::new();
    for (index, byte) in [0x10, 0x11, 1, 0].into_iter().enumerate() {
        protocol.push(byte);
        parse_worker_protocol(&mut protocol, &mut barrier, &mut output).unwrap();
        assert!(barrier.hidden_prompt == if index == 3 { Some(1) } else { None });
    }
    assert!(output.is_empty() && barrier.snapshot().occupied);
    barrier.retire(Retired::Cancel);
    protocol.extend_from_slice(&[0x11, 2, 0, 0x30, 0]);
    parse_worker_protocol(&mut protocol, &mut barrier, &mut output).unwrap();
    assert!(barrier.hidden_prompt.is_none() && barrier.candidate.is_none());
    assert!(barrier.retired == Some(Retired::Cancel) && barrier.snapshot().occupied);
}

#[test]
fn malformed_or_pipelined_prompts_and_premature_candidate_are_refused() {
    for bytes in [
        vec![0x11, 1, 0],
        vec![0x10, 0x10],
        vec![0x10, 0x11, 0, 0],
        vec![0x10, 0x11, 2, 0],
        vec![0x10, 0x11, 33, 0],
        vec![0x10, 0x11, 1, 0, 0x30, 0],
        vec![0x10, 0x12],
    ] {
        let mut barrier = Barrier::new(100);
        barrier.start_released = true;
        assert!(parse_worker_protocol(&mut bytes.clone(), &mut barrier, &mut Vec::new()).is_err());
        assert!(barrier.hidden_prompt.is_none() && barrier.candidate.is_none());
    }
    for bytes in [vec![0x30, 0], vec![0x11, 1, 0], vec![0x11, 2, 0]] {
        let mut barrier = Barrier::new(100);
        barrier.start_released = true;
        parse_worker_protocol(&mut vec![0x10, 0x11, 1, 0], &mut barrier, &mut Vec::new()).unwrap();
        assert!(parse_worker_protocol(&mut bytes.clone(), &mut barrier, &mut Vec::new()).is_err());
        assert!(barrier.candidate.is_none());
    }
}

#[test]
fn response_bounds_and_original_wire_maximum_are_exact() {
    for (sequence, value) in [
        (0, vec![1]),
        (33, vec![1]),
        (1, vec![]),
        (1, vec![0]),
        (1, vec![1; 513]),
    ] {
        assert!(ResponseFrame::new(sequence, zeroize::Zeroizing::new(value)).is_err());
    }
    assert!(
        ResponseFrame::new(
            original_wire::MAX_PROMPTS,
            zeroize::Zeroizing::new(vec![1; original_wire::MAX_RESPONSE])
        )
        .is_ok()
    );
}

#[test]
fn full_nonblocking_pipe_retains_whole_frame_then_releases_it_once() {
    use rustix::{
        io::Errno,
        pipe::{PipeFlags, pipe_with},
    };
    let (reader, writer) = pipe_with(PipeFlags::NONBLOCK | PipeFlags::CLOEXEC).unwrap();
    let mut total = 0;
    let mut full = false;
    for _ in 0..256 {
        match rustix::io::write(&writer, &[0x55; 4096]) {
            Ok(count) => total += count,
            Err(Errno::AGAIN) => {
                full = true;
                break;
            }
            _ => panic!("private pipe fill failed"),
        }
    }
    assert!(full && total != 0);
    let mut frame = ResponseFrame::new(1, zeroize::Zeroizing::new(vec![0x42; 512])).unwrap();
    assert!(!frame.write_once(&writer).unwrap());
    assert!(frame.offset == 0 && frame.bytes.len() == 517);
    let mut reader = File::from(reader);
    reader.read_exact(&mut vec![0; total]).unwrap();
    assert!(frame.write_once(&writer).unwrap());
    let mut actual = [0; 517];
    reader.read_exact(&mut actual).unwrap();
    assert!(actual[..5] == [0x20, 1, 0, 0, 2] && actual[5..] == [0x42; 512]);
    assert!(frame.write_once(&writer).unwrap());
    assert!(rustix::io::read(&reader, &mut [0]).unwrap_err() == Errno::AGAIN);
}
