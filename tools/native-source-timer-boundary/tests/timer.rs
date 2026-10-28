#![forbid(unsafe_code)]
use pokelinux_lifecycle::{
    authority::living,
    boottime_ns, ipc,
    session::{Provider, Snapshot},
};
use pokelinux_session_owner::{Compositor, SourceOwner};
use rustix::{
    fd::OwnedFd,
    net::{self, SocketFlags},
};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
static NEXT: AtomicU32 = AtomicU32::new(0);
fn tick() {
    std::thread::sleep(Duration::from_millis(1));
}
struct Case {
    owner: SourceOwner,
    control: Option<OwnedFd>,
    listener: OwnedFd,
    root: PathBuf,
    until: u64,
}
impl Case {
    fn new() -> Self {
        assert_eq!(
            rustix::process::getuid().as_raw(),
            0,
            "isolated root required"
        );
        let root = PathBuf::from(
            std::env::var_os("SOURCE_TIMER_BOUNDARY_ROOT").expect("protected root test parent"),
        )
        .join(format!(
            "case-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let path = root.join("control.sock");
        let listener = ipc::listener(&path, 0o666).unwrap();
        let start = boottime_ns();
        let owner = SourceOwner::spawn(
            Compositor {
                helper: PathBuf::from(
                    std::env::var_os("SOURCE_TIMER_BOUNDARY_OWNER")
                        .expect("unchanged public default owner"),
                ),
                executable: PathBuf::from(
                    std::env::var_os("SOURCE_TIMER_BOUNDARY_FIXTURE")
                        .expect("native Wayland fixture"),
                ),
                arguments: vec!["--control".into(), path.to_str().unwrap().into()],
                environment: BTreeMap::new(),
                source: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into(),
            },
            start + 5_000_000_000,
        )
        .expect("actual retained Source bootstrap");
        // Own before any fallible post-spawn setup or assertion.
        let mut case = Self {
            owner,
            control: None,
            listener,
            root,
            until: start + 8_000_000_000,
        };
        loop {
            match net::accept_with(&case.listener, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
                Ok(fd) => {
                    let actual = net::sockopt::socket_peercred(&fd).unwrap();
                    assert_eq!(actual.pid, case.owner.pid());
                    assert_eq!(actual.uid.as_raw(), 1000);
                    assert_eq!(actual.gid.as_raw(), 1000);
                    case.control = Some(fd);
                    break;
                }
                Err(rustix::io::Errno::AGAIN) if boottime_ns() < case.until => tick(),
                other => panic!("actual fixture connect: {other:?}"),
            }
        }
        case.message("ready");
        case
    }
    fn message(&mut self, expected: &str) {
        loop {
            assert!(boottime_ns() < self.until, "original control cutoff");
            match ipc::receive(self.control.as_ref().unwrap()) {
                Ok(Some((value, cred))) => {
                    assert_eq!(value, expected.as_bytes());
                    assert_eq!(cred.pid, self.owner.pid());
                    assert_eq!(cred.uid.as_raw(), 1000);
                    assert_eq!(cred.gid.as_raw(), 1000);
                    assert!(boottime_ns() < self.until);
                    return;
                }
                Ok(None) => tick(),
                other => panic!("actual control: {other:?}"),
            }
        }
    }
    fn dispatch(&mut self, number: u64) {
        ipc::send(self.control.as_ref().unwrap(), b"dispatch").unwrap();
        self.message(&format!("tick:{number}"));
    }
    fn sample_after(&mut self, previous: u64) -> Snapshot {
        loop {
            assert!(boottime_ns() < self.until);
            match self.owner.poll() {
                Ok(Some(s)) if s.sequence > previous => return s,
                Ok(_) => tick(),
                other => panic!("actual Rust Observer source: {other:?}"),
            }
        }
    }
    fn stop_and_wait(&mut self) {
        self.owner.stop();
        while self.owner.occupied() {
            assert!(boottime_ns() < self.until, "original child wait cutoff");
            let _ = self.owner.poll();
            assert!(boottime_ns() < self.until);
            if self.owner.occupied() {
                tick();
            }
        }
        assert!(self.owner.exit_status().is_some());
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        self.owner.stop();
        while self.owner.occupied() {
            let _ = self.owner.poll();
            if boottime_ns() >= self.until {
                std::process::abort();
            }
            tick();
        }
        eprintln!(
            "ACTUAL_TIMER_SOURCE_WAIT pid={} status={:?} waited_at_boottime_ns={} original_until_ns={} stdout=DEVNULL stderr=DEVNULL",
            self.owner.pid().as_raw_pid(),
            self.owner.exit_status(),
            boottime_ns(),
            self.until
        );
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
#[ignore = "requires isolated actual ARM kernel, root, production bootstrap and native libwayland fixture"]
fn publication_waits_for_two_real_timer_dispatches_then_fresh_sequence_and_cadence() {
    let mut case = Case::new();
    let silence_until = boottime_ns() + 150_000_000;
    while boottime_ns() < silence_until {
        match case.owner.poll() {
            Ok(None) => {}
            Ok(Some(value)) => panic!(
                "EARLY_SOURCE_PUBLICATION sequence={} sampled_at_ns={} before_first_timer_dispatch=true",
                value.sequence, value.sampled_at_ns
            ),
            Err(error) => panic!("SOURCE_STARTUP_SETUP_FAILURE {error:?}"),
        }
        tick();
    }
    assert!(case.owner.occupied());
    assert!(case.owner.exit_status().is_none());
    case.dispatch(1);
    assert!(case.owner.poll().unwrap().is_none());
    case.dispatch(2);
    let first = case.sample_after(0);
    assert_eq!(first.sequence, 2);
    assert!(
        first.locked,
        "state-observing hook remains active before dispatch"
    );
    assert!(boottime_ns() < first.until);
    case.dispatch(3);
    let second = case.sample_after(first.sequence);
    assert_eq!(second.sequence, 3);
    assert!(second.sampled_at_ns > first.sampled_at_ns);
    eprintln!(
        "ACTUAL_TIMER_CADENCE first_sequence={} second_sequence={} sampled_at_gap_ns={} configured_timer_ms=25",
        first.sequence,
        second.sequence,
        second.sampled_at_ns - first.sampled_at_ns
    );
    assert!(boottime_ns() < second.until);
    case.stop_and_wait();
}
#[test]
#[ignore = "requires isolated actual ARM kernel, root, production bootstrap and native libwayland fixture"]
fn activated_source_expiry_stays_terminal_after_resumed_real_timer_dispatch() {
    let mut case = Case::new();
    case.dispatch(1);
    assert!(case.owner.poll().unwrap().is_none());
    case.dispatch(2);
    let last = case.sample_after(0);
    assert_eq!(last.sequence, 2);
    let alive = case.owner.duplicate_pidfd().unwrap();
    loop {
        assert!(boottime_ns() < case.until);
        match case.owner.poll() {
            Err(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
                break;
            }
            Ok(Some(s)) => {
                assert_eq!(s.sequence, last.sequence);
                tick();
            }
            Ok(None) => panic!("established Source lost sample without terminal refusal"),
        }
    }
    assert!(boottime_ns() >= last.until);
    assert!(living(&alive).unwrap());
    assert!(case.owner.occupied());
    case.dispatch(3);
    assert_eq!(
        case.owner.poll().unwrap_err().kind(),
        std::io::ErrorKind::TimedOut,
        "fresh timer packet must never revive expired Source"
    );
    assert!(living(&alive).unwrap());
    case.stop_and_wait();
    assert!(!living(&alive).unwrap());
}
