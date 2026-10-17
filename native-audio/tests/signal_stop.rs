//! Real production process, Linux signal, backend child and partial frontend.
//! No PipeWire server, audio effects or hardware readiness are claimed here.
use pokelinux_power::clock::{BootClock, Clock};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use std::fs;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

struct Owner(Child);
impl Drop for Owner {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            let _ = self.0.kill();
        }
        // Retain the actual Child until reaped, including assertion unwinding.
        let _ = self.0.wait();
    }
}
fn now() -> u64 {
    BootClock.now().elapsed().as_nanos().try_into().unwrap()
}
fn readable(fd: &impl std::os::fd::AsFd) -> bool {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .unwrap()
        == 1
}

#[test]
fn sigterm_drains_actual_backend_and_partial_frontend_then_exits_zero() {
    let root = std::env::temp_dir().join(format!("pokelinux-audio-signal-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let socket = root.join("owner.sock");
    let profile = root.join("profile.json");
    fs::write(&profile, br#"{"sources":{"media":"source.media","call":"source.call","capture":"source.microphone"},"routes":[{"id":"vm-speaker","label":"Virtual speaker","target":"route.speaker","roles":["media","call"]},{"id":"vm-capture","label":"Virtual capture","target":"route.capture","roles":["capture"]}]}"#).unwrap();
    let uid = rustix::process::getuid().as_raw();
    let mut owner = Owner(
        Command::new(env!("CARGO_BIN_EXE_pokelinux-audio"))
            .arg(&socket)
            .arg(&profile)
            .arg(uid.checked_add(1).unwrap().to_string())
            .arg(uid.checked_add(2).unwrap().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let pid = owner.0.id();
    let mainfd = pidfd_open(Pid::from_raw(pid as i32).unwrap(), PidfdFlags::NONBLOCK).unwrap();
    let ready_until = now() + 5_000_000_000;
    let backend_pid = loop {
        assert!(now() < ready_until, "original backend readiness cutoff");
        assert!(
            owner.0.try_wait().unwrap().is_none(),
            "normal owner exited before readiness"
        );
        if socket.exists() {
            let children = fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).unwrap();
            if let Some(child) = children.split_whitespace().find(|child| {
                fs::read(format!("/proc/{child}/cmdline")).is_ok_and(|bytes| {
                    bytes.split(|b| *b == 0).nth(1) == Some(b"--backend-worker".as_slice())
                })
            }) {
                break child.parse::<i32>().unwrap();
            }
        }
        sleep(Duration::from_millis(2));
    };
    let backend = pidfd_open(Pid::from_raw(backend_pid).unwrap(), PidfdFlags::NONBLOCK).unwrap();
    assert!(
        !readable(&backend),
        "retain an actual live backend before Stop"
    );
    let mut partial = UnixStream::connect(&socket).unwrap();
    partial.write_all(b"{").unwrap();
    while fs::read_dir(format!("/proc/{pid}/task")).unwrap().count() < 2 {
        assert!(now() < ready_until, "original frontend readiness cutoff");
        sleep(Duration::from_millis(2));
    }
    let stop_until = now() + 5_000_000_000;
    pidfd_send_signal(&mainfd, Signal::TERM).unwrap();
    let status = loop {
        assert!(now() < stop_until, "original signal Stop cutoff");
        if let Some(status) = owner.0.try_wait().unwrap() {
            break status;
        }
        sleep(Duration::from_millis(2));
    };
    println!(
        "ACTUAL_AUDIO_SIGNAL_PARENT_WAIT pid={pid} status={status:?} original_until_ns={stop_until} observed_ns={}",
        now()
    );
    assert_eq!(
        status.code(),
        Some(0),
        "SIGTERM must enter owned Stop/drain: {status:?}"
    );
    assert!(readable(&mainfd) && readable(&backend));
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    assert!(
        !std::path::Path::new(&format!("/proc/{backend_pid}")).exists(),
        "backend must be reaped before normal Main exit"
    );
    drop(partial);
    drop(owner);
    fs::remove_dir_all(root).unwrap();
}
