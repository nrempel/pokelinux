//! Real Linux socket/process boundary. Source deadlines here are authored inputs;
//! actor acceptance and genuine compositor startup are separate integration gates.
use super::*;
use pokelinux_session_owner::readiness::Readiness;
use std::{ffi::OsString, os::unix::net::UnixDatagram};

fn receiver(root: &RootDir) -> UnixDatagram {
    let socket = UnixDatagram::bind(root.0.join("notify.sock")).unwrap();
    socket.set_nonblocking(true).unwrap();
    rustix::net::sockopt::set_socket_passcred(&socket, true).unwrap();
    socket
}
fn notification(socket: &UnixDatagram) -> Option<Vec<u8>> {
    pokelinux_lifecycle::ipc::receive(socket)
        .unwrap()
        .map(|(body, cred)| {
            assert_eq!(cred.pid.as_raw_nonzero().get() as u32, std::process::id());
            assert_eq!(cred.uid.as_raw(), 0);
            assert_eq!(cred.gid.as_raw(), 0);
            body
        })
}
fn notifier(root: &RootDir, peer: &Peer, label: &str) -> Readiness {
    Readiness::new(
        source_path(root, label),
        peer.watch(),
        SourceIdentity {
            pid: peer.pid(),
            uid: 1000,
            gid: 1000,
        },
        OsString::from(root.0.join("notify.sock")),
        boottime_ns() + 2_000_000_000,
    )
    .unwrap()
}
fn fresh() -> Option<u64> {
    Some(boottime_ns() + 100_000_000)
}

#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn current_source_and_exact_peer_publish_once_from_main_process() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let mut ready = notifier(&root, &peer, "desktop");
    ready.poll(None, false).unwrap();
    assert_eq!(notification(&socket), None);
    ready.poll(fresh(), false).unwrap();
    assert_eq!(notification(&socket), Some(b"READY=1".to_vec()));
    ready.poll(fresh(), false).unwrap();
    assert_eq!(notification(&socket), None);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn stale_source_cannot_publish_until_a_current_admission() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let mut ready = notifier(&root, &peer, "desktop");
    ready
        .poll(Some(boottime_ns().saturating_sub(1)), false)
        .unwrap();
    assert_eq!(notification(&socket), None);
    ready.poll(fresh(), false).unwrap();
    assert_eq!(notification(&socket), Some(b"READY=1".to_vec()));
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn unavailable_wayland_is_pending_and_stop_fences_later_bind() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let mut peer = Peer::with_delayed_bind(&root.0, "desktop", 1000, true);
    let mut ready = notifier(&root, &peer, "desktop");
    ready.poll(fresh(), false).unwrap();
    assert_eq!(notification(&socket), None);
    ready.poll(None, true).unwrap();
    peer.child.stdin.as_mut().unwrap().write_all(b"B").unwrap();
    while !source_path(&root, "desktop").exists() {
        assert!(boottime_ns() < peer.until);
        peer.pump();
        tick();
    }
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn different_live_process_cannot_replace_owned_compositor() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let expected = Peer::new(&root.0, "expected", 1000);
    let foreign = Peer::new(&root.0, "foreign", 1000);
    let mut ready = notifier(&root, &expected, "foreign");
    assert_eq!(
        ready.poll(fresh(), false).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(notification(&socket), None);
    fs::remove_file(source_path(&root, "foreign")).unwrap();
    fs::rename(
        source_path(&root, "expected"),
        source_path(&root, "foreign"),
    )
    .unwrap();
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
    drop(foreign);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn actual_wrong_uid_cannot_supply_ready() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 0);
    let mut ready = notifier(&root, &peer, "desktop");
    assert!(ready.poll(fresh(), false).is_err());
    assert_eq!(notification(&socket), None);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn retained_dead_compositor_rejects_new_live_socket_owner() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let mut ready = notifier(&root, &peer, "desktop");
    drop(peer);
    let replacement = Peer::new(&root.0, "replacement", 1000);
    fs::remove_file(source_path(&root, "desktop")).unwrap();
    fs::rename(
        source_path(&root, "replacement"),
        source_path(&root, "desktop"),
    )
    .unwrap();
    assert!(ready.poll(fresh(), false).is_err());
    assert_eq!(notification(&socket), None);
    drop(replacement);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn original_startup_deadline_cannot_be_extended_by_late_source() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let until = boottime_ns() + 10_000_000;
    let mut ready = Readiness::new(
        source_path(&root, "desktop"),
        peer.watch(),
        SourceIdentity {
            pid: peer.pid(),
            uid: 1000,
            gid: 1000,
        },
        OsString::from(root.0.join("notify.sock")),
        until,
    )
    .unwrap();
    while boottime_ns() < until {
        tick();
    }
    assert_eq!(
        ready.poll(fresh(), false).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn full_real_wayland_backlog_never_blocks_or_survives_stop() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let mut queued = Vec::new();
    let mut full = false;
    for _ in 0..256 {
        let fd = rustix::net::socket_with(
            rustix::net::AddressFamily::UNIX,
            rustix::net::SocketType::STREAM,
            rustix::net::SocketFlags::NONBLOCK | rustix::net::SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        match rustix::net::connect(
            &fd,
            &rustix::net::SocketAddrUnix::new(source_path(&root, "desktop")).unwrap(),
        ) {
            Ok(()) => queued.push(fd),
            Err(rustix::io::Errno::AGAIN) => {
                full = true;
                break;
            }
            other => panic!("actual backlog fill: {other:?}"),
        }
    }
    assert!(full, "bounded actual backlog positive control not reached");
    let mut ready = notifier(&root, &peer, "desktop");
    let until = fresh().unwrap();
    ready.poll(Some(until), false).unwrap();
    assert!(
        boottime_ns() < until,
        "poll exceeded original Source sample budget"
    );
    assert_eq!(notification(&socket), None);
    ready.poll(None, true).unwrap();
    drop(queued);
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
}
#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn failed_notification_is_not_implicitly_retried_after_destination_appears() {
    let root = RootDir::new();
    let peer = Peer::new(&root.0, "desktop", 1000);
    let mut ready = notifier(&root, &peer, "desktop");
    assert!(ready.poll(fresh(), false).is_err());
    let socket = receiver(&root);
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
}

#[test]
#[ignore = "requires isolated root process/UID fixture"]
fn full_notification_queue_is_bounded_and_does_not_retry() {
    let root = RootDir::new();
    let socket = receiver(&root);
    let peer = Peer::new(&root.0, "desktop", 1000);
    let filler = UnixDatagram::unbound().unwrap();
    filler.set_nonblocking(true).unwrap();
    let mut count = 0;
    let mut full = false;
    for _ in 0..256 {
        match filler.send_to(b"fill", root.0.join("notify.sock")) {
            Ok(4) => count += 1,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                full = true;
                break;
            }
            other => panic!("actual notification fill: {other:?}"),
        }
    }
    assert!(
        full && count > 0,
        "actual notification queue positive control missing"
    );
    let mut ready = notifier(&root, &peer, "desktop");
    let until = fresh().unwrap();
    assert_eq!(
        ready.poll(Some(until), false).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(
        boottime_ns() < until,
        "notification blocked beyond Source budget"
    );
    for _ in 0..count {
        assert_eq!(notification(&socket), Some(b"fill".to_vec()));
    }
    let _ = ready.poll(fresh(), false);
    assert_eq!(notification(&socket), None);
}
