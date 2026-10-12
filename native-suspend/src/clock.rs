use std::time::Duration;
/// A monotonically increasing clock that includes time asleep. Implementations
/// injected in tests must model the same domain as Linux CLOCK_BOOTTIME.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Duration;
}
pub struct BootClock;
impl Clock for BootClock {
    fn now(&self) -> Duration {
        let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
        Duration::new(
            value.tv_sec.try_into().expect("nonnegative boot time"),
            value.tv_nsec.try_into().expect("valid boot nanoseconds"),
        )
    }
}
