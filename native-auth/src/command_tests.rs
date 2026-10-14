//! Ordinary Linux pipe/codec tests using public dummy inputs only.
//! No Attempt, worker, PAM, cgroup, broker, unlock or physical device executes.
use super::{Command, Decoder, MAX_RESPONSE, State};
use std::{fs::File, io, io::Read, io::Write};
use zeroize::Zeroizing;

fn response(sequence: u16, value: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(vec![0x20]);
    bytes.extend_from_slice(&sequence.to_le_bytes());
    bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn assert_sticky(decoder: &mut Decoder, kind: io::ErrorKind) {
    for input in [b"C".as_slice(), b"".as_slice()] {
        let mut calls = 0;
        let error = decoder
            .feed(input, |_| {
                calls += 1;
                Ok(())
            })
            .unwrap_err();
        assert!(error.kind() == kind && calls == 0);
    }
    decoder.discard();
    decoder.retire();
    assert!(matches!(decoder.state, State::Unusable(value) if value == kind));
    assert!(decoder.feed(b"?", |_| panic!("unusable callback")).is_err());
}

#[test]
fn controls_and_binary_responses_survive_every_two_piece_split() {
    let mut wire = Zeroizing::new(b"?C".to_vec());
    wire.extend_from_slice(&response(32, b"C?S \xff"));
    wire.push(b'S');
    wire.extend_from_slice(&response(1, b"last"));
    wire.push(b'?');
    for split in 0..=wire.len() {
        let mut decoder = Decoder::new();
        let mut commands = Vec::new();
        for fragment in [&wire[..split], &wire[split..]] {
            decoder
                .feed(fragment, |command| {
                    commands.push(command);
                    Ok(())
                })
                .unwrap();
        }
        let mut commands = commands.into_iter();
        assert!(matches!(commands.next(), Some(Command::Snapshot)));
        assert!(matches!(commands.next(), Some(Command::Cancel)));
        match commands.next().unwrap() {
            Command::Response { sequence, value } => {
                assert!(sequence == 32 && value.as_slice() == b"C?S \xff");
            }
            _ => panic!("response command missing"),
        }
        assert!(matches!(commands.next(), Some(Command::Stop)));
        match commands.next().unwrap() {
            Command::Response { sequence, value } => {
                assert!(sequence == 1 && value.as_slice() == b"last");
            }
            _ => panic!("response command missing"),
        }
        assert!(matches!(commands.next(), Some(Command::Snapshot)));
        assert!(commands.next().is_none());
        assert!(matches!(decoder.state, State::Opcode));
    }
}

#[test]
fn maximum_body_is_bounded_and_not_dispatched_before_its_last_byte() {
    let value = Zeroizing::new(vec![b'x'; MAX_RESPONSE]);
    let wire = response(32, &value);
    let mut decoder = Decoder::default();
    let mut calls = 0;
    for (index, byte) in wire.iter().enumerate() {
        decoder
            .feed(std::slice::from_ref(byte), |command| {
                calls += 1;
                match command {
                    Command::Response { sequence, value } => {
                        assert!(sequence == 32 && value.len() == MAX_RESPONSE);
                        assert!(value.iter().all(|byte| *byte == b'x'));
                    }
                    _ => panic!("unexpected control"),
                }
                Ok(())
            })
            .unwrap();
        assert!(calls == usize::from(index + 1 == wire.len()));
        if let State::Body {
            expected, value, ..
        } = &decoder.state
        {
            assert!(*expected == MAX_RESPONSE);
            assert!(value.len() <= MAX_RESPONSE && value.capacity() <= MAX_RESPONSE);
        }
    }
    assert!(matches!(decoder.state, State::Opcode));
}

#[test]
fn every_unknown_opcode_latches_a_protocol_fault() {
    for opcode in 0..=u8::MAX {
        if matches!(opcode, b'C' | b'S' | b'?' | 0x20) {
            continue;
        }
        let mut decoder = Decoder::new();
        let error = decoder
            .feed(&[opcode, b'C'], |_| panic!("invalid opcode callback"))
            .unwrap_err();
        assert!(error.kind() == io::ErrorKind::InvalidData);
        assert_sticky(&mut decoder, io::ErrorKind::InvalidData);
    }
}

#[test]
fn invalid_sequence_or_length_faults_at_the_header_before_body_allocation() {
    for (sequence, length) in [
        (0u16, 1u16),
        (33, 1),
        (256, 1),
        (u16::MAX, 1),
        (1, 0),
        (1, 513),
        (1, u16::MAX),
    ] {
        let mut header = vec![0x20];
        header.extend_from_slice(&sequence.to_le_bytes());
        header.extend_from_slice(&length.to_le_bytes());
        for split in 0..header.len() {
            let mut decoder = Decoder::new();
            decoder
                .feed(&header[..split], |_| panic!("partial header callback"))
                .unwrap();
            let error = decoder
                .feed(&header[split..], |_| panic!("invalid header callback"))
                .unwrap_err();
            assert!(error.kind() == io::ErrorKind::InvalidData);
            assert_sticky(&mut decoder, io::ErrorKind::InvalidData);
        }
    }
}

#[test]
fn nul_at_each_body_position_clears_partial_secret_and_drops_following_commands() {
    for nul in 0..8 {
        let mut value = Zeroizing::new(vec![b'x'; 8]);
        value[nul] = 0;
        let mut wire = response(1, &value);
        wire.extend_from_slice(b"?CS");
        for split in 0..5 + nul + 1 {
            let mut decoder = Decoder::new();
            decoder
                .feed(&wire[..split], |_| panic!("partial response callback"))
                .unwrap();
            let error = decoder
                .feed(&wire[split..], |_| panic!("NUL response callback"))
                .unwrap_err();
            assert!(error.kind() == io::ErrorKind::InvalidData);
            assert_sticky(&mut decoder, io::ErrorKind::InvalidData);
        }
    }
}

#[test]
fn discard_drops_partial_header_and_body_without_reviving_faults() {
    let wire = response(1, b"public-dummy");
    for length in 1..wire.len() {
        let mut decoder = Decoder::new();
        decoder
            .feed(&wire[..length], |_| panic!("incomplete callback"))
            .unwrap();
        decoder.discard();
        assert!(matches!(decoder.state, State::Opcode));
        let mut calls = 0;
        decoder
            .feed(b"?", |command| {
                assert!(matches!(command, Command::Snapshot));
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert!(calls == 1);
    }
}

#[test]
fn retire_drops_partial_header_and_body_and_permanently_refuses_input() {
    let wire = response(1, b"public-dummy");
    for length in 0..wire.len() {
        let mut decoder = Decoder::new();
        decoder
            .feed(&wire[..length], |_| panic!("incomplete callback"))
            .unwrap();
        decoder.retire();
        assert_sticky(&mut decoder, io::ErrorKind::BrokenPipe);
    }
}

#[test]
fn callback_failure_on_response_keeps_original_error_and_drops_tail() {
    let mut wire = response(7, b"public-dummy");
    wire.extend_from_slice(b"?CS");
    let mut decoder = Decoder::new();
    let mut calls = 0;
    let error = decoder
        .feed(&wire, |command| {
            calls += 1;
            match command {
                Command::Response { sequence, value } => {
                    assert!(sequence == 7 && value.as_slice() == b"public-dummy");
                }
                _ => panic!("tail command dispatched"),
            }
            Err(io::Error::new(io::ErrorKind::TimedOut, "callback failure"))
        })
        .unwrap_err();
    assert!(calls == 1 && error.kind() == io::ErrorKind::TimedOut);
    assert!(error.to_string() == "callback failure");
    assert_sticky(&mut decoder, io::ErrorKind::TimedOut);
}

#[test]
fn cancellation_callback_can_stop_dispatch_before_a_coalesced_response() {
    let mut wire = Zeroizing::new(b"C".to_vec());
    wire.extend_from_slice(&response(1, b"public-dummy"));
    let mut decoder = Decoder::new();
    let mut calls = 0;
    let result = decoder.feed(&wire, |command| {
        calls += 1;
        assert!(matches!(command, Command::Cancel));
        Err(io::ErrorKind::Interrupted.into())
    });
    assert!(result.unwrap_err().kind() == io::ErrorKind::Interrupted && calls == 1);
    assert_sticky(&mut decoder, io::ErrorKind::Interrupted);
}

#[test]
fn large_invalid_input_is_not_retained() {
    let wire = Zeroizing::new(vec![b'x'; 1_048_576]);
    let mut decoder = Decoder::new();
    assert!(
        decoder
            .feed(&wire, |_| panic!("invalid input callback"))
            .unwrap_err()
            .kind()
            == io::ErrorKind::InvalidData
    );
    assert_sticky(&mut decoder, io::ErrorKind::InvalidData);
}

#[test]
fn real_pipe_fragmentation_preserves_commands_and_zeroizing_read_ownership() {
    let (input, output) = rustix::pipe::pipe().unwrap();
    let mut wire = Zeroizing::new(b"?".to_vec());
    wire.extend_from_slice(&response(32, b"C?S \xff"));
    wire.extend_from_slice(b"SC");
    std::thread::scope(|threads| {
        let writer = threads.spawn(move || {
            let mut writer = File::from(output);
            for chunk in wire.chunks(3) {
                writer.write_all(chunk).unwrap();
            }
        });
        let mut reader = File::from(input);
        let mut decoder = Decoder::new();
        let mut bytes = Zeroizing::new([0u8; 2]);
        let mut position = 0;
        loop {
            let count = reader.read(bytes.as_mut()).unwrap();
            if count == 0 {
                break;
            }
            decoder
                .feed(&bytes[..count], |command| {
                    match (position, command) {
                        (0, Command::Snapshot) | (2, Command::Stop) | (3, Command::Cancel) => {}
                        (1, Command::Response { sequence, value }) => {
                            assert!(sequence == 32 && value.as_slice() == b"C?S \xff");
                        }
                        _ => panic!("pipe command order changed"),
                    }
                    position += 1;
                    Ok(())
                })
                .unwrap();
        }
        writer.join().unwrap();
        assert!(position == 4 && matches!(decoder.state, State::Opcode));
    });
}

#[test]
fn real_pipe_eof_with_an_incomplete_body_can_be_retired_without_dispatch() {
    let (input, output) = rustix::pipe::pipe().unwrap();
    let wire = response(1, b"public-dummy");
    let mut writer = File::from(output);
    writer.write_all(&wire[..wire.len() - 1]).unwrap();
    drop(writer);
    let mut reader = File::from(input);
    let mut decoder = Decoder::new();
    let mut bytes = Zeroizing::new([0u8; 64]);
    loop {
        let count = reader.read(bytes.as_mut()).unwrap();
        if count == 0 {
            break;
        }
        decoder
            .feed(&bytes[..count], |_| panic!("incomplete pipe callback"))
            .unwrap();
    }
    assert!(matches!(decoder.state, State::Body { .. }));
    decoder.retire();
    assert_sticky(&mut decoder, io::ErrorKind::BrokenPipe);
}

#[test]
fn retiring_batch_discards_all_response_values_before_target_application() {
    for control in *b"CS" {
        let mut input = response(1, b"public-before");
        input.push(control);
        input.extend_from_slice(&response(2, b"public-after"));
        input.push(0xff);
        let mut decoder = Decoder::new();
        let batch = decoder.batch(&input);
        let expected = if control == b'C' {
            crate::owner::Retired::Cancel
        } else {
            crate::owner::Retired::Stop
        };
        assert!(batch.retired == Some(expected) && batch.response.is_none());
        assert!(matches!(decoder.state, State::Unusable(_)));
        assert!(decoder.batch(&response(1, b"late")).response.is_none());
    }
}

#[test]
fn duplicate_response_batch_discards_both_values_and_latches_protocol() {
    let mut input = response(1, b"public-one");
    input.extend_from_slice(&response(1, b"public-two"));
    let mut decoder = Decoder::new();
    let batch = decoder.batch(&input);
    assert!(batch.retired == Some(crate::owner::Retired::Protocol));
    assert!(batch.response.is_none());
    assert!(matches!(decoder.state, State::Unusable(_)));
}

#[test]
fn batch_retirement_clears_an_incomplete_response_and_preserves_snapshot_intent() {
    let mut decoder = Decoder::new();
    let input = response(1, b"public-partial");
    let batch = decoder.batch(&input[..input.len() - 1]);
    assert!(batch.retired.is_none() && batch.response.is_none());
    assert!(matches!(decoder.state, State::Body { .. }));
    decoder.retire();
    assert!(matches!(decoder.state, State::Unusable(_)));
    let retired = super::Batch::retired_snapshot();
    assert!(retired.snapshot && retired.response.is_none() && retired.retired.is_none());
}
