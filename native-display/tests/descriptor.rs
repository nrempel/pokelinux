//! Actual host kernel/exec boundary, separate from original compositor evidence.
use pokelinux_display::{Context, MAX_INITIALIZATION_NS, Metadata, SourceIdentity, boottime_ns};
use rustix::{
    io::{FdFlags, fcntl_setfd},
    process::{PidfdFlags, pidfd_open},
};
use std::{
    os::{
        fd::AsRawFd,
        unix::net::{UnixListener, UnixStream},
    },
    path::PathBuf,
};

#[test]
fn exec_probe() {
    let Ok(raw) = std::env::var("NATIVE_DISPLAY_EXPECT_FD") else {
        return;
    };
    let target = PathBuf::from(std::env::var_os("NATIVE_DISPLAY_EXPECT_LINK").unwrap());
    assert_ne!(
        std::fs::read_link(format!("/proc/self/fd/{raw}")).ok(),
        Some(target),
        "privileged creator connection crossed exec"
    );
}

#[test]
fn inherited_creator_stream_cannot_cross_later_exec() {
    assert_eq!(
        rustix::process::getuid().as_raw(),
        0,
        "run inside isolated root container"
    );
    let root = PathBuf::from(format!(
        "/run/native-display-descriptor-{}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let source = UnixListener::bind(root.join("source")).unwrap();
    let stream = UnixStream::connect(root.join("source")).unwrap();
    let accepted = source.accept().unwrap().0;
    fcntl_setfd(&stream, FdFlags::empty()).unwrap();
    let raw = stream.as_raw_fd();
    let target = std::fs::read_link(format!("/proc/self/fd/{raw}")).unwrap();
    let listener = UnixListener::bind(root.join("restricted")).unwrap();
    let pid = rustix::process::getpid();
    let expected = pidfd_open(pid, PidfdFlags::NONBLOCK).unwrap();
    let context = Context::new(
        stream,
        &expected,
        SourceIdentity {
            pid,
            uid: 0,
            gid: 0,
        },
        listener.into(),
        Metadata {
            app: "org.pokelinux.test".into(),
            instance: "0123456789abcdef0123456789abcdef".into(),
        },
        boottime_ns() + MAX_INITIALIZATION_NS,
    )
    .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "exec_probe", "--nocapture"])
        .env("NATIVE_DISPLAY_EXPECT_FD", raw.to_string())
        .env("NATIVE_DISPLAY_EXPECT_LINK", target)
        .output()
        .unwrap();
    drop(context);
    drop(accepted);
    drop(source);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn listening_tcp_descriptor_cannot_become_a_wayland_context() {
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let root = PathBuf::from(format!("/run/native-display-family-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let source = UnixListener::bind(root.join("source")).unwrap();
    let stream = UnixStream::connect(root.join("source")).unwrap();
    let accepted = source.accept().unwrap().0;
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let pid = rustix::process::getpid();
    let expected = pidfd_open(pid, PidfdFlags::NONBLOCK).unwrap();
    let result = Context::new(
        stream,
        &expected,
        SourceIdentity {
            pid,
            uid: 0,
            gid: 0,
        },
        tcp.into(),
        Metadata {
            app: "org.pokelinux.test".into(),
            instance: "0123456789abcdef0123456789abcdef".into(),
        },
        boottime_ns() + MAX_INITIALIZATION_NS,
    );
    let refused = matches!(&result, Err(e) if e.kind() == std::io::ErrorKind::InvalidInput);
    drop(result);
    drop(accepted);
    drop(source);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        refused,
        "listening TCP descriptor was admitted as a Wayland context"
    );
}
