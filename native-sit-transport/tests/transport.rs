use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
use pokelinux_sit_transport::{Error, Outcome, Snapshot, StopReason, WorkerTransport};
use rustix::{
    io::{Errno, read, write},
    net::{AddressFamily, SocketFlags, SocketType, socketpair},
};
use std::{os::fd::OwnedFd, time::Duration};

fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .expect("actual Linux socketpair fixture")
}
fn deadline() -> Deadline {
    Deadline::at(BootClock.now() + Duration::from_secs(30))
}
fn bytes(text: &str, count: usize) -> Vec<u8> {
    assert!((1..=983).contains(&count) && text.len() <= count * 4);
    let hex: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    assert_eq!(hex.len(), count * 2);
    hex.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn request() -> [u8; 983] {
    bytes(include_str!("fixtures/initial-v4.hex"), 983)
        .try_into()
        .unwrap()
}
fn response() -> Vec<u8> {
    bytes(include_str!("fixtures/base-v4.hex"), 144)
}
fn queue(peer: &OwnedFd, bytes: &[u8]) {
    assert_eq!(
        write(peer, bytes).expect("prequeue actual socket bytes"),
        bytes.len()
    );
}
fn no_peer_bytes(peer: &OwnedFd) {
    assert_eq!(read(peer, &mut [0u8; 984][..]), Err(Errno::AGAIN));
}
fn dispatch_and_receive_request(transport: &mut WorkerTransport, peer: &OwnedFd) {
    let expected = request();
    transport
        .dispatch(&expected)
        .expect("MISSING_SINGLE_TRANSPORT_DISPATCH");
    let mut received = [0u8; 984];
    assert_eq!(read(peer, &mut received[..]).unwrap(), 983);
    assert_eq!(&received[..983], expected.as_slice());
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::OutcomeUnknown,
            terminal: None
        }
    );
}
fn started() -> (WorkerTransport, OwnedFd) {
    let (fd, peer) = pair();
    let mut transport = WorkerTransport::new(fd, deadline());
    dispatch_and_receive_request(&mut transport, &peer);
    (transport, peer)
}

#[test]
fn consumes_and_retains_the_handle_until_drop() {
    let (fd, peer) = pair();
    let transport = WorkerTransport::new(fd, deadline());
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::NotSent,
            terminal: None
        }
    );
    no_peer_bytes(&peer);
    drop(transport);
    assert_eq!(read(&peer, &mut [0u8; 1][..]).unwrap(), 0);
}

#[test]
fn expired_before_dispatch_sends_nothing_and_stays_terminal() {
    let (fd, peer) = pair();
    let mut transport = WorkerTransport::new(fd, Deadline::at(Duration::ZERO));
    assert_eq!(transport.dispatch(&request()), Err(Error::Deadline));
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::NotSent,
            terminal: Some(StopReason::Deadline)
        }
    );
    assert_eq!(read(&peer, &mut [0u8; 984][..]).unwrap(), 0);
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::Terminal(StopReason::Deadline))
    );
    assert!(matches!(
        transport.read_chunk(),
        Err(Error::Terminal(StopReason::Deadline))
    ));
}

#[test]
fn dispatches_every_request_byte_once_and_refuses_a_second_dispatch() {
    let (mut transport, peer) = started();
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::AlreadyDispatched)
    );
    no_peer_bytes(&peer);
    assert_eq!(transport.snapshot().outcome, Outcome::OutcomeUnknown);
    assert_eq!(transport.snapshot().terminal, None);
}

#[test]
fn owned_full_read_survives_the_next_read_and_transport_drop() {
    let (mut transport, peer) = started();
    let first: Vec<u8> = (0..65_568).map(|i| (17 * i + 3) as u8).collect();
    let second = vec![0xa5; 144];
    queue(&peer, &first);
    queue(&peer, &second);
    let retained = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    let later = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    assert_eq!(retained.as_bytes(), first.as_slice());
    assert_eq!(later.as_bytes(), second.as_slice());
    drop(transport);
    drop(peer);
    assert_eq!(retained.as_bytes(), first.as_slice());
    assert_eq!(later.as_bytes(), second.as_slice());
}

#[test]
fn short_read_stays_opaque_and_owned() {
    let (mut transport, peer) = started();
    let input = response()[..16].to_vec();
    queue(&peer, &input);
    let chunk = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    assert_eq!(chunk.as_bytes(), input.as_slice());
    assert_eq!(transport.snapshot().outcome, Outcome::OutcomeUnknown);
    assert_eq!(transport.snapshot().terminal, None);
}

#[test]
fn coalesced_input_is_one_opaque_owned_chunk() {
    let (mut transport, peer) = started();
    let input = response().repeat(2);
    assert_eq!(input.len(), 288);
    queue(&peer, &input);
    let chunk = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    assert_eq!(chunk.as_bytes(), input.as_slice());
    assert_eq!(transport.snapshot().terminal, None);
}

#[test]
fn non_rcm_input_is_copied_without_semantic_validation() {
    let (mut transport, peer) = started();
    let input: [u8; 32] = std::array::from_fn(|i| (37 * i + 11) as u8);
    queue(&peer, &input);
    let chunk = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    assert_eq!(chunk.as_bytes(), input.as_slice());
    assert_eq!(transport.snapshot().terminal, None);
}

#[test]
fn zero_read_stops_locally_and_preserves_uncertainty() {
    let (mut transport, peer) = started();
    drop(peer);
    match transport.read_chunk() {
        Err(error) => assert_eq!(error, Error::ZeroRead, "MISSING_LOCAL_ZERO_READ_STOP"),
        Ok(_) => panic!("zero read must not produce a received chunk"),
    }
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::OutcomeUnknown,
            terminal: Some(StopReason::ZeroRead)
        }
    );
    assert!(matches!(
        transport.read_chunk(),
        Err(Error::Terminal(StopReason::ZeroRead))
    ));
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::Terminal(StopReason::ZeroRead))
    );
}

#[test]
fn refusal_before_dispatch_preserves_queued_bytes_without_proving_freshness() {
    let (fd, peer) = pair();
    let input = response();
    queue(&peer, &input);
    let mut transport = WorkerTransport::new(fd, deadline());
    assert!(matches!(transport.read_chunk(), Err(Error::NotDispatched)));
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::NotSent,
            terminal: None
        }
    );
    dispatch_and_receive_request(&mut transport, &peer);
    let chunk = transport
        .read_chunk()
        .expect("MISSING_OWNED_TRANSPORT_READ");
    assert_eq!(chunk.as_bytes(), input.as_slice());
}

#[test]
fn actual_empty_nonblocking_read_error_is_terminal_without_retry() {
    let (mut transport, peer) = started();
    match transport.read_chunk() {
        Err(error) => assert_eq!(
            error,
            Error::Read(Errno::AGAIN),
            "MISSING_TERMINAL_READ_ERROR"
        ),
        Ok(_) => panic!("empty nonblocking read must return its actual syscall error"),
    }
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::OutcomeUnknown,
            terminal: Some(StopReason::Read)
        }
    );
    assert!(matches!(
        transport.read_chunk(),
        Err(Error::Terminal(StopReason::Read))
    ));
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::Terminal(StopReason::Read))
    );
    assert_eq!(read(&peer, &mut [0u8; 984][..]).unwrap(), 0);
}
