use crate::{Backend, BackendError, BootClock, Cancellation, Clock};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{OFlags, SeekFrom, fcntl_getfl, fcntl_setfl, seek};
use rustix::io::{Errno, read, write};
use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::time::Duration;

/// Owns only descriptors supplied by the caller. It never opens a path.
/// Count input must be a newline-terminated unsigned decimal attribute.
/// Generic descriptor errors are uncertain; EINVAL is not automatically
/// promoted to a proven changed-count rejection.
pub struct DescriptorBackend {
    count: OwnedFd,
    state: OwnedFd,
    abort: OwnedFd,
    unlock: OwnedFd,
    clock: Arc<dyn Clock>,
}
impl DescriptorBackend {
    pub fn new(
        count: OwnedFd,
        state: OwnedFd,
        abort: OwnedFd,
        unlock: OwnedFd,
    ) -> Result<Self, std::io::Error> {
        Self::with_clock(count, state, abort, unlock, Arc::new(BootClock))
    }
    pub fn with_clock(
        count: OwnedFd,
        state: OwnedFd,
        abort: OwnedFd,
        unlock: OwnedFd,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, std::io::Error> {
        for fd in [&count, &state, &abort, &unlock] {
            let flags = fcntl_getfl(fd)?;
            fcntl_setfl(fd, flags | OFlags::NONBLOCK)?;
        }
        Ok(Self {
            count,
            state,
            abort,
            unlock,
            clock,
        })
    }
}
// Created once per operation and retained across EINTR/EAGAIN and polling.
// Linux CLOCK_BOOTTIME includes sleep; late syscall success remains uncertain.
struct Deadline<'a> {
    clock: &'a dyn Clock,
    end: Duration,
}
impl<'a> Deadline<'a> {
    fn new(clock: &'a dyn Clock, budget: Duration) -> Result<Self, BackendError> {
        let end = clock
            .now()
            .checked_add(budget)
            .ok_or(BackendError::Uncertain)?;
        Ok(Self { clock, end })
    }
    fn remaining(&self) -> Result<Duration, BackendError> {
        let now = self.clock.now();
        if now >= self.end {
            Err(BackendError::Uncertain)
        } else {
            Ok(self.end - now)
        }
    }
}
fn rewind(fd: &OwnedFd) -> Result<(), BackendError> {
    match seek(fd, SeekFrom::Start(0)) {
        Ok(_) => Ok(()),
        Err(Errno::SPIPE) => Ok(()),
        Err(_) => Err(BackendError::Uncertain),
    }
}
fn wait(
    fd: &OwnedFd,
    flags: PollFlags,
    cancel: Option<&Cancellation>,
    deadline: &Deadline<'_>,
) -> Result<(), BackendError> {
    loop {
        let remaining = deadline.remaining()?;
        if cancel.is_some_and(Cancellation::cancelled) {
            return Err(BackendError::Cancelled);
        }
        let timeout = remaining.min(Duration::from_millis(20));
        let mut fds = [PollFd::new(fd, flags)];
        let result = poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: timeout.as_nanos() as i64,
            }),
        );
        deadline.remaining()?; // Including successful readiness and EINTR.
        match result {
            Ok(0) | Err(Errno::INTR) => {}
            Ok(_) => return Ok(()),
            Err(_) => return Err(BackendError::Uncertain),
        }
    }
}
fn attribute_write(
    fd: &OwnedFd,
    bytes: &[u8],
    cancel: Option<&Cancellation>,
    clock: &dyn Clock,
) -> Result<(), BackendError> {
    attribute_write_with(fd, bytes, cancel, clock, || write(fd, bytes))
}
fn attribute_write_with(
    fd: &OwnedFd,
    bytes: &[u8],
    cancel: Option<&Cancellation>,
    clock: &dyn Clock,
    mut perform: impl FnMut() -> rustix::io::Result<usize>,
) -> Result<(), BackendError> {
    let deadline = Deadline::new(clock, Duration::from_secs(1))?;
    rewind(fd)?;
    deadline.remaining()?;
    loop {
        deadline.remaining()?;
        if cancel.is_some_and(Cancellation::cancelled) {
            return Err(BackendError::Cancelled);
        }
        let result = perform();
        deadline.remaining()?; // A late successful write is not current success.
        match result {
            Ok(n) if n == bytes.len() => return Ok(()),
            Err(Errno::INTR) => {}
            Err(Errno::AGAIN) => wait(fd, PollFlags::OUT, cancel, &deadline)?,
            _ => return Err(BackendError::Uncertain),
        }
    }
}
impl Backend for DescriptorBackend {
    fn read_count(&self, cancel: &Cancellation) -> Result<Vec<u8>, BackendError> {
        let deadline = Deadline::new(self.clock.as_ref(), Duration::from_secs(2))?;
        rewind(&self.count)?;
        deadline.remaining()?;
        let mut count = Vec::with_capacity(21);
        loop {
            deadline.remaining()?;
            if cancel.cancelled() {
                return Err(BackendError::Cancelled);
            }
            let mut buf = [0; 32];
            let result = read(&self.count, &mut buf);
            deadline.remaining()?; // Late read success cannot authorize commit.
            match result {
                Ok(0) => return Err(BackendError::Uncertain),
                Ok(n) => {
                    for (i, byte) in buf[..n].iter().enumerate() {
                        if *byte == b'\n' {
                            if i != n - 1 || count.is_empty() {
                                return Err(BackendError::Uncertain);
                            }
                            return Ok(count);
                        }
                        if !byte.is_ascii_digit() || count.len() >= 20 {
                            return Err(BackendError::Uncertain);
                        }
                        count.push(*byte);
                    }
                }
                Err(Errno::INTR) => {}
                Err(Errno::AGAIN) => wait(&self.count, PollFlags::IN, Some(cancel), &deadline)?,
                Err(_) => return Err(BackendError::Uncertain),
            }
        }
    }
    fn commit_count(&self, count: &[u8], cancel: &Cancellation) -> Result<(), BackendError> {
        attribute_write(&self.count, count, Some(cancel), self.clock.as_ref())
    }
    fn enter_mem(&self, cancel: &Cancellation) -> Result<(), BackendError> {
        attribute_write(&self.state, b"mem", Some(cancel), self.clock.as_ref())
    }
    fn begin_wake(&self) -> Result<(), BackendError> {
        attribute_write(&self.abort, b"userspace-abort", None, self.clock.as_ref())
    }
    fn finish_wake(&self) -> Result<(), BackendError> {
        attribute_write(&self.unlock, b"userspace-abort", None, self.clock.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::sync::atomic::{AtomicU64, Ordering};
    struct Moving(AtomicU64);
    impl Clock for Moving {
        fn now(&self) -> Duration {
            Duration::from_secs(self.0.load(Ordering::Acquire))
        }
    }
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    #[test]
    fn eintr_does_not_restart_the_absolute_write_budget() {
        let temp = Temp(
            std::env::temp_dir().join(format!("pokelinux-suspend-eintr-{}", std::process::id())),
        );
        let fd: OwnedFd = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&temp.0)
            .unwrap()
            .into();
        let clock = Moving(AtomicU64::new(0));
        let mut calls = 0;
        let result = attribute_write_with(&fd, b"mem", None, &clock, || {
            calls += 1;
            if calls == 1 {
                clock.0.store(30, Ordering::Release);
                Err(Errno::INTR)
            } else {
                write(&fd, b"mem")
            }
        });
        assert_eq!(result, Err(BackendError::Uncertain));
        assert_eq!(calls, 1);
        assert!(fs::read(&temp.0).unwrap().is_empty());
    }
}
