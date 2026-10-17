//! The one clock edge deliberately uses the fallible libc symbol. Existing
//! native --wrap=clock_gettime controls must intercept this same call.
pub fn now() -> Option<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: value is a live, correctly aligned output timespec for libc.
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) } != 0
        || value.tv_sec < 0
        || !(0..1_000_000_000).contains(&value.tv_nsec)
    {
        return None;
    }
    u64::try_from(value.tv_sec)
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(u64::try_from(value.tv_nsec).ok()?)
}
