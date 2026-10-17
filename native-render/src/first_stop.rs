//! Fixed control-thread Stop observations, never executor/native authority.
//!
//! The caller performs its terminal latch and Stop before recording. No native
//! object, FD, worker, callback or queue is retained by this independent handle.
use std::{
    alloc::{Layout, alloc, dealloc},
    cell::{Cell, UnsafeCell},
    ptr,
    thread::{self, ThreadId},
};

const OK: i32 = 0;
const INVALID: i32 = 1;
const CAPACITY: i32 = 3;
const ABI: u32 = 1;
const TERMINAL_REOBSERVED: u32 = 20;

thread_local! {
    // Initialization is const and nonallocating. Only create may initialize
    // Rust thread metadata; a foreign record/snapshot/release only reads here.
    static CONTROL_ID: Cell<Option<ThreadId>> = const { Cell::new(None) };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FirstStopFactsV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub origin: u32,
    pub executor_present: u32,
    pub error: i32,
    pub event_mask: u32,
    pub revoke_session: u64,
    pub revoke_reason: u32,
    pub broker_present: u32,
    pub broker_probe_failed: u32,
    pub broker_transport_failed: u32,
    pub broker_pending_present: u32,
    pub broker_pending_type: u32,
    pub broker_pending_sequence: u64,
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FirstStopSnapshotV1 {
    pub stop_requests: u64,
    pub has_first: u32,
    pub clock_known: u32,
    pub observed_boottime_ns: u64,
    pub clock_errno: i32,
    pub reserved: u32,
    pub first: FirstStopFactsV1,
}

const _: () = {
    assert!(size_of::<FirstStopFactsV1>() == 80);
    assert!(size_of::<FirstStopSnapshotV1>() == 112);
    assert!(std::mem::offset_of!(FirstStopFactsV1, revoke_session) == 24);
    assert!(std::mem::offset_of!(FirstStopFactsV1, broker_pending_sequence) == 56);
    assert!(std::mem::offset_of!(FirstStopFactsV1, reserved) == 64);
    assert!(std::mem::offset_of!(FirstStopSnapshotV1, first) == 32);
};

pub struct PrFirstStop {
    control: ThreadId,
    inner: UnsafeCell<FirstStopSnapshotV1>,
}

fn valid(facts: &FirstStopFactsV1) -> bool {
    if facts.abi_version != ABI
        || facts.struct_size as usize != size_of::<FirstStopFactsV1>()
        || !(1..=TERMINAL_REOBSERVED).contains(&facts.origin)
        || facts.reserved != [0; 4]
        || [
            facts.executor_present,
            facts.broker_present,
            facts.broker_probe_failed,
            facts.broker_transport_failed,
            facts.broker_pending_present,
        ]
        .iter()
        .any(|value| *value > 1)
        || (!matches!(facts.origin, 4 | 5 | 7 | 8 | 10 | 11 | 13 | 18 | 19) && facts.error != 0)
        || (!matches!(facts.origin, 4 | 7) && facts.event_mask != 0)
    {
        return false;
    }
    if facts.origin == 3 {
        if !(1..=5).contains(&facts.revoke_reason) {
            return false;
        }
    } else if facts.revoke_session != 0 || facts.revoke_reason != 0 {
        return false;
    }
    if facts.broker_present == 0 && facts.broker_probe_failed != 0 {
        return false;
    }
    if (facts.broker_present == 0 || facts.broker_probe_failed == 1)
        && (facts.broker_transport_failed != 0 || facts.broker_pending_present != 0)
    {
        return false;
    }
    // These raw values describe an observed head, never an inferred empty queue.
    facts.broker_pending_present != 0
        || (facts.broker_pending_type == 0 && facts.broker_pending_sequence == 0)
}

#[derive(Clone, Copy)]
struct ClockObservation {
    known: u32,
    ns: u64,
    error: i32,
}
impl ClockObservation {
    fn known(ns: u64) -> Self {
        Self {
            known: 1,
            ns,
            error: 0,
        }
    }
    fn unavailable(error: i32) -> Self {
        Self {
            known: 0,
            ns: 0,
            error,
        }
    }
}

// This module supplies the raw 64-bit Linux/Android clock ABI only. Other
// targets explicitly report ENOSYS; no hardware/Android execution is implied.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_pointer_width = "64"
))]
fn clock_observation() -> ClockObservation {
    use std::ffi::{c_int, c_long};
    #[repr(C)]
    struct Timespec {
        seconds: c_long,
        nanoseconds: c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: c_int, value: *mut Timespec) -> c_int;
        #[cfg(target_os = "linux")]
        fn __errno_location() -> *mut c_int;
        #[cfg(target_os = "android")]
        fn __errno() -> *mut c_int;
    }
    let mut value = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    if unsafe { clock_gettime(7, &mut value) } != 0 {
        // Read actual thread errno immediately, with no allocation/formatting.
        #[cfg(target_os = "linux")]
        let error = unsafe { *__errno_location() };
        #[cfg(target_os = "android")]
        let error = unsafe { *__errno() };
        return ClockObservation::unavailable(error);
    }
    // A kernel value not representable by this u64-ns ABI is an EOVERFLOW
    // observation error, rather than a fabricated timestamp or syscall errno.
    if value.seconds < 0 || !(0..1_000_000_000).contains(&value.nanoseconds) {
        return ClockObservation::unavailable(75);
    }
    match (value.seconds as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(value.nanoseconds as u64))
    {
        Some(ns) => ClockObservation::known(ns),
        None => ClockObservation::unavailable(75),
    }
}
#[cfg(not(all(
    any(target_os = "linux", target_os = "android"),
    target_pointer_width = "64"
)))]
fn clock_observation() -> ClockObservation {
    ClockObservation::unavailable(38) // ENOSYS: this clock ABI is unsupported.
}

