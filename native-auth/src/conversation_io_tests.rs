//! Ordinary host transport setup-state regression using private pipes only.
//! Include as owner::conversation_io_tests after the behavior-preserving
//! owner::conversation extraction. No Attempt, Leaf, PAM or privileged fixture.
use super::{Barrier, conversation::Conversation};
use rustix::{
    event::{self, PollFd, PollFlags},
    fd::{AsFd, OwnedFd},
    fs::{self, OFlags},
    process::{self, Pid, PidfdFlags, Signal},
};
use std::{
    io::{self, Write},
    os::unix::process::ExitStatusExt,
    path::PathBuf,
    process::{Child, ChildStdout, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const HELPER_FILTER: &str =
    "owner::conversation_io_tests::failed_initialize_blocking_stdout_helper";
const HELPER_ENV: &str = "POKELINUX_CONVERSATION_FAILED_INIT_HELPER";
const READY: &[u8] = b"POKELINUX-CONVERSATION-FAILED-INIT-READY\n";
const START_BOUND: Duration = Duration::from_secs(3);
const CALL_BOUND: Duration = Duration::from_millis(500);
const CLOSE_BOUND: Duration = Duration::from_secs(3);

// The only child runs this exact ignored test. Keeping its stdout separate
// gives the parent a readiness boundary and actual EOF after an actual wait.
struct Helper {
    child: Child,
    alive: Option<OwnedFd>,
    output: Option<ChildStdout>,
    reaped: bool,
}

impl Helper {
    fn spawn() -> io::Result<Self> {
        let child = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                HELPER_FILTER,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env(HELPER_ENV, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        // Establish cleanup ownership before any subsequent fallible setup.
        let mut helper = Self {
            child,
            alive: None,
            output: None,
            reaped: false,
        };
        helper.output = helper.child.stdout.take();
        let output = helper.output.as_ref().ok_or(io::ErrorKind::Other)?;
        fs::fcntl_setfl(output, fs::fcntl_getfl(output)? | OFlags::NONBLOCK)?;
        let pid = Pid::from_raw(helper.child.id() as i32).ok_or(io::ErrorKind::InvalidInput)?;
        helper.alive = Some(process::pidfd_open(pid, PidfdFlags::NONBLOCK)?);
        Ok(helper)
    }

    fn output(&self) -> &ChildStdout {
        self.output.as_ref().expect("owned helper stdout")
    }

    fn alive(&self) -> &OwnedFd {
        self.alive.as_ref().expect("owned helper pidfd")
    }

    fn await_ready(&self, deadline: Instant) -> io::Result<()> {
        let mut matched = 0;
        let mut total = 0;
        loop {
            if Instant::now() >= deadline {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let mut bytes = [0; 128];
            match rustix::io::read(self.output(), &mut bytes) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(count) => {
                    total += count;
                    if total > 4096 {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    // Retain only the constant marker match, never helper logs.
                    for byte in bytes[..count].iter().copied() {
                        matched = if byte == READY[matched] {
                            matched + 1
                        } else {
                            usize::from(byte == READY[0])
                        };
                        if matched == READY.len() {
                            return Ok(());
                        }
                    }
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(rustix::io::Errno::AGAIN) => {
                    if !wait_readable(self.output(), deadline)? {
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn wait_by(&mut self, deadline: Instant) -> io::Result<Option<ExitStatus>> {
        if !wait_readable(self.alive(), deadline)? {
            return Ok(None);
        }
        // pidfd readiness means this exact child has exited. This is an actual
        // Child wait, rather than treating successful signalling as cleanup.
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(Some(status))
    }

    fn kill_exact(&self) -> io::Result<()> {
        match process::pidfd_send_signal(self.alive(), Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn observe_closed(&self, deadline: Instant) -> io::Result<()> {
        let mut total = 0;
        loop {
            if Instant::now() >= deadline {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let mut bytes = [0; 128];
            match rustix::io::read(self.output(), &mut bytes) {
                Ok(0) => break,
                Ok(count) => {
                    total += count;
                    if total > 4096 {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(rustix::io::Errno::AGAIN) => {
                    if !wait_readable(self.output(), deadline)? {
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }
        if !wait_readable(self.alive(), Instant::now() + Duration::from_millis(1))? {
            return Err(io::ErrorKind::Other.into());
        }
        match std::fs::metadata(PathBuf::from(format!("/proc/{}", self.child.id()))) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            _ => Err(io::ErrorKind::Other.into()),
        }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        if !self.reaped {
            // Infrastructure-error/panic cleanup only. A successful case takes
            // the natural-exit path and never enters this fallback. Child
            // remains unreaped here, so its exact PID cannot be recycled.
            if let Some(alive) = &self.alive {
                let _ = process::pidfd_send_signal(alive, Signal::KILL);
            } else {
                let _ = self.child.kill();
            }
            let _ = self.child.wait();
        }
    }
}

fn wait_readable(fd: &impl AsFd, deadline: Instant) -> io::Result<bool> {
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Ok(false);
        };
        let mut fds = [PollFd::new(fd, PollFlags::IN)];
        let timeout = event::Timespec {
            tv_sec: remaining.as_secs() as _,
            tv_nsec: remaining.subsec_nanos() as _,
        };
        match event::poll(&mut fds, Some(&timeout)) {
            Ok(0) => return Ok(false),
            Ok(_) if fds[0].revents().intersects(PollFlags::IN | PollFlags::HUP) => {
                return Ok(true);
            }
            Ok(_) => return Err(io::ErrorKind::Other.into()),
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[test]
fn failed_initialize_refuses_retained_blocking_stdout_without_reading() {
    let mut helper = Helper::spawn().expect("spawn exact private I/O helper");
    helper
        .await_ready(Instant::now() + START_BOUND)
        .expect("helper reached failed initialization with retained blocking pipe");
    let natural_status = helper
        .wait_by(Instant::now() + CALL_BOUND)
        .expect("observe exact helper exit");
    let timed_out = natural_status.is_none();
    let status = match natural_status {
        Some(status) => status,
        None => {
            helper.kill_exact().expect("kill exact blocked helper");
            helper
                .wait_by(Instant::now() + CLOSE_BOUND)
                .expect("observe killed helper")
                .expect("killed helper exited within cleanup bound")
        }
    };
    helper
        .observe_closed(Instant::now() + CLOSE_BOUND)
        .expect("actual helper stdout EOF, pidfd death and proc disappearance");
    eprintln!(
        "CONVERSATION_IO_BOUNDARY timed_out={} wait_code={:?} wait_signal={:?} stdout_EOF=true pidfd_dead=true proc_absent=true",
        timed_out,
        status.code(),
        status.signal()
    );
    assert!(
        !timed_out,
        "read_protocol blocked on stdout retained after failed initialization"
    );
    assert!(
        status.success(),
        "helper must return local error and exit zero"
    );
}

#[test]
#[ignore = "runs only as the exact private subprocess of the parent regression"]
fn failed_initialize_blocking_stdout_helper() {
    assert!(std::env::var_os(HELPER_ENV).as_deref() == Some(std::ffi::OsStr::new("1")));
    let (reader, retained_writer) =
        rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC).unwrap();
    assert!(!fs::fcntl_getfl(&reader).unwrap().contains(OFlags::NONBLOCK));
    let mut conversation = Conversation::new(crate::boottime_ns() + 15_000_000_000);
    assert!(
        conversation
            .initialize(None::<&OwnedFd>, Some(&reader))
            .is_err()
    );
    assert!(!fs::fcntl_getfl(&reader).unwrap().contains(OFlags::NONBLOCK));
    // Arbitrary ID is pure protocol state; this is never a fake Attempt/Leaf.
    let mut barrier = Barrier::new(100);
    {
        let mut stdout = io::stdout().lock();
        stdout.write_all(READY).unwrap();
        stdout.flush().unwrap();
    }
    let result = conversation.read_protocol(Some(&reader), &mut barrier);
    assert!(
        result.is_err(),
        "unavailable transport must return a local error"
    );
    assert!(!barrier.worker_stdout_eof);
    assert!(!fs::fcntl_getfl(&reader).unwrap().contains(OFlags::NONBLOCK));
    // This writer stays open throughout the call: EOF cannot unblock baseline.
    drop(retained_writer);
}
