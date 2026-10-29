use pokelinux_lifecycle::{
    authority::living,
    boottime_ns,
    session::{Provider, Update},
};
use pokelinux_session::Observer;
use pokelinux_session_owner::{Compositor, SourceOwner};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, SocketFlags, SocketType},
    process::{Pid, PidfdFlags},
};
use std::{
    collections::BTreeMap,
    io,
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
fn fixture() -> PathBuf {
    PathBuf::from(
        std::env::var_os("SOURCE_OWNER_FIXTURE").expect("isolated root-installed authored fixture"),
    )
}
fn spec(mode: &str) -> Compositor {
    Compositor {
        helper: fixture(),
        executable: fixture(),
        arguments: vec![mode.into()],
        environment: BTreeMap::new(),
        source: SOURCE.into(),
    }
}
fn tick() {
    std::thread::sleep(Duration::from_millis(1));
}
struct Guard {
    owner: SourceOwner,
    until: u64,
}
impl Guard {
    fn start(mode: &str, initialization_ns: u64) -> Self {
        let start = boottime_ns();
        Self {
            owner: SourceOwner::spawn(spec(mode), start + initialization_ns)
                .expect("actual owned compositor Source must start"),
            until: start + 3_000_000_000,
        }
    }
    fn sample(&mut self) {
        loop {
            match self.owner.poll() {
                Ok(Update::Current(s)) => {
                    assert_eq!(s.source, SOURCE);
                    break;
                }
                Ok(Update::AwaitingFirst) if boottime_ns() < self.until => tick(),
                other => panic!("actual Source did not produce sample: {other:?}"),
            }
        }
    }
    fn drain(&mut self) {
        self.owner.stop();
        while self.owner.occupied() {
            let _ = self.owner.poll();
            assert!(boottime_ns() < self.until, "original actual-reap cutoff");
            tick();
        }
        assert!(
            self.owner.exit_status().is_some(),
            "actual try_wait status required"
        );
    }
}
impl Deref for Guard {
    type Target = SourceOwner;
    fn deref(&self) -> &SourceOwner {
        &self.owner
    }
}
impl DerefMut for Guard {
    fn deref_mut(&mut self) -> &mut SourceOwner {
        &mut self.owner
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.owner.stop();
        while self.owner.occupied() {
            let _ = self.owner.poll();
            if boottime_ns() >= self.until {
                std::process::abort();
            }
            tick();
        }
    }
}
struct Peer {
    child: Child,
    until: u64,
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        loop {
            match self.child.try_wait() {
                Ok(Some(s)) => {
                    eprintln!(
                        "ACTUAL_AUTHORED_SOURCE_WAIT pid={} status={s} stdout=DEVNULL stderr=DEVNULL",
                        self.child.id()
                    );
                    break;
                }
                Ok(None) if boottime_ns() < self.until => tick(),
                _ => std::process::abort(),
            }
        }
    }
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn expired_deadline_refuses_before_spawn() {
    assert_eq!(
        SourceOwner::spawn(spec("valid"), boottime_ns())
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn actual_authored_uid1000_source_observer_control() {
    let start = boottime_ns();
    let until = start + 2_000_000_000;
    let (receive, send) = net::socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&receive, true).unwrap();
    let child = Command::new(fixture())
        .arg("valid")
        .stdin(Stdio::from(send))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let peer = Peer { child, until };
    let pid = Pid::from_raw(peer.child.id() as i32).unwrap();
    let pidfd = rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK).unwrap();
    let mut observer = Observer::new(receive, pidfd, pid, 1000, SOURCE.into()).unwrap();
    loop {
        match observer.poll() {
            Ok(()) if observer.snapshot().is_some() => break,
            Ok(()) if boottime_ns() < until => tick(),
            other => panic!("actual authored Source setup: {other:?}"),
        }
    }
    assert_eq!(observer.snapshot().unwrap().source, SOURCE);
    drop(peer);
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn valid_source_stop_retains_until_actual_wait() {
    let mut owner = Guard::start("valid", 1_000_000_000);
    owner.sample();
    let alive = owner.duplicate_pidfd().unwrap();
    assert!(living(&alive).unwrap());
    owner.stop();
    owner.stop();
    assert!(
        owner.occupied(),
        "unwaited child custody is retained even after KILL"
    );
    assert!(owner.exit_status().is_none());
    owner.drain();
    assert!(!living(&alive).unwrap());
    assert!(owner.poll().is_err());
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn no_first_sample_expires_original_deadline() {
    let start = boottime_ns();
    let original = start + 100_000_000;
    let mut owner = Guard {
        owner: SourceOwner::spawn(spec("quiet"), original).expect("actual quiet child must start"),
        until: start + 3_000_000_000,
    };
    loop {
        match owner.poll() {
            Ok(Update::AwaitingFirst) if boottime_ns() < original + 100_000_000 => tick(),
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::TimedOut);
                break;
            }
            other => panic!("no first Source sample: {other:?}"),
        }
    }
    assert!(boottime_ns() >= original);
    assert!(owner.occupied());
    owner.drain();
    assert!(owner.poll().is_err());
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn source_unload_is_sticky_while_child_lives() {
    let mut owner = Guard::start("unload", 1_000_000_000);
    owner.sample();
    let alive: OwnedFd = owner.duplicate_pidfd().unwrap();
    loop {
        match owner.poll() {
            Err(_) => break,
            Ok(_) if boottime_ns() < owner.until => tick(),
            other => panic!("actual source unload: {other:?}"),
        }
    }
    assert!(
        living(&alive).unwrap(),
        "Source closes before the same child exits"
    );
    assert!(owner.occupied());
    assert!(owner.poll().is_err());
    owner.drain();
    assert!(!living(&alive).unwrap());
}
#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn wrong_bootstrap_helper_retains_child_until_wait() {
    let start = boottime_ns();
    let mut input = spec("valid");
    input.helper = "/usr/bin/true".into();
    let mut owner = Guard {
        owner: SourceOwner::spawn(input, start + 1_000_000_000)
            .expect("post-spawn errors retain child owner"),
        until: start + 3_000_000_000,
    };
    assert!(owner.occupied());
    let pid = owner.pid();
    assert!(pid.as_raw_nonzero().get() > 0);
    loop {
        match owner.poll() {
            Err(_) => break,
            Ok(_) if boottime_ns() < owner.until => tick(),
            other => panic!("wrong helper not terminal: {other:?}"),
        }
    }
    owner.drain();
    assert!(owner.exit_status().is_some());
    assert!(owner.poll().is_err());
}

#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn spawn_accepts_original_startup_window_over_five_seconds() {
    let start = boottime_ns();
    let mut owner = Guard::start("valid", 6_000_000_000);
    assert!(
        boottime_ns() - start < 1_000_000_000,
        "spawn must return promptly without waiting for readiness"
    );
    assert!(owner.occupied());
    assert!(owner.exit_status().is_none());
    let alive = owner.duplicate_pidfd().unwrap();
    owner.sample();
    owner.drain();
    assert!(!living(&alive).unwrap());
}

