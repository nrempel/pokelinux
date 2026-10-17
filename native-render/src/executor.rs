//! Retained render worker and CPU-only control path.
//!
//! NativeOwner is the trusted, worker-local adapter boundary. A factory is a
//! copied function pointer, never a captured control/Wayland/graphics object.
//! Its production AQ/GL binding and authenticated device scope remain required.
use crate::{
    bootstrap::{OwnedBootstrap, SessionRenderEvent},
    commit::{CommitAuthority, CommitTicket, GateError, OperationPolicy, SealedAttempt},
    frame::Frame,
};
use std::{
    collections::VecDeque,
    io,
    marker::PhantomData,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle, ThreadId},
};

const FRAME_CREDITS: u32 = 3;
const COMPLETION_CAPACITY: usize = 128;
const RECORDS_PER_FRAME: u32 = 5;
const MAX_NATIVE_SOURCES: usize = 16;
pub const MAX_SESSION_CHANGES: usize = 32;
const RECOVERY_BUDGET_NS: u64 = 20_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollSource {
    pub fd: i32,
    pub token: u64,
    pub generation: u64,
    pub events: i16,
}
impl PollSource {
    pub const EMPTY: Self = Self {
        fd: -1,
        token: 0,
        generation: 0,
        events: 0,
    };
}
#[derive(Clone, Copy, Debug)]
pub struct NativeReady {
    pub source: PollSource,
    pub revents: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FenceFact {
    pub identity: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationClock {
    Monotonic,
    Realtime,
    Boottime,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Preserve the backend's raw sequence/timestamp, including zero. The clock
/// domain is explicit; no monotonic-to-BOOTTIME relabelling is performed.
pub struct PresentationFact {
    pub correlation_id: u64,
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub clock: PresentationClock,
}
pub enum RetirementProgress {
    Pending,
    Verified(RetirementProof),
}
pub struct NativeProgress {
    pub fence: Option<FenceFact>,
    pub presentation: Option<PresentationFact>,
    pub retirement: RetirementProgress,
}
impl NativeProgress {
    pub fn pending() -> Self {
        Self {
            fence: None,
            presentation: None,
            retirement: RetirementProgress::Pending,
        }
    }
    pub fn retired() -> Self {
        Self {
            fence: None,
            presentation: None,
            retirement: RetirementProgress::Verified(RetirementProof),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Attempt,
    /// Immediate attempt outcome, including refusal, failure or uncertainty.
    /// This variant alone NEVER proves actual successful submission.
    SubmissionOutcome,
    Fence(FenceFact),
    Presented(PresentationFact),
    Retired,
}
/// Copied historical fact, never a ticket or authority for another native call.
#[derive(Clone, Copy, Debug)]
pub struct GrantObservation {
    serial: u64,
    policy: OperationPolicy,
    at_boottime_ns: u64,
}
impl GrantObservation {
    pub fn serial(&self) -> u64 {
        self.serial
    }
    pub fn policy(&self) -> OperationPolicy {
        self.policy
    }
    pub fn granted_at_boottime_ns(&self) -> u64 {
        self.at_boottime_ns
    }
}
impl From<&SealedAttempt> for GrantObservation {
    fn from(value: &SealedAttempt) -> Self {
        Self {
            serial: value.serial(),
            policy: value.policy(),
            at_boottime_ns: value.granted_at_boottime_ns(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Busy,
    Capacity,
    Stopping,
    Stale,
    Expired,
    Clock,
    Exhausted,
    Native,
    Protocol,
    WorkerPanic,
    Wake,
}
impl From<GateError> for Error {
    fn from(value: GateError) -> Self {
        match value {
            GateError::Invalid => Self::Invalid,
            GateError::Stale => Self::Stale,
            GateError::Stopping => Self::Stopping,
            GateError::Expired => Self::Expired,
            GateError::Clock => Self::Clock,
            GateError::Exhausted => Self::Exhausted,
        }
    }
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Starting = 0,
    Bootstrapping = 8,
    Idle = 1,
    Preparing = 2,
    Executing = 3,
    Retiring = 4,
    Destroying = 5,
    Quarantined = 6,
    Exited = 7,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    pub id: u64,
    pub policy: OperationPolicy,
    pub control_cookie: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeObject {
    pub id: u32,
    pub generation: u64,
    pub output_id: u64,
    pub output_generation: u64,
}
/// Values resolved by the trusted native adapter from the ACTUAL complete
/// request, including old/new routing. Shape checks here do not authenticate
/// these values. Unknown/multi-output effects must be refused by that adapter.
pub struct NativeScope<'a> {
    pub policy: OperationPolicy,
    pub device_lease_id: u64,
    pub device_generation: u64,
    pub objects: &'a [NativeObject],
}
/// The adapter borrows this on the worker stack ONLY at the final native call.
/// An earlier top-level output commit wrapper is not that boundary. The actual
/// outcome is separate from the grant and may outlive revocation/Stop.
#[derive(Clone)]
pub struct RecordedNativeScope {
    pub policy: OperationPolicy,
    pub device_lease_id: u64,
    pub device_generation: u64,
    count: usize,
    objects: [NativeObject; 128],
}
impl RecordedNativeScope {
    pub fn objects(&self) -> &[NativeObject] {
        &self.objects[..self.count]
    }
}
pub struct Attempt {
    expected_device: (u64, u64),
    scope: Option<RecordedNativeScope>,
    ticket: Option<CommitTicket>,
    policy: OperationPolicy,
    sealed: Option<SealedAttempt>,
}
impl Attempt {
    pub(crate) fn reject_invalid(&mut self) -> Result<(), Error> {
        self.ticket.take().ok_or(Error::Stale)?;
        Err(Error::Invalid)
    }
    pub fn authorize(&mut self, actual: &NativeScope<'_>) -> Result<&SealedAttempt, Error> {
        let ticket = self.ticket.take().ok_or(Error::Stale)?;
        if actual.policy != self.policy
            || actual.device_lease_id == 0
            || actual.device_generation == 0
            || (actual.device_lease_id, actual.device_generation) != self.expected_device
            || actual.objects.is_empty()
            || actual.objects.len() > 128
            || actual.objects.iter().enumerate().any(|(i, o)| {
                o.id == 0
                    || o.generation == 0
                    || o.output_id != self.policy.output_id
                    || o.output_generation != self.policy.output_generation
                    || actual.objects[..i].iter().any(|p| p.id == o.id)
            })
        {
            return Err(Error::Invalid);
        }
        // Reserve/copy the complete bounded scope BEFORE the one-shot grant.
        // No request pointer or post-grant allocation survives this boundary.
        let mut objects = [NativeObject {
            id: 0,
            generation: 0,
            output_id: 0,
            output_generation: 0,
        }; 128];
        objects[..actual.objects.len()].copy_from_slice(actual.objects);
        let scope = RecordedNativeScope {
            policy: actual.policy,
            device_lease_id: actual.device_lease_id,
            device_generation: actual.device_generation,
            count: actual.objects.len(),
            objects,
        };
        self.sealed = Some(ticket.seal()?);
        self.scope = Some(scope);
        Ok(self.sealed.as_ref().expect("sealed value just installed"))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeResult {
    NotEntered,
    /// A grant exists but native entry/effects were not reported reliably.
    Unknown,
    Returned {
        operation: u32,
        value: i64,
        errno: i32,
        succeeded: bool,
        extra_attempt_refused: bool,
    },
}
pub struct RetirementContext<'a> {
    pub admission: Admission,
    pub grant: Option<&'a SealedAttempt>,
    pub native: NativeResult,
    pub scope: Option<&'a RecordedNativeScope>,
}
/// A trusted adapter reports this only after actual descendant lifetime drain.
/// Its construction is not independent evidence of that real backend behavior.
pub struct RetirementProof;
pub struct Completion {
    pub kind: CompletionKind,
    pub scope: Option<RecordedNativeScope>,
    pub admission: Admission,
    pub grant: Option<GrantObservation>,
    pub native: NativeResult,
    pub resources_retired: bool,
    pub error: Option<Error>,
}

/// Copied observations from the worker's real output registry. These IDs are
/// local registry keys, not kernel object IDs or permission by themselves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiscoveredOutput {
    pub output_id: u64,
    pub output_generation: u64,
    pub device_lease_id: u64,
    pub device_generation: u64,
    pub mode_id: u64,
    pub mode_generation: u64,
    pub modifier: u64,
    pub connector_id: u32,
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
    pub drm_format: u32,
    pub flags: u32,
}
pub const MAX_DISCOVERED_OUTPUTS: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Discovery {
    pub session_generation: u64,
    pub output_count: u32,
    pub outputs: [DiscoveredOutput; MAX_DISCOVERED_OUTPUTS],
}
// Value identity is captured before the owner takes the FD pack. This binds
// observations to the transported input, not to actual DRM routing authority.
#[derive(Clone, Copy)]
struct BootstrapIdentity {
    session_generation: u64,
    count: usize,
    devices: [(u64, u64, u64); crate::bootstrap::MAX_BOOTSTRAP_DEVICES],
    end_sequence: u64,
}
impl BootstrapIdentity {
    fn capture(input: &OwnedBootstrap) -> Self {
        let mut value = Self {
            session_generation: input.session_generation(),
            count: 0,
            devices: [(0, 0, 0); crate::bootstrap::MAX_BOOTSTRAP_DEVICES],
            end_sequence: input
                .events()
                .last()
                .expect("complete initial prefix")
                .metadata()
                .session_sequence,
        };
        for event in input
            .events()
            .iter()
            .filter(|v| v.metadata().event_type == 2)
        {
            let device = event.metadata();
            value.devices[value.count] = (
                device.device_lease_id,
                device.device_generation,
                device.rdev,
            );
            value.count += 1;
        }
        value
    }
    fn accepts_change(&self, value: &SessionRenderEvent) -> bool {
        value.version == 1
            && value.event_type == 5
            && value.session_generation == self.session_generation
            && self.devices[..self.count].contains(&(
                value.device_lease_id,
                value.device_generation,
                value.rdev,
            ))
            && value.change_type <= 1
            && value.active <= 1
            && value.boot_vga <= 1
            && value.reserved == [0; 4]
            && value.seat_name.contains(&0)
            && value.device_path.contains(&0)
    }
    fn accepts(&self, value: &Discovery) -> bool {
        if value.session_generation != self.session_generation
            || value.output_count as usize > MAX_DISCOVERED_OUTPUTS
        {
            return false;
        }
        let count = value.output_count as usize;
        if value.outputs[count..]
            .iter()
            .any(|v| *v != DiscoveredOutput::default())
        {
            return false;
        }
        for (i, output) in value.outputs[..count].iter().enumerate() {
            if output.output_id == 0
                || output.output_generation == 0
                || !self.devices[..self.count].iter().any(|device| {
                    (device.0, device.1) == (output.device_lease_id, output.device_generation)
                })
                || output.mode_id == 0
                || output.mode_generation == 0
                || output.connector_id == 0
                || !(1..=16_384).contains(&output.width)
                || !(1..=16_384).contains(&output.height)
                || output.refresh_millihz == 0
                || output.drm_format == 0
                || output.flags & !1 != 0
                || value.outputs[..i].iter().any(|prior| {
                    prior.output_id == output.output_id
                        || ((
                            prior.device_lease_id,
                            prior.device_generation,
                            prior.connector_id,
                        ) == (
                            output.device_lease_id,
                            output.device_generation,
                            output.connector_id,
                        ))
                })
            {
                return false;
            }
        }
        true
    }
}
/// Implementations are constructed, used and destroyed on the single worker.
/// They need NOT implement Send. prepare/execute can block; progress and
/// dispatch must return bounded work without waiting for another submission; no control mutex is held over any of them. prepare cannot retain a
/// borrow beyond the frame's worker-owned lifetime. Verified progress must
/// actually drain native descendants; an FD close alone is insufficient. Err leaves custody
/// quarantined on this live worker, never silently released or retried.
/// Adapter calls must keep owned resources in the owner on error/panic; locals
/// must not own an untracked native descendant. After successful shutdown all
/// native resources are gone; its remaining Rust destructor is CPU-only.
pub trait NativeOwner {
    /// The deferred holder is retained before this potentially blocking call.
    /// Take the input only into owner-held state before fallible native work.
    /// Return bounded copied output observations, never AQ pointers/callbacks.
    fn bootstrap(&mut self, input: &mut Option<OwnedBootstrap>) -> Result<Discovery, Error>;
    /// Worker-only single application plus fresh registry query. The adapter
    /// must deny display calls and retain partial native custody on failure.
    /// No callback is retried and no control mutex is held while it runs.
    fn apply_device_change(&mut self, event: &SessionRenderEvent) -> Result<Discovery, Error>;
    fn prepare(&mut self, frame: &Frame, admission: Admission) -> Result<(), Error>;
    fn execute(
        &mut self,
        frame: &Frame,
        admission: Admission,
        attempt: &mut Attempt,
    ) -> Result<NativeResult, Error>;
    fn advance(&mut self, context: &RetirementContext<'_>) -> Result<NativeProgress, Error>;
    /// FDs remain owned by this worker-local adapter. No callbacks or pointers
    /// escape. Each token/generation must change when a source is replaced.
    fn poll_sources(
        &mut self,
        storage: &mut [PollSource; MAX_NATIVE_SOURCES],
    ) -> Result<usize, Error>;
    fn dispatch(&mut self, ready: NativeReady) -> Result<(), Error>;
    /// Progress native cleanup even with live entries. Stop grants no modeset,
    /// disable or blackout permission; require a separate authentic StopRetire
    /// ledger at every display-changing native attempt. Verified is aggregate
    /// native drain, and each retained entry still needs its own verified drain.
    fn shutdown_progress(&mut self) -> Result<RetirementProgress, Error>;
}
/// The factory creates a deferred holder only. It must not acquire native
/// custody in a local that could unwind before returning the retained owner.
pub struct OwnerFactoryConfig {
    pub(crate) native: crate::executor_ffi::NativeOwnerFunctionsV2,
    pub(crate) options: crate::executor_ffi::OwnerOptionsV1,
}
pub type OwnerFactory = fn(OwnerFactoryConfig) -> Result<Box<dyn NativeOwner>, Error>;

/// One admission path for Rust values and C handle slots. The owned value is
/// taken only after every refusal point; C never has to move/rebox on refusal.
pub(crate) trait FrameCustody {
    fn frame(&self) -> Option<&Frame>;
    fn take_frame(&mut self) -> Frame;
}
impl FrameCustody for Option<Frame> {
    fn frame(&self) -> Option<&Frame> {
        self.as_ref()
    }
    fn take_frame(&mut self) -> Frame {
        self.take().expect("validated owned frame")
    }
}

struct EventFd(OwnedFd, #[cfg(test)] AtomicBool);
unsafe extern "C" {
    fn eventfd(init: u32, flags: i32) -> i32;
    fn read(fd: i32, data: *mut std::ffi::c_void, count: usize) -> isize;
    fn write(fd: i32, data: *const std::ffi::c_void, count: usize) -> isize;
    fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
    fn timerfd_create(clock: i32, flags: i32) -> i32;
    fn timerfd_settime(fd: i32, flags: i32, value: *const TimerSpec, old: *mut TimerSpec) -> i32;
}
#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}
impl EventFd {
    fn new() -> Result<Self, Error> {
        let fd = unsafe { eventfd(0, 0x800 | 0x80000) }; // Linux NONBLOCK | CLOEXEC
        if fd < 0 {
            return Err(Error::Wake);
        }
        Ok(Self(
            unsafe { OwnedFd::from_raw_fd(fd) },
            #[cfg(test)]
            AtomicBool::new(false),
        ))
    }
    fn notify(&self) -> Result<(), Error> {
        #[cfg(test)]
        if self.1.swap(false, Ordering::AcqRel) {
            return Err(Error::Wake);
        }
        let value = 1u64;
        loop {
            let n = unsafe { write(self.0.as_raw_fd(), (&value as *const u64).cast(), 8) };
            if n == 8 {
                return Ok(());
            }
            let e = io::Error::last_os_error();
            if n < 0 && e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            // A full eventfd is already readable; the durable record remains.
            if n < 0 && e.kind() == io::ErrorKind::WouldBlock {
                return Ok(());
            }
            return Err(Error::Wake);
        }
    }
    fn clear(&self) -> Result<(), Error> {
        loop {
            let mut value = 0u64;
            let n = unsafe { read(self.0.as_raw_fd(), (&mut value as *mut u64).cast(), 8) };
            if n == 8 {
                continue;
            }
            let e = io::Error::last_os_error();
            if n < 0 && e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if n < 0 && e.kind() == io::ErrorKind::WouldBlock {
                return Ok(());
            }
            return Err(Error::Wake);
        }
    }
    fn wait(&self) -> Result<(), Error> {
        let mut p = PollFd {
            fd: self.0.as_raw_fd(),
            events: 1,
            revents: 0,
        };
        loop {
            let n = unsafe { poll(&mut p, 1, -1) };
            if n > 0 {
                return if p.revents & !1 == 0 {
                    self.clear()
                } else {
                    Err(Error::Wake)
                };
            }
            if n < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Error::Wake);
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimerTime {
    seconds: std::ffi::c_long,
    nanoseconds: std::ffi::c_long,
}
#[repr(C)]
#[derive(Default)]
struct TimerSpec {
    interval: TimerTime,
    value: TimerTime,
}
struct StartupDeadline {
    fd: OwnedFd,
}
impl StartupDeadline {
    fn new(until: u64) -> Result<Self, Error> {
        let fd = unsafe { timerfd_create(7, 0x800 | 0x80000) }; // CLOCK_BOOTTIME, NONBLOCK | CLOEXEC
        if fd < 0 {
            return Err(Error::Wake);
        }
        let value = Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        };
        value.arm(until)?;
        Ok(value)
    }
    fn arm(&self, until: u64) -> Result<(), Error> {
        let spec = TimerSpec {
            interval: TimerTime::default(),
            value: TimerTime {
                seconds: (until / 1_000_000_000)
                    .try_into()
                    .map_err(|_| Error::Invalid)?,
                nanoseconds: (until % 1_000_000_000) as std::ffi::c_long,
            },
        };
        if unsafe { timerfd_settime(self.fd.as_raw_fd(), 1, &spec, std::ptr::null_mut()) } != 0 {
            return Err(Error::Wake);
        }
        Ok(())
    }
    fn disarm(&self) -> Result<(), Error> {
        if unsafe {
            timerfd_settime(
                self.fd.as_raw_fd(),
                0,
                &TimerSpec::default(),
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(Error::Wake);
        }
        loop {
            let mut value = 0u64;
            let result = unsafe { read(self.fd.as_raw_fd(), (&mut value as *mut u64).cast(), 8) };
            if result == 8 {
                continue;
            }
            let error = io::Error::last_os_error();
            if result < 0 && error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if result < 0 && error.kind() == io::ErrorKind::WouldBlock {
                return Ok(());
            }
            return Err(Error::Wake);
        }
    }
}
struct Job {
    expected_device: (u64, u64),
    frame: Frame,
    admission: Admission,
    ticket: CommitTicket,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionReconciliationV2 {
    pub session_generation: u64,
    pub invalidation_epoch: u64,
    pub admitted_epoch: u64,
    pub applied_epoch: u64,
    pub accepted_epoch: u64,
    pub last_admitted_sequence: u64,
    pub last_applied_sequence: u64,
    pub discovery_sequence: u64,
    pub bootstrap_end_sequence: u64,
    pub reconcile_not_after_boottime_ns: u64,
}
#[derive(Clone, Copy)]
struct SessionChange {
    epoch: u64,
    event: SessionRenderEvent,
}
#[derive(Clone, Copy)]
struct DiscoveryCandidate {
    epoch: u64,
    sequence: u64,
    discovery: Discovery,
}
struct SessionState {
    changes: VecDeque<SessionChange>,
    admitted_epoch: u64,
    applied_epoch: u64,
    last_admitted_sequence: u64,
    last_applied_sequence: u64,
    candidate: Option<DiscoveryCandidate>,
}
struct Shared {
    ingress: Mutex<VecDeque<Job>>,
    completions: Mutex<VecDeque<Completion>>,
    identity: BootstrapIdentity,
    session: Mutex<SessionState>,
    invalidation_epoch: AtomicU64,
    accepted_epoch: AtomicU64,
    queued_changes: AtomicU32,
    reconcile_until: AtomicU64,
    ever_ready: AtomicBool,
    beginning_recovery: AtomicBool,
    startup: AtomicU32, // 0 starting, 3 copied candidate, 1 control accepted, 2 terminal
    startup_deadline: StartupDeadline,
    stop: AtomicBool,
    alive: AtomicBool,
    stage: AtomicU32,
    credits: AtomicU32,
    reserved_records: AtomicU32,
    queued: AtomicU32,
    accepted: AtomicU64,
    retired: AtomicU64,
    failed: AtomicBool,
    wake: EventFd,
    complete: EventFd,
    stop_gate: crate::commit::StopHandle,
}
impl Shared {
    fn stop(&self) -> Result<(), Error> {
        // Stop is independent of ingress/completion mutexes and every native call.
        self.startup.store(2, Ordering::Release);
        self.stop_gate.stop();
        self.stop.store(true, Ordering::Release);
        let wake = self.wake.notify();
        let timer = self.startup_deadline.disarm();
        if timer.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        wake.and(timer)
    }
    fn available(&self) -> bool {
        !self.stop.load(Ordering::Acquire)
            && !self.beginning_recovery.load(Ordering::Acquire)
            && self.startup.load(Ordering::Acquire) == 1
            && self.invalidation_epoch.load(Ordering::Acquire)
                == self.accepted_epoch.load(Ordering::Acquire)
    }
    fn terminal(&self, error: Error) -> Error {
        self.failed.store(true, Ordering::Release);
        let _ = self.stop();
        let _ = self.complete.notify();
        error
    }
    fn deadline_eligible(&self) -> bool {
        if self.stop.load(Ordering::Acquire) || self.startup.load(Ordering::Acquire) == 2 {
            return false;
        }
        // An in-progress clean post-Ready begin has already fenced authority,
        // but has not published its new cutoff yet. Defer without using the old
        // deadline. Only serialized control may turn an expired sample into Stop.
        if self.beginning_recovery.load(Ordering::Acquire) {
            return false;
        }
        if self.available() {
            return true;
        }
        let until = self.reconcile_until.load(Ordering::Acquire);
        let now = crate::commit::boottime_ns();
        match now {
            Ok(now) if now < until => true,
            Ok(_) => false,
            Err(_) => {
                self.terminal(Error::Clock);
                false
            }
        }
    }
    // Only serialized control may expire the active cutoff. A worker's sample
    // can straddle control acceptance and a new burst even with version rereads;
    // it may reject work, but must never Stop that new burst using an old sample.
    fn startup_before_deadline(&self) -> bool {
        let eligible = self.deadline_eligible();
        if !eligible
            && !self.stop.load(Ordering::Acquire)
            && !self.beginning_recovery.load(Ordering::Acquire)
        {
            self.terminal(Error::Expired);
        }
        eligible
    }
    fn worker_before_deadline(&self) -> bool {
        let eligible = self.deadline_eligible();
        if !eligible && self.complete.notify().is_err() {
            self.quarantine();
        }
        eligible
    }
    // A candidate is never availability on its own. Only this control-thread
    // observer accepts the latest epoch before the active fixed deadline.
    fn observe_startup(&self) -> Result<(), Error> {
        if self.stop.load(Ordering::Acquire) || self.available() {
            return Ok(());
        }
        if !self.startup_before_deadline() {
            return Ok(());
        }
        let session = match self.session.try_lock() {
            Ok(value) => value,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
            Err(_) => return Err(self.terminal(Error::Protocol)),
        };
        let epoch = self.invalidation_epoch.load(Ordering::Acquire);
        if session
            .candidate
            .as_ref()
            .is_none_or(|candidate| candidate.epoch != epoch)
            || session.admitted_epoch != epoch
            || session.applied_epoch != epoch
        {
            return Ok(());
        }
        self.startup_deadline
            .disarm()
            .map_err(|e| self.terminal(e))?;
        if !self.startup_before_deadline() {
            return Ok(());
        }
        if self
            .startup
            .compare_exchange(3, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.accepted_epoch.store(epoch, Ordering::Release);
            self.ever_ready.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn publish_discovery(&self, value: Discovery, epoch: u64, sequence: u64) {
        if !self.identity.accepts(&value) {
            self.terminal(Error::Protocol);
            return;
        }
        if !self.worker_before_deadline() {
            return;
        }
        let mut session = match self.session.lock() {
            Ok(value) => value,
            Err(_) => self.quarantine(),
        };
        // Native apply is once and ordered. A later invalidate may make this
        // perfectly valid result stale; retain the candidate, never grant it.
        if epoch != 0 && session.applied_epoch.checked_add(1) != Some(epoch) {
            drop(session);
            self.quarantine();
        }
        session.applied_epoch = epoch;
        session.last_applied_sequence = sequence;
        session.candidate = Some(DiscoveryCandidate {
            epoch,
            sequence,
            discovery: value,
        });
        let mut state = self.startup.load(Ordering::Acquire);
        while state != 2 {
            match self
                .startup
                .compare_exchange(state, 3, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(actual) => state = actual,
            }
        }
        drop(session);
        if self.complete.notify().is_err() {
            self.quarantine();
        }
    }
    fn publish(&self, value: Completion) -> Result<(), Error> {
        let mut queue = self.completions.lock().map_err(|_| Error::Protocol)?;
        if queue.len() == COMPLETION_CAPACITY {
            return Err(Error::Protocol);
        }
        queue.push_back(value); // capacity reserved before worker construction
        self.complete.notify()
    }
    fn quarantine(&self) -> ! {
        self.failed.store(true, Ordering::Release);
        let _ = self.stop();
        self.stage
            .store(Stage::Quarantined as u32, Ordering::Release);
        let _ = self.complete.notify();
        // This worker and its native owner/frame retain custody. No retry,
        // replacement, detach, resource-drop or successful drain is fabricated.
        loop {
            thread::park();
        }
    }
}
pub struct Snapshot {
    pub stage: Stage,
    pub worker_alive: bool,
    pub stopping: bool,
    pub ready: bool,
    pub failed: bool,
    pub occupied_credits: u32,
    pub reserved_completion_records: u32,
    pub queued_frames: u32,
    pub accepted: u64,
    pub retired: u64,
    pub joined: bool,
}
/// Control-thread-only owner. Normal release uses try_join then try_release.
/// Rust Drop is a last-resort stop+JOIN and can block indefinitely; it never
/// detaches an alive worker. It must not run in the responsive Source path.
#[must_use]
pub struct Executor {
    control: ThreadId,
    shared: Arc<Shared>,
    authority: CommitAuthority,
    security_generation: u64,
    next_id: u64,
    handle: Option<JoinHandle<()>>,
    joined: bool,
    discovered: Option<Discovery>,
    _control_only: PhantomData<Rc<()>>,
}
impl Executor {
    pub fn new(
        security_generation: u64,
        factory: OwnerFactory,
        config: OwnerFactoryConfig,
        bootstrap: OwnedBootstrap,
        startup_not_after_boottime_ns: u64,
    ) -> Result<Self, Error> {
        if crate::commit::boottime_ns()? >= startup_not_after_boottime_ns {
            return Err(Error::Expired);
        }
        let authority = CommitAuthority::new(security_generation)?;
        let startup_deadline = StartupDeadline::new(startup_not_after_boottime_ns)?;
        let identity = BootstrapIdentity::capture(&bootstrap);
        let shared = Arc::new(Shared {
            ingress: Mutex::new(VecDeque::with_capacity(FRAME_CREDITS as usize)),
            completions: Mutex::new(VecDeque::with_capacity(COMPLETION_CAPACITY)),
            identity,
            session: Mutex::new(SessionState {
                changes: VecDeque::with_capacity(MAX_SESSION_CHANGES),
                admitted_epoch: 0,
                applied_epoch: 0,
                last_admitted_sequence: identity.end_sequence,
                last_applied_sequence: 0,
                candidate: None,
            }),
            invalidation_epoch: AtomicU64::new(0),
            accepted_epoch: AtomicU64::new(0),
            queued_changes: AtomicU32::new(0),
            reconcile_until: AtomicU64::new(startup_not_after_boottime_ns),
            ever_ready: AtomicBool::new(false),
            beginning_recovery: AtomicBool::new(false),
            startup: AtomicU32::new(0),
            startup_deadline,
            stop: AtomicBool::new(false),
            alive: AtomicBool::new(true),
            stage: AtomicU32::new(Stage::Starting as u32),
            credits: AtomicU32::new(0),
            reserved_records: AtomicU32::new(0),
            queued: AtomicU32::new(0),
            accepted: AtomicU64::new(0),
            retired: AtomicU64::new(0),
            failed: AtomicBool::new(false),
            wake: EventFd::new()?,
            complete: EventFd::new()?,
            stop_gate: authority.stop_handle(),
        });
        let worker = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("pokelinux-render".into())
            .spawn(move || run(worker, factory, config, Some(bootstrap)))
            .map_err(|_| Error::Native)?;
        Ok(Self {
            control: thread::current().id(),
            shared,
            authority,
            security_generation,
            next_id: 0,
            handle: Some(handle),
            joined: false,
            discovered: None,
            _control_only: PhantomData,
        })
    }
    fn control(&self) -> Result<(), Error> {
        if thread::current().id() == self.control {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
    /// On every refusal the frame stays in its original slot. Ordinary refusal
    /// preserves earlier authority; terminal serial exhaustion fail-closes it. All credits/queue capacity reserve before issue/CAS.
    pub fn try_submit(
        &mut self,
        frame: &mut Option<Frame>,
        policy: OperationPolicy,
        cutoff: u64,
        cookie: u64,
    ) -> Result<Admission, Error> {
        self.try_submit_custody(frame, policy, cutoff, cookie)
    }
    pub(crate) fn try_submit_custody(
        &mut self,
        frame: &mut impl FrameCustody,
        policy: OperationPolicy,
        cutoff: u64,
        cookie: u64,
    ) -> Result<Admission, Error> {
        self.control()?;
        self.shared.observe_startup()?;
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(Error::Stopping);
        }
        if !self.shared.available() || self.discovered.is_none() {
            return Err(Error::Busy);
        }
        let f = frame.frame().ok_or(Error::Invalid)?;
        if f.security_generation() != policy.security_generation
            || f.scene_sequence() != policy.scene_sequence
            || f.target().output_id != policy.output_id
            || f.target().output_generation != policy.output_generation
        {
            return Err(Error::Invalid);
        }
        let discovery = self.discovered.as_ref().ok_or(Error::Busy)?;
        if policy.session_generation != discovery.session_generation {
            return Err(Error::Invalid);
        }
        let output = discovery.outputs[..discovery.output_count as usize]
            .iter()
            .find(|output| {
                output.output_id == policy.output_id
                    && output.output_generation == policy.output_generation
                    && output.width == f.target().width
                    && output.height == f.target().height
            })
            .ok_or(Error::Invalid)?;
        let expected_device = (output.device_lease_id, output.device_generation);
        let id = self.next_id.checked_add(1).ok_or(Error::Exhausted)?;
        let mut queue = self.shared.ingress.try_lock().map_err(|_| Error::Busy)?;
        if self.shared.credits.load(Ordering::Acquire) >= FRAME_CREDITS
            || self.shared.reserved_records.load(Ordering::Acquire) + RECORDS_PER_FRAME
                > COMPLETION_CAPACITY as u32
        {
            return Err(Error::Capacity);
        }
        let admission = Admission {
            id,
            policy,
            control_cookie: cookie,
        };
        let ticket = self.authority.issue(policy, cutoff)?;
        // After successful publication no allocation/fallible admission remains.
        self.shared
            .reserved_records
            .fetch_add(RECORDS_PER_FRAME, Ordering::AcqRel);
        self.shared.credits.fetch_add(1, Ordering::AcqRel);
        self.shared.queued.fetch_add(1, Ordering::AcqRel);
        queue.push_back(Job {
            expected_device,
            frame: frame.take_frame(),
            admission,
            ticket,
        });
        self.next_id = id;
        self.shared.accepted.fetch_add(1, Ordering::AcqRel);
        drop(queue);
        if self.shared.wake.notify().is_err() {
            self.shared.failed.store(true, Ordering::Release);
            let _ = self.shared.stop();
        }
        Ok(admission)
    }
    pub fn invalidate(&mut self, security_generation: u64) -> Result<(), Error> {
        self.control()?;
        if security_generation < self.security_generation {
            return Err(Error::Stale);
        }
        if security_generation == self.security_generation {
            self.authority.invalidate_current()?;
        } else {
            self.authority.publish_security(security_generation)?;
            self.security_generation = security_generation;
        }
        Ok(())
    }
    /// Invalidate current commit authority immediately, independently of every
    /// queue/native call. This session identity is NOT a security generation.
    pub fn invalidate_session(&mut self, generation: u64) -> Result<(), Error> {
        self.control()?;
        if generation != self.shared.identity.session_generation {
            return Err(Error::Invalid);
        }
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(Error::Stopping);
        }
        let old = self.shared.invalidation_epoch.load(Ordering::Acquire);
        let new_burst = old == self.shared.accepted_epoch.load(Ordering::Acquire)
            && self.shared.ever_ready.load(Ordering::Acquire);
        if new_burst {
            self.shared
                .beginning_recovery
                .store(true, Ordering::Release);
        }
        let result = (|| {
            self.authority
                .invalidate_current()
                .map_err(|e| self.shared.terminal(e.into()))?;
            self.discovered = None;
            let next = old
                .checked_add(1)
                .ok_or_else(|| self.shared.terminal(Error::Exhausted))?;
            self.shared
                .invalidation_epoch
                .store(next, Ordering::Release);
            // Authority and dirty fence precede every clock, arm, queue and wake.
            if new_burst {
                let now =
                    crate::commit::boottime_ns().map_err(|e| self.shared.terminal(e.into()))?;
                let until = now
                    .checked_add(RECOVERY_BUDGET_NS)
                    .ok_or_else(|| self.shared.terminal(Error::Exhausted))?;
                self.shared.reconcile_until.store(until, Ordering::Release);
                self.shared
                    .startup_deadline
                    .arm(until)
                    .map_err(|e| self.shared.terminal(e))?;
            } else if !self.shared.startup_before_deadline() {
                return Err(Error::Expired);
            }
            if self.shared.stop.load(Ordering::Acquire) {
                self.shared
                    .startup_deadline
                    .disarm()
                    .map_err(|e| self.shared.terminal(e))?;
                return Err(Error::Stopping);
            }
            Ok(())
        })();
        self.shared
            .beginning_recovery
            .store(false, Ordering::Release);
        result?;
        // Wake only after the new deadline is fully published and ungated.
        self.shared
            .wake
            .notify()
            .map_err(|e| self.shared.terminal(e))
    }
    pub fn try_session_change(&mut self, event: &SessionRenderEvent) -> Result<(), Error> {
        self.control()?;
        if !self.shared.identity.accepts_change(event) {
            return Err(Error::Invalid);
        }
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(Error::Stopping);
        }
        let mut session = self.shared.session.try_lock().map_err(|_| Error::Busy)?;
        let next = session
            .last_admitted_sequence
            .checked_add(1)
            .ok_or_else(|| self.shared.terminal(Error::Exhausted))?;
        if event.session_sequence != next {
            return Err(Error::Stale);
        }
        let epoch = session
            .admitted_epoch
            .checked_add(1)
            .ok_or_else(|| self.shared.terminal(Error::Exhausted))?;
        if epoch > self.shared.invalidation_epoch.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }
        if session.changes.len() == MAX_SESSION_CHANGES {
            return Err(Error::Capacity);
        }
        if !self.shared.startup_before_deadline() {
            return Err(Error::Stopping);
        }
        session.changes.push_back(SessionChange {
            epoch,
            event: *event,
        });
        session.admitted_epoch = epoch;
        session.last_admitted_sequence = event.session_sequence;
        self.shared.queued_changes.fetch_add(1, Ordering::AcqRel);
        drop(session);
        // An accepted value remains owned even when wake fails. Fail closed but
        // return accepted so native never retries an already-copied event.
        if let Err(e) = self.shared.wake.notify() {
            self.shared.terminal(e);
        }
        Ok(())
    }
    pub fn reconciliation(&self) -> Result<SessionReconciliationV2, Error> {
        self.control()?;
        self.shared.observe_startup()?;
        let session = self.shared.session.try_lock().map_err(|_| Error::Busy)?;
        Ok(SessionReconciliationV2 {
            session_generation: self.shared.identity.session_generation,
            invalidation_epoch: self.shared.invalidation_epoch.load(Ordering::Acquire),
            admitted_epoch: session.admitted_epoch,
            applied_epoch: session.applied_epoch,
            accepted_epoch: self.shared.accepted_epoch.load(Ordering::Acquire),
            last_admitted_sequence: session.last_admitted_sequence,
            last_applied_sequence: session.last_applied_sequence,
            discovery_sequence: session.candidate.as_ref().map_or(0, |value| value.sequence),
            bootstrap_end_sequence: self.shared.identity.end_sequence,
            reconcile_not_after_boottime_ns: self.shared.reconcile_until.load(Ordering::Acquire),
        })
    }
    pub fn request_stop(&self) -> Result<(), Error> {
        self.control()?;
        self.shared.stop()
    }
    /// CPU-only copied discovery channel. No native object escapes the worker.
    pub fn try_discovery(&mut self) -> Result<Option<Discovery>, Error> {
        self.control()?;
        self.shared.observe_startup()?;
        if !self.shared.available() {
            self.discovered = None;
            return Ok(None);
        }
        let session = self.shared.session.try_lock().map_err(|_| Error::Busy)?;
        if !self.shared.available() {
            self.discovered = None;
            return Ok(None);
        }
        let record = session.candidate.as_ref().map(|value| value.discovery);
        self.discovered = record;
        Ok(record)
    }
    /// Borrowed CLOCK_BOOTTIME watch. Register it AND completion_fd before
    /// waiting. On readability call snapshot/try_discovery/try_submit/try_join,
    /// which enforce the active fixed cutoff; never merely drain this FD. Unregister
    /// both before release. This timer includes suspend but does not wake a phone.
    pub fn startup_deadline_fd(&self) -> Result<i32, Error> {
        self.control()?;
        Ok(self.shared.startup_deadline.fd.as_raw_fd())
    }
    pub fn completion_fd(&self) -> Result<i32, Error> {
        self.control()?;
        Ok(self.shared.complete.0.as_raw_fd())
    }
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        self.control()?;
        self.shared.observe_startup()?;
        let stage = match self.shared.stage.load(Ordering::Acquire) {
            0 => Stage::Starting,
            1 => Stage::Idle,
            2 => Stage::Preparing,
            3 => Stage::Executing,
            4 => Stage::Retiring,
            5 => Stage::Destroying,
            6 => Stage::Quarantined,
            8 => Stage::Bootstrapping,
            _ => Stage::Exited,
        };
        Ok(Snapshot {
            stage,
            worker_alive: self.shared.alive.load(Ordering::Acquire),
            stopping: self.shared.stop.load(Ordering::Acquire),
            ready: self.shared.available(),
            failed: self.shared.failed.load(Ordering::Acquire),
            occupied_credits: self.shared.credits.load(Ordering::Acquire),
            reserved_completion_records: self.shared.reserved_records.load(Ordering::Acquire),
            queued_frames: self.shared.queued.load(Ordering::Acquire),
            accepted: self.shared.accepted.load(Ordering::Acquire),
            retired: self.shared.retired.load(Ordering::Acquire),
            joined: self.joined,
        })
    }
    /// try_lock BEFORE clearing eventfd. Partial drains rearm while holding the
    /// same short completion lock so a durable record never loses its wakeup.
    pub fn try_next_completion(&self) -> Result<Option<Completion>, Error> {
        self.control()?;
        let mut queue = self
            .shared
            .completions
            .try_lock()
            .map_err(|_| Error::Busy)?;
        self.shared.complete.clear()?;
        if queue.len() > 1 && self.shared.complete.notify().is_err() {
            self.shared.failed.store(true, Ordering::Release);
            let _ = self.shared.stop();
            return Err(Error::Wake); // record and its credit remain owned
        }
        let result = queue.pop_front();
        if result.is_some() {
            self.shared.reserved_records.fetch_sub(1, Ordering::AcqRel);
        }
        Ok(result)
    }
    pub fn try_join(&mut self) -> Result<(), Error> {
        self.control()?;
        self.shared.observe_startup()?;
        if self.joined {
            return Ok(());
        }
        let handle = self.handle.as_ref().ok_or(Error::Protocol)?;
        if !handle.is_finished() {
            return Err(Error::Busy);
        }
        let joined = self.handle.take().expect("retained ready handle").join();
        self.joined = true;
        self.shared.alive.store(false, Ordering::Release);
        joined.map_err(|_| {
            self.shared.failed.store(true, Ordering::Release);
            Error::WorkerPanic
        })
    }
    /// The caller must first unregister both borrowed completion and startup
    /// deadline FDs. None is a
    /// successful already-empty slot. BUSY leaves ownership untouched.
    pub fn try_release(slot: &mut Option<Self>) -> Result<(), Error> {
        if let Some(owner) = slot.as_ref() {
            owner.control()?;
            if !owner.joined {
                return Err(Error::Busy);
            }
        }
        drop(slot.take());
        Ok(())
    }
}
impl Drop for Executor {
    fn drop(&mut self) {
        let _ = self.shared.stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
fn native_call<T>(shared: &Shared, operation: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(value) => value,
        Err(_) => shared.quarantine(),
    }
}
/// One bounded worker-owned lifetime. The grant stays move-only here; records
/// contain observations only. Native custody and undelivered-record capacity are
/// independent, so delivering SubmissionOutcome cannot free a scanout frame.
struct Entry {
    frame: Option<Frame>,
    admission: Admission,
    attempt: Attempt,
    native: NativeResult,
    error: Option<Error>,
    prepared: bool,
    records: u32,
    fence: bool,
    presented: bool,
}
impl Entry {
    fn publish(&mut self, shared: &Shared, kind: CompletionKind) {
        if self.records >= RECORDS_PER_FRAME {
            shared.quarantine();
        }
        let record = Completion {
            kind,
            scope: self.attempt.scope.clone(),
            admission: self.admission,
            grant: self.attempt.sealed.as_ref().map(GrantObservation::from),
            native: self.native,
            resources_retired: kind == CompletionKind::Retired,
            error: self.error,
        };
        self.records += 1;
        if shared.publish(record).is_err() {
            shared.quarantine();
        }
    }
}
fn sources(
    shared: &Shared,
    owner: &mut dyn NativeOwner,
) -> ([PollSource; MAX_NATIVE_SOURCES], usize) {
    let mut storage = [PollSource::EMPTY; MAX_NATIVE_SOURCES];
    let count = match native_call(shared, || owner.poll_sources(&mut storage)) {
        Ok(n) if n <= MAX_NATIVE_SOURCES => n,
        _ => shared.quarantine(),
    };
    for (i, source) in storage[..count].iter().enumerate() {
        if source.fd < 0
            || source.fd == shared.wake.0.as_raw_fd()
            || source.fd == shared.complete.0.as_raw_fd()
            || source.token == 0
            || source.generation == 0
            || source.events == 0
            || source.events & !5 != 0
            || storage[..i]
                .iter()
                .any(|prior| prior.fd == source.fd || prior.token == source.token)
        {
            shared.quarantine();
        }
    }
    (storage, count)
}
fn wait_native(shared: &Shared, owner: &mut dyn NativeOwner, blocking: bool) {
    let (before, count) = sources(shared, owner);
    if count == 0 && blocking {
        if shared.wake.wait().is_err() {
            shared.quarantine();
        }
        return;
    }
    let mut fds = [PollFd {
        fd: -1,
        events: 0,
        revents: 0,
    }; MAX_NATIVE_SOURCES + 1];
    fds[0] = PollFd {
        fd: shared.wake.0.as_raw_fd(),
        events: 1,
        revents: 0,
    };
    for (i, source) in before[..count].iter().enumerate() {
        fds[i + 1] = PollFd {
            fd: source.fd,
            events: source.events,
            revents: 0,
        };
    }
    loop {
        let ready = unsafe { poll(fds.as_mut_ptr(), count + 1, if blocking { -1 } else { 0 }) };
        if ready > 0 {
            break;
        }
        if ready == 0 && !blocking {
            return;
        }
        if ready < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        shared.quarantine();
    }
    if fds[0].revents != 0 && (fds[0].revents != 1 || shared.wake.clear().is_err()) {
        shared.quarantine();
    }
    for (i, source) in before[..count].iter().enumerate() {
        let revents = fds[i + 1].revents;
        if revents == 0 {
            continue;
        }
        // POLLNVAL violates the borrowed-FD custody contract. HUP/ERR are native
        // facts to dispatch, never inferred retirement or FD-reuse permission.
        if revents & 32 != 0 {
            shared.quarantine();
        }
        let (current, current_count) = sources(shared, owner);
        if !current[..current_count].contains(source) {
            continue;
        }
        // A prior callback may replace/remove a source. Revalidate the full
        // value tuple before every dispatch; never deliver stale fd readiness.
        if native_call(shared, || {
            owner.dispatch(NativeReady {
                source: *source,
                revents,
            })
        })
        .is_err()
        {
            shared.quarantine();
        }
    }
}
fn run(
    shared: Arc<Shared>,
    factory: OwnerFactory,
    config: OwnerFactoryConfig,
    mut bootstrap: Option<OwnedBootstrap>,
) {
    let mut owner = match native_call(&shared, || factory(config)) {
        Ok(owner) => owner,
        Err(_) => shared.quarantine(),
    };
    // The native holder is retained above, and the not-yet-transferred input
    // remains outside catch_unwind. Bootstrap cannot unwind either custody.
    shared
        .stage
        .store(Stage::Bootstrapping as u32, Ordering::Release);
    let bootstrap_entered = shared.worker_before_deadline();
    if bootstrap_entered {
        match native_call(&shared, || owner.bootstrap(&mut bootstrap)) {
            Ok(discovery) => {
                if bootstrap.is_some() {
                    shared.quarantine();
                } // Take input into owner-held state before native work.
                shared.publish_discovery(discovery, 0, shared.identity.end_sequence);
            }
            Err(_) => {
                shared.failed.store(true, Ordering::Release);
                let _ = shared.stop();
                // Terminal availability is observable even while native cleanup
                // remains Pending or blocked and the startup timer is disarmed.
                if shared.complete.notify().is_err() {
                    shared.quarantine();
                }
                drop(bootstrap.take()); // Untransferred FD duplicates only, never vendor cleanup proof.
            }
        }
    } else {
        drop(bootstrap.take());
    }
    let mut entries = Vec::<Entry>::with_capacity(FRAME_CREDITS as usize);
    // Retain an uncertain applied record through verified aggregate shutdown.
    let mut retained_change: Option<SessionChange> = None;
    loop {
        shared.stage.store(Stage::Idle as u32, Ordering::Release);
        if !bootstrap_entered && !shared.stop.load(Ordering::Acquire) {
            // Empty deferred holder: await control's mandatory timer observation
            // or Stop; do not call native source callbacks without bootstrap.
            if shared.wake.wait().is_err() {
                shared.quarantine();
            }
            continue;
        }
        let session_eligible =
            !shared.stop.load(Ordering::Acquire) && shared.worker_before_deadline();
        if session_eligible {
            retained_change = match shared.session.lock() {
                Ok(mut session) => session.changes.pop_front(),
                Err(_) => shared.quarantine(),
            };
            if let Some(change) = retained_change.as_ref() {
                shared.queued_changes.fetch_sub(1, Ordering::AcqRel);
                shared
                    .stage
                    .store(Stage::Bootstrapping as u32, Ordering::Release);
                // The queue lock is gone before the real worker callback.
                match native_call(&shared, || owner.apply_device_change(&change.event)) {
                    Ok(discovery) => {
                        shared.publish_discovery(
                            discovery,
                            change.epoch,
                            change.event.session_sequence,
                        );
                        if !shared.stop.load(Ordering::Acquire) {
                            retained_change = None;
                        }
                    }
                    Err(error) => {
                        shared.terminal(error);
                    }
                }
            }
        }
        let job = match shared.ingress.lock() {
            Ok(mut queue) => queue.pop_front(),
            Err(_) => shared.quarantine(),
        };
        // No control mutex remains over any native call, wait or resource drop.
        if let Some(job) = job {
            if entries.len() == FRAME_CREDITS as usize {
                shared.quarantine();
            }
            shared.queued.fetch_sub(1, Ordering::AcqRel);
            entries.push(Entry {
                frame: Some(job.frame),
                admission: job.admission,
                attempt: Attempt {
                    expected_device: job.expected_device,
                    scope: None,
                    ticket: Some(job.ticket),
                    policy: job.admission.policy,
                    sealed: None,
                },
                native: NativeResult::NotEntered,
                error: None,
                prepared: false,
                records: 0,
                fence: false,
                presented: false,
            });
            let entry = entries.last_mut().expect("retained entry");
            if !shared.stop.load(Ordering::Acquire) {
                entry.prepared = true; // Partial prepare resources also require actual drain.
                shared
                    .stage
                    .store(Stage::Preparing as u32, Ordering::Release);
                match native_call(&shared, || {
                    owner.prepare(entry.frame.as_ref().expect("owned frame"), entry.admission)
                }) {
                    Ok(()) if !shared.stop.load(Ordering::Acquire) => {
                        shared
                            .stage
                            .store(Stage::Executing as u32, Ordering::Release);
                        match native_call(&shared, || {
                            owner.execute(
                                entry.frame.as_ref().expect("owned frame"),
                                entry.admission,
                                &mut entry.attempt,
                            )
                        }) {
                            Ok(result) => entry.native = result,
                            Err(error) => entry.error = Some(error),
                        }
                    }
                    Ok(()) => entry.error = Some(Error::Stopping),
                    Err(error) => entry.error = Some(error),
                }
                if entry.native == NativeResult::NotEntered && entry.attempt.sealed.is_some() {
                    entry.native = NativeResult::Unknown;
                }
                if entry.native != NativeResult::NotEntered && entry.attempt.sealed.is_none() {
                    shared.quarantine();
                }
            } else {
                entry.error = Some(Error::Stopping);
            }
            if entry.attempt.sealed.is_some() {
                entry.publish(&shared, CompletionKind::Attempt);
            }
            entry.publish(&shared, CompletionKind::SubmissionOutcome);
        }
        // Stop requests native lifecycle progress while the last scanout is
        // retained. This call is NOT display-changing authority: an adapter may
        // disable/blackout only with a distinct authentic StopRetire grant.
        let mut shutdown_ready = if shared.stop.load(Ordering::Acquire) {
            shared
                .stage
                .store(Stage::Destroying as u32, Ordering::Release);
            match native_call(&shared, || owner.shutdown_progress()) {
                Ok(RetirementProgress::Verified(_)) => true,
                Ok(RetirementProgress::Pending) => false,
                Err(_) => shared.quarantine(),
            }
        } else {
            false
        };
        let mut i = 0;
        while i < entries.len() {
            let entry = &mut entries[i];
            shared
                .stage
                .store(Stage::Retiring as u32, Ordering::Release);
            let progress = if entry.prepared {
                let context = RetirementContext {
                    admission: entry.admission,
                    grant: entry.attempt.sealed.as_ref(),
                    native: entry.native,
                    scope: entry.attempt.scope.as_ref(),
                };
                match native_call(&shared, || owner.advance(&context)) {
                    Ok(value) => value,
                    Err(_) => shared.quarantine(),
                }
            } else {
                NativeProgress::retired()
            }; // No native call ever received this frame.
            if (progress.fence.is_some() || progress.presentation.is_some())
                && (entry.attempt.sealed.is_none() || entry.native == NativeResult::NotEntered)
            {
                shared.quarantine();
            }
            if let Some(fence) = progress.fence {
                if entry.fence || fence.identity == 0 {
                    shared.quarantine();
                }
                entry.fence = true;
                entry.publish(&shared, CompletionKind::Fence(fence));
            }
            if let Some(presentation) = progress.presentation {
                if entry.presented || presentation.correlation_id == 0 {
                    shared.quarantine();
                }
                entry.presented = true;
                entry.publish(&shared, CompletionKind::Presented(presentation));
            }
            if matches!(progress.retirement, RetirementProgress::Verified(_)) {
                let mut entry = entries.remove(i);
                drop(entry.frame.take()); // CPU clones release here only after native proof.
                shared.credits.fetch_sub(1, Ordering::AcqRel);
                shared.retired.fetch_add(1, Ordering::AcqRel);
                entry.publish(&shared, CompletionKind::Retired);
                shared
                    .reserved_records
                    .fetch_sub(RECORDS_PER_FRAME - entry.records, Ordering::AcqRel);
            } else {
                i += 1;
            }
        }
        // Retirement may consume the final native event and remove all poll
        // sources. Recheck aggregate lifecycle readiness before sleeping; its
        // pre-retirement Pending result is no longer a current observation.
        if !shutdown_ready
            && entries.is_empty()
            && shared.stop.load(Ordering::Acquire)
            && shared.queued.load(Ordering::Acquire) == 0
        {
            shared
                .stage
                .store(Stage::Destroying as u32, Ordering::Release);
            shutdown_ready = match native_call(&shared, || owner.shutdown_progress()) {
                Ok(RetirementProgress::Verified(_)) => true,
                Ok(RetirementProgress::Pending) => false,
                Err(_) => shared.quarantine(),
            };
        }
        if shutdown_ready && entries.is_empty() && shared.queued.load(Ordering::Acquire) == 0 {
            break;
        }
        // One ingress operation is the finite budget. Even continuously
        // refilled queues pass an actual nonblocking native dispatch point.
        // When ingress is empty the same poll includes the control wake fd.
        let blocking = shared.queued.load(Ordering::Acquire) == 0
            && (shared.stop.load(Ordering::Acquire)
                || shared.queued_changes.load(Ordering::Acquire) == 0
                || !session_eligible);
        wait_native(&shared, &mut *owner, blocking);
    }
    native_call(&shared, || drop(owner)); // Actual creation worker, after verified native drain.
    let _retained_change = retained_change; // Copied metadata only, retained through actual native drain.
    shared.stage.store(Stage::Exited as u32, Ordering::Release);
    shared.alive.store(false, Ordering::Release);
    let _ = shared.complete.notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commit::boottime_ns, frame::*};
    use std::{
        mem::size_of,
        ptr,
        sync::mpsc::{self, Receiver, Sender},
        time::Duration,
    };
    struct Plan {
        checkpoint: Sender<(&'static str, ThreadId)>,
        release: Receiver<()>,
        mode: u32,
    }
    static PLAN: Mutex<Option<Plan>> = Mutex::new(None);
    static SERIAL: Mutex<()> = Mutex::new(());
    struct Owner {
        plan: Plan,
        thread: ThreadId,
        _not_send: Rc<()>,
    }
    fn factory(_: OwnerFactoryConfig) -> Result<Box<dyn NativeOwner>, Error> {
        let plan = PLAN.lock().unwrap().take().ok_or(Error::Invalid)?;
        let thread = thread::current().id();
        plan.checkpoint.send(("created", thread)).unwrap();
        Ok(Box::new(Owner {
            plan,
            thread,
            _not_send: Rc::new(()),
        }))
    }
    impl Owner {
        fn checkpoint(&self, name: &'static str) {
            assert_eq!(self.thread, thread::current().id());
            self.plan.checkpoint.send((name, self.thread)).unwrap();
        }
    }
    impl NativeOwner for Owner {
        fn apply_device_change(&mut self, _: &SessionRenderEvent) -> Result<Discovery, Error> {
            Err(Error::Invalid)
        }
        fn bootstrap(&mut self, input: &mut Option<OwnedBootstrap>) -> Result<Discovery, Error> {
            let discovery = bootstrap_discovery(input.as_ref().ok_or(Error::Invalid)?);
            drop(input.take());
            Ok(discovery)
        }
        fn prepare(&mut self, frame: &Frame, _: Admission) -> Result<(), Error> {
            self.checkpoint("prepare");
            if self.plan.mode == 3 {
                panic!("authored prepare panic");
            }
            // Read the real immutable copied value arrays on the worker.
            frame
                .with_borrow(|view| assert_eq!(view.draw_count, 1))
                .unwrap();
            if self.plan.mode == 0 {
                self.plan
                    .release
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            Ok(())
        }
        fn execute(
            &mut self,
            _: &Frame,
            admission: Admission,
            attempt: &mut Attempt,
        ) -> Result<NativeResult, Error> {
            self.checkpoint("execute");
            let objects = [NativeObject {
                id: 41,
                generation: 3,
                output_id: admission.policy.output_id,
                output_generation: admission.policy.output_generation,
            }];
            let scope = NativeScope {
                policy: admission.policy,
                device_lease_id: 59,
                device_generation: 61,
                objects: &objects,
            };
            attempt.authorize(&scope)?;
            self.checkpoint("sealed");
            if self.plan.mode == 4 {
                panic!("authored execute panic after grant");
            }
            assert_eq!(attempt.authorize(&scope).unwrap_err(), Error::Stale);
            if self.plan.mode == 1 {
                self.plan
                    .release
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            if self.plan.mode == 2 {
                return Err(Error::Native);
            }
            Ok(NativeResult::Returned {
                operation: 1,
                value: 0,
                errno: 0,
                succeeded: true,
                extra_attempt_refused: true,
            })
        }
        fn advance(&mut self, context: &RetirementContext<'_>) -> Result<NativeProgress, Error> {
            self.checkpoint("retire");
            if self.plan.mode == 7 {
                let valid = context.scope.is_some_and(|scope| {
                    scope.policy == context.admission.policy
                        && scope.device_lease_id == 59
                        && scope.device_generation == 61
                        && scope.objects().len() == 1
                        && scope.objects()[0].id == 41
                });
                self.checkpoint(if valid {
                    "scope_valid"
                } else {
                    "scope_missing"
                });
            }
            if self.plan.mode == 5 {
                panic!("authored retire panic");
            }
            if self.plan.mode == 2 {
                self.plan
                    .release
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            Ok(NativeProgress::retired())
        }
        fn poll_sources(
            &mut self,
            _: &mut [PollSource; MAX_NATIVE_SOURCES],
        ) -> Result<usize, Error> {
            Ok(0)
        }
        fn dispatch(&mut self, _: NativeReady) -> Result<(), Error> {
            Err(Error::Protocol)
        }
        fn shutdown_progress(&mut self) -> Result<RetirementProgress, Error> {
            self.checkpoint("shutdown");
            if self.plan.mode == 6 {
                panic!("authored shutdown panic");
            }
            Ok(RetirementProgress::Verified(RetirementProof))
        }
    }
    impl Drop for Owner {
        fn drop(&mut self) {
            self.checkpoint("drop");
        }
    }
    fn frame(sequence: u64) -> Frame {
        frame_target(
            sequence,
            FrameTargetV1 {
                output_id: 11,
                output_generation: 17,
                width: 32,
                height: 32,
                transformed_width: 32,
                transformed_height: 32,
                scale: 1.0,
                ..Default::default()
            },
        )
    }
    fn frame_target(sequence: u64, target: FrameTargetV1) -> Frame {
        let rect = FrameRectV1 {
            x1: 0,
            y1: 0,
            x2: 32,
            y2: 32,
        };
        let rects = [rect; 3];
        let draw = FrameDrawV1 {
            scissor_first: 2,
            scissor_count: 1,
            box_: FrameBoxV1 {
                width: 32.0,
                height: 32.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let input = FrameBorrowV1 {
            abi_version: 1,
            struct_size: size_of::<FrameBorrowV1>() as u32,
            flags: 3,
            reserved: 0,
            scene_sequence: sequence,
            security_generation: 1,
            target,
            damage_first: 0,
            damage_count: 1,
            final_damage_first: 1,
            final_damage_count: 1,
            draw_count: 1,
            resource_count: 0,
            rect_count: 3,
            reserved2: 0,
            draws: &draw,
            resources: ptr::null(),
            rects: rects.as_ptr(),
        };
        unsafe { Frame::capture_borrow(&input) }.unwrap()
    }
    fn policy(sequence: u64) -> OperationPolicy {
        OperationPolicy {
            security_generation: 1,
            output_id: 11,
            output_generation: 17,
            scene_sequence: sequence,
            session_generation: 29,
        }
    }
    fn start(mode: u32) -> (Executor, Sender<()>, Receiver<(&'static str, ThreadId)>) {
        let (checkpoint, events) = mpsc::channel();
        let (release, go) = mpsc::channel();
        *PLAN.lock().unwrap() = Some(Plan {
            checkpoint,
            release: go,
            mode,
        });
        let mut executor = Executor::new(
            1,
            factory,
            crate::executor_ffi::test_config(),
            bootstrap_input(),
            boottime_ns().unwrap() + 5_000_000_000,
        )
        .unwrap();
        assert!(await_discovery(&mut executor).is_some());
        (executor, release, events)
    }
    fn await_stage(
        events: &Receiver<(&'static str, ThreadId)>,
        expected: &'static str,
    ) -> ThreadId {
        loop {
            let (name, id) = events.recv_timeout(Duration::from_secs(2)).unwrap();
            if name == expected {
                return id;
            }
        }
    }
    fn submit(e: &mut Executor, sequence: u64) {
        let mut f = Some(frame(sequence));
        let until = boottime_ns().unwrap() + 100_000_000;
        let cutoff = boottime_ns().unwrap() + 5_000_000_000;
        loop {
            match e.try_submit(&mut f, policy(sequence), cutoff, sequence * 101) {
                Ok(_) => break,
                Err(Error::Busy) if boottime_ns().unwrap() < until => thread::yield_now(),
                other => panic!("valid fixture admission {other:?}"),
            }
        }
        assert!(f.is_none());
    }
    fn join(e: &mut Executor) {
        let until = boottime_ns().unwrap() + 2_000_000_000;
        loop {
            match e.try_join() {
                Ok(()) => return,
                Err(Error::Busy) if boottime_ns().unwrap() < until => thread::yield_now(),
                other => panic!("actual join failed: {other:?}"),
            }
        }
    }
    fn readable(fd: i32) -> bool {
        let mut p = PollFd {
            fd,
            events: 1,
            revents: 0,
        };
        let ready = unsafe { poll(&mut p, 1, 0) };
        ready == 1 && p.revents == 1
    }

    #[test]
    fn held_native_prepare_keeps_stop_snapshot_capacity_and_join_ownership_responsive() {
        let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
        let (executor, release, events) = start(0);
        let mut slot = Some(executor);
        let e = slot.as_mut().unwrap();
        let worker = await_stage(&events, "created");
        submit(e, 1);
        await_stage(&events, "prepare");
        submit(e, 2);
        submit(e, 3);
        let mut refused = Some(frame(4));
        let capacity = e.try_submit(
            &mut refused,
            policy(4),
            boottime_ns().unwrap() + 5_000_000_000,
            404,
        );
        let start = boottime_ns().unwrap();
        let stopped = e.request_stop();
        let snapshots_ok = (0..1024).all(|_| {
            let s = e.snapshot().unwrap();
            s.stopping && s.worker_alive && s.occupied_credits == 3
        });
        let elapsed = boottime_ns().unwrap() - start;
        let join_busy = e.try_join();
        let release_busy = Executor::try_release(&mut slot);
        // Always release the authored barrier before assertions or final Drop.
        let released = release.send(());
        let e = slot.as_mut().unwrap();
        join(e);
        let s = e.snapshot().unwrap();
        let fd = e.completion_fd().unwrap();
        let was_readable = readable(fd);
        let shared = Arc::clone(&e.shared);
        let lock = shared.completions.lock().unwrap();
        let busy = matches!(e.try_next_completion(), Err(Error::Busy));
        drop(lock);
        let wake_survived_busy = readable(fd);
        let mut completions = Vec::new();
        while let Some(c) = e.try_next_completion().unwrap() {
            if c.kind == CompletionKind::Retired {
                completions.push(c);
            }
        }
        let drained_empty = !readable(fd);
        let closure: Vec<_> = events.try_iter().collect();
        let release_final = Executor::try_release(&mut slot);
        assert_eq!(capacity, Err(Error::Capacity));
        assert!(refused.is_some());
        assert!(stopped.is_ok() && snapshots_ok && elapsed < 100_000_000);
        assert_eq!(join_busy, Err(Error::Busy));
        assert_eq!(release_busy, Err(Error::Busy));
        assert!(released.is_ok() && s.joined && !s.worker_alive && s.retired == 3);
        assert!(was_readable && busy && wake_survived_busy && drained_empty);
        assert_eq!(completions.len(), 3);
        assert!(completions.iter().all(|c| c.grant.is_none()
            && c.native == NativeResult::NotEntered
            && c.resources_retired));
        assert!(closure.iter().all(|(_, id)| *id == worker));
        assert!(closure.iter().any(|(name, _)| *name == "drop"));
        assert!(release_final.is_ok() && slot.is_none());
        println!(
            "held-native-component: actual worker joined; native-owner destructor same thread; 1024 stop snapshots took {elapsed}ns"
        );
    }

    #[test]
    fn prior_sealed_attempt_survives_stop_until_actual_owner_return_and_drain() {
        let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
        let (mut e, release, events) = start(1);
        await_stage(&events, "created");
        submit(&mut e, 1);
        await_stage(&events, "sealed");
        let stopped = e.request_stop();
        let busy = e.try_join();
        let before = e.snapshot().unwrap();
        let released = release.send(());
        join(&mut e);
        let completion = next_retired(&e).unwrap();
        assert!(stopped.is_ok() && released.is_ok() && before.worker_alive);
        assert_eq!(busy, Err(Error::Busy));
        assert_eq!(completion.grant.unwrap().policy(), policy(1));
        assert!(matches!(
            completion.native,
            NativeResult::Returned {
                succeeded: true,
                extra_attempt_refused: true,
                ..
            }
        ));
        assert!(completion.resources_retired && completion.error.is_none());
    }

    #[test]
    fn native_error_after_seal_cannot_be_reported_as_no_native_entry() {
        let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
        let (mut e, release, events) = start(2);
        await_stage(&events, "created");
        submit(&mut e, 1);
        await_stage(&events, "retire");
        let held = e.snapshot().unwrap();
        let no_early_retirement = e
            .try_next_completion()
            .unwrap()
            .is_none_or(|c| c.kind != CompletionKind::Retired);
        e.request_stop().unwrap();
        let still_busy = e.try_join();
        release.send(()).unwrap();
        join(&mut e);
        let completion = next_retired(&e).unwrap();
        assert!(held.worker_alive && held.stage == Stage::Retiring && held.occupied_credits == 1);
        assert!(no_early_retirement);
        assert_eq!(still_busy, Err(Error::Busy));
        assert!(completion.resources_retired);
        assert!(completion.grant.is_some());
        assert_eq!(completion.error, Some(Error::Native));
        assert_ne!(
            completion.native,
            NativeResult::NotEntered,
            "an unreported result after a grant is uncertain; it is not proof that no native effect entered"
        );
    }
    #[test]
    fn rearm_failure_cannot_discard_an_unreturned_completion_or_its_credit() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = start(0);
        await_stage(&events, "created");
        submit(&mut e, 1);
        await_stage(&events, "prepare");
        submit(&mut e, 2);
        submit(&mut e, 3);
        e.request_stop().unwrap();
        release.send(()).unwrap();
        join(&mut e);
        // Controlled notify-error injection at the actual completion protocol.
        // It is not an observed kernel eventfd failure or graphics fault.
        e.shared.complete.1.store(true, Ordering::Release);
        let failed = e.try_next_completion();
        let credits_after_error = e.snapshot().unwrap().occupied_credits;
        let records_after_error = e.snapshot().unwrap().reserved_completion_records;
        let mut retained = Vec::new();
        while let Some(c) = e.try_next_completion().unwrap() {
            retained.push(c);
        }
        assert!(matches!(failed, Err(Error::Wake)));
        assert_eq!(
            credits_after_error, 0,
            "actual retirement releases native frame credits independently"
        );
        assert_eq!(
            records_after_error, 6,
            "all original records remain undelivered on Err"
        );
        assert_eq!(
            retained.len(),
            6,
            "all original outcomes must remain deliverable"
        );
    }

    #[test]
    #[ignore = "owned child mode; parent observes and reaps the entire authored fixture"]
    fn native_panic_retains_custody_child() {
        use std::io::Write;
        let mode: u32 = std::env::var("POKELINUX_RENDER_PANIC_MODE")
            .unwrap()
            .parse()
            .unwrap();
        assert!((3..=6).contains(&mode));
        let (mut e, _release, events) = start(mode);
        await_stage(&events, "created");
        submit(&mut e, 1);
        if mode == 6 {
            await_stage(&events, "retire");
            e.request_stop().unwrap();
        }
        let until = boottime_ns().unwrap() + 2_000_000_000;
        while e.snapshot().unwrap().stage != Stage::Quarantined
            && !e.handle.as_ref().unwrap().is_finished()
            && boottime_ns().unwrap() < until
        {
            thread::yield_now();
        }
        let snapshot = e.snapshot().unwrap();
        assert_eq!(
            snapshot.stage,
            Stage::Quarantined,
            "adapter panic must retain owner/frame instead of unwinding them"
        );
        assert!(snapshot.worker_alive && snapshot.failed && snapshot.stopping);
        assert_eq!(e.try_join(), Err(Error::Busy));
        assert!(!events.try_iter().any(|(name, _)| name == "drop"));
        println!("OWNER_QUARANTINED mode={mode}");
        std::io::stdout().flush().unwrap();
        // Parent kills and actually reaps THIS entire authored test process.
        // This is not native retirement or safe production recovery evidence.
        loop {
            thread::park();
        }
    }

    #[test]
    fn native_panics_preserve_worker_custody_until_owned_process_reaping() {
        use std::{
            io::Read,
            os::unix::process::ExitStatusExt,
            process::{Command, Stdio},
        };
        unsafe extern "C" {
            fn fcntl(fd: i32, cmd: i32, ...) -> i32;
        }
        struct OwnedChild(std::process::Child, bool);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                if !self.1 {
                    let _ = self.0.kill();
                    let _ = self.0.wait();
                }
            }
        }
        let mut qualified = Vec::new();
        for mode in 3..=6 {
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "executor::tests::native_panic_retains_custody_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("POKELINUX_RENDER_PANIC_MODE", mode.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut owned = OwnedChild(child, false);
            let mut pipe = owned.0.stdout.take().unwrap();
            let flags = unsafe { fcntl(pipe.as_raw_fd(), 3) };
            let setup_ok = flags >= 0 && unsafe { fcntl(pipe.as_raw_fd(), 4, flags | 0x800) } == 0;
            let original_cutoff = boottime_ns().unwrap() + 3_000_000_000;
            let mut bytes = Vec::new();
            let mut eof = false;
            while setup_ok && boottime_ns().unwrap() < original_cutoff && bytes.len() < 16384 {
                let mut data = [0u8; 1024];
                match pipe.read(&mut data) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => bytes.extend_from_slice(&data[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
                if bytes.windows(17).any(|v| v == b"OWNER_QUARANTINED") {
                    break;
                }
                if owned.0.try_wait().unwrap().is_some() {
                    continue;
                }
                thread::yield_now();
            }
            let saw_quarantine = bytes.windows(17).any(|v| v == b"OWNER_QUARANTINED");
            let _ = owned.0.kill();
            let status = owned.0.wait().unwrap();
            owned.1 = true;
            let waited_before_cutoff = boottime_ns().unwrap() < original_cutoff;
            while !eof && boottime_ns().unwrap() < original_cutoff && bytes.len() < 16384 {
                let mut data = [0u8; 1024];
                match pipe.read(&mut data) {
                    Ok(0) => eof = true,
                    Ok(n) => bytes.extend_from_slice(&data[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
                    Err(_) => break,
                }
            }
            println!(
                "panic-child mode={mode} actual_wait={} signal={:?} EOF={eof} within_original_cutoff={waited_before_cutoff} quarantine={saw_quarantine}",
                owned.0.id(),
                status.signal()
            );
            qualified.push(
                setup_ok
                    && saw_quarantine
                    && status.signal() == Some(9)
                    && eof
                    && waited_before_cutoff
                    && bytes.len() < 16384,
            );
        }
        assert!(
            qualified.iter().all(|v| *v),
            "each native-call panic must quarantine before owner/frame unwind; every authored child is actually reaped"
        );
    }

    #[test]
    fn final_native_scope_is_copied_before_grant_and_retained_for_real_drain() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, _release, events) = start(7);
        await_stage(&events, "created");
        submit(&mut e, 1);
        await_stage(&events, "retire");
        let (scope_result, _) = events.recv_timeout(Duration::from_secs(2)).unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        let completion = next_retired(&e).unwrap();
        assert!(completion.grant.is_some() && completion.resources_retired);
        assert_eq!(
            scope_result, "scope_valid",
            "the native request borrow has ended before retire; its complete value scope must survive"
        );
        assert_eq!(
            completion.scope.unwrap().objects()[0].id,
            41,
            "actual scope must also remain in the delivered outcome"
        );
    }
    fn next_retired(e: &Executor) -> Option<Completion> {
        while let Some(record) = e.try_next_completion().unwrap() {
            if record.kind == CompletionKind::Retired {
                return Some(record);
            }
        }
        None
    }
    struct AsyncPlan {
        mode: u32,
        checkpoint: Sender<(&'static str, u64)>,
        control: Sender<AsyncSignal>,
    }
    struct AsyncSignal {
        wake: Arc<EventFd>,
        release_all: Arc<AtomicBool>,
        advance: Sender<()>,
    }
    struct AsyncOwner {
        plan: AsyncPlan,
        wake: Arc<EventFd>,
        release_all: Arc<AtomicBool>,
        thread: ThreadId,
        executed: u64,
        live_registry: u32,
        releases: Receiver<()>,
        shutdown_started: bool,
        dispatched: bool,
        _not_send: Rc<()>,
    }
    static ASYNC_PLAN: Mutex<Option<AsyncPlan>> = Mutex::new(None);
    fn async_factory(_: OwnerFactoryConfig) -> Result<Box<dyn NativeOwner>, Error> {
        let plan = ASYNC_PLAN.lock().unwrap().take().ok_or(Error::Invalid)?;
        let wake = Arc::new(EventFd::new()?);
        let release_all = Arc::new(AtomicBool::new(false));
        let (advance, releases) = mpsc::channel();
        plan.control
            .send(AsyncSignal {
                wake: Arc::clone(&wake),
                release_all: Arc::clone(&release_all),
                advance,
            })
            .unwrap();
        Ok(Box::new(AsyncOwner {
            plan,
            wake,
            release_all,
            thread: thread::current().id(),
            executed: 0,
            live_registry: 0,
            releases,
            shutdown_started: false,
            dispatched: false,
            _not_send: Rc::new(()),
        }))
    }
    impl AsyncOwner {
        fn mark(&self, name: &'static str, id: u64) {
            assert_eq!(self.thread, thread::current().id());
            let _ = self.plan.checkpoint.send((name, id));
        }
    }
    impl NativeOwner for AsyncOwner {
        fn apply_device_change(&mut self, _: &SessionRenderEvent) -> Result<Discovery, Error> {
            Err(Error::Invalid)
        }
        fn bootstrap(&mut self, input: &mut Option<OwnedBootstrap>) -> Result<Discovery, Error> {
            let discovery = bootstrap_discovery(input.as_ref().ok_or(Error::Invalid)?);
            drop(input.take());
            Ok(discovery)
        }
        fn prepare(&mut self, frame: &Frame, admission: Admission) -> Result<(), Error> {
            assert_eq!(frame.scene_sequence(), admission.policy.scene_sequence);
            self.live_registry += 1;
            self.mark("prepare", admission.id);
            if self.plan.mode == 5 && admission.id >= 2 {
                self.releases
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            Ok(())
        }
        fn execute(
            &mut self,
            _: &Frame,
            admission: Admission,
            attempt: &mut Attempt,
        ) -> Result<NativeResult, Error> {
            let object = [NativeObject {
                id: 41,
                generation: 3,
                output_id: admission.policy.output_id,
                output_generation: admission.policy.output_generation,
            }];
            attempt.authorize(&NativeScope {
                policy: admission.policy,
                device_lease_id: 59,
                device_generation: 61,
                objects: &object,
            })?;
            self.executed = admission.id;
            self.mark("executed", admission.id);
            if admission.id == 2 {
                self.wake.notify()?;
            }
            Ok(NativeResult::Returned {
                operation: 1,
                value: 0,
                errno: 0,
                succeeded: true,
                extra_attempt_refused: false,
            })
        }
        fn advance(&mut self, context: &RetirementContext<'_>) -> Result<NativeProgress, Error> {
            let ready = (self.plan.mode == 2 || self.plan.mode == 6)
                || (self.plan.mode == 5 && (context.admission.id != 1 || self.dispatched))
                || (self.dispatched && self.release_all.load(Ordering::Acquire))
                || (self.plan.mode == 0
                    && context.admission.id == 1
                    && self.executed >= 2
                    && self.dispatched);
            if ready {
                self.live_registry -= 1;
                self.mark("retired", context.admission.id);
                if context.grant.is_none() {
                    return Ok(NativeProgress::retired());
                }
                return Ok(NativeProgress {
                    fence: Some(FenceFact {
                        identity: context.admission.id + 100,
                    }),
                    presentation: Some(PresentationFact {
                        correlation_id: context.admission.id + 1000,
                        sequence: if self.plan.mode == 6 {
                            0
                        } else {
                            context.admission.id + 200
                        },
                        timestamp_ns: if self.plan.mode == 6 {
                            0
                        } else {
                            boottime_ns().unwrap()
                        },
                        clock: if self.plan.mode == 6 {
                            PresentationClock::Monotonic
                        } else {
                            PresentationClock::Boottime
                        },
                    }),
                    retirement: RetirementProgress::Verified(RetirementProof),
                });
            }
            self.mark("advance_pending", context.admission.id);
            Ok(NativeProgress::pending())
        }
        fn poll_sources(
            &mut self,
            storage: &mut [PollSource; MAX_NATIVE_SOURCES],
        ) -> Result<usize, Error> {
            if self.plan.mode == 8 && self.live_registry == 0 {
                return Ok(0);
            }
            storage[0] = PollSource {
                fd: self.wake.0.as_raw_fd(),
                token: 61,
                generation: 1,
                events: 1,
            };
            Ok(1)
        }
        fn dispatch(&mut self, ready: NativeReady) -> Result<(), Error> {
            assert_eq!(
                ready.source,
                PollSource {
                    fd: self.wake.0.as_raw_fd(),
                    token: 61,
                    generation: 1,
                    events: 1
                }
            );
            assert_eq!(ready.revents, 1);
            self.wake.clear()?;
            self.dispatched = true;
            self.mark("dispatched", 1);
            Ok(())
        }
        fn shutdown_progress(&mut self) -> Result<RetirementProgress, Error> {
            if self.plan.mode == 8 && self.live_registry != 0 {
                self.mark("shutdown_pending", 0);
                return Ok(RetirementProgress::Pending);
            }
            if self.plan.mode == 4 && !self.shutdown_started {
                self.shutdown_started = true;
                self.mark("shutdown_started", 0);
            }
            if (self.plan.mode == 3 || self.plan.mode == 4)
                && !(self.dispatched && self.release_all.load(Ordering::Acquire))
            {
                self.mark("shutdown_pending", 0);
                return Ok(RetirementProgress::Pending);
            }
            self.mark("shutdown_complete", 0);
            Ok(RetirementProgress::Verified(RetirementProof))
        }
    }
    impl Drop for AsyncOwner {
        fn drop(&mut self) {
            self.mark("drop", 0);
        }
    }
    fn async_start(mode: u32) -> (Executor, AsyncSignal, Receiver<(&'static str, u64)>) {
        let (tx, rx) = mpsc::channel();
        let (control, controls) = mpsc::channel();
        *ASYNC_PLAN.lock().unwrap() = Some(AsyncPlan {
            mode,
            checkpoint: tx,
            control,
        });
        let mut executor = Executor::new(
            1,
            async_factory,
            crate::executor_ffi::test_config(),
            bootstrap_input(),
            boottime_ns().unwrap() + 5_000_000_000,
        )
        .unwrap();
        assert!(await_discovery(&mut executor).is_some());
        (
            executor,
            controls.recv_timeout(Duration::from_secs(2)).unwrap(),
            rx,
        )
    }
    fn async_wait(events: &Receiver<(&'static str, u64)>, name: &'static str, id: u64) {
        let until = boottime_ns().unwrap() + 2_000_000_000;
        loop {
            let value = events.recv_timeout(Duration::from_millis(100)).unwrap();
            if value == (name, id) {
                return;
            }
            assert!(
                boottime_ns().unwrap() < until,
                "missing actual worker checkpoint {name} {id}"
            );
        }
    }
    fn drain(e: &Executor, into: &mut Vec<Completion>) {
        loop {
            match e.try_next_completion() {
                Ok(Some(record)) => into.push(record),
                Ok(None) | Err(Error::Busy) => break,
                Err(error) => panic!("completion delivery failed: {error:?}"),
            }
        }
    }
    fn release_native(signal: &AsyncSignal) {
        signal.release_all.store(true, Ordering::Release);
        signal.wake.notify().unwrap();
    }
    #[test]
    fn async_replacement_and_kernel_ready_precede_actual_retirement() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(0);
        submit(&mut e, 1);
        async_wait(&events, "advance_pending", 1);
        let mut records = Vec::new();
        let until = boottime_ns().unwrap() + 20_000_000;
        while boottime_ns().unwrap() < until {
            drain(&e, &mut records);
            thread::yield_now();
        }
        let first_retired_early = records.iter().any(|r| r.kind == CompletionKind::Retired);
        let credit_after_submission_delivery = e.snapshot().unwrap().occupied_credits;
        submit(&mut e, 2);
        async_wait(&events, "executed", 2);
        let until = boottime_ns().unwrap() + 500_000_000;
        while boottime_ns().unwrap() < until
            && !records
                .iter()
                .any(|r| r.admission.id == 1 && r.kind == CompletionKind::Retired)
        {
            drain(&e, &mut records);
            thread::yield_now();
        }
        let before_stop = e.snapshot().unwrap();
        e.request_stop().unwrap();
        release_native(&signal);
        join(&mut e);
        drain(&e, &mut records);
        assert!(!first_retired_early, "Pending is not descendant retirement");
        assert_eq!(
            credit_after_submission_delivery, 1,
            "submission delivery must not release live frame custody"
        );
        assert_eq!(
            before_stop.occupied_credits, 1,
            "replacement must enter before the old scanout can retire; replacement remains live"
        );
        let first: Vec<_> = records
            .iter()
            .filter(|r| r.admission.id == 1)
            .map(|r| r.kind)
            .collect();
        assert_eq!(first.len(), 5);
        assert_eq!(first[0], CompletionKind::Attempt);
        assert_eq!(first[1], CompletionKind::SubmissionOutcome);
        assert_eq!(first[2], CompletionKind::Fence(FenceFact { identity: 101 }));
        assert!(matches!(
            first[3],
            CompletionKind::Presented(PresentationFact { sequence: 201, .. })
        ));
        assert_eq!(first[4], CompletionKind::Retired);
        assert_eq!(e.snapshot().unwrap().occupied_credits, 0);
    }
    #[test]
    fn async_stop_and_drained_submission_keep_all_live_frame_credits() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(1);
        let mut records = Vec::new();
        for id in 1..=3 {
            submit(&mut e, id);
            async_wait(&events, "executed", id);
            drain(&e, &mut records);
        }
        let mut refused = Some(frame(4));
        let refusal = e.try_submit(
            &mut refused,
            policy(4),
            boottime_ns().unwrap() + 5_000_000_000,
            404,
        );
        e.request_stop().unwrap();
        let alive = e.snapshot().unwrap();
        let join_before_drain = e.try_join();
        release_native(&signal);
        join(&mut e);
        drain(&e, &mut records);
        assert_eq!(refusal, Err(Error::Capacity));
        assert!(refused.is_some());
        assert!(alive.worker_alive && alive.occupied_credits == 3 && alive.stopping);
        assert_eq!(join_before_drain, Err(Error::Busy));
        assert_eq!(
            records
                .iter()
                .filter(|r| r.kind == CompletionKind::Retired)
                .count(),
            3
        );
    }
    #[test]
    fn async_pending_shutdown_needs_real_event_before_owner_drop_and_join() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(3);
        e.request_stop().unwrap();
        async_wait(&events, "shutdown_pending", 0);
        let premature = e.try_join();
        let alive = e.snapshot().unwrap().worker_alive;
        release_native(&signal);
        join(&mut e);
        let tail: Vec<_> = events.try_iter().collect();
        assert_eq!(premature, Err(Error::Busy));
        assert!(alive);
        assert!(
            tail.contains(&("dispatched", 1))
                && tail.contains(&("shutdown_complete", 0))
                && tail.contains(&("drop", 0))
        );
    }
    #[test]
    fn async_stop_progress_runs_while_last_frame_is_still_live() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(4);
        submit(&mut e, 1);
        async_wait(&events, "advance_pending", 1);
        e.request_stop().unwrap();
        let until = boottime_ns().unwrap() + 100_000_000;
        let mut shutdown_started = false;
        while boottime_ns().unwrap() < until && !shutdown_started {
            shutdown_started |= events.try_iter().any(|v| v == ("shutdown_started", 0));
            thread::yield_now();
        }
        let retained = e.snapshot().unwrap();
        release_native(&signal);
        join(&mut e);
        assert!(
            shutdown_started,
            "native stop-retire progress must run while the last scanout is still retained"
        );
        assert!(retained.worker_alive && retained.occupied_credits == 1);
    }
    #[test]
    fn async_queued_replacement_cannot_starve_ready_native_fd() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(5);
        submit(&mut e, 1);
        async_wait(&events, "advance_pending", 1);
        submit(&mut e, 2);
        async_wait(&events, "prepare", 2);
        submit(&mut e, 3);
        signal.wake.notify().unwrap();
        signal.advance.send(()).unwrap();
        let until = boottime_ns().unwrap() + 2_000_000_000;
        let mut dispatched_before_next_prepare = false;
        loop {
            let value = events.recv_timeout(Duration::from_millis(100)).unwrap();
            if value == ("prepare", 3) {
                break;
            }
            dispatched_before_next_prepare |= value == ("dispatched", 1);
            assert!(boottime_ns().unwrap() < until);
        }
        e.request_stop().unwrap();
        signal.advance.send(()).unwrap();
        release_native(&signal);
        join(&mut e);
        assert!(
            dispatched_before_next_prepare,
            "queued ingress must pass a bounded native readiness dispatch point"
        );
    }
    #[test]
    #[ignore = "owned child mode; the parent retains actual wait/PIPE EOF"]
    fn async_zero_raw_sequence_child() {
        use std::io::Write;
        let (mut e, _signal, events) = async_start(6);
        submit(&mut e, 1);
        async_wait(&events, "retired", 1);
        let until = boottime_ns().unwrap() + 1_000_000_000;
        loop {
            let snapshot = e.snapshot().unwrap();
            if snapshot.stage == Stage::Quarantined {
                println!("ZERO_RAW_SEQUENCE_QUARANTINED");
                std::io::stdout().flush().unwrap();
                loop {
                    thread::park();
                }
            }
            if snapshot.retired == 1 {
                break;
            }
            assert!(boottime_ns().unwrap() < until);
            thread::yield_now();
        }
        e.request_stop().unwrap();
        join(&mut e);
        let mut records = Vec::new();
        drain(&e, &mut records);
        assert!(records.iter().any(|r| matches!(
            r.kind,
            CompletionKind::Presented(PresentationFact {
                correlation_id: 1001,
                sequence: 0,
                timestamp_ns: 0,
                clock: PresentationClock::Monotonic
            })
        )));
        println!("ZERO_RAW_SEQUENCE_DELIVERED");
        std::io::stdout().flush().unwrap();
    }
    #[test]
    fn async_zero_raw_sequence_and_clock_domain_are_preserved() {
        use std::{
            io::Read,
            process::{Command, Stdio},
        };
        unsafe extern "C" {
            fn fcntl(fd: i32, cmd: i32, ...) -> i32;
        }
        struct Child(std::process::Child, bool);
        impl Drop for Child {
            fn drop(&mut self) {
                if !self.1 {
                    let _ = self.0.kill();
                    let _ = self.0.wait();
                }
            }
        }
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "executor::tests::async_zero_raw_sequence_child",
                    "--ignored",
                    "--nocapture",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
            false,
        );
        let mut pipe = child.0.stdout.take().unwrap();
        let flags = unsafe { fcntl(pipe.as_raw_fd(), 3) };
        assert!(flags >= 0 && unsafe { fcntl(pipe.as_raw_fd(), 4, flags | 0x800) } == 0);
        let until = boottime_ns().unwrap() + 3_000_000_000;
        let mut bytes = Vec::new();
        let mut eof = false;
        while boottime_ns().unwrap() < until && bytes.len() < 16384 {
            let mut chunk = [0u8; 1024];
            match pipe.read(&mut chunk) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("child pipe {error}"),
            }
            if bytes
                .windows(29)
                .any(|v| v == b"ZERO_RAW_SEQUENCE_QUARANTINED")
            {
                break;
            }
            if child.0.try_wait().unwrap().is_some() {
                continue;
            }
            thread::yield_now();
        }
        if child.0.try_wait().unwrap().is_none() {
            child.0.kill().unwrap();
        }
        let status = child.0.wait().unwrap();
        child.1 = true;
        let waited_in_time = boottime_ns().unwrap() < until;
        while !eof && boottime_ns().unwrap() < until && bytes.len() < 16384 {
            let mut chunk = [0u8; 1024];
            match pipe.read(&mut chunk) {
                Ok(0) => eof = true,
                Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
                Err(error) => panic!("child pipe {error}"),
            }
        }
        println!(
            "raw-sequence-child actual_wait={} success={} PIPE_EOF={eof} original_cutoff={waited_in_time}",
            child.0.id(),
            status.success()
        );
        assert!(waited_in_time && eof && bytes.len() < 16384);
        assert!(
            status.success()
                && bytes
                    .windows(27)
                    .any(|v| v == b"ZERO_RAW_SEQUENCE_DELIVERED"),
            "raw DRM sequence zero and zero timestamp must survive with separate correlation and real clock domain"
        );
    }
    #[test]
    fn async_completion_budget_preserves_every_undelivered_fact_and_refused_frame() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, _signal, _events) = async_start(2);
        for id in 1..=25 {
            submit(&mut e, id);
            let until = boottime_ns().unwrap() + 2_000_000_000;
            while e.snapshot().unwrap().retired < id {
                assert!(boottime_ns().unwrap() < until);
                thread::yield_now();
            }
        }
        let mut frame_slot = Some(frame(26));
        let result = e.try_submit(
            &mut frame_slot,
            policy(26),
            boottime_ns().unwrap() + 5_000_000_000,
            2626,
        );
        let before = e.snapshot().unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        let mut records = Vec::new();
        drain(&e, &mut records);
        assert_eq!(result, Err(Error::Capacity));
        assert!(frame_slot.is_some());
        assert_eq!(before.accepted, 25);
        assert_eq!(before.occupied_credits, 0);
        assert_eq!(before.reserved_completion_records, 125);
        assert_eq!(records.len(), 125);
        for id in 1..=25 {
            assert_eq!(records.iter().filter(|r| r.admission.id == id).count(), 5);
        }
        assert_eq!(e.snapshot().unwrap().reserved_completion_records, 0);
    }
    #[test]
    fn async_stop_lifecycle_rechecked_after_final_retirement() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, signal, events) = async_start(8);
        submit(&mut e, 1);
        async_wait(&events, "advance_pending", 1);
        e.request_stop().unwrap();
        async_wait(&events, "shutdown_pending", 0);
        release_native(&signal);
        let until = boottime_ns().unwrap() + 2_000_000_000;
        while e.snapshot().unwrap().retired != 1 && boottime_ns().unwrap() < until {
            thread::yield_now();
        }
        let until = boottime_ns().unwrap() + 100_000_000;
        let mut joined_without_extra_wake = false;
        while boottime_ns().unwrap() < until {
            match e.try_join() {
                Ok(()) => {
                    joined_without_extra_wake = true;
                    break;
                }
                Err(Error::Busy) => thread::yield_now(),
                other => panic!("actual join {other:?}"),
            }
        }
        // Always wake the old loop for real owned cleanup before asserting RED.
        e.request_stop().unwrap();
        join(&mut e);
        assert!(
            joined_without_extra_wake,
            "verified final retirement must recheck pending shutdown before sleeping for a nonexistent event"
        );
        assert!(e.snapshot().unwrap().joined && !e.snapshot().unwrap().worker_alive);
    }
    fn bootstrap_input() -> OwnedBootstrap {
        use crate::bootstrap::{BorrowedBootstrapEvent, SessionRenderEvent};
        use std::{
            fs::File,
            os::{fd::AsFd, unix::fs::MetadataExt},
        };
        let file = File::open("/dev/null").unwrap();
        let mut init = SessionRenderEvent {
            event_type: 1,
            session_sequence: 1,
            session_generation: 29,
            active: 1,
            ..Default::default()
        };
        init.seat_name[..5].copy_from_slice(b"seat0");
        let mut add = SessionRenderEvent {
            event_type: 2,
            session_sequence: 2,
            session_generation: 29,
            device_lease_id: 59,
            device_generation: 61,
            rdev: file.metadata().unwrap().rdev(),
            ..Default::default()
        };
        add.device_path[..9].copy_from_slice(b"/dev/null");
        let end = SessionRenderEvent {
            event_type: 3,
            session_sequence: 3,
            session_generation: 29,
            ..Default::default()
        };
        OwnedBootstrap::capture(
            29,
            &[
                BorrowedBootstrapEvent {
                    metadata: &init,
                    primary: None,
                    render: None,
                },
                BorrowedBootstrapEvent {
                    metadata: &add,
                    primary: Some(file.as_fd()),
                    render: None,
                },
                BorrowedBootstrapEvent {
                    metadata: &end,
                    primary: None,
                    render: None,
                },
            ],
        )
        .unwrap()
    }
    fn bootstrap_discovery(input: &OwnedBootstrap) -> Discovery {
        let device = input
            .events()
            .iter()
            .find(|event| event.metadata().event_type == 2)
            .unwrap()
            .metadata();
        let mut outputs = [DiscoveredOutput::default(); MAX_DISCOVERED_OUTPUTS];
        outputs[0] = DiscoveredOutput {
            output_id: 11,
            output_generation: 17,
            device_lease_id: device.device_lease_id,
            device_generation: device.device_generation,
            mode_id: 97,
            mode_generation: 101,
            modifier: 0,
            connector_id: 41,
            width: 32,
            height: 32,
            refresh_millihz: 60_000,
            drm_format: 0x34325241,
            flags: 1,
        };
        Discovery {
            session_generation: input.session_generation(),
            output_count: 1,
            outputs,
        }
    }
    struct BootstrapPlan {
        mode: u32,
        events: Sender<(&'static str, ThreadId)>,
        release: Receiver<()>,
    }
    static BOOTSTRAP_PLAN: Mutex<Option<BootstrapPlan>> = Mutex::new(None);
    struct BootstrapOwner {
        plan: BootstrapPlan,
        input: Option<OwnedBootstrap>,
        thread: ThreadId,
        _not_send: Rc<()>,
    }
    fn bootstrap_factory(_: OwnerFactoryConfig) -> Result<Box<dyn NativeOwner>, Error> {
        let plan = BOOTSTRAP_PLAN
            .lock()
            .unwrap()
            .take()
            .ok_or(Error::Invalid)?;
        let thread = thread::current().id();
        let _ = plan.events.send(("created", thread));
        Ok(Box::new(BootstrapOwner {
            plan,
            input: None,
            thread,
            _not_send: Rc::new(()),
        }))
    }
    impl BootstrapOwner {
        fn mark(&self, event: &'static str) {
            assert_eq!(thread::current().id(), self.thread);
            let _ = self.plan.events.send((event, self.thread));
        }
    }
    impl NativeOwner for BootstrapOwner {
        fn apply_device_change(&mut self, _: &SessionRenderEvent) -> Result<Discovery, Error> {
            Err(Error::Invalid)
        }
        fn bootstrap(&mut self, input: &mut Option<OwnedBootstrap>) -> Result<Discovery, Error> {
            self.input = input.take(); // Retain all duplicate-FD custody before the held native step.
            self.mark("bootstrap");
            if self.plan.mode < 12 {
                self.plan
                    .release
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            self.mark("bootstrap_return");
            if self.plan.mode == 131 {
                return Err(Error::Native);
            }
            let pack = self.input.as_ref().ok_or(Error::Invalid)?;
            let mut value = if self.plan.mode == 150 {
                assert_eq!(pack.device_count(), 0);
                Discovery {
                    session_generation: pack.session_generation(),
                    output_count: 0,
                    outputs: [DiscoveredOutput::default(); MAX_DISCOVERED_OUTPUTS],
                }
            } else {
                bootstrap_discovery(pack)
            };
            match self.plan.mode {
                100 | 130 => value.session_generation += 1,
                101 => value.output_count = 33,
                102 => value.outputs[0].output_id = 0,
                103 => value.outputs[0].output_generation = 0,
                104 => value.outputs[0].device_lease_id += 1,
                105 => value.outputs[0].device_generation += 1,
                106 => value.outputs[0].mode_id = 0,
                107 => value.outputs[0].mode_generation = 0,
                108 => value.outputs[0].connector_id = 0,
                109 => value.outputs[0].width = 0,
                110 => value.outputs[0].height = 0,
                111 => value.outputs[0].refresh_millihz = 0,
                112 => value.outputs[0].drm_format = 0,
                113 => value.outputs[0].flags = 2,
                114 => {
                    value.output_count = 2;
                    value.outputs[1] = value.outputs[0];
                    value.outputs[1].connector_id += 1;
                }
                115 => {
                    value.output_count = 2;
                    value.outputs[1] = value.outputs[0];
                    value.outputs[1].output_id += 1;
                }
                116 => value.outputs[1] = value.outputs[0],
                117 => value.outputs[0].width = 16_385,
                118 => value.outputs[0].height = 16_385,
                119 => value.session_generation = 0,
                _ => {}
            }
            Ok(value)
        }
        fn prepare(&mut self, _: &Frame, _: Admission) -> Result<(), Error> {
            self.mark("prepare");
            Ok(())
        }
        fn execute(
            &mut self,
            _: &Frame,
            admission: Admission,
            attempt: &mut Attempt,
        ) -> Result<NativeResult, Error> {
            self.mark("execute");
            if self.plan.mode == 140 || self.plan.mode == 141 {
                let objects = [NativeObject {
                    id: 41,
                    generation: 3,
                    output_id: admission.policy.output_id,
                    output_generation: admission.policy.output_generation,
                }];
                let scope = NativeScope {
                    policy: admission.policy,
                    device_lease_id: if self.plan.mode == 140 { 60 } else { 59 },
                    device_generation: if self.plan.mode == 141 { 62 } else { 61 },
                    objects: &objects,
                };
                attempt.authorize(&scope)?;
                return Ok(NativeResult::Returned {
                    operation: 1,
                    value: 0,
                    errno: 0,
                    succeeded: true,
                    extra_attempt_refused: false,
                });
            }
            Ok(NativeResult::NotEntered)
        }
        fn advance(&mut self, _: &RetirementContext<'_>) -> Result<NativeProgress, Error> {
            Ok(NativeProgress::retired())
        }
        fn poll_sources(
            &mut self,
            _: &mut [PollSource; MAX_NATIVE_SOURCES],
        ) -> Result<usize, Error> {
            Ok(0)
        }
        fn dispatch(&mut self, _: NativeReady) -> Result<(), Error> {
            Err(Error::Protocol)
        }
        fn shutdown_progress(&mut self) -> Result<RetirementProgress, Error> {
            if self.plan.mode == 130 || self.plan.mode == 131 {
                assert!(self.input.as_ref().unwrap().events()[1].primary().is_some());
                self.mark("shutdown_held");
                self.plan
                    .release
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| Error::Native)?;
            }
            drop(self.input.take());
            self.mark("shutdown");
            Ok(RetirementProgress::Verified(RetirementProof))
        }
    }
    impl Drop for BootstrapOwner {
        fn drop(&mut self) {
            self.mark("drop");
        }
    }
    fn bootstrap_start(mode: u32) -> (Executor, Sender<()>, Receiver<(&'static str, ThreadId)>) {
        bootstrap_start_until(mode, boottime_ns().unwrap() + 5_000_000_000)
    }
    fn bootstrap_start_until(
        mode: u32,
        cutoff: u64,
    ) -> (Executor, Sender<()>, Receiver<(&'static str, ThreadId)>) {
        let (tx, events) = mpsc::channel();
        let (release, go) = mpsc::channel();
        *BOOTSTRAP_PLAN.lock().unwrap() = Some(BootstrapPlan {
            mode,
            events: tx,
            release: go,
        });
        (
            Executor::new(
                1,
                bootstrap_factory,
                crate::executor_ffi::test_config(),
                bootstrap_input(),
                cutoff,
            )
            .unwrap(),
            release,
            events,
        )
    }
    fn bootstrap_observed(
        events: &Receiver<(&'static str, ThreadId)>,
    ) -> (bool, Vec<(&'static str, ThreadId)>) {
        let until = boottime_ns().unwrap() + 100_000_000;
        let mut seen = Vec::new();
        while boottime_ns().unwrap() < until {
            for event in events.try_iter() {
                let entered = event.0 == "bootstrap";
                seen.push(event);
                if entered {
                    return (true, seen);
                }
            }
            thread::yield_now();
        }
        (false, seen)
    }
    #[test]
    fn bootstrap_dispatch_precedes_frame_admission_and_preserves_held_owner() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = bootstrap_start(10);
        let (entered, mut seen) = bootstrap_observed(&events);
        let mut input = Some(frame(1));
        let admitted = e.try_submit(
            &mut input,
            policy(1),
            boottime_ns().unwrap() + 5_000_000_000,
            101,
        );
        let start = boottime_ns().unwrap();
        let responsive = (0..1024).all(|_| !e.snapshot().unwrap().ready);
        let elapsed = boottime_ns().unwrap() - start;
        let busy_join = e.try_join();
        let before = e.snapshot().unwrap();
        e.request_stop().unwrap();
        let _ = release.send(());
        join(&mut e);
        seen.extend(events.try_iter());
        assert!(entered, "required worker bootstrap was never entered");
        assert_eq!(admitted, Err(Error::Busy));
        assert!(input.is_some());
        assert!(
            responsive
                && elapsed < 100_000_000
                && before.accepted == 0
                && before.occupied_credits == 0
        );
        assert_eq!(busy_join, Err(Error::Busy));
        let thread = seen.iter().find(|v| v.0 == "created").unwrap().1;
        assert!(seen.iter().all(|v| v.1 == thread));
        assert!(seen.iter().any(|v| v.0 == "drop"));
    }
    #[test]
    fn bootstrap_late_success_after_stop_cannot_publish_ready_or_outputs() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = bootstrap_start(11);
        let (entered, _) = bootstrap_observed(&events);
        e.request_stop().unwrap();
        let was_busy = e.try_join();
        let _ = release.send(());
        join(&mut e);
        let mut input = Some(frame(1));
        let refused = e.try_submit(
            &mut input,
            policy(1),
            boottime_ns().unwrap() + 5_000_000_000,
            101,
        );
        let ready = e.snapshot().unwrap().ready;
        let discovery = e.try_discovery().unwrap();
        assert!(entered, "required worker bootstrap was never entered");
        assert_eq!(was_busy, Err(Error::Busy));
        assert!(!ready && discovery.is_none());
        assert_eq!(refused, Err(Error::Stopping));
        assert!(input.is_some());
    }
    #[test]
    fn bootstrap_copies_actual_output_mode_and_format_before_valid_frame() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, _release, _events) = bootstrap_start(12);
        let until = boottime_ns().unwrap() + 100_000_000;
        let mut discovery = None;
        while discovery.is_none() && boottime_ns().unwrap() < until {
            match e.try_discovery() {
                Ok(value) => discovery = value,
                Err(Error::Busy) => {}
                Err(error) => panic!("discovery {error:?}"),
            }
            thread::yield_now();
        }
        let ready = e.snapshot().unwrap().ready;
        let admitted = if discovery.is_some() {
            let mut input = Some(frame(1));
            let until = boottime_ns().unwrap() + 100_000_000;
            let cutoff = boottime_ns().unwrap() + 5_000_000_000;
            loop {
                let result = e.try_submit(&mut input, policy(1), cutoff, 101);
                if result != Err(Error::Busy) || boottime_ns().unwrap() >= until {
                    break Some(result);
                }
                thread::yield_now();
            }
        } else {
            None
        };
        e.request_stop().unwrap();
        join(&mut e);
        assert!(
            discovery.is_some(),
            "real worker discovery was not published"
        );
        assert!(ready);
        assert!(admitted.unwrap().is_ok());
        let d = discovery.unwrap();
        assert_eq!((d.session_generation, d.output_count), (29, 1));
        assert_eq!(
            d.outputs[0],
            bootstrap_discovery(&bootstrap_input()).outputs[0]
        );
        assert_eq!(
            (
                d.outputs[0].mode_id,
                d.outputs[0].mode_generation,
                d.outputs[0].refresh_millihz,
                d.outputs[0].drm_format
            ),
            (97, 101, 60000, 0x34325241)
        );
    }

    fn await_discovery(e: &mut Executor) -> Option<Discovery> {
        let until = boottime_ns().unwrap() + 100_000_000;
        loop {
            match e.try_discovery() {
                Ok(Some(value)) => return Some(value),
                Ok(None) | Err(Error::Busy) => {}
                Err(error) => panic!("discovery {error:?}"),
            }
            if e.snapshot().unwrap().stopping || boottime_ns().unwrap() >= until {
                return None;
            }
            thread::yield_now();
        }
    }
    #[test]
    fn bootstrap_rejects_malformed_or_unbound_discovery_before_ready() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let mut accepted = Vec::new();
        for mode in 100..120 {
            let (mut e, _release, events) = bootstrap_start(mode);
            let discovery = await_discovery(&mut e);
            let before = e.snapshot().unwrap();
            e.request_stop().unwrap();
            join(&mut e);
            let seen: Vec<_> = events.try_iter().collect();
            assert!(seen.iter().any(|v| v.0 == "bootstrap_return"));
            assert!(seen.iter().any(|v| v.0 == "shutdown") && seen.iter().any(|v| v.0 == "drop"));
            if discovery.is_some() || before.ready || !before.failed || !before.stopping {
                accepted.push(mode);
            }
        }
        assert!(
            accepted.is_empty(),
            "unchecked discovery became Ready: {accepted:?}"
        );
    }
    #[test]
    fn bootstrap_frame_admission_matches_discovered_session_output_and_dimensions() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, _release, _events) = bootstrap_start(12);
        assert!(await_discovery(&mut e).is_some());
        let mut accepted = Vec::new();
        for case in 0..5 {
            let sequence = case + 1;
            let mut target = *frame(sequence).target();
            let mut requested = policy(sequence);
            match case {
                0 => requested.session_generation += 1,
                1 => {
                    target.output_id += 1;
                    requested.output_id += 1;
                }
                2 => {
                    target.output_generation += 1;
                    requested.output_generation += 1;
                }
                3 => {
                    target.width += 1;
                    target.transformed_width += 1;
                }
                4 => {
                    target.height += 1;
                    target.transformed_height += 1;
                }
                _ => unreachable!(),
            }
            let mut input = Some(frame_target(sequence, target));
            let until = boottime_ns().unwrap() + 100_000_000;
            let cutoff = boottime_ns().unwrap() + 5_000_000_000;
            let result = loop {
                let result = e.try_submit(&mut input, requested, cutoff, sequence * 101);
                if result != Err(Error::Busy) || boottime_ns().unwrap() >= until {
                    break result;
                }
                thread::yield_now();
            };
            if result != Err(Error::Invalid) || input.is_none() {
                accepted.push(case);
            }
        }
        let before = e.snapshot().unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        assert!(
            accepted.is_empty(),
            "frames absent from actual discovery admitted: {accepted:?}"
        );
        assert_eq!(
            (
                before.accepted,
                before.occupied_credits,
                before.reserved_completion_records
            ),
            (0, 0, 0)
        );
    }
    #[test]
    fn bootstrap_invalid_result_retains_worker_and_owned_input_through_shutdown() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = bootstrap_start(130);
        let discovery = await_discovery(&mut e);
        let before = e.snapshot().unwrap();
        // Both baseline and repair reach held native shutdown before assertions.
        e.request_stop().unwrap();
        await_stage(&events, "shutdown_held");
        let start = boottime_ns().unwrap();
        let responsive = (0..1024).all(|_| {
            let s = e.snapshot().unwrap();
            s.worker_alive && !s.ready && s.stopping
        });
        let elapsed = boottime_ns().unwrap() - start;
        let busy = e.try_join();
        release.send(()).unwrap();
        join(&mut e);
        let seen: Vec<_> = events.try_iter().collect();
        assert!(
            discovery.is_none() && !before.ready && before.failed && before.stopping,
            "malformed successful bootstrap must become terminal before native cleanup"
        );
        assert!(responsive && elapsed < 100_000_000);
        assert_eq!(busy, Err(Error::Busy));
        assert_eq!(
            seen.iter().map(|v| v.0).collect::<Vec<_>>(),
            vec!["shutdown", "drop"]
        );
    }

    #[test]
    fn bootstrap_final_attempt_must_match_discovered_device_before_grant() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let mut wrongly_granted = Vec::new();
        for mode in 140..=141 {
            let (mut e, _release, _events) = bootstrap_start(mode);
            assert!(await_discovery(&mut e).is_some());
            let mut input = Some(frame(1));
            let cutoff = boottime_ns().unwrap() + 5_000_000_000;
            let until = boottime_ns().unwrap() + 100_000_000;
            loop {
                match e.try_submit(&mut input, policy(1), cutoff, 101) {
                    Ok(_) => break,
                    Err(Error::Busy) if boottime_ns().unwrap() < until => thread::yield_now(),
                    other => panic!("valid admission {other:?}"),
                }
            }
            let until = boottime_ns().unwrap() + 100_000_000;
            while e.snapshot().unwrap().retired != 1 && boottime_ns().unwrap() < until {
                thread::yield_now();
            }
            e.request_stop().unwrap();
            join(&mut e);
            let mut records = Vec::new();
            drain(&e, &mut records);
            assert!(
                records
                    .iter()
                    .any(|r| r.kind == CompletionKind::Retired && r.resources_retired)
            );
            if records
                .iter()
                .any(|r| r.grant.is_some() || r.kind == CompletionKind::Attempt)
                || !records.iter().any(|r| {
                    r.kind == CompletionKind::SubmissionOutcome
                        && r.error == Some(Error::Invalid)
                        && r.native == NativeResult::NotEntered
                })
            {
                wrongly_granted.push(mode);
            }
        }
        assert!(
            wrongly_granted.is_empty(),
            "wrong discovered device obtained final grant: {wrongly_granted:?}"
        );
    }

    #[test]
    fn bootstrap_expired_constructor_refuses_before_starting_worker() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (tx, events) = mpsc::channel();
        let (_release, go) = mpsc::channel();
        *BOOTSTRAP_PLAN.lock().unwrap() = Some(BootstrapPlan {
            mode: 12,
            events: tx,
            release: go,
        });
        let result = Executor::new(
            1,
            bootstrap_factory,
            crate::executor_ffi::test_config(),
            bootstrap_input(),
            boottime_ns().unwrap() - 1,
        );
        let refused = match result {
            Err(Error::Expired) => true,
            Ok(mut e) => {
                e.request_stop().unwrap();
                join(&mut e);
                false
            }
            Err(error) => panic!("constructor {error:?}"),
        };
        let unused = BOOTSTRAP_PLAN.lock().unwrap().take().is_some();
        assert!(
            refused && unused && events.try_iter().next().is_none(),
            "expired original startup deadline must refuse before worker spawn"
        );
    }
    #[test]
    fn bootstrap_original_deadline_watch_expires_held_native_without_detach_or_late_ready() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let cutoff = boottime_ns().unwrap() + 100_000_000;
        let (mut e, release, events) = bootstrap_start_until(10, cutoff);
        let (entered, _) = bootstrap_observed(&events);
        let fd = e.startup_deadline_fd().unwrap();
        let mut pollfd = PollFd {
            fd,
            events: 1,
            revents: 0,
        };
        let readable = unsafe { poll(&mut pollfd, 1, 200) };
        let before = e.snapshot().unwrap();
        let busy = e.try_join();
        let mut input = Some(frame(1));
        let refused = e.try_submit(
            &mut input,
            policy(1),
            boottime_ns().unwrap() + 5_000_000_000,
            101,
        );
        e.request_stop().unwrap();
        release.send(()).unwrap();
        join(&mut e);
        let after = e.snapshot().unwrap();
        let discovery = e.try_discovery().unwrap();
        assert!(
            entered && fd >= 0 && readable == 1 && pollfd.revents == 1,
            "held native bootstrap needs a real owned startup deadline watch"
        );
        assert!(
            before.stopping && before.failed && before.worker_alive && !before.ready,
            "original BOOTTIME startup cutoff must expire without restarting"
        );
        assert_eq!(busy, Err(Error::Busy));
        assert_eq!(refused, Err(Error::Stopping));
        assert!(input.is_some());
        assert!(!after.ready && after.joined && !after.worker_alive && discovery.is_none());
    }
    #[test]
    fn bootstrap_candidate_observed_after_deadline_cannot_become_ready() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let cutoff = boottime_ns().unwrap() + 100_000_000;
        let (mut e, _release, events) = bootstrap_start_until(12, cutoff);
        await_stage(&events, "bootstrap_return");
        while boottime_ns().unwrap() <= cutoff {
            thread::yield_now();
        }
        let discovery = e.try_discovery().unwrap();
        let before = e.snapshot().unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        assert!(
            discovery.is_none() && before.stopping && before.failed && !before.ready,
            "late control observation must not activate expired bootstrap evidence"
        );
    }
    #[test]
    fn bootstrap_accepted_ready_disarms_deadline_and_stale_watch_does_not_stop_owner() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let cutoff = boottime_ns().unwrap() + 100_000_000;
        let (mut e, _release, _events) = bootstrap_start_until(12, cutoff);
        let discovery = await_discovery(&mut e);
        while boottime_ns().unwrap() <= cutoff {
            thread::yield_now();
        }
        let fd = e.startup_deadline_fd().unwrap();
        let is_readable = readable(fd);
        let after = e.snapshot().unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        assert!(
            discovery.is_some() && after.ready && !after.stopping && !after.failed && !is_readable,
            "accepted startup must disarm its timer; stale readiness cannot stop a live owner"
        );
    }

    #[test]
    fn bootstrap_invalid_frame_does_not_supersede_an_earlier_unsealed_ticket() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = start(0);
        submit(&mut e, 1);
        await_stage(&events, "prepare");
        let mut invalid_policy = policy(2);
        invalid_policy.session_generation += 1;
        let mut input = Some(frame(2));
        let refused = e.try_submit(
            &mut input,
            invalid_policy,
            boottime_ns().unwrap() + 5_000_000_000,
            202,
        );
        let before = e.snapshot().unwrap();
        release.send(()).unwrap();
        await_stage(&events, "sealed");
        e.request_stop().unwrap();
        join(&mut e);
        let completion = next_retired(&e).unwrap();
        assert_eq!(refused, Err(Error::Invalid));
        assert!(input.is_some());
        assert_eq!(before.accepted, 1);
        assert_eq!(completion.grant.unwrap().policy(), policy(1));
        assert!(completion.error.is_none());
    }
    #[test]
    fn bootstrap_empty_input_and_no_outputs_never_mint_frame_authority() {
        use crate::bootstrap::{BorrowedBootstrapEvent, SessionRenderEvent};
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let mut init = SessionRenderEvent {
            event_type: 1,
            session_sequence: 1,
            session_generation: 29,
            ..Default::default()
        };
        init.seat_name[..5].copy_from_slice(b"seat0");
        let end = SessionRenderEvent {
            event_type: 3,
            session_sequence: 2,
            session_generation: 29,
            ..Default::default()
        };
        let input = OwnedBootstrap::capture(
            29,
            &[
                BorrowedBootstrapEvent {
                    metadata: &init,
                    primary: None,
                    render: None,
                },
                BorrowedBootstrapEvent {
                    metadata: &end,
                    primary: None,
                    render: None,
                },
            ],
        )
        .unwrap();
        let (tx, events) = mpsc::channel();
        let (_release, go) = mpsc::channel();
        *BOOTSTRAP_PLAN.lock().unwrap() = Some(BootstrapPlan {
            mode: 150,
            events: tx,
            release: go,
        });
        let mut e = Executor::new(
            1,
            bootstrap_factory,
            crate::executor_ffi::test_config(),
            input,
            boottime_ns().unwrap() + 5_000_000_000,
        )
        .unwrap();
        let discovery = await_discovery(&mut e).unwrap();
        let mut frame_slot = Some(frame(1));
        let refused = e.try_submit(
            &mut frame_slot,
            policy(1),
            boottime_ns().unwrap() + 5_000_000_000,
            101,
        );
        let before = e.snapshot().unwrap();
        e.request_stop().unwrap();
        join(&mut e);
        let seen: Vec<_> = events.try_iter().collect();
        assert_eq!(discovery.output_count, 0);
        assert!(before.ready);
        assert_eq!(refused, Err(Error::Invalid));
        assert!(frame_slot.is_some());
        assert_eq!(before.accepted, 0);
        assert!(seen.iter().any(|v| v.0 == "shutdown") && seen.iter().any(|v| v.0 == "drop"));
    }

    #[test]
    fn bootstrap_error_wakes_control_before_blocked_cleanup_finishes() {
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let (mut e, release, events) = bootstrap_start(131);
        await_stage(&events, "shutdown_held");
        let wake = readable(e.completion_fd().unwrap());
        let timer_ready = readable(e.startup_deadline_fd().unwrap());
        let before = e.snapshot().unwrap();
        let busy = e.try_join();
        release.send(()).unwrap();
        join(&mut e);
        assert!(
            before.failed
                && before.stopping
                && !before.ready
                && before.worker_alive
                && !timer_ready
        );
        assert_eq!(busy, Err(Error::Busy));
        assert!(
            wake,
            "terminal bootstrap error must wake control before native cleanup returns"
        );
    }
}
