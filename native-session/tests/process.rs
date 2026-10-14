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
    let (_p, mut o) = start(0);
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

fn await_boottime(target: u64) {
    while boottime_ns() < target {
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn queue_timely_successor(peer: &mut Peer, sampled_at: u64, mode: &str) -> u64 {
    await_boottime(sampled_at + 60_000_000);
    let sent_after = boottime_ns();
    peer.command(mode);
    assert!(
        boottime_ns() < sampled_at + pokelinux_session::SAMPLE_BUDGET_NS,
        "fixture scheduling missed the actual original source lease"
    );
    // The sender samples real BOOTTIME after receiving this command and sends
    // before acknowledging it. No frame time is authored in these positives.
    await_boottime(sampled_at + pokelinux_session::SAMPLE_BUDGET_NS + 10_000_000);
    assert!(
        boottime_ns() < sent_after + pokelinux_session::SAMPLE_BUDGET_NS,
        "actual queued successor itself expired before the test poll"
    );
    sent_after
}

#[test]
fn timely_actual_queued_heartbeat_is_drained_before_cached_expiry() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let initial = observer.snapshot().unwrap().clone();
    let original_until = initial.sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS;
    let successor_after = queue_timely_successor(&mut peer, initial.sampled_at_ns, "heartbeat");
    assert!(boottime_ns() >= original_until);
    observer
        .poll()
        .expect("a real timely fresh queued heartbeat must precede expiry detection");
    let current = observer.snapshot().unwrap();
    assert_eq!(current.sequence, 2);
    assert!(current.sampled_at_ns >= successor_after);
    assert!(current.sampled_at_ns < original_until);
    assert_eq!(observer.revision(), 1);
    // A new source sample never renews a deadline already captured by old work.
    assert!(boottime_ns() >= original_until);
}

#[test]
fn timely_actual_queued_lock_cycle_preserves_every_semantic_revision() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let initial = observer.snapshot().unwrap().clone();
    queue_timely_successor(&mut peer, initial.sampled_at_ns, "cycle");
    observer
        .poll()
        .expect("timely fresh queued lock/unlock must be examined before cache expiry");
    let current = observer.snapshot().unwrap();
    assert_eq!(current.sequence, 3);
    assert!(!current.locked);
    assert_eq!(current.focus, initial.focus);
    assert_eq!(observer.revision(), 3);
}

#[test]
fn genuine_actual_successor_sampling_gap_remains_terminal() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let until = observer.snapshot().unwrap().sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS;
    await_boottime(until + 10_000_000);
    peer.command("heartbeat");
    assert!(
        observer.poll().is_err(),
        "a real sampling gap revived the source"
    );
    assert!(observer.snapshot().is_none());
}

#[test]
fn expired_snapshot_observation_cannot_later_admit_queued_successor() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let sampled_at = observer.snapshot().unwrap().sampled_at_ns;
    queue_timely_successor(&mut peer, sampled_at, "heartbeat");
    assert!(observer.snapshot().is_none());
    assert!(
        observer.poll().is_err(),
        "an observed expired established source was implicitly restored"
    );
}

#[test]
fn backwards_intermediate_actual_sample_cannot_hide_in_fresh_final_frame() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let sampled_at = observer.snapshot().unwrap().sampled_at_ns;
    peer.command(&format!("backward-cycle:{}", sampled_at - 1));
    assert!(
        observer.poll().is_err(),
        "backwards intermediate sample coalesced away"
    );
    assert!(observer.snapshot().is_none());
}

#[test]
fn expired_intermediate_actual_packet_is_not_skipped_for_fresh_final_frame() {
    let (mut peer, mut observer) = start(0);
    peer.command("expired-then-fresh");
    assert!(observer.poll().is_err());
    assert!(observer.snapshot().is_none());
}

#[test]
fn previously_returned_timeout_stays_terminal_after_actual_fresh_packet() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let until = observer.snapshot().unwrap().sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS;
    await_boottime(until + 10_000_000);
    assert!(observer.poll().is_err());
    peer.command("heartbeat");
    assert!(observer.poll().is_err());
    assert!(observer.snapshot().is_none());
}

