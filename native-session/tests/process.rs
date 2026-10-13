use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_session::Observer;
use rustix::{
    fd::{AsRawFd, OwnedFd},
    net::{self, AddressFamily, SocketFlags, SocketType},
    process::{Pid, PidfdFlags},
};
use std::{
    io,
    process::{Child, Command},
    time::Duration,
};
const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
struct Peer {
    child: Child,
    command: OwnedFd,
}
impl Peer {
    fn command(&mut self, mode: &str) {
        ipc::send(&self.command, mode.as_bytes()).unwrap();
        let until = boottime_ns() + 2_000_000_000;
        loop {
            match ipc::receive(&self.command).unwrap() {
                Some((b, c)) => {
                    assert_eq!(b, b"ready");
                    assert_eq!(c.pid.as_raw_nonzero().get() as u32, self.child.id());
                    break;
                }
                None if boottime_ns() < until => std::thread::sleep(Duration::from_millis(1)),
                _ => panic!("actual fixture readiness timeout"),
            }
        }
    }
    fn reap(&mut self) {
        let until = boottime_ns() + 2_000_000_000;
        loop {
            match self.child.try_wait().unwrap() {
                Some(s) => {
                    assert!(s.success());
                    break;
                }
                None if boottime_ns() < until => std::thread::sleep(Duration::from_millis(1)),
                _ => panic!("actual fixture exit not observed"),
            }
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = ipc::send(&self.command, b"quit");
        let until = boottime_ns() + 2_000_000_000;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if boottime_ns() < until => std::thread::sleep(Duration::from_millis(1)),
                _ => {
                    let _ = self.child.kill();
                    break;
                }
            }
        }
        let until = boottime_ns() + 2_000_000_000;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if boottime_ns() < until => std::thread::sleep(Duration::from_millis(1)),
                _ => std::process::abort(),
            }
        }
    }
}
fn pair() -> (OwnedFd, OwnedFd) {
    net::socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap()
}
fn start(uid_offset: u32) -> (Peer, Observer) {
    let (p, o, _) = start_transferred_pidfd(uid_offset, false);
    (p, o)
}
fn start_transferred_pidfd(uid_offset: u32, transferred: bool) -> (Peer, Observer, i32) {
    let (receive, send) = pair();
    let (control, child_control) = pair();
    net::sockopt::set_socket_passcred(&receive, true).unwrap();
    net::sockopt::set_socket_passcred(&control, true).unwrap();
    net::sockopt::set_socket_passcred(&child_control, true).unwrap();
    let creation = net::sockopt::socket_peercred(&receive).unwrap();
    assert_eq!(
        creation.pid.as_raw_nonzero().get() as u32,
        std::process::id()
    );
    // Serial isolated harness only; narrow actual FD inheritance, no test process threads.
    rustix::io::fcntl_setfd(&send, rustix::io::FdFlags::empty()).unwrap();
    rustix::io::fcntl_setfd(&child_control, rustix::io::FdFlags::empty()).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_session-peer-fixture"))
        .args([
            send.as_raw_fd().to_string(),
            child_control.as_raw_fd().to_string(),
            SOURCE.into(),
        ])
        .spawn()
        .unwrap();
    let pid = Pid::from_raw(child.id() as i32).unwrap();
    assert_ne!(child.id(), std::process::id());
    let alive = rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK).unwrap();
    let alive_raw = alive.as_raw_fd();
    if transferred {
        // An actually inherited descriptor arrives without CLOEXEC at exec.
        rustix::io::fcntl_setfd(&alive, rustix::io::FdFlags::empty()).unwrap();
    }
    drop(send);
    drop(child_control);
    let observer = Observer::new(
        receive,
        alive,
        pid,
        rustix::process::getuid().as_raw() + uid_offset,
        SOURCE.into(),
    )
    .unwrap();
    (
        Peer {
            child,
            command: control,
        },
        observer,
        alive_raw,
    )
}
#[test]
fn actual_packet_sender_overrides_socketpair_creation_credentials() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let state = observer
        .snapshot()
        .expect("actual source packet must be admitted");
    assert!(!state.locked);
    assert_eq!(state.focus.as_ref().unwrap().uid, 11010);
}
#[test]
fn delegated_same_uid_descriptor_cannot_impersonate_source() {
    let (mut p, mut o) = start(0);
    p.command("delegated");
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "delegated actual sender was admitted"
    );
}
#[test]
fn actual_sender_uid_must_match_installed_source() {
    let (mut p, mut o) = start(1);
    p.command("valid");
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "wrong actual source UID was admitted"
    );
}
fn invalid_packet(mode: &str) {
    let (mut p, mut o) = start(0);
    p.command(mode);
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "invalid actual packet admitted: {mode}"
    );
}
#[test]
fn wrong_source_nonce_is_refused() {
    invalid_packet("wrong-source");
}
#[test]
fn initial_sequence_gap_is_refused() {
    invalid_packet("gap");
}
#[test]
fn oversize_actual_packet_is_refused() {
    invalid_packet("oversize");
}
#[test]
fn contradictory_locked_native_focus_is_refused() {
    invalid_packet("locked-focus");
}
#[test]
fn expired_actual_packet_does_not_refresh_source() {
    let (mut p, mut o) = start(0);
    p.command("stale");
    let _ = o.poll();
    assert!(o.snapshot().is_none(), "stale actual packet was admitted");
}
#[test]
fn idle_live_source_does_not_extend_original_sample() {
    let (mut p, mut o) = start(0);
    p.command("valid");
    o.poll().unwrap();
    assert!(o.snapshot().is_some());
    std::thread::sleep(Duration::from_millis(150));
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "living silent source extended original deadline"
    );
}
#[test]
fn buffered_packet_from_actually_reaped_source_is_refused() {
    let (mut p, mut o) = start(0);
    p.command("die");
    p.reap();
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "buffered dead-source frame was admitted"
    );
}
#[test]
fn bounded_turn_overflow_is_terminal_and_discards_all_state() {
    let (mut p, mut o) = start(0);
    p.command("burst");
    let _ = o.poll();
    assert!(o.snapshot().is_none(), "flooded source was admitted");
    p.command("valid");
    let _ = o.poll();
    assert!(o.snapshot().is_none(), "failed source was reactivated");
}
#[test]
fn invalid_source_stays_failed_even_after_valid_packet() {
    let (mut p, mut o) = start(0);
    p.command("wrong-source");
    let _ = o.poll();
    assert!(o.snapshot().is_none());
    p.command("valid");
    let _ = o.poll();
    assert!(
        o.snapshot().is_none(),
        "failed source implicitly reactivated"
    );
}
#[test]
fn unknown_source_has_no_positive_state() {
    let (_p, o) = start(0);
    assert!(o.snapshot().is_none());
}
#[allow(dead_code)]
fn _io_type(_: io::Error) {}