fn record_with_clock(
    state: &mut FirstStopSnapshotV1,
    facts: FirstStopFactsV1,
    clock: impl FnOnce() -> ClockObservation,
) -> i32 {
    if !valid(&facts) || (state.has_first == 0 && facts.origin == TERMINAL_REOBSERVED) {
        return INVALID;
    }
    if state.has_first == 0 {
        let observation = clock();
        state.first = facts;
        state.clock_known = observation.known;
        state.observed_boottime_ns = observation.ns;
        state.clock_errno = observation.error;
        state.has_first = 1;
    }
    state.stop_requests = state.stop_requests.saturating_add(1);
    OK
}

/// Check cached identity before inner-state access or any caller data/output.
/// # Safety
/// A nonnull handle is live for this serialized call, with no concurrent release.
unsafe fn owned<'a>(raw: *mut PrFirstStop) -> Option<&'a PrFirstStop> {
    let id = CONTROL_ID.try_with(Cell::get).ok().flatten()?;
    if raw.is_null() {
        return None;
    }
    let envelope = unsafe { &*raw };
    (envelope.control == id).then_some(envelope)
}

/// # Safety
/// Live, aligned, uniquely writable empty output slot; serialized control call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_first_stop_create_v1(result: *mut *mut PrFirstStop) -> i32 {
    if result.is_null() || !unsafe { *result }.is_null() {
        return INVALID;
    }
    let control = match CONTROL_ID.try_with(|slot| {
        if let Some(id) = slot.get() {
            id
        } else {
            let id = thread::current().id();
            slot.set(Some(id));
            id
        }
    }) {
        Ok(id) => id,
        Err(_) => return INVALID,
    };
    let raw = unsafe { alloc(Layout::new::<PrFirstStop>()) }.cast::<PrFirstStop>();
    if raw.is_null() {
        return CAPACITY;
    }
    unsafe {
        raw.write(PrFirstStop {
            control,
            inner: UnsafeCell::new(FirstStopSnapshotV1::default()),
        });
        result.write(raw);
    }
    OK
}

