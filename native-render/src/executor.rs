//! Retained render worker and CPU-only control path.
//!
//! NativeOwner is the trusted, worker-local adapter boundary. A factory is a
//! copied function pointer, never a captured control/Wayland/graphics object.
//! Its production AQ/GL binding and authenticated device scope remain required.
use crate::{
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
    scope: Option<RecordedNativeScope>,
    ticket: Option<CommitTicket>,
    policy: OperationPolicy,
    sealed: Option<SealedAttempt>,
}
impl Attempt {
    pub fn authorize(&mut self, actual: &NativeScope<'_>) -> Result<&SealedAttempt, Error> {
        let ticket = self.ticket.take().ok_or(Error::Stale)?;
        if actual.policy != self.policy
            || actual.device_lease_id == 0
            || actual.device_generation == 0
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
        self.scope = Some(RecordedNativeScope {
            policy: actual.policy,
            device_lease_id: actual.device_lease_id,
            device_generation: actual.device_generation,
            count: actual.objects.len(),
            objects,
        });
        self.sealed = Some(ticket.seal()?);
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
pub type OwnerFactory = fn() -> Result<Box<dyn NativeOwner>, Error>;

struct EventFd(OwnedFd, #[cfg(test)] AtomicBool);
unsafe extern "C" {
    fn eventfd(init: u32, flags: i32) -> i32;
    fn read(fd: i32, data: *mut std::ffi::c_void, count: usize) -> isize;
    fn write(fd: i32, data: *const std::ffi::c_void, count: usize) -> isize;
    fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
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
struct Job {
    frame: Frame,
    admission: Admission,
    ticket: CommitTicket,
}
struct Shared {
    ingress: Mutex<VecDeque<Job>>,
    completions: Mutex<VecDeque<Completion>>,
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
        self.stop_gate.stop();
        self.stop.store(true, Ordering::Release);
        self.wake.notify()
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
    _control_only: PhantomData<Rc<()>>,
}
impl Executor {
    pub fn new(security_generation: u64, factory: OwnerFactory) -> Result<Self, Error> {
        let authority = CommitAuthority::new(security_generation)?;
        let shared = Arc::new(Shared {
            ingress: Mutex::new(VecDeque::with_capacity(FRAME_CREDITS as usize)),
            completions: Mutex::new(VecDeque::with_capacity(COMPLETION_CAPACITY)),
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
            .spawn(move || run(worker, factory))
            .map_err(|_| Error::Native)?;
        Ok(Self {
            control: thread::current().id(),
            shared,
            authority,
            security_generation,
            next_id: 0,
            handle: Some(handle),
            joined: false,
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
    /// On EVERY refusal the frame stays in its original slot; earlier authority
    /// stays unchanged. All credits/queue capacity reserve before issue/CAS.
    pub fn try_submit(
        &mut self,
        frame: &mut Option<Frame>,
        policy: OperationPolicy,
        cutoff: u64,
        cookie: u64,
    ) -> Result<Admission, Error> {
        self.control()?;
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(Error::Stopping);
        }
        let f = frame.as_ref().ok_or(Error::Invalid)?;
        if f.security_generation() != policy.security_generation
            || f.scene_sequence() != policy.scene_sequence
            || f.target().output_id != policy.output_id
            || f.target().output_generation != policy.output_generation
        {
            return Err(Error::Invalid);
        }
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
            frame: frame.take().expect("validated owned frame"),
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
    pub fn request_stop(&self) -> Result<(), Error> {
        self.control()?;
        self.shared.stop()
    }
    pub fn completion_fd(&self) -> Result<i32, Error> {
        self.control()?;
        Ok(self.shared.complete.0.as_raw_fd())
    }
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        self.control()?;
        let stage = match self.shared.stage.load(Ordering::Acquire) {
            0 => Stage::Starting,
            1 => Stage::Idle,
            2 => Stage::Preparing,
            3 => Stage::Executing,
            4 => Stage::Retiring,
            5 => Stage::Destroying,
            6 => Stage::Quarantined,
            _ => Stage::Exited,
        };
        Ok(Snapshot {
            stage,
            worker_alive: self.shared.alive.load(Ordering::Acquire),
            stopping: self.shared.stop.load(Ordering::Acquire),
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
    /// The caller must first unregister the borrowed completion fd. None is a
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
fn run(shared: Arc<Shared>, factory: OwnerFactory) {
    let mut owner = match native_call(&shared, factory) {
        Ok(owner) => owner,
        Err(_) => shared.quarantine(),
    };
    let mut entries = Vec::<Entry>::with_capacity(FRAME_CREDITS as usize);
    loop {
        shared.stage.store(Stage::Idle as u32, Ordering::Release);
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
        let blocking = shared.queued.load(Ordering::Acquire) == 0;
        wait_native(&shared, &mut *owner, blocking);
    }
    native_call(&shared, || drop(owner)); // Actual creation worker, after verified native drain.
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
    fn factory() -> Result<Box<dyn NativeOwner>, Error> {
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
            target: FrameTargetV1 {
                output_id: 11,
                output_generation: 17,
                width: 32,
                height: 32,
                transformed_width: 32,
                transformed_height: 32,
                scale: 1.0,
                ..Default::default()
            },
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
        (Executor::new(1, factory).unwrap(), release, events)
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
        e.try_submit(
            &mut f,
            policy(sequence),
            boottime_ns().unwrap() + 5_000_000_000,
            sequence * 101,
        )
        .unwrap();
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
    fn async_factory() -> Result<Box<dyn NativeOwner>, Error> {
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
        let executor = Executor::new(1, async_factory).unwrap();
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
}