#[test]
fn unchanged_actual_heartbeat_does_not_cancel_semantic_revision() {
    let (mut p, mut o) = start(0);
    p.command("valid");
    o.poll().unwrap();
    let initial = o.revision();
    assert_eq!(initial, 1);
    p.command("heartbeat");
    o.poll().unwrap();
    assert_eq!(o.snapshot().unwrap().sequence, 2);
    assert_eq!(
        o.revision(),
        initial,
        "unchanged heartbeat cancelled the semantic revision"
    );
}
#[test]
fn actual_lock_unlock_drained_together_retains_both_semantic_changes() {
    let (mut p, mut o) = start(0);
    p.command("valid");
    o.poll().unwrap();
    let initial_focus = o.snapshot().unwrap().focus.clone();
    assert_eq!(o.revision(), 1);
    p.command("cycle");
    o.poll().unwrap();
    assert_eq!(o.snapshot().unwrap().focus, initial_focus);
    assert!(!o.snapshot().unwrap().locked);
    assert_eq!(
        o.revision(),
        3,
        "final unlocked focus erased intermediate lock fencing"
    );
}

#[test]
fn transferred_source_pidfd_does_not_leak_into_an_actual_later_exec() {
    let (p, _o, fd) = start_transferred_pidfd(0, true);
    assert!(
        std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}"))
            .unwrap()
            .contains(&format!("Pid:\t{}", p.child.id()))
    );
    let mut inspector = Command::new(env!("CARGO_BIN_EXE_session-peer-fixture"))
        .args([
            "--expect-source-fd-closed",
            &fd.to_string(),
            &p.child.id().to_string(),
        ])
        .spawn()
        .unwrap();
    let until = boottime_ns() + 2_000_000_000;
    loop {
        match inspector.try_wait().unwrap() {
            Some(status) => {
                assert!(
                    status.success(),
                    "owned source pidfd leaked into actual unrelated exec"
                );
                return;
            }
            None if boottime_ns() < until => std::thread::sleep(Duration::from_millis(1)),
            _ => {
                let _ = inspector.kill();
                if inspector.wait().is_err() {
                    std::process::abort();
                }
                panic!("exec inspector deadline");
            }
        }
    }
}