/// # Safety
/// Live serialized creator-thread handle, live nonaliasing input value. The
/// caller has already performed authority Stop; this operation cannot do it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_first_stop_record_v1(
    owner: *mut PrFirstStop,
    facts: *const FirstStopFactsV1,
) -> i32 {
    let Some(envelope) = (unsafe { owned(owner) }) else {
        return INVALID;
    };
    if facts.is_null() {
        return INVALID;
    }
    let facts = unsafe { facts.read() };
    record_with_clock(
        unsafe { &mut *envelope.inner.get() },
        facts,
        clock_observation,
    )
}

/// # Safety
/// Live serialized creator-thread handle and live nonaliasing writable output.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_first_stop_snapshot_v1(
    owner: *mut PrFirstStop,
    result: *mut FirstStopSnapshotV1,
) -> i32 {
    let Some(envelope) = (unsafe { owned(owner) }) else {
        return INVALID;
    };
    if result.is_null() {
        return INVALID;
    }
    unsafe { result.write(*envelope.inner.get()) };
    OK
}

/// # Safety
/// Live unique slot; no concurrent use/release. Broker/callback sources are
/// already retired. No executor/native join or resource retirement is performed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_first_stop_release(slot: *mut *mut PrFirstStop) -> i32 {
    if slot.is_null() {
        return INVALID;
    }
    let raw = unsafe { *slot };
    if raw.is_null() {
        return OK;
    }
    if unsafe { owned(raw) }.is_none() {
        return INVALID;
    }
    unsafe {
        slot.write(ptr::null_mut());
        ptr::drop_in_place(raw);
        dealloc(raw.cast(), Layout::new::<PrFirstStop>());
    }
    OK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(origin: u32) -> FirstStopFactsV1 {
        FirstStopFactsV1 {
            abi_version: ABI,
            struct_size: 80,
            origin,
            ..Default::default()
        }
    }

    #[test]
    fn fixed_first_survives_counter_max_minus_one_and_max() {
        let mut state = FirstStopSnapshotV1::default();
        assert_eq!(
            record_with_clock(&mut state, facts(1), || ClockObservation::known(0)),
            OK
        );
        let first = state;
        state.stop_requests = u64::MAX - 1;
        for _ in 0..2 {
            assert_eq!(
                record_with_clock(&mut state, facts(TERMINAL_REOBSERVED), || {
                    panic!("later clock read")
                }),
                OK
            );
            assert_eq!(state.stop_requests, u64::MAX);
            assert_eq!(state.first, first.first);
            assert_eq!(
                (
                    state.has_first,
                    state.clock_known,
                    state.observed_boottime_ns,
                    state.clock_errno
                ),
                (1, 1, 0, 0)
            );
        }
    }

    #[test]
    fn unknown_clock_is_explicit_and_invalid_input_is_atomic() {
        let mut state = FirstStopSnapshotV1::default();
        let empty = state;
        assert_eq!(
            record_with_clock(&mut state, facts(TERMINAL_REOBSERVED), || {
                panic!("invalid clock read")
            }),
            INVALID
        );
        assert_eq!(state, empty);
        let mut bad = facts(1);
        bad.broker_pending_present = 1;
        assert_eq!(
            record_with_clock(&mut state, bad, || panic!("invalid clock read")),
            INVALID
        );
        assert_eq!(state, empty);
        assert_eq!(
            record_with_clock(&mut state, facts(2), || ClockObservation::unavailable(5)),
            OK
        );
        let first = state;
        assert_eq!(
            (
                state.stop_requests,
                state.has_first,
                state.clock_known,
                state.observed_boottime_ns,
                state.clock_errno
            ),
            (1, 1, 0, 0, 5)
        );
        bad = facts(1);
        bad.reserved[3] = 1;
        assert_eq!(
            record_with_clock(&mut state, bad, || panic!("invalid clock read")),
            INVALID
        );
        assert_eq!(state, first);
        assert_eq!(
            record_with_clock(&mut state, facts(1), || panic!("later clock read")),
            OK
        );
        assert_eq!(state.stop_requests, 2);
        assert_eq!(state.first, first.first);
        assert_eq!(
            (
                state.clock_known,
                state.observed_boottime_ns,
                state.clock_errno
            ),
            (0, 0, 5)
        );
    }
}
