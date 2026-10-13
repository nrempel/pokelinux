#![cfg(target_os = "linux")]
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use std::{
    io::{Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::net::{UnixListener, UnixStream},
    },
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

static KERNEL_TEST: Mutex<()> = Mutex::new(());

struct Peer {
    child: Child,
    socket: UnixStream,
    path: std::path::PathBuf,
}
impl Peer {
    fn spawn(tag: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("pidfd-identity-{}-{tag}.sock", std::process::id()));
        assert!(!path.exists());
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "pidfd_peer_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("PIDFD_IDENTITY_SOCKET", &path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(3);
        let socket = loop {
            if let Ok(socket) = UnixStream::connect(&path) {
                break socket;
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "peer exited before connection"
            );
            assert!(Instant::now() < until, "peer did not create listener");
            std::thread::sleep(Duration::from_millis(2));
        };
        assert_eq!(
            rustix::net::sockopt::socket_peercred(&socket)
                .unwrap()
                .pid
                .as_raw_nonzero()
                .get() as u32,
            child.id()
        );
        Self {
            child,
            socket,
            path,
        }
    }
    fn handle(&self) -> OwnedFd {
        socket_peer_pidfd(&self.socket).unwrap()
    }
    fn request_exit(&mut self) {
        self.socket.write_all(&[9]).unwrap();
    }
    fn reap(&mut self) {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(Instant::now() < until, "owned peer did not exit");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.path);
    }
}
fn exited(fd: &OwnedFd) -> bool {
    let mut poll = [rustix::event::PollFd::new(fd, rustix::event::PollFlags::IN)];
    rustix::event::poll(
        &mut poll,
        Some(&rustix::event::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .unwrap()
        != 0
}
fn assert_denied(result: std::io::Result<bool>) {
    assert!(!result.unwrap_or(false));
}

#[test]
#[ignore = "owned subprocess entrypoint"]
fn pidfd_peer_child() {
    let listener = UnixListener::bind(std::env::var_os("PIDFD_IDENTITY_SOCKET").unwrap()).unwrap();
    let (mut socket, _) = listener.accept().unwrap();
    let mut byte = [0];
    socket.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [9]);
}

#[test]
fn same_live_socket_peer_handles_match_without_scalar_inode_identity() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut peer = Peer::spawn("same");
    let (left, right) = (peer.handle(), peer.handle());
    assert!(
        ProcessView::open()
            .unwrap()
            .matches(&left, &right, peer.child.id())
            .unwrap()
    );
    assert!(
        rustix::io::fcntl_getfd(&left)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    assert!(
        rustix::io::fcntl_getfd(&right)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    peer.request_exit();
    peer.reap();
}
#[test]
fn different_live_owned_peers_and_wrong_expected_pid_are_denied() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut a = Peer::spawn("different-a");
    let mut b = Peer::spawn("different-b");
    let (left, right) = (a.handle(), b.handle());
    let lstat = rustix::fs::fstat(&left).unwrap();
    let rstat = rustix::fs::fstat(&right).unwrap();
    let filesystem = rustix::fs::fstatfs(&left).unwrap().f_type;
    let scalar_alias = (lstat.st_dev, lstat.st_ino) == (rstat.st_dev, rstat.st_ino);
    eprintln!(
        "different live peers: pidfd filesystem={filesystem:#x}, device/inode alias={scalar_alias}"
    );
    if filesystem == 0x09041934 {
        assert!(
            scalar_alias,
            "qualified anon-inode boundary must expose scalar alias"
        );
    }
    let view = ProcessView::open().unwrap();
    assert_denied(view.matches(&left, &right, a.child.id()));
    assert_denied(view.matches(&left, &left, b.child.id()));
    a.request_exit();
    a.reap();
    b.request_exit();
    b.reap();
}
#[test]
fn exited_unreaped_owned_peer_is_denied_even_with_positive_fdinfo_pid() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut peer = Peer::spawn("exited");
    let (left, right) = (peer.handle(), peer.handle());
    peer.request_exit();
    let until = Instant::now() + Duration::from_secs(3);
    while !exited(&left) {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let fdinfo =
        std::fs::read_to_string(format!("/proc/self/fdinfo/{}", left.as_raw_fd())).unwrap();
    assert!(
        fdinfo
            .lines()
            .any(|line| line == format!("Pid:\t{}", peer.child.id()))
    );
    assert_denied(
        ProcessView::open()
            .unwrap()
            .matches(&left, &right, peer.child.id()),
    );
    assert!(ProcessView::open().unwrap().is_pidfd(&left).unwrap());
    peer.reap();
}
#[test]
fn reaped_owned_peer_is_denied_and_retained_handle_does_not_retarget() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut peer = Peer::spawn("reaped");
    let (left, right) = (peer.handle(), peer.handle());
    let original_pid = peer.child.id();
    peer.request_exit();
    peer.reap();
    assert!(ProcessView::open().unwrap().is_pidfd(&left).unwrap());
    let fdinfo =
        std::fs::read_to_string(format!("/proc/self/fdinfo/{}", left.as_raw_fd())).unwrap();
    assert!(fdinfo.lines().any(|line| line == "Pid:\t-1"));
    let mut next = Peer::spawn("replacement-child");
    let next_handle = next.handle();
    let view = ProcessView::open().unwrap();
    assert_denied(view.matches(&left, &right, original_pid));
    assert_denied(view.matches(&left, &next_handle, next.child.id()));
    next.request_exit();
    next.reap();
}
#[test]
fn arbitrary_descriptors_and_out_of_range_expected_pids_are_denied_without_leaks() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let null: OwnedFd = std::fs::File::open("/dev/null").unwrap().into();
    let mut peer = Peer::spawn("typed");
    let handle = peer.handle();
    let view = ProcessView::open().unwrap();
    assert!(
        !view.is_pidfd(&null).unwrap(),
        "non-pidfd was accepted by type-only boundary"
    );
    assert!(view.is_pidfd(&handle).unwrap());
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..16 {
        assert_denied(view.matches(&null, &handle, peer.child.id()));
        assert_denied(view.matches(&handle, &null, peer.child.id()));
        for pid in [0, i32::MAX as u32 + 1, u32::MAX] {
            assert_denied(view.matches(&handle, &handle, pid));
        }
    }
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
    assert!(rustix::io::fcntl_getfd(&null).is_ok());
    assert!(rustix::io::fcntl_getfd(&handle).is_ok());
    peer.request_exit();
    peer.reap();
}
#[test]
fn unrelated_fd_slot_replacement_cannot_retarget_held_owned_pidfds() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut a = Peer::spawn("slot-a");
    let mut b = Peer::spawn("slot-b");
    let (left, right, other) = (a.handle(), a.handle(), b.handle());
    let scratch = rustix::io::fcntl_dupfd_cloexec(&left, 128).unwrap();
    let number = scratch.as_raw_fd();
    drop(scratch);
    let replacement = rustix::io::fcntl_dupfd_cloexec(&other, number).unwrap();
    assert_eq!(replacement.as_raw_fd(), number);
    let view = ProcessView::open().unwrap();
    assert!(view.matches(&left, &right, a.child.id()).unwrap());
    assert_denied(view.matches(&left, &replacement, a.child.id()));
    a.request_exit();
    a.reap();
    b.request_exit();
    b.reap();
}
#[test]
#[ignore = "isolated inherited proc view/fork entrypoint"]
fn pidfd_inherited_view_child() {
    let view = ProcessView::open().unwrap();
    // A real socketpair records this process, so this obtains its exact pidfd.
    let (a, _b) = UnixStream::pair().unwrap();
    let owned = socket_peer_pidfd(&a).unwrap();
    let expected = std::process::id();
    // SAFETY: the child only takes the early owner mismatch branch and immediately
    // _exit's. It does not use inherited locks or unwind/run Rust destructors.
    let child = unsafe { libc::fork() };
    assert!(child >= 0);
    if child == 0 {
        let rejected =
            view.matches(&owned, &owned, expected).is_err() && view.is_pidfd(&owned).is_err();
        unsafe { libc::_exit(if rejected { 0 } else { 17 }) };
    }
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(child, &mut status, 0) }, child);
    assert!(libc::WIFEXITED(status));
    assert_eq!(
        libc::WEXITSTATUS(status),
        0,
        "inherited proc view was accepted in a different process"
    );
}
#[test]
fn inherited_proc_view_is_rejected_in_an_actual_forked_process() {
    let _guard = KERNEL_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "pidfd_inherited_view_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdin(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
}