#[test]
fn actual_owned_focus_pidfd_is_retained_after_sender_closes_its_transfer_copy() {
    let (mut p, mut o) = start(0);
    p.command("owned-focus");
    o.poll().unwrap();
    let frame = o
        .snapshot()
        .expect("actual owned focused peer must be admitted");
    assert_eq!(frame.focus.as_ref().unwrap().pid, p.child.id());
}
#[test]
fn actual_owned_focus_requires_exactly_one_descriptor() {
    invalid_packet("owned-focus-missing");
}
#[test]
fn actual_owned_focus_absent_metadata_rejects_unexpected_descriptor() {
    invalid_packet("owned-focus-unexpected");
}
#[test]
fn actual_owned_focus_rejects_two_transferred_descriptors_without_leaking() {
    let (mut p, mut o) = start(0);
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    p.command("owned-focus-two");
    let _ = o.poll();
    assert!(o.snapshot().is_none());
    let after = std::fs::read_dir("/proc/self/fd").unwrap().count();
    assert!(
        after <= before,
        "rejected SCM_RIGHTS descriptors leaked: {before} -> {after}"
    );
}
#[test]
fn actual_owned_focus_refuses_regular_file_ancillary() {
    invalid_packet("owned-focus-file");
}

#[test]
fn actual_ancillary_truncation_drops_every_delivered_handle() {
    let (mut p, mut o) = start(0);
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    p.command("owned-focus-truncated-rights");
    let _ = o.poll();
    assert!(o.snapshot().is_none());
    assert!(std::fs::read_dir("/proc/self/fd").unwrap().count() <= before);
}
#[test]
fn scalar_pidfs_wire_is_refused_without_compatibility() {
    invalid_packet("owned-focus-old-wire");
}
#[test]
fn repeated_actual_transfer_retains_one_handle_and_cloexec_fences_later_exec() {
    let (mut p, mut o) = start(0);
    p.command("owned-focus");
    o.poll().unwrap();
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..100 {
        p.command("owned-focus-heartbeat");
        o.poll().unwrap();
    }
    assert_eq!(o.revision(), 1);
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
    let matches: Vec<_> = std::fs::read_dir("/proc/self/fdinfo")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            std::fs::read_to_string(entry.path())
                .ok()?
                .contains(&format!("Pid:\t{}", p.child.id()))
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    assert_eq!(
        matches.len(),
        2,
        "source and focus each retain their actual own pidfd"
    );
    for fd in matches {
        let status = Command::new(env!("CARGO_BIN_EXE_session-peer-fixture"))
            .args(["--expect-source-fd-closed", &fd, &p.child.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success(), "focused pidfd inherited across exec");
    }
}
#[test]
fn dead_focused_app_does_not_fail_source_and_changed_handle_fences_same_metadata() {
    let (mut p, mut o) = start(0);
    p.command("focus-child-live");
    o.poll().unwrap();
    let metadata = o.snapshot().unwrap().focus.clone();
    assert_eq!(o.revision(), 1);
    p.command("focus-child-dead");
    o.poll().unwrap();
    assert_eq!(o.revision(), 2);
    assert_eq!(o.snapshot().unwrap().focus, metadata);
    p.command("focus-child-new-same-metadata");
    o.poll().unwrap();
    assert_eq!(o.snapshot().unwrap().focus, metadata);
    assert_eq!(
        o.revision(),
        3,
        "same numeric metadata erased changed owned handle"
    );
}

#[test]
fn source_death_watch_must_name_the_actual_expected_live_source() {
    let (p, _original) = start(0);
    let (receive, _send) = pair();
    let wrong =
        rustix::process::pidfd_open(rustix::process::getpid(), PidfdFlags::NONBLOCK).unwrap();
    assert_ne!(p.child.id(), std::process::id());
    let observer = Observer::new(
        receive,
        wrong,
        Pid::from_raw(p.child.id() as i32).unwrap(),
        rustix::process::getuid().as_raw(),
        SOURCE.into(),
    );
    assert!(
        observer.is_err(),
        "a different living task was accepted as the source death watch"
    );
}
