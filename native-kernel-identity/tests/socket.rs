#![cfg(target_os = "linux")]
use pokelinux_kernel_identity::socket_peer_pidfd;
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[test]
#[ignore = "owned subprocess entrypoint, invoked by the real boundary test"]
fn socket_peer_child() {
    let path = std::env::var_os("KERNEL_IDENTITY_TEST_SOCKET").unwrap();
    let listener = UnixListener::bind(path).unwrap();
    let (mut socket, _) = listener.accept().unwrap();
    let mut byte = [0];
    socket.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [9]);
}
#[test]
fn actual_kernel_socket_origin_is_cloexec_and_exits_with_the_peer() {
    let path = std::env::temp_dir().join(format!("kernel-origin-{}.sock", std::process::id()));
    assert!(!path.exists());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "socket_peer_child", "--nocapture"])
        .env("KERNEL_IDENTITY_TEST_SOCKET", &path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while !path.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut socket = UnixStream::connect(&path).unwrap();
    let cred = rustix::net::sockopt::socket_peercred(&socket).unwrap();
    assert_eq!(cred.pid.as_raw_nonzero().get() as u32, child.id());
    let peer = socket_peer_pidfd(&socket).unwrap();
    assert!(
        rustix::io::fcntl_getfd(&peer)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    let mut poll = [rustix::event::PollFd::new(
        &peer,
        rustix::event::PollFlags::IN,
    )];
    let zero = rustix::event::Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(rustix::event::poll(&mut poll, Some(&zero)).unwrap(), 0);
    socket.write_all(&[9]).unwrap();
    while rustix::event::poll(&mut poll, Some(&zero)).unwrap() == 0 {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(child.wait().unwrap().success());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn actual_kernel_nonsocket_error_preserves_the_borrowed_input() {
    let file = std::fs::File::open("/dev/null").unwrap();
    assert_eq!(
        socket_peer_pidfd(&file).unwrap_err().raw_os_error(),
        Some(libc::ENOTSOCK)
    );
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
}
