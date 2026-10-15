//! Fixed real-syscall controls driven by tests/syscall_boundaries.py.
#![forbid(unsafe_code)]

use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
use pokelinux_sit_transport::{Error, Outcome, Snapshot, StopReason, WorkerTransport};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::{Errno, read},
    process::{Resource, Rlimit, setrlimit},
};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::Path,
    time::Duration,
};

fn request() -> [u8; 983] {
    let hex: String = include_str!("../tests/fixtures/initial-v4.hex")
        .split_whitespace()
        .collect();
    assert_eq!(hex.len(), 983 * 2);
    std::array::from_fn(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
}

fn terminal(transport: &mut WorkerTransport, reason: StopReason) {
    assert_eq!(
        transport.snapshot(),
        Snapshot {
            outcome: Outcome::OutcomeUnknown,
            terminal: Some(reason),
        }
    );
    assert_eq!(transport.dispatch(&request()), Err(Error::Terminal(reason)));
    assert!(matches!(
        transport.read_chunk(),
        Err(Error::Terminal(actual)) if actual == reason
    ));
    assert_eq!(transport.snapshot().outcome, Outcome::OutcomeUnknown);
}

fn readonly(path: &Path) {
    let file = File::open(path).expect("open actual readonly regular file");
    assert!(file.metadata().unwrap().is_file());
    let mut transport = WorkerTransport::new(
        file.into(),
        Deadline::at(BootClock.now() + Duration::from_secs(5)),
    );
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::Write(Errno::BADF)),
        "MISSING_ACTUAL_EBADF_TERMINAL_WRITE"
    );
    terminal(&mut transport, StopReason::Write);
    println!("readonly-terminal-write-unknown");
}

fn partial(path: &Path) {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("create actual empty regular file");
    assert!(file.metadata().unwrap().is_file());
    // Sample the original deadline before clamping this child's file writes.
    // This also completes first-use clock/runtime probing before the fixture limit.
    let until = BootClock.now() + Duration::from_secs(5);
    // This fresh child owns the limit. No signal handler or limit reset masks retry.
    setrlimit(
        Resource::Fsize,
        Rlimit {
            current: Some(16),
            maximum: Some(16),
        },
    )
    .expect("set actual child-only RLIMIT_FSIZE=16");
    let mut transport = WorkerTransport::new(file.into(), Deadline::at(until));
    assert_eq!(
        transport.dispatch(&request()),
        Err(Error::IncompleteWrite { written: 16 }),
        "MISSING_ACTUAL_PARTIAL16_TERMINAL_WRITE"
    );
    terminal(&mut transport, StopReason::Write);
    println!("partial16-terminal-write-unknown");
}

fn late_read(path: &Path) {
    let socket = UnixStream::connect(path).expect("connect actual blocking Unix stream");
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let descriptor = socket.as_raw_fd();
    let until = BootClock.now() + Duration::from_secs(2);
    let mut transport = WorkerTransport::new(socket.into(), Deadline::at(until));
    transport
        .dispatch(&request())
        .expect("MISSING_SINGLE_TRANSPORT_DISPATCH");
    // This scalar is solely a /proc syscall-witness argument, not a descriptor alias.
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&until.as_nanos().to_be_bytes()).unwrap();
    stdout.write_all(&descriptor.to_be_bytes()).unwrap();
    stdout.flush().unwrap();
    match transport.read_chunk() {
        Err(error) => assert_eq!(error, Error::Deadline, "MISSING_POST_READ_DEADLINE"),
        Ok(_) => panic!("late positive read must not expose a received chunk"),
    }
    terminal(&mut transport, StopReason::Deadline);
    stdout.write_all(b"late-read-deadline-unknown\n").unwrap();
    stdout.flush().unwrap();
    // Keep the transport value alive until the parent observes peer EOF.
    let stdin = std::io::stdin();
    let mut ready = [PollFd::new(&stdin, PollFlags::IN)];
    assert_eq!(
        poll(
            &mut ready,
            Some(&Timespec {
                tv_sec: 5,
                tv_nsec: 0,
            }),
        )
        .unwrap(),
        1,
        "parent EOF acknowledgement exceeded 5s"
    );
    let mut acknowledgement = [0];
    assert_eq!(read(&stdin, &mut acknowledgement[..]).unwrap(), 1);
    assert_eq!(acknowledgement, [0x7f]);
    assert_eq!(transport.snapshot().terminal, Some(StopReason::Deadline));
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    assert_eq!(
        args.len(),
        3,
        "expected readonly|partial|late-read and fixture path"
    );
    let path = Path::new(&args[2]);
    match args[1].to_str().unwrap() {
        "readonly" => readonly(path),
        "partial" => partial(path),
        "late-read" => late_read(path),
        _ => panic!("unknown fixed syscall control"),
    }
}
