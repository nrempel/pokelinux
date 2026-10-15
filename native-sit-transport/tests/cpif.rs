use pokelinux_process_owner::deadline::{BootClock, Clock};
use pokelinux_sit_codec::response::DecodeError;
use pokelinux_sit_transport::{
    Error as TransportError, Outcome, StopReason,
    cpif::{CpifReadPolicy, Error, Observation},
};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::{Errno, read, write},
    net::{AddressFamily, SendFlags, SocketFlags, SocketType, send, socketpair},
};
use std::{num::NonZeroU16, os::fd::OwnedFd, time::Duration};
fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap()
}
fn bytes(text: &str) -> Vec<u8> {
    let text: Vec<_> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    text.as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn request() -> [u8; 983] {
    bytes(include_str!("fixtures/initial-v4.hex"))
        .try_into()
        .unwrap()
}
fn response() -> Vec<u8> {
    bytes(include_str!("fixtures/base-v4.hex"))
}
fn policy(fd: OwnedFd, until: Duration, budget: u16) -> CpifReadPolicy {
    CpifReadPolicy::new(fd, until, NonZeroU16::new(budget).unwrap()).unwrap()
}
fn dispatch(p: &mut CpifReadPolicy, peer: &OwnedFd) {
    p.dispatch(&request()).unwrap();
    let mut b = [0; 984];
    assert_eq!(read(peer, &mut b[..]).unwrap(), 983);
    assert_eq!(&b[..983], &request());
}
fn started(budget: u16) -> (CpifReadPolicy, OwnedFd) {
    let (fd, peer) = pair();
    let mut p = policy(fd, BootClock.now() + Duration::from_secs(20), budget);
    dispatch(&mut p, &peer);
    (p, peer)
}
fn chunk(p: &mut CpifReadPolicy) -> pokelinux_sit_transport::cpif::LocalChunk {
    match p.observe(0).expect("MISSING_CPIF_READ_POLICY") {
        Observation::Chunk(c) => c,
        Observation::Idle => panic!("positive bytes lost"),
    }
}
fn no_retry(peer: &OwnedFd) {
    assert_eq!(read(peer, &mut [0; 984][..]), Err(Errno::AGAIN));
}
#[test]
fn owns_handle_and_preserves_existing_one_shot_dispatch() {
    let (mut p, peer) = started(2);
    assert_eq!(p.snapshot().outcome, Outcome::OutcomeUnknown);
    assert_eq!(
        p.dispatch(&request()),
        Err(TransportError::AlreadyDispatched)
    );
    no_retry(&peer);
    drop(p);
    assert_eq!(read(&peer, &mut [0; 1][..]).unwrap(), 0);
}
#[test]
fn no_read_before_dispatch_preserves_prequeued_bytes() {
    let (fd, peer) = pair();
    let expected = response();
    assert_eq!(write(&peer, &expected).unwrap(), 144);
    let mut p = policy(fd, BootClock.now() + Duration::from_secs(20), 1);
    assert!(matches!(
        p.observe(0),
        Err(Error::Transport(TransportError::NotDispatched))
    ));
    assert_eq!(p.remaining(), 1);
    dispatch(&mut p, &peer);
    let c = chunk(&mut p);
    assert_eq!(c.bytes(), expected);
    assert!(c.syntax().is_ok());
}
#[test]
fn supported_syntax_retains_all_bytes_without_granting_freshness() {
    let (fd, peer) = pair();
    let expected = response();
    assert_eq!(write(&peer, &expected).unwrap(), 144);
    let mut p = policy(fd, BootClock.now() + Duration::from_secs(20), 2);
    dispatch(&mut p, &peer);
    let c = chunk(&mut p);
    assert_eq!(c.bytes(), expected);
    let syntax = c.syntax().as_ref().unwrap();
    assert_eq!(syntax.token, 0);
    assert_eq!(syntax.context.get(), 1);
    assert_eq!(syntax.ipv4, Some([192, 0, 2, 7]));
    assert_eq!(syntax.reported_mtu_v4, 1420);
    assert_eq!(p.remaining(), 1);
    no_retry(&peer);
    drop(p);
    assert_eq!(c.bytes(), expected);
}
#[test]
fn short_coalesced_malformed_and_token_mismatch_are_syntax_refusals() {
    let good = response();
    let mut malformed = good.clone();
    malformed[0] = 9;
    for (input, token, expected) in [
        (good[..16].to_vec(), 0, DecodeError::UnsupportedLength),
        (good.repeat(2), 0, DecodeError::UnsupportedLength),
        (malformed, 0, DecodeError::WrongType),
        (good, 9, DecodeError::TokenMismatch),
    ] {
        let (mut p, peer) = started(2);
        assert_eq!(write(&peer, &input).unwrap(), input.len());
        let Observation::Chunk(c) = p.observe(token).expect("MISSING_CPIF_READ_POLICY") else {
            panic!("missing bytes")
        };
        assert_eq!(c.bytes(), input);
        assert_eq!(c.syntax(), &Err(expected));
        assert_eq!(p.snapshot().terminal, None);
        no_retry(&peer);
    }
}
#[test]
fn independent_short_chunks_are_never_assembled() {
    let (mut p, peer) = started(2);
    let full = response();
    for part in [&full[..72], &full[72..]] {
        assert_eq!(write(&peer, part).unwrap(), 72);
        let c = chunk(&mut p);
        assert_eq!(c.bytes(), part);
        assert_eq!(c.syntax(), &Err(DecodeError::UnsupportedLength));
    }
    assert!(matches!(p.observe(0), Err(Error::WorkLimit)));
    assert_eq!(p.snapshot().terminal, Some(StopReason::WorkLimit));
    assert_eq!(read(&peer, &mut [0; 1][..]).unwrap(), 0);
}
#[test]
fn actual_ready_zero_is_idle_then_same_generation_receives_bytes() {
    let (fd, peer) = pair();
    assert_eq!(send(&peer, &[], SendFlags::empty()).unwrap(), 0);
    {
        let mut fds = [PollFd::new(&fd, PollFlags::IN)];
        assert_eq!(poll(&mut fds, Some(&Timespec::default())).unwrap(), 1);
        assert!(fds[0].revents().contains(PollFlags::IN));
    }
    let mut p = policy(fd, BootClock.now() + Duration::from_secs(20), 2);
    dispatch(&mut p, &peer);
    assert!(matches!(
        p.observe(0).expect("MISSING_CPIF_IDLE_POLICY"),
        Observation::Idle
    ));
    assert_eq!(p.snapshot().terminal, None);
    assert_eq!(p.remaining(), 1);
    let expected = response();
    assert_eq!(write(&peer, &expected).unwrap(), 144);
    assert_eq!(chunk(&mut p).bytes(), expected);
    no_retry(&peer);
}
#[test]
fn repeated_ready_zero_spends_finite_work_without_reopening() {
    let (mut p, peer) = started(2);
    for left in [1, 0] {
        assert_eq!(send(&peer, &[], SendFlags::empty()).unwrap(), 0);
        assert!(matches!(
            p.observe(0).expect("MISSING_CPIF_IDLE_POLICY"),
            Observation::Idle
        ));
        assert_eq!(p.remaining(), left);
    }
    assert!(matches!(p.observe(0), Err(Error::WorkLimit)));
    assert_eq!(p.snapshot().outcome, Outcome::OutcomeUnknown);
    assert_eq!(p.snapshot().terminal, Some(StopReason::WorkLimit));
    assert_eq!(read(&peer, &mut [0; 1][..]).unwrap(), 0);
    assert!(matches!(
        p.observe(0),
        Err(Error::Transport(TransportError::Terminal(
            StopReason::WorkLimit
        )))
    ));
}
#[test]
fn empty_descriptor_wait_wakes_at_original_boottime_deadline() {
    let (fd, peer) = pair();
    let until = BootClock.now() + Duration::from_millis(100);
    let mut p = policy(fd, until, 4);
    dispatch(&mut p, &peer);
    let start = BootClock.now();
    assert!(
        matches!(
            p.observe(0),
            Err(Error::Transport(TransportError::Deadline))
        ),
        "MISSING_CPIF_DEADLINE_WAIT"
    );
    assert!(BootClock.now() >= until);
    assert!(BootClock.now() - start < Duration::from_secs(5));
    assert_eq!(p.snapshot().terminal, Some(StopReason::Deadline));
    assert_eq!(p.snapshot().outcome, Outcome::OutcomeUnknown);
    assert_eq!(read(&peer, &mut [0; 1][..]).unwrap(), 0);
}
#[test]
fn queued_positive_after_original_deadline_is_not_exposed() {
    let (fd, peer) = pair();
    let until = BootClock.now() + Duration::from_millis(50);
    let mut p = policy(fd, until, 4);
    dispatch(&mut p, &peer);
    assert_eq!(write(&peer, &response()).unwrap(), 144);
    std::thread::sleep(Duration::from_millis(70));
    assert!(matches!(
        p.observe(0),
        Err(Error::Transport(TransportError::Deadline))
    ));
    assert_eq!(p.snapshot().terminal, Some(StopReason::Deadline));
}
#[test]
fn real_peer_hangup_is_terminal_separate_from_idle_zero() {
    let (mut p, peer) = started(2);
    drop(peer);
    assert!(
        matches!(p.observe(0),Err(Error::Readiness(mask)) if mask & 0x10!=0),
        "MISSING_CPIF_HANGUP_POLICY"
    );
    assert_eq!(p.snapshot().terminal, Some(StopReason::Readiness));
    assert_eq!(p.snapshot().outcome, Outcome::OutcomeUnknown);
}
#[test]
fn full_native_bound_copy_survives_later_reads_and_drop() {
    let (mut p, peer) = started(2);
    let expected: Vec<_> = (0..65_568).map(|i| (i * 17 + 3) as u8).collect();
    assert_eq!(write(&peer, &expected).unwrap(), expected.len());
    let c = chunk(&mut p);
    assert_eq!(c.bytes(), expected);
    assert_eq!(c.syntax(), &Err(DecodeError::UnsupportedLength));
    assert_eq!(write(&peer, &response()).unwrap(), 144);
    let later = chunk(&mut p);
    assert!(later.syntax().is_ok());
    drop(p);
    assert_eq!(c.bytes(), expected);
}
