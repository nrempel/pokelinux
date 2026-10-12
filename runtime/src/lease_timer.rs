//! Independent, one-shot Linux BOOTTIME expiry for the observer event loop.
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
        timerfd_settime,
    },
};
use std::{io, time::Duration};

pub struct LeaseTimer(OwnedFd);
/// Async ownership of the same BOOTTIME timerfd, with one expiry consumer.
pub struct AsyncLeaseTimer(async_io::Async<LeaseTimer>);
impl AsyncLeaseTimer {
    pub fn arm(&self, deadline: Option<Duration>) -> io::Result<()> {
        self.0.get_ref().arm(deadline)
    }
    /// Cancellation leaves the timer armed. Readiness is only a prompt to
    /// check the current lease; it does not identify which lease expired.
    pub async fn expired(&self) -> io::Result<()> {
        self.0
            .read_with(|timer| {
                if timer.consume()? {
                    Ok(())
                } else {
                    Err(io::ErrorKind::WouldBlock.into())
                }
            })
            .await
    }
}
impl LeaseTimer {
    pub fn into_async(self) -> io::Result<AsyncLeaseTimer> {
        Ok(AsyncLeaseTimer(async_io::Async::new(self)?))
    }
    pub fn new() -> io::Result<Self> {
        Ok(Self(timerfd_create(
            TimerfdClockId::Boottime,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?))
    }
    /// The absolute deadline uses CLOCK_BOOTTIME. None disarms; a past deadline
    /// expires immediately. BOOTTIME does not request permission to wake a phone.
    pub fn arm(&self, deadline: Option<Duration>) -> io::Result<()> {
        let value = match deadline {
            None => Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            Some(value) => {
                // timerfd interprets an all-zero value as disarm, even with
                // ABSTIME. Preserve the meaning of an already-expired lease.
                let value = value.max(Duration::from_nanos(1));
                Timespec {
                    tv_sec: value.as_secs().try_into().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidInput, "lease deadline out of range")
                    })?,
                    tv_nsec: value.subsec_nanos().into(),
                }
            }
        };
        timerfd_settime(
            &self.0,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: value,
            },
        )?;
        Ok(())
    }
    /// Drain readiness without blocking. The owner must still compare the
    /// current lease identity/deadline: readiness is not command authority.
    pub fn consume(&self) -> io::Result<bool> {
        let mut count = [0; 8];
        loop {
            match rustix::io::read(&self.0, &mut count) {
                Ok(8) => return Ok(true),
                Ok(_) => return Err(io::Error::other("invalid timer read")),
                Err(rustix::io::Errno::INTR) => continue,
                Err(rustix::io::Errno::AGAIN) => return Ok(false),
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl AsFd for LeaseTimer {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
