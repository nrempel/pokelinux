#![cfg(target_os = "linux")]

use omarchy_phone_core::{MAX_FRAME, ipc::receive_request};
use std::{
    io::{self, Write},
    os::unix::net::UnixStream,
    thread,
    time::{Duration, Instant},
};

const FRAME: &[u8] = b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"epoch\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"session\":\"cccccccccccccccccccccccccccccccc\",\"command\":{\"kind\":\"dial\",\"generation\":\"dddddddddddddddddddddddddddddddd\",\"offer\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\",\"number\":\"5550100\"}}\n";

#[test]
fn fragmented_real_socket_request_and_timeout_restoration() {
    let (mut reader, mut writer) = UnixStream::pair().unwrap();
    reader
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let old_timeout = reader.read_timeout().unwrap();
    let producer = thread::spawn(move || {
        for part in FRAME.chunks(7) {
            writer.write_all(part).unwrap();
        }
    });
    receive_request(&mut reader, Duration::from_secs(1)).expect("valid fragmented request");
    assert_eq!(reader.read_timeout().unwrap(), old_timeout);
    producer.join().unwrap();
}

#[test]
fn slow_writer_cannot_extend_absolute_deadline() {
    let (mut reader, mut writer) = UnixStream::pair().unwrap();
    let producer = thread::spawn(move || {
        for byte in FRAME {
            if writer.write_all(&[*byte]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(8));
        }
    });
    let start = Instant::now();
    let error = receive_request(&mut reader, Duration::from_millis(60)).unwrap_err();
    assert!(
        matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ),
        "{error}"
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(reader.read_timeout().unwrap(), None);
    drop(reader);
    producer.join().unwrap();
}

#[test]
fn truncated_oversized_and_coalesced_frames_fail() {
    let mut two = FRAME.to_vec();
    two.extend(FRAME);
    for data in [
        FRAME[..FRAME.len() - 1].to_vec(),
        vec![b' '; MAX_FRAME + 1],
        two,
    ] {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&data).unwrap();
        drop(writer);
        assert_eq!(
            receive_request(&mut reader, Duration::from_secs(1))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(reader.read_timeout().unwrap(), None);
    }
}
