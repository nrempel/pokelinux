//! The exact constructor used by runtime::run, not the full session actor.
//! Real normal brightness Main/worker execution touches only SyntheticTree files.
use super::selected_brightness;
use crate::config::Brightness;
use pokelinux_lifecycle::{
    boottime_ns,
    brightness::{Event, Intent, Owner, Provider},
};
use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType},
    process::{self, Pid, Signal, WaitOptions},
};
use std::{
    fs,
    io::{self, Read},
    os::{fd::AsFd, unix::fs::PermissionsExt},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};
const SECOND: u64 = 1_000_000_000;
fn within(until: u64, mut condition: impl FnMut() -> bool) {
    loop {
        assert!(
            boottime_ns() < until,
            "original runtime constructor test cutoff"
        );
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "runtime-brightness-{}-{}",
            std::process::id(),
            boottime_ns()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for name in ["class", "class/panel", "runtime"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        for (name, bytes) in [
            ("max_brightness", b"400\n"),
            ("brightness", b"200\n"),
            ("actual_brightness", b"120\n"),
        ] {
            fs::write(root.join("class/panel").join(name), bytes).unwrap();
        }
        Self(root)
    }
    fn socket(&self) -> PathBuf {
        self.0.join("runtime/owner.sock")
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Main {
    child: Child,
    status: Option<ExitStatus>,
    out_eof: bool,
    err_eof: bool,
    out_bytes: usize,
    err_bytes: usize,
    close: u64,
    reported: bool,
}
impl Main {
    fn spawn(tree: &Tree, close: u64) -> Self {
        let path = std::env::var_os("NATIVE_BRIGHTNESS_SERVICE")
            .expect("qualified normal service ELF must be explicitly selected");
        let mut child = Command::new(path)
            .arg("--serve")
            .arg(tree.socket())
            .arg("synthetic-tree")
            .arg(tree.0.join("class"))
            .arg("panel")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        for fd in [
            child.stdout.as_mut().unwrap().as_fd(),
            child.stderr.as_mut().unwrap().as_fd(),
        ] {
            fcntl_setfl(fd, fcntl_getfl(fd).unwrap() | OFlags::NONBLOCK).unwrap();
        }
        Self {
            child,
            status: None,
            out_eof: false,
            err_eof: false,
            out_bytes: 0,
            err_bytes: 0,
            close,
            reported: false,
        }
    }
    fn poll(&mut self) -> bool {
        for (stream, eof, count) in [
            (
                self.child.stdout.as_mut().unwrap() as &mut dyn Read,
                &mut self.out_eof,
                &mut self.out_bytes,
            ),
            (
                self.child.stderr.as_mut().unwrap() as &mut dyn Read,
                &mut self.err_eof,
                &mut self.err_bytes,
            ),
        ] {
            for _ in 0..8 {
                let mut bytes = [0; 1024];
                match stream.read(&mut bytes) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        *count += n;
                        assert!(*count <= 8192, "bounded Main output");
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        break;
                    }
                    Err(e) => panic!("actual Main PIPE read {e}"),
                }
            }
        }
        if self.status.is_none() {
            self.status = self.child.try_wait().unwrap();
        }
        self.status.is_some() && self.out_eof && self.err_eof
    }
    fn finish(&mut self, until: u64, cleanup: bool) {
        within(until.min(self.close), || self.poll());
        if !self.reported {
            assert!(matches!(
                process::waitpid(Pid::from_raw(self.child.id() as i32), WaitOptions::NOHANG),
                Err(rustix::io::Errno::CHILD)
            ));
            println!(
                "ACTUAL_RUNTIME_SELECTED_BRIGHTNESS_MAIN_WAIT pid={} exit={:?} stdout_EOF={} stderr_EOF={} ECHILD=true cleanup={} original_close_ns={} observed_ns={}",
                self.child.id(),
                self.status.unwrap().code(),
                self.out_eof,
                self.err_eof,
                cleanup,
                self.close,
                boottime_ns()
            );
            self.reported = true;
        }
    }
    fn stop(&mut self, until: u64) {
        process::kill_process(Pid::from_raw(self.child.id() as i32).unwrap(), Signal::TERM)
            .unwrap();
        self.finish(until, false);
        assert_eq!(self.status.unwrap().code(), Some(0));
    }
}
impl Drop for Main {
    fn drop(&mut self) {
        if self.status.is_none() {
            let _ =
                process::kill_process(Pid::from_raw(self.child.id() as i32).unwrap(), Signal::TERM);
        }
        // Cleanup has its original separate bound; no functional deadline is renewed.
        while !self.poll() {
            if boottime_ns() >= self.close {
                std::process::abort();
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.finish(self.close, true);
    }
}
fn observe(owner: &mut Owner, main: &mut Main, until: u64) {
    let mut waiting = false;
    within(until, || {
        assert!(
            main.status.is_none(),
            "normal Main exited during typed Inspect"
        );
        let _ = main.poll();
        if !waiting {
            waiting = owner.inspect(until).unwrap();
        }
        match owner.poll() {
            Some(Event::Observed) => waiting = false,
            None => {}
            _ => panic!("readonly Inspect failed"),
        }
        owner.state().is_current(boottime_ns()) && !owner.state().pending
    });
}
#[test]
#[ignore = "explicit root-only normal service execution; selected ELF required"]
fn explicit_disabled_constructor_has_no_provider() {
    assert!(
        selected_brightness(&Brightness::Disabled)
            .unwrap()
            .is_none()
    );
    println!("PASS explicit Disabled constructor control");
}
#[test]
#[ignore = "explicit root-only normal service execution; selected ELF required"]
fn explicit_owner_constructor_uses_real_service_inspect_set_and_readback() {
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let start = boottime_ns();
    let work = start + 8 * SECOND;
    let close = start + 10 * SECOND;
    let tree = Tree::new();
    let selected = selected_brightness(&Brightness::Owner {
        socket: tree.socket(),
    })
    .unwrap();
    assert!(
        selected.is_some(),
        "Explicit Owner must construct the production brightness provider"
    );
    let mut owner = selected.unwrap();
    assert!(!owner.state().connected && !owner.state().available);
    let mut main = Main::spawn(&tree, close);
    within(work.min(boottime_ns() + SECOND), || {
        assert!(!main.poll(), "Main exited before listener");
        tree.socket().exists()
    });
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    net::connect(&fd, &SocketAddrUnix::new(tree.socket()).unwrap()).unwrap();
    let peer = net::sockopt::socket_peercred(&fd).unwrap();
    assert_eq!(
        (
            peer.pid.as_raw_pid() as u32,
            peer.uid.as_raw(),
            peer.gid.as_raw()
        ),
        (main.child.id(), 0, 0)
    );
    drop(fd);
    observe(&mut owner, &mut main, work.min(boottime_ns() + SECOND));
    let before = owner.state();
    assert_eq!(
        (
            before.requested_percent,
            before.observed_percent,
            before.actual_percent
        ),
        (None, Some(50), Some(30))
    );
    let intent = Intent {
        id: 19,
        percent: 75,
        expected_revision: before.revision,
        until_ns: work.min(boottime_ns() + SECOND),
    };
    assert!(
        owner.submit(&intent).unwrap(),
        "one fresh explicit Set must be admitted"
    );
    let mut completed = None;
    within(intent.until_ns, || {
        let _ = main.poll();
        match owner.poll() {
            Some(Event::Completed { id, state }) => {
                assert_eq!(id, 19);
                completed = Some(state);
                true
            }
            None => false,
            _ => panic!("explicit Set failed"),
        }
    });
    let state = completed.unwrap();
    assert!(state.is_current(boottime_ns()) && !state.pending && state.revision > before.revision);
    assert_eq!(
        (
            state.requested_percent,
            state.observed_percent,
            state.actual_percent
        ),
        (Some(75), Some(75), Some(30))
    );
    assert_eq!(
        fs::read(tree.0.join("class/panel/brightness")).unwrap(),
        b"300\n"
    );
    assert_eq!(
        fs::read(tree.0.join("class/panel/actual_brightness")).unwrap(),
        b"120\n"
    );
    owner.revoke();
    main.stop(work.min(boottime_ns() + 2 * SECOND));
    println!(
        "PASS runtime selected constructor -> normal Main -> typed Inspect/oneSet/readback; SyntheticTree only, not full actor/Source/UI/hardware"
    );
}