#[test]
#[ignore = "requires isolated root and UID transition capabilities"]
fn timing_loss_and_resume_retain_same_child_until_explicit_stop_and_wait() {
    let mut owner = Guard::start("valid", 1_000_000_000);
    owner.sample();
    let original_pid = owner.pid();
    let alive = owner.duplicate_pidfd().unwrap();
    rustix::process::pidfd_send_signal(&alive, rustix::process::Signal::STOP).unwrap();
    loop {
        match owner.poll() {
            Ok(Update::AuthorityLost(_)) => break,
            Ok(Update::Current(_) | Update::Recovering) if boottime_ns() < owner.until => tick(),
            other => panic!("actual timing loss did not remain nonterminal: {other:?}"),
        }
    }
    assert_eq!(owner.pid(), original_pid);
    assert!(living(&alive).unwrap());
    assert!(owner.occupied());
    assert!(owner.exit_status().is_none());
    rustix::process::pidfd_send_signal(&alive, rustix::process::Signal::CONT).unwrap();
    loop {
        match owner.poll() {
            Ok(Update::Current(_)) => break,
            Ok(Update::Recovering) if boottime_ns() < owner.until => tick(),
            other => panic!("same live child did not resume Source facts: {other:?}"),
        }
    }
    assert_eq!(owner.pid(), original_pid);
    assert!(living(&alive).unwrap());
    assert!(owner.occupied());
    assert!(owner.exit_status().is_none());
    owner.stop();
    assert!(owner.occupied(), "explicit Stop released unwaited custody");
    assert!(owner.exit_status().is_none());
    owner.drain();
    assert!(!living(&alive).unwrap());
    assert!(!owner.occupied());
    assert!(owner.exit_status().is_some());
    assert!(owner.poll().is_err());
}