#[test]
fn exactly_contiguous_actual_wire_leases_do_not_extend_old_deadline() {
    let (mut peer, mut observer) = start(0);
    peer.command("valid");
    observer.poll().unwrap();
    let until = observer.snapshot().unwrap().sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS;
    await_boottime(until);
    // An authored exact-boundary field travels through the real credential/FD
    // interface after BOOTTIME reaches it. This is a wire boundary test, not an
    // original-compositor sampling or scheduling claim.
    peer.command(&format!("timestamp-heartbeat:{until}"));
    observer
        .poll()
        .expect("contiguous half-open leases must be accepted");
    assert_eq!(observer.snapshot().unwrap().sampled_at_ns, until);
    assert!(boottime_ns() >= until);
}

#[test]
fn actual_mapped_locker_is_admitted_with_its_owned_kernel_handle() {
    let (mut p, mut o) = start(0);
    p.command("owned-locker");
    o.poll()
        .expect("fresh mapped locker with an actual live matching pidfd must be admitted");
    let frame = serde_json::to_value(o.snapshot().unwrap()).unwrap();
    assert_eq!(frame["locker"]["state"], "mapped");
    assert_eq!(frame["locker"]["pid"], p.child.id());
    assert!(frame["focus"].is_null());
    assert_eq!(frame["locked"], true);
}
#[test]
fn actual_locker_shape_and_handle_contradictions_are_terminal() {
    for mode in [
        "owned-locker-missing",
        "owned-locker-two",
        "owned-locker-file",
        "owned-locker-wrong-pid",
        "owned-locker-zero-pid",
        "owned-locker-unlocked",
        "owned-locker-and-focus",
        "owned-locker-unknown-field",
        "owned-locker-null",
        "owned-locker-unavailable-rights",
        "owned-locker-legacy-wire",
    ] {
        invalid_packet(mode);
    }
}

#[test]
fn owned_locker_heartbeats_retain_one_cloexec_handle_and_unavailable_releases_it() {
    use pokelinux_lifecycle::session::Provider;
    let (mut p, mut o) = start(0);
    p.command("owned-locker");
    let snapshot = Provider::poll(&mut o).unwrap().unwrap();
    let locker = snapshot.locker.as_ref().unwrap();
    assert_eq!(locker.metadata.pid, p.child.id());
    assert!(
        pokelinux_kernel_identity::ProcessView::open()
            .unwrap()
            .matches(locker.pidfd(), locker.pidfd(), p.child.id())
            .unwrap()
    );
    assert!(
        rustix::io::fcntl_getfd(locker.pidfd())
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    let mut inspector = Command::new(env!("CARGO_BIN_EXE_session-peer-fixture"))
        .args([
            "--expect-source-fd-closed",
            &locker.pidfd().as_raw_fd().to_string(),
            &p.child.id().to_string(),
        ])
        .spawn()
        .unwrap();
    assert!(inspector.wait().unwrap().success());
    drop(snapshot);
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..100 {
        p.command("owned-locker-heartbeat");
        let current = Provider::poll(&mut o).unwrap().unwrap();
        assert!(current.focus.is_none());
        assert_eq!(current.locker.as_ref().unwrap().metadata.pid, p.child.id());
        assert_eq!(current.revision, 1);
    }
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
    p.command("owned-locker-unavailable");
    p.command("owned-locker");
    let current = Provider::poll(&mut o).unwrap().unwrap();
    assert_eq!(
        current.revision, 3,
        "queued owner disappearance cannot be coalesced away"
    );
    drop(current);
    p.command("owned-locker-unavailable");
    let current = Provider::poll(&mut o).unwrap().unwrap();
    assert!(current.locker.is_none());
    assert_eq!(current.revision, 4);
    assert_eq!(
        std::fs::read_dir("/proc/self/fd").unwrap().count() + 1,
        before
    );
    Provider::stop(&mut o);
    assert!(o.snapshot().is_none());
    assert!(Provider::poll(&mut o).is_err());
}
#[test]
fn dead_locker_packet_cannot_keep_or_reactivate_owned_locker_state() {
    use pokelinux_lifecycle::session::Provider;
    let (mut p, mut o) = start(0);
    p.command("locker-child-live");
    assert!(Provider::poll(&mut o).unwrap().unwrap().locker.is_some());
    p.command("locker-child-dead");
    assert!(Provider::poll(&mut o).is_err());
    assert!(o.snapshot().is_none());
    p.command("owned-locker");
    assert!(Provider::poll(&mut o).is_err());
    assert!(o.snapshot().is_none());
}
