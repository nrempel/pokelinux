//! Absolute Linux boot-time deadlines include suspend. They do not wake hardware.
use rustix::time::{ClockId, clock_gettime};
use std::time::Duration;

/// Implementations must be bounded, monotonic and non-reentrant: sampled under
/// the owner gate. Inject a controlled clock for scheduling tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}
pub struct BootClock;
impl Clock for BootClock {
    fn now(&self) -> Duration {
        let time = clock_gettime(ClockId::Boottime);
        Duration::new(
            time.tv_sec.try_into().expect("nonnegative boot time"),
            time.tv_nsec.try_into().expect("valid boot nanoseconds"),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline(Duration);
impl Deadline {
    pub fn at(boot_time: Duration) -> Self {
        Self(boot_time)
    }
    pub fn elapsed(self, now: Duration) -> bool {
        now >= self.0
    }
}
