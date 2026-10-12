//! Native policy uses boot time, which includes time spent suspended.
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BootTime(Duration);
impl BootTime {
    /// For an injected clock or an already sampled CLOCK_BOOTTIME value.
    pub fn from_elapsed(elapsed: Duration) -> Self {
        Self(elapsed)
    }
    pub fn elapsed(self) -> Duration {
        self.0
    }
    pub fn checked_after(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }
}

pub trait Clock {
    fn now(&self) -> BootTime;
}

#[cfg(target_os = "linux")]
pub struct BootClock;
#[cfg(target_os = "linux")]
impl Clock for BootClock {
    fn now(&self) -> BootTime {
        let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
        BootTime::from_elapsed(Duration::new(
            value.tv_sec.try_into().expect("nonnegative boot time"),
            value.tv_nsec.try_into().expect("valid boot nanoseconds"),
        ))
    }
}
