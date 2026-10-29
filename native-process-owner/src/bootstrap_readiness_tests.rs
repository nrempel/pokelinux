//! Actual socket readiness only; the public guard and authentication are
//! exercised separately through the normal owned worker integration.
use super::*;

fn pair() -> (OwnedFd, OwnedFd) {
    net::socketpair(
        net::AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap()
}

#[test]
fn readiness_hint_does_not_consume_a_queued_record() {
    let (channel, peer) = pair();
    let parent = process::pidfd_open(process::getpid(), PidfdFlags::empty()).unwrap();
    let transferred = rustix::io::fcntl_dupfd_cloexec(&parent, 3).unwrap();
    assert!(!poll_message_readiness(&channel, &parent, &transferred).unwrap());
    assert_eq!(net::send(&peer, b"one", SendFlags::NOSIGNAL).unwrap(), 3);
    assert!(poll_message_readiness(&channel, &parent, &transferred).unwrap());
    assert!(poll_message_readiness(&channel, &parent, &transferred).unwrap());
    let mut bytes = [0; 8];
    assert_eq!(
        net::recv(&channel, &mut bytes, RecvFlags::DONTWAIT).unwrap(),
        (3, 3)
    );
    assert_eq!(&bytes[..3], b"one");
    assert!(!poll_message_readiness(&channel, &parent, &transferred).unwrap());
}

#[test]
fn channel_hangup_is_terminal_even_with_a_queued_record() {
    let (channel, peer) = pair();
    let parent = process::pidfd_open(process::getpid(), PidfdFlags::empty()).unwrap();
    let transferred = rustix::io::fcntl_dupfd_cloexec(&parent, 3).unwrap();
    assert_eq!(net::send(&peer, b"one", SendFlags::NOSIGNAL).unwrap(), 3);
    drop(peer);
    assert_eq!(
        poll_message_readiness(&channel, &parent, &transferred)
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    let mut bytes = [0; 8];
    assert_eq!(
        net::recv(&channel, &mut bytes, RecvFlags::DONTWAIT).unwrap(),
        (3, 3)
    );
    assert_eq!(&bytes[..3], b"one");
}
