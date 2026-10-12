//! One-shot suspend coordination over injected backend descriptors.
//! No timers, detached workers, host paths, or retained Binder ABI are provided.
mod clock;
pub use clock::{BootClock, Clock};
mod descriptors;
pub use descriptors::DescriptorBackend;

use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Ready,
    Stopped,
    Degraded,
    AuthorityLost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    ReadingCount,
    CommittingCount,
    EnteringMem,
}
/// A single atomic observation, including the terminal reason and current phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub lifecycle: Lifecycle,
    pub phase: Phase,
    pub wake_locks: u32,
    pub armed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    Returned,
    Blocked,
    Disabled,
    Busy,
    CountChanged,
    Cancelled,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    WriterBusy,
    Unavailable,
    AbortFailed,
    Capacity,
    Io,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendError {
    /// Only a backend that can positively identify count rejection may emit this.
    CountChanged,
    Cancelled,
    Uncertain,
}
#[derive(Clone)]
pub struct Cancellation(Arc<Control>);
impl Cancellation {
    pub fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
}
/// Backends must not detach workers. A successful mem write means only that the
/// injected write returned, not that hardware actually suspended or resumed.
pub trait Backend: Send + Sync + 'static {
    fn read_count(&self, cancel: &Cancellation) -> Result<Vec<u8>, BackendError>;
    fn commit_count(&self, count: &[u8], cancel: &Cancellation) -> Result<(), BackendError>;
    fn enter_mem(&self, cancel: &Cancellation) -> Result<(), BackendError>;
    fn begin_wake(&self) -> Result<(), BackendError>;
    fn finish_wake(&self) -> Result<(), BackendError>;
}
const LIFE: u64 = 3;
const PHASE: u64 = 3 << 2;
const ARMED: u64 = 1 << 4;
const COUNT_ONE: u64 = 1 << 8;
struct Control {
    state: AtomicU64,
    cancelled: AtomicBool,
}
impl Control {
    fn snapshot(&self) -> Snapshot {
        let s = self.state.load(Ordering::Acquire);
        Snapshot {
            lifecycle: match s & LIFE {
                0 => Lifecycle::Ready,
                1 => Lifecycle::Stopped,
                2 => Lifecycle::Degraded,
                _ => Lifecycle::AuthorityLost,
            },
            phase: match (s & PHASE) >> 2 {
                0 => Phase::Idle,
                1 => Phase::ReadingCount,
                2 => Phase::CommittingCount,
                _ => Phase::EnteringMem,
            },
            wake_locks: (s >> 8) as u32,
            armed: s & ARMED != 0,
        }
    }
    fn phase(&self, phase: Phase) {
        let n = match phase {
            Phase::Idle => 0,
            Phase::ReadingCount => 1,
            Phase::CommittingCount => 2,
            Phase::EnteringMem => 3,
        };
        let _ = self
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| {
                Some((s & !PHASE) | (n << 2))
            });
    }
    fn terminate(&self, lifecycle: Lifecycle) {
        self.cancelled.store(true, Ordering::Release);
        let n = match lifecycle {
            Lifecycle::Ready => 0,
            Lifecycle::Stopped => 1,
            Lifecycle::Degraded => 2,
            Lifecycle::AuthorityLost => 3,
        };
        let _ = self
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| {
                // Uncertainty takes precedence over a concurrent stop or controller loss.
                let next = if n == 2 || s & LIFE == 0 {
                    (s & !LIFE) | n
                } else {
                    s
                };
                Some(next & !ARMED)
            });
    }
}
struct BackendUnwind<'a> {
    control: &'a Control,
    reset_phase: bool,
}
impl Drop for BackendUnwind<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.control.terminate(Lifecycle::Degraded);
            if self.reset_phase {
                self.control.phase(Phase::Idle);
            }
        }
    }
}
// Admission is still protected by counter_gate. A backend unwind must not
// leave a counted object that was never returned to a caller.
struct Reservation<'a> {
    control: &'a Control,
    unpublished: bool,
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if self.unpublished {
            self.control.state.fetch_sub(COUNT_ONE, Ordering::AcqRel);
        }
    }
}
struct Inner {
    backend: Box<dyn Backend>,
    control: Arc<Control>,
    counter_gate: Mutex<()>,
    abort_gate: Mutex<()>,
    attempt_gate: Mutex<()>,
    _writer_guard: OwnedFd,
}
impl Inner {
    fn wait_gate<'a>(&self, mutex: &'a Mutex<()>) -> Result<MutexGuard<'a, ()>, Error> {
        loop {
            if self.control.cancelled.load(Ordering::Acquire) {
                return Err(Error::Unavailable);
            }
            match mutex.try_lock() {
                Ok(g) => return Ok(g),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
                Err(TryLockError::Poisoned(_)) => {
                    self.control.terminate(Lifecycle::Degraded);
                    return Err(Error::Unavailable);
                }
            }
        }
    }
    fn release(&self) {
        match self.counter_gate.lock() {
            Ok(_g) => {
                self.control.state.fetch_sub(COUNT_ONE, Ordering::AcqRel);
            }
            Err(_) => self.control.terminate(Lifecycle::Degraded),
        }
    }
}
#[derive(Clone)]
pub struct Coordinator(Arc<Inner>);
/// Dropping this authority permanently disables this coordinator instance.
/// It is a native lifetime primitive, not a remote Binder token.
pub struct Authority(Arc<Inner>);
impl Drop for Authority {
    fn drop(&mut self) {
        self.0.control.terminate(Lifecycle::AuthorityLost);
    }
}
struct WakeEntry {
    inner: Arc<Inner>,
    released: AtomicBool,
}
impl WakeEntry {
    fn release(&self) {
        if !self.released.swap(true, Ordering::AcqRel) {
            self.inner.release();
        }
    }
}
impl Drop for WakeEntry {
    fn drop(&mut self) {
        self.release();
    }
}
/// Clones share one object. Explicit release is idempotent across all clones;
/// otherwise the last reference releases it. There is no name deduplication.
#[derive(Clone)]
pub struct WakeLock(Arc<WakeEntry>);
impl WakeLock {
    pub fn release(&self) {
        self.0.release();
    }
}
impl Coordinator {
    /// The caller supplies an independently opened, shared writer-guard file.
    /// All coordinators for a backend must use that same file. The lock remains
    /// held while any authority, wake object, or attempt owner retains Inner.
    pub fn new(backend: impl Backend, writer_guard: OwnedFd) -> Result<Self, Error> {
        rustix::fs::flock(
            &writer_guard,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .map_err(|e| {
            if e == rustix::io::Errno::WOULDBLOCK {
                Error::WriterBusy
            } else {
                Error::Io
            }
        })?;
        Ok(Self(Arc::new(Inner {
            backend: Box::new(backend),
            control: Arc::new(Control {
                state: AtomicU64::new(0),
                cancelled: AtomicBool::new(false),
            }),
            counter_gate: Mutex::new(()),
            abort_gate: Mutex::new(()),
            attempt_gate: Mutex::new(()),
            _writer_guard: writer_guard,
        })))
    }
    pub fn arm(&self) -> Result<Authority, Error> {
        let _g = self.0.wait_gate(&self.0.counter_gate)?;
        self.0
            .control
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| {
                if s & (LIFE | ARMED) == 0 {
                    Some(s | ARMED)
                } else {
                    None
                }
            })
            .map_err(|_| Error::Unavailable)?;
        Ok(Authority(self.0.clone()))
    }
    pub fn acquire(&self) -> Result<WakeLock, Error> {
        // Serialize the shared kernel abort name so one waiter cannot clear
        // another waiter's abort before counter admission.
        let _abort = self.0.wait_gate(&self.0.abort_gate)?;
        let _unwind = BackendUnwind {
            control: &self.0.control,
            reset_phase: false,
        };
        if self.0.backend.begin_wake().is_err() {
            self.0.control.terminate(Lifecycle::Degraded);
            return Err(Error::AbortFailed);
        }
        // Android 17's ordering: kernel abort first, then counter gate.
        let gate = self.0.wait_gate(&self.0.counter_gate);
        let result = match gate {
            Ok(_g) => {
                let s = self.snapshot();
                if s.lifecycle != Lifecycle::Ready {
                    Err(Error::Unavailable)
                } else if s.wake_locks == u32::MAX {
                    Err(Error::Capacity)
                } else {
                    if self
                        .0
                        .control
                        .state
                        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                            if state & LIFE == 0 {
                                Some(state + COUNT_ONE)
                            } else {
                                None
                            }
                        })
                        .is_err()
                    {
                        if self.0.backend.finish_wake().is_err() {
                            self.0.control.terminate(Lifecycle::Degraded);
                        }
                        return Err(Error::Unavailable);
                    }
                    let mut reservation = Reservation {
                        control: &self.0.control,
                        unpublished: true,
                    };
                    if self.0.backend.finish_wake().is_err() {
                        self.0.control.terminate(Lifecycle::Degraded);
                        return Err(Error::AbortFailed);
                    } else {
                        let lock = WakeLock(Arc::new(WakeEntry {
                            inner: self.0.clone(),
                            released: AtomicBool::new(false),
                        }));
                        reservation.unpublished = false;
                        return Ok(lock);
                    }
                }
            }
            Err(e) => Err(e),
        };
        if self.0.backend.finish_wake().is_err() {
            self.0.control.terminate(Lifecycle::Degraded);
        }
        result
    }
    pub fn attempt(&self) -> Attempt {
        if self.snapshot().lifecycle != Lifecycle::Ready {
            return Attempt::Disabled;
        }
        let _attempt = match self.0.attempt_gate.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => return Attempt::Busy,
            Err(TryLockError::Poisoned(_)) => {
                self.0.control.terminate(Lifecycle::Degraded);
                return Attempt::Uncertain;
            }
        };
        let s = self.snapshot();
        if s.lifecycle != Lifecycle::Ready || !s.armed {
            return Attempt::Disabled;
        }
        let _unwind = BackendUnwind {
            control: &self.0.control,
            reset_phase: true,
        };
        let cancel = Cancellation(self.0.control.clone());
        self.0.control.phase(Phase::ReadingCount);
        let result = self.attempt_inner(&cancel);
        self.0.control.phase(Phase::Idle);
        result
    }
    fn attempt_inner(&self, cancel: &Cancellation) -> Attempt {
        let count = match self.0.backend.read_count(cancel) {
            Ok(c) if !c.is_empty() && c.len() <= 20 && c.iter().all(u8::is_ascii_digit) => c,
            Err(BackendError::Cancelled) => return Attempt::Cancelled,
            _ => {
                self.0.control.terminate(Lifecycle::Degraded);
                return Attempt::Uncertain;
            }
        };
        let _gate = match self.0.wait_gate(&self.0.counter_gate) {
            Ok(g) => g,
            Err(_) => return Attempt::Cancelled,
        };
        let s = self.snapshot();
        if s.lifecycle != Lifecycle::Ready || !s.armed || cancel.cancelled() {
            return Attempt::Cancelled;
        }
        if s.wake_locks != 0 {
            return Attempt::Blocked;
        }
        // The same gate spans count commit and mem. No second writer can enter.
        self.0.control.phase(Phase::CommittingCount);
        match self.0.backend.commit_count(&count, cancel) {
            Ok(()) => {}
            Err(BackendError::CountChanged) => return Attempt::CountChanged,
            Err(_) => {
                self.0.control.terminate(Lifecycle::Degraded);
                return Attempt::Uncertain;
            }
        }
        if cancel.cancelled() {
            return Attempt::Cancelled;
        }
        self.0.control.phase(Phase::EnteringMem);
        if self.0.backend.enter_mem(cancel).is_err() {
            self.0.control.terminate(Lifecycle::Degraded);
            return Attempt::Uncertain;
        }
        Attempt::Returned
    }
    /// Nonblocking cancellation. The caller still owns and must join its worker.
    pub fn stop(&self) {
        self.0.control.terminate(Lifecycle::Stopped);
    }
    pub fn snapshot(&self) -> Snapshot {
        self.0.control.snapshot()
    }
}
