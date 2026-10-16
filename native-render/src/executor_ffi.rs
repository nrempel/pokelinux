//! Current C boundary for the retained Rust render executor.
//!
//! Callback tables and scalar options are copied, never control userdata. Native
//! holders are acquired, retained and destroyed on the worker. An authored table
//! can test this ABI but cannot authenticate DRM routing or retirement itself.
use crate::{
    bootstrap::{BorrowedBootstrapEvent, MAX_BOOTSTRAP_EVENTS, OwnedBootstrap, SessionRenderEvent},
    commit::OperationPolicy,
    executor::{self, *},
    frame::Frame,
};
use std::{
    cell::UnsafeCell,
    ffi::c_void,
    os::fd::{AsRawFd, BorrowedFd},
    ptr,
    thread::{self, ThreadId},
};

const ABI: u32 = 1;
const EMPTY: i32 = 13;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OwnerOptionsV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub owner_id: u64,
    pub reserved: [u64; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NativeOwnerFunctionsV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub reserved: [u64; 4],
    pub create: Option<unsafe extern "C" fn(*const OwnerOptionsV1) -> *mut c_void>,
    pub destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    pub bootstrap:
        Option<unsafe extern "C" fn(*mut c_void, *const BootstrapViewV1, *mut DiscoveryV1) -> i32>,
    pub prepare: Option<unsafe extern "C" fn(*mut c_void, *const Frame, *const AdmissionV1) -> i32>,
    pub execute: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *const Frame,
            *const AdmissionV1,
            *mut PrAttempt,
            *mut NativeResultV1,
        ) -> i32,
    >,
    pub advance: Option<
        unsafe extern "C" fn(*mut c_void, *const RetirementContextV1, *mut NativeProgressV1) -> i32,
    >,
    pub poll_sources:
        Option<unsafe extern "C" fn(*mut c_void, *mut PollSourceV1, u32, *mut u32) -> i32>,
    pub dispatch: Option<unsafe extern "C" fn(*mut c_void, *const NativeReadyV1) -> i32>,
    pub shutdown_progress: Option<unsafe extern "C" fn(*mut c_void, *mut u32) -> i32>,
}
impl OwnerFactoryConfig {
    /// Validates and owns only a copied current table and bounded scalar options.
    /// No native holder exists until the retained worker calls bootstrap.
    ///
    /// # Safety
    /// All callbacks obey pokelinux_render_executor.h, including no unwinding,
    /// worker-local custody, synchronous borrows, truthful native progress, and
    /// code lifetime through destruction and actual join. No pointer encoding in
    /// scalar options and no control/Wayland/AQ/GL graph is transferred here.
    pub unsafe fn new(
        native: NativeOwnerFunctionsV1,
        options: OwnerOptionsV1,
    ) -> Result<Self, Error> {
        if native.abi_version != ABI
            || native.struct_size as usize != size_of::<NativeOwnerFunctionsV1>()
            || native.reserved != [0; 4]
            || native.create.is_none()
            || native.destroy.is_none()
            || native.bootstrap.is_none()
            || native.prepare.is_none()
            || native.execute.is_none()
            || native.advance.is_none()
            || native.poll_sources.is_none()
            || native.dispatch.is_none()
            || native.shutdown_progress.is_none()
            || options.abi_version != ABI
            || options.struct_size as usize != size_of::<OwnerOptionsV1>()
            || options.owner_id == 0
            || options.reserved != [0; 4]
        {
            return Err(Error::Invalid);
        }
        Ok(Self { native, options })
    }
    pub fn native_functions(&self) -> NativeOwnerFunctionsV1 {
        self.native
    }
    pub fn options(&self) -> OwnerOptionsV1 {
        self.options
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BootstrapEventV1 {
    pub metadata: SessionRenderEvent,
    pub primary_fd: i32,
    pub render_fd: i32,
}
#[repr(C)]
pub struct BootstrapViewV1 {
    pub session_generation: u64,
    pub event_count: u32,
    pub reserved: u32,
    pub events: *const BootstrapEventV1,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PolicyV1 {
    pub security_generation: u64,
    pub output_id: u64,
    pub output_generation: u64,
    pub scene_sequence: u64,
    pub session_generation: u64,
}
impl From<PolicyV1> for OperationPolicy {
    fn from(v: PolicyV1) -> Self {
        Self {
            security_generation: v.security_generation,
            output_id: v.output_id,
            output_generation: v.output_generation,
            scene_sequence: v.scene_sequence,
            session_generation: v.session_generation,
        }
    }
}
impl From<OperationPolicy> for PolicyV1 {
    fn from(v: OperationPolicy) -> Self {
        Self {
            security_generation: v.security_generation,
            output_id: v.output_id,
            output_generation: v.output_generation,
            scene_sequence: v.scene_sequence,
            session_generation: v.session_generation,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AdmissionV1 {
    pub id: u64,
    pub policy: PolicyV1,
    pub control_cookie: u64,
}
impl From<Admission> for AdmissionV1 {
    fn from(v: Admission) -> Self {
        Self {
            id: v.id,
            policy: v.policy.into(),
            control_cookie: v.control_cookie,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct NativeObjectV1 {
    pub id: u32,
    pub reserved: u32,
    pub generation: u64,
    pub output_id: u64,
    pub output_generation: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeScopeV1 {
    pub policy: PolicyV1,
    pub device_lease_id: u64,
    pub device_generation: u64,
    pub object_count: u32,
    pub reserved: u32,
    pub objects: [NativeObjectV1; 128],
}
impl Default for NativeScopeV1 {
    fn default() -> Self {
        Self {
            policy: PolicyV1::default(),
            device_lease_id: 0,
            device_generation: 0,
            object_count: 0,
            reserved: 0,
            objects: [NativeObjectV1::default(); 128],
        }
    }
}
impl From<&RecordedNativeScope> for NativeScopeV1 {
    fn from(v: &RecordedNativeScope) -> Self {
        let mut value = Self {
            policy: v.policy.into(),
            device_lease_id: v.device_lease_id,
            device_generation: v.device_generation,
            object_count: v.objects().len() as u32,
            ..Self::default()
        };
        for (out, obj) in value.objects.iter_mut().zip(v.objects()) {
            *out = NativeObjectV1 {
                id: obj.id,
                reserved: 0,
                generation: obj.generation,
                output_id: obj.output_id,
                output_generation: obj.output_generation,
            };
        }
        value
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GrantV1 {
    pub serial: u64,
    pub policy: PolicyV1,
    pub at_boottime_ns: u64,
}
impl From<&crate::commit::SealedAttempt> for GrantV1 {
    fn from(v: &crate::commit::SealedAttempt) -> Self {
        Self {
            serial: v.serial(),
            policy: v.policy().into(),
            at_boottime_ns: v.granted_at_boottime_ns(),
        }
    }
}
impl From<GrantObservation> for GrantV1 {
    fn from(v: GrantObservation) -> Self {
        Self {
            serial: v.serial(),
            policy: v.policy().into(),
            at_boottime_ns: v.granted_at_boottime_ns(),
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NativeResultV1 {
    pub kind: u32,
    pub operation: u32,
    pub value: i64,
    pub error_number: i32,
    pub succeeded: u32,
    pub extra_attempt_refused: u32,
    pub reserved: u32,
}
impl TryFrom<NativeResultV1> for NativeResult {
    type Error = Error;
    fn try_from(v: NativeResultV1) -> Result<Self, Error> {
        if v.reserved != 0 || v.succeeded > 1 || v.extra_attempt_refused > 1 {
            return Err(Error::Protocol);
        }
        match v.kind {
            0 | 1
                if v.operation == 0
                    && v.value == 0
                    && v.error_number == 0
                    && v.succeeded == 0
                    && v.extra_attempt_refused == 0 =>
            {
                Ok(if v.kind == 0 {
                    Self::NotEntered
                } else {
                    Self::Unknown
                })
            }
            2 if v.operation != 0 => Ok(Self::Returned {
                operation: v.operation,
                value: v.value,
                errno: v.error_number,
                succeeded: v.succeeded == 1,
                extra_attempt_refused: v.extra_attempt_refused == 1,
            }),
            _ => Err(Error::Protocol),
        }
    }
}
impl From<NativeResult> for NativeResultV1 {
    fn from(v: NativeResult) -> Self {
        match v {
            NativeResult::NotEntered => Self::default(),
            NativeResult::Unknown => Self {
                kind: 1,
                ..Self::default()
            },
            NativeResult::Returned {
                operation,
                value,
                errno,
                succeeded,
                extra_attempt_refused,
            } => Self {
                kind: 2,
                operation,
                value,
                error_number: errno,
                succeeded: u32::from(succeeded),
                extra_attempt_refused: u32::from(extra_attempt_refused),
                reserved: 0,
            },
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NativeProgressV1 {
    pub fence_identity: u64,
    pub presentation_correlation_id: u64,
    pub presentation_sequence: u64,
    pub presentation_timestamp_ns: u64,
    pub presentation_clock: u32,
    pub has_presentation: u32,
    pub resources_retired: u32,
    pub reserved: u32,
}
impl TryFrom<NativeProgressV1> for NativeProgress {
    type Error = Error;
    fn try_from(v: NativeProgressV1) -> Result<Self, Error> {
        if v.reserved != 0
            || v.has_presentation > 1
            || v.resources_retired > 1
            || (v.has_presentation == 0
                && (v.presentation_correlation_id != 0
                    || v.presentation_sequence != 0
                    || v.presentation_timestamp_ns != 0
                    || v.presentation_clock != 0))
        {
            return Err(Error::Protocol);
        }
        let presentation = if v.has_presentation == 1 {
            if v.presentation_correlation_id == 0 {
                return Err(Error::Protocol);
            }
            Some(PresentationFact {
                correlation_id: v.presentation_correlation_id,
                sequence: v.presentation_sequence,
                timestamp_ns: v.presentation_timestamp_ns,
                clock: match v.presentation_clock {
                    0 => PresentationClock::Monotonic,
                    1 => PresentationClock::Realtime,
                    2 => PresentationClock::Boottime,
                    _ => return Err(Error::Protocol),
                },
            })
        } else {
            None
        };
        Ok(Self {
            fence: (v.fence_identity != 0).then_some(FenceFact {
                identity: v.fence_identity,
            }),
            presentation,
            retirement: if v.resources_retired == 1 {
                RetirementProgress::Verified(RetirementProof)
            } else {
                RetirementProgress::Pending
            },
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RetirementContextV1 {
    pub admission: AdmissionV1,
    pub native: NativeResultV1,
    pub has_grant: u32,
    pub has_scope: u32,
    pub grant: GrantV1,
    pub scope: NativeScopeV1,
}
impl From<&RetirementContext<'_>> for RetirementContextV1 {
    fn from(v: &RetirementContext<'_>) -> Self {
        Self {
            admission: v.admission.into(),
            native: v.native.into(),
            has_grant: u32::from(v.grant.is_some()),
            has_scope: u32::from(v.scope.is_some()),
            grant: v.grant.map(GrantV1::from).unwrap_or_default(),
            scope: v.scope.map(NativeScopeV1::from).unwrap_or_default(),
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollSourceV1 {
    pub fd: i32,
    pub reserved: u32,
    pub token: u64,
    pub generation: u64,
    pub events: i16,
    pub reserved2: [u16; 3],
}
impl Default for PollSourceV1 {
    fn default() -> Self {
        Self {
            fd: -1,
            reserved: 0,
            token: 0,
            generation: 0,
            events: 0,
            reserved2: [0; 3],
        }
    }
}
impl From<PollSource> for PollSourceV1 {
    fn from(v: PollSource) -> Self {
        Self {
            fd: v.fd,
            token: v.token,
            generation: v.generation,
            events: v.events,
            ..Self::default()
        }
    }
}
#[repr(C)]
pub struct NativeReadyV1 {
    pub source: PollSourceV1,
    pub revents: i16,
    pub reserved: [u16; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DiscoveredOutputV1 {
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
macro_rules! output_conversion {
    ($from:ty,$to:ty) => {
        impl From<$from> for $to {
            fn from(v: $from) -> Self {
                Self {
                    output_id: v.output_id,
                    output_generation: v.output_generation,
                    device_lease_id: v.device_lease_id,
                    device_generation: v.device_generation,
                    mode_id: v.mode_id,
                    mode_generation: v.mode_generation,
                    modifier: v.modifier,
                    connector_id: v.connector_id,
                    width: v.width,
                    height: v.height,
                    refresh_millihz: v.refresh_millihz,
                    drm_format: v.drm_format,
                    flags: v.flags,
                }
            }
        }
    };
}
output_conversion!(DiscoveredOutputV1, DiscoveredOutput);
output_conversion!(DiscoveredOutput, DiscoveredOutputV1);
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DiscoveryV1 {
    pub session_generation: u64,
    pub output_count: u32,
    pub reserved: u32,
    pub outputs: [DiscoveredOutputV1; 32],
}
impl TryFrom<DiscoveryV1> for Discovery {
    type Error = Error;
    fn try_from(v: DiscoveryV1) -> Result<Self, Error> {
        if v.reserved != 0 {
            return Err(Error::Protocol);
        }
        Ok(Self {
            session_generation: v.session_generation,
            output_count: v.output_count,
            outputs: v.outputs.map(DiscoveredOutput::from),
        })
    }
}
impl From<Discovery> for DiscoveryV1 {
    fn from(v: Discovery) -> Self {
        Self {
            session_generation: v.session_generation,
            output_count: v.output_count,
            reserved: 0,
            outputs: v.outputs.map(DiscoveredOutputV1::from),
        }
    }
}
#[repr(C)]
pub struct SnapshotV1 {
    pub stage: u32,
    pub worker_alive: u32,
    pub stopping: u32,
    pub ready: u32,
    pub failed: u32,
    pub occupied_credits: u32,
    pub reserved_completion_records: u32,
    pub queued_frames: u32,
    pub accepted: u64,
    pub retired: u64,
    pub joined: u32,
    pub reserved: u32,
}
impl From<executor::Snapshot> for SnapshotV1 {
    fn from(v: executor::Snapshot) -> Self {
        Self {
            stage: v.stage as u32,
            worker_alive: u32::from(v.worker_alive),
            stopping: u32::from(v.stopping),
            ready: u32::from(v.ready),
            failed: u32::from(v.failed),
            occupied_credits: v.occupied_credits,
            reserved_completion_records: v.reserved_completion_records,
            queued_frames: v.queued_frames,
            accepted: v.accepted,
            retired: v.retired,
            joined: u32::from(v.joined),
            reserved: 0,
        }
    }
}
#[repr(C)]
#[derive(Default)]
pub struct CompletionV1 {
    pub kind: u32,
    pub resources_retired: u32,
    pub error: i32,
    pub has_grant: u32,
    pub has_scope: u32,
    pub reserved: u32,
    pub admission: AdmissionV1,
    pub native: NativeResultV1,
    pub grant: GrantV1,
    pub scope: NativeScopeV1,
    pub facts: NativeProgressV1,
}
impl From<Completion> for CompletionV1 {
    fn from(v: Completion) -> Self {
        let mut out = Self {
            resources_retired: u32::from(v.resources_retired),
            error: v.error.map(code).unwrap_or(0),
            has_grant: u32::from(v.grant.is_some()),
            has_scope: u32::from(v.scope.is_some()),
            admission: v.admission.into(),
            native: v.native.into(),
            grant: v.grant.map(GrantV1::from).unwrap_or_default(),
            scope: v
                .scope
                .as_ref()
                .map(NativeScopeV1::from)
                .unwrap_or_default(),
            ..Self::default()
        };
        out.kind = match v.kind {
            CompletionKind::Attempt => 1,
            CompletionKind::SubmissionOutcome => 2,
            CompletionKind::Fence(f) => {
                out.facts.fence_identity = f.identity;
                3
            }
            CompletionKind::Presented(p) => {
                out.facts.has_presentation = 1;
                out.facts.presentation_correlation_id = p.correlation_id;
                out.facts.presentation_sequence = p.sequence;
                out.facts.presentation_timestamp_ns = p.timestamp_ns;
                out.facts.presentation_clock = match p.clock {
                    PresentationClock::Monotonic => 0,
                    PresentationClock::Realtime => 1,
                    PresentationClock::Boottime => 2,
                };
                4
            }
            CompletionKind::Retired => {
                out.facts.resources_retired = 1;
                5
            }
        };
        out
    }
}
fn code(e: Error) -> i32 {
    match e {
        Error::Invalid => 1,
        Error::Busy => 2,
        Error::Capacity => 3,
        Error::Stopping => 4,
        Error::Stale => 5,
        Error::Expired => 6,
        Error::Clock => 7,
        Error::Exhausted => 8,
        Error::Native => 9,
        Error::Protocol => 10,
        Error::WorkerPanic => 11,
        Error::Wake => 12,
    }
}
fn status(c: i32) -> Result<(), Error> {
    match c {
        0 => Ok(()),
        1 => Err(Error::Invalid),
        2 => Err(Error::Busy),
        3 => Err(Error::Capacity),
        4 => Err(Error::Stopping),
        5 => Err(Error::Stale),
        6 => Err(Error::Expired),
        7 => Err(Error::Clock),
        8 => Err(Error::Exhausted),
        9 => Err(Error::Native),
        10 => Err(Error::Protocol),
        11 => Err(Error::WorkerPanic),
        12 => Err(Error::Wake),
        _ => Err(Error::Protocol),
    }
}
fn returned(r: Result<(), Error>) -> i32 {
    r.map(|()| 0).unwrap_or_else(code)
}

/// Immutable control guard is checked BEFORE dereferencing the inner !Send
/// executor, any ownership slot, or output buffer. Calls must be nonreentrant.
pub struct PrExecutor {
    control: ThreadId,
    inner: UnsafeCell<Executor>,
}
pub struct PrAttempt {
    worker: ThreadId,
    inner: *mut Attempt,
}
unsafe fn with_executor<T>(
    raw: *mut PrExecutor,
    call: impl FnOnce(&mut Executor) -> Result<T, Error>,
) -> Result<T, Error> {
    if raw.is_null() {
        return Err(Error::Invalid);
    }
    let envelope = unsafe { &*raw };
    if envelope.control != thread::current().id() {
        return Err(Error::Invalid);
    }
    call(unsafe { &mut *envelope.inner.get() })
}
struct CFrameSlot {
    slot: *mut *mut Frame,
}
impl FrameCustody for CFrameSlot {
    fn frame(&self) -> Option<&Frame> {
        unsafe { (*self.slot).as_ref() }
    }
    fn take_frame(&mut self) -> Frame {
        // This is reached only after every admission refusal and reservation.
        let raw = unsafe { *self.slot };
        unsafe {
            self.slot.write(ptr::null_mut());
            *Box::from_raw(raw)
        }
    }
}
struct NativeAdapter {
    functions: NativeOwnerFunctionsV1,
    options: OwnerOptionsV1,
    owner: *mut c_void,
    bootstrap: Option<OwnedBootstrap>,
    shutdown_verified: bool,
}
fn native_factory(config: OwnerFactoryConfig) -> Result<Box<dyn NativeOwner>, Error> {
    Ok(Box::new(NativeAdapter {
        functions: config.native,
        options: config.options,
        owner: ptr::null_mut(),
        bootstrap: None,
        shutdown_verified: false,
    }))
}
impl NativeOwner for NativeAdapter {
    fn bootstrap(&mut self, input: &mut Option<OwnedBootstrap>) -> Result<Discovery, Error> {
        // Self has already been retained by run(). Retain duplicate descriptors
        // before the first fallible callback; the C holder is CPU-only on create.
        self.bootstrap = input.take();
        self.owner = unsafe { (self.functions.create.expect("validated create"))(&self.options) };
        if self.owner.is_null() {
            return Err(Error::Native);
        }
        let pack = self.bootstrap.as_ref().ok_or(Error::Protocol)?;
        let mut events = [BootstrapEventV1::default(); MAX_BOOTSTRAP_EVENTS];
        for (out, event) in events.iter_mut().zip(pack.events()) {
            *out = BootstrapEventV1 {
                metadata: *event.metadata(),
                primary_fd: event.primary().map(|fd| fd.as_raw_fd()).unwrap_or(-1),
                render_fd: event.render().map(|fd| fd.as_raw_fd()).unwrap_or(-1),
            };
        }
        let view = BootstrapViewV1 {
            session_generation: pack.session_generation(),
            event_count: pack.events().len() as u32,
            reserved: 0,
            events: events.as_ptr(),
        };
        let mut value = DiscoveryV1::default();
        status(unsafe {
            (self.functions.bootstrap.expect("validated bootstrap"))(self.owner, &view, &mut value)
        })?;
        value.try_into()
    }
    fn prepare(&mut self, frame: &Frame, admission: Admission) -> Result<(), Error> {
        status(unsafe {
            (self.functions.prepare.expect("validated prepare"))(
                self.owner,
                frame,
                &admission.into(),
            )
        })
    }
    fn execute(
        &mut self,
        frame: &Frame,
        admission: Admission,
        attempt: &mut Attempt,
    ) -> Result<NativeResult, Error> {
        let mut token = PrAttempt {
            worker: thread::current().id(),
            inner: attempt,
        };
        let mut value = NativeResultV1::default();
        status(unsafe {
            (self.functions.execute.expect("validated execute"))(
                self.owner,
                frame,
                &admission.into(),
                &mut token,
                &mut value,
            )
        })?;
        value.try_into()
    }
    fn advance(&mut self, context: &RetirementContext<'_>) -> Result<NativeProgress, Error> {
        let mut value = NativeProgressV1::default();
        status(unsafe {
            (self.functions.advance.expect("validated advance"))(
                self.owner,
                &context.into(),
                &mut value,
            )
        })?;
        value.try_into()
    }
    fn poll_sources(&mut self, storage: &mut [PollSource; 16]) -> Result<usize, Error> {
        let mut values = [PollSourceV1::default(); 16];
        let mut count = 0;
        status(unsafe {
            (self.functions.poll_sources.expect("validated sources"))(
                self.owner,
                values.as_mut_ptr(),
                16,
                &mut count,
            )
        })?;
        if count > 16 {
            return Err(Error::Protocol);
        }
        for (out, value) in storage.iter_mut().zip(&values[..count as usize]) {
            if value.reserved != 0 || value.reserved2 != [0; 3] {
                return Err(Error::Protocol);
            }
            *out = PollSource {
                fd: value.fd,
                token: value.token,
                generation: value.generation,
                events: value.events,
            };
        }
        Ok(count as usize)
    }
    fn dispatch(&mut self, ready: NativeReady) -> Result<(), Error> {
        let value = NativeReadyV1 {
            source: ready.source.into(),
            revents: ready.revents,
            reserved: [0; 3],
        };
        status(unsafe {
            (self.functions.dispatch.expect("validated dispatch"))(self.owner, &value)
        })
    }
    fn shutdown_progress(&mut self) -> Result<RetirementProgress, Error> {
        let mut retired = 1;
        if !self.owner.is_null() {
            retired = 0;
            status(unsafe {
                (self
                    .functions
                    .shutdown_progress
                    .expect("validated shutdown"))(self.owner, &mut retired)
            })?;
        }
        match retired {
            0 if !self.shutdown_verified => Ok(RetirementProgress::Pending),
            1 => {
                self.shutdown_verified = true;
                Ok(RetirementProgress::Verified(RetirementProof))
            }
            _ => Err(Error::Protocol),
        }
    }
}
impl Drop for NativeAdapter {
    fn drop(&mut self) {
        if !self.shutdown_verified {
            std::process::abort();
        }
        if !self.owner.is_null() {
            unsafe { (self.functions.destroy.expect("validated destroy"))(self.owner) };
        }
        // The descriptor pack drops only after the worker-local holder's final
        // destructor, so its synchronous borrows never outlive their custody.
    }
}

/// # Safety
/// Inputs obey the live/aligned/nonaliasing synchronous-borrow contract in the
/// current header. Descriptors remain open for capture; result is an empty slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_bootstrap_capture_v1(
    generation: u64,
    events: *const BootstrapEventV1,
    count: u32,
    result: *mut *mut OwnedBootstrap,
) -> i32 {
    returned((|| {
        if result.is_null()
            || unsafe { !(*result).is_null() }
            || events.is_null()
            || !(2..=34).contains(&count)
        {
            return Err(Error::Invalid);
        }
        let values = unsafe { std::slice::from_raw_parts(events, count as usize) };
        let mut borrowed = Vec::with_capacity(count as usize);
        for value in values {
            if value.primary_fd < -1 || value.render_fd < -1 {
                return Err(Error::Invalid);
            }
            borrowed.push(BorrowedBootstrapEvent {
                metadata: &value.metadata,
                primary: if value.primary_fd >= 0 {
                    Some(unsafe { BorrowedFd::borrow_raw(value.primary_fd) })
                } else {
                    None
                },
                render: if value.render_fd >= 0 {
                    Some(unsafe { BorrowedFd::borrow_raw(value.render_fd) })
                } else {
                    None
                },
            });
        }
        let pack = OwnedBootstrap::capture(generation, &borrowed).map_err(|e| match e {
            crate::bootstrap::BootstrapError::Capacity => Error::Capacity,
            crate::bootstrap::BootstrapError::Invalid => Error::Invalid,
            _ => Error::Native,
        })?;
        unsafe { result.write(Box::into_raw(Box::new(pack))) };
        Ok(())
    })())
}
/// # Safety
/// Null or a live uniquely owned capture handle, released once; no native custody.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_bootstrap_release(input: *mut OwnedBootstrap) {
    if !input.is_null() {
        unsafe { drop(Box::from_raw(input)) };
    }
}
/// # Safety
/// Current header contracts apply, including callback/code lifetimes. Input is
/// uniquely owned; result is empty and nonaliasing. After shape validation the
/// bootstrap slot is consumed even when the original cutoff/spawn refuses.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_create_v1(
    functions: *const NativeOwnerFunctionsV1,
    options: *const OwnerOptionsV1,
    security_generation: u64,
    input: *mut *mut OwnedBootstrap,
    cutoff: u64,
    result: *mut *mut PrExecutor,
) -> i32 {
    returned((|| {
        if functions.is_null()
            || options.is_null()
            || input.is_null()
            || result.is_null()
            || unsafe { !(*result).is_null() || (*input).is_null() }
        {
            return Err(Error::Invalid);
        }
        let config = unsafe { OwnerFactoryConfig::new(*functions, *options) }?;
        let pack = unsafe { *Box::from_raw(*input) };
        unsafe { input.write(ptr::null_mut()) };
        let executor = Executor::new(security_generation, native_factory, config, pack, cutoff)?;
        let envelope = Box::new(PrExecutor {
            control: thread::current().id(),
            inner: UnsafeCell::new(executor),
        });
        unsafe { result.write(Box::into_raw(envelope)) };
        Ok(())
    })())
}
/// # Safety
/// Live serialized control-thread handle; writable nonaliasing outputs. Borrowed
/// watches must both be removed before release, never read as native authority.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_watch_fds(
    owner: *mut PrExecutor,
    completion: *mut i32,
    startup: *mut i32,
) -> i32 {
    returned(unsafe {
        with_executor(owner, |e| {
            if completion.is_null() || startup.is_null() || completion == startup {
                return Err(Error::Invalid);
            }
            let a = e.completion_fd()?;
            let b = e.startup_deadline_fd()?;
            completion.write(a);
            startup.write(b);
            Ok(())
        })
    })
}
/// # Safety
/// Live serialized control-thread handle; writable nonaliasing result. Wrong
/// thread refuses before accessing any inner executor or result storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_snapshot_v1(
    owner: *mut PrExecutor,
    result: *mut SnapshotV1,
) -> i32 {
    returned(unsafe {
        with_executor(owner, |e| {
            if result.is_null() {
                return Err(Error::Invalid);
            }
            result.write(e.snapshot()?.into());
            Ok(())
        })
    })
}
/// # Safety
/// Live serialized control-thread handle and writable nonaliasing result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_discovery_v1(
    owner: *mut PrExecutor,
    result: *mut DiscoveryV1,
) -> i32 {
    match unsafe {
        with_executor(owner, |e| {
            if result.is_null() {
                return Err(Error::Invalid);
            }
            match e.try_discovery()? {
                Some(value) => {
                    result.write(value.into());
                    Ok(true)
                }
                None => Ok(false),
            }
        })
    } {
        Ok(true) => 0,
        Ok(false) => EMPTY,
        Err(e) => code(e),
    }
}
/// # Safety
/// Live serialized control-thread handle. Frame is a live uniquely owned handle
/// slot; inputs and output are valid/nonaliasing. No concurrent Frame borrows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_try_submit_v1(
    owner: *mut PrExecutor,
    frame: *mut *mut Frame,
    policy: *const PolicyV1,
    cutoff: u64,
    cookie: u64,
    result: *mut AdmissionV1,
) -> i32 {
    returned(unsafe {
        with_executor(owner, |e| {
            if frame.is_null() || policy.is_null() || result.is_null() {
                return Err(Error::Invalid);
            }
            let admission = e.try_submit_custody(
                &mut CFrameSlot { slot: frame },
                (*policy).into(),
                cutoff,
                cookie,
            )?;
            result.write(admission.into());
            Ok(())
        })
    })
}
/// # Safety
/// Live serialized control-thread handle; no concurrent or reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_invalidate(owner: *mut PrExecutor, generation: u64) -> i32 {
    returned(unsafe { with_executor(owner, |e| e.invalidate(generation)) })
}
/// # Safety
/// Live serialized control-thread handle; no concurrent or reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_stop(owner: *mut PrExecutor) -> i32 {
    returned(unsafe { with_executor(owner, |e| e.request_stop()) })
}
/// # Safety
/// Live serialized control-thread handle, writable nonaliasing result. No borrowed
/// native pointers are returned; these records are historical observations only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_next_completion_v1(
    owner: *mut PrExecutor,
    result: *mut CompletionV1,
) -> i32 {
    match unsafe {
        with_executor(owner, |e| {
            if result.is_null() {
                return Err(Error::Invalid);
            }
            match e.try_next_completion()? {
                Some(value) => {
                    result.write(value.into());
                    Ok(true)
                }
                None => Ok(false),
            }
        })
    } {
        Ok(true) => 0,
        Ok(false) => EMPTY,
        Err(e) => code(e),
    }
}
/// # Safety
/// Live serialized control-thread handle. Busy never detaches or drops it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_try_join(owner: *mut PrExecutor) -> i32 {
    returned(unsafe { with_executor(owner, Executor::try_join) })
}
/// # Safety
/// Writable unique handle slot, serialized control thread; both borrowed watches
/// already unregistered. Busy leaves the exact slot intact until actual join.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_executor_try_release(slot: *mut *mut PrExecutor) -> i32 {
    returned((|| {
        if slot.is_null() {
            return Err(Error::Invalid);
        }
        let raw = unsafe { *slot };
        if raw.is_null() {
            return Ok(());
        }
        unsafe {
            with_executor(raw, |e| {
                if e.snapshot()?.joined {
                    Ok(())
                } else {
                    Err(Error::Busy)
                }
            })
        }?;
        unsafe {
            slot.write(ptr::null_mut());
            drop(Box::from_raw(raw))
        };
        Ok(())
    })())
}
/// # Safety
/// Live synchronous execute token, on its worker only. Scope/result are live,
/// aligned, nonaliasing borrows. Token cannot be retained, used after callback,
/// reentered or converted into reusable native authority.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_attempt_authorize_v1(
    token: *mut PrAttempt,
    scope: *const NativeScopeV1,
    result: *mut GrantV1,
) -> i32 {
    returned((|| {
        if token.is_null() {
            return Err(Error::Invalid);
        }
        let envelope = unsafe { &*token };
        if envelope.worker != thread::current().id() {
            return Err(Error::Invalid);
        }
        let attempt = unsafe { &mut *envelope.inner };
        if scope.is_null() || result.is_null() {
            return attempt.reject_invalid();
        }
        let value = unsafe { &*scope };
        let count = value.object_count as usize;
        if value.reserved != 0
            || count == 0
            || count > 128
            || value.objects[..count.min(128)]
                .iter()
                .any(|v| v.reserved != 0)
            || value.objects[count.min(128)..]
                .iter()
                .any(|v| *v != NativeObjectV1::default())
        {
            return attempt.reject_invalid();
        }
        let mut objects = [NativeObject {
            id: 0,
            generation: 0,
            output_id: 0,
            output_generation: 0,
        }; 128];
        for (out, obj) in objects.iter_mut().zip(&value.objects[..count]) {
            *out = NativeObject {
                id: obj.id,
                generation: obj.generation,
                output_id: obj.output_id,
                output_generation: obj.output_generation,
            };
        }
        let actual = NativeScope {
            policy: value.policy.into(),
            device_lease_id: value.device_lease_id,
            device_generation: value.device_generation,
            objects: &objects[..count],
        };
        let grant = attempt.authorize(&actual)?;
        unsafe { result.write(grant.into()) };
        Ok(())
    })())
}

#[cfg(test)]
pub(crate) fn test_config() -> OwnerFactoryConfig {
    unsafe extern "C" fn create(_: *const OwnerOptionsV1) -> *mut c_void {
        ptr::null_mut()
    }
    unsafe extern "C" fn destroy(_: *mut c_void) {}
    unsafe extern "C" fn bootstrap(
        _: *mut c_void,
        _: *const BootstrapViewV1,
        _: *mut DiscoveryV1,
    ) -> i32 {
        9
    }
    unsafe extern "C" fn prepare(_: *mut c_void, _: *const Frame, _: *const AdmissionV1) -> i32 {
        9
    }
    unsafe extern "C" fn execute(
        _: *mut c_void,
        _: *const Frame,
        _: *const AdmissionV1,
        _: *mut PrAttempt,
        _: *mut NativeResultV1,
    ) -> i32 {
        9
    }
    unsafe extern "C" fn advance(
        _: *mut c_void,
        _: *const RetirementContextV1,
        _: *mut NativeProgressV1,
    ) -> i32 {
        9
    }
    unsafe extern "C" fn sources(_: *mut c_void, _: *mut PollSourceV1, _: u32, _: *mut u32) -> i32 {
        9
    }
    unsafe extern "C" fn dispatch(_: *mut c_void, _: *const NativeReadyV1) -> i32 {
        9
    }
    unsafe extern "C" fn shutdown(_: *mut c_void, _: *mut u32) -> i32 {
        9
    }
    // Existing independent Rust test factories ignore this table. If accidentally
    // invoked, create fails without acquiring any native resource.
    unsafe {
        OwnerFactoryConfig::new(
            NativeOwnerFunctionsV1 {
                abi_version: 1,
                struct_size: size_of::<NativeOwnerFunctionsV1>() as u32,
                create: Some(create),
                destroy: Some(destroy),
                bootstrap: Some(bootstrap),
                prepare: Some(prepare),
                execute: Some(execute),
                advance: Some(advance),
                poll_sources: Some(sources),
                dispatch: Some(dispatch),
                shutdown_progress: Some(shutdown),
                ..Default::default()
            },
            OwnerOptionsV1 {
                abi_version: 1,
                struct_size: size_of::<OwnerOptionsV1>() as u32,
                owner_id: 1,
                ..Default::default()
            },
        )
    }
    .unwrap()
}

// Seal the independently authored Linux64 C contract. Explicit reserved fields
// avoid exporting Rust enum/Option layouts or uninitialized struct padding.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<SessionRenderEvent>() == 408);
    assert!(size_of::<BootstrapEventV1>() == 416);
    assert!(size_of::<BootstrapViewV1>() == 24);
    assert!(size_of::<PolicyV1>() == 40);
    assert!(size_of::<AdmissionV1>() == 56);
    assert!(size_of::<NativeObjectV1>() == 32);
    assert!(size_of::<NativeScopeV1>() == 4160);
    assert!(size_of::<GrantV1>() == 56);
    assert!(size_of::<NativeResultV1>() == 32);
    assert!(size_of::<NativeProgressV1>() == 48);
    assert!(size_of::<RetirementContextV1>() == 4312);
    assert!(size_of::<PollSourceV1>() == 32);
    assert!(size_of::<NativeReadyV1>() == 40);
    assert!(size_of::<DiscoveredOutputV1>() == 80);
    assert!(size_of::<DiscoveryV1>() == 2576);
    assert!(size_of::<OwnerOptionsV1>() == 48);
    assert!(size_of::<NativeOwnerFunctionsV1>() == 112);
    assert!(size_of::<SnapshotV1>() == 56);
    assert!(size_of::<CompletionV1>() == 4376);
    assert!(std::mem::offset_of!(CompletionV1, admission) == 24);
    assert!(std::mem::offset_of!(CompletionV1, native) == 80);
    assert!(std::mem::offset_of!(CompletionV1, grant) == 112);
    assert!(std::mem::offset_of!(CompletionV1, scope) == 168);
    assert!(std::mem::offset_of!(CompletionV1, facts) == 4328);
};
