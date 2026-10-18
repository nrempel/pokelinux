//! Test-only C ABI pointer/scalar adapter. No protocol, policy or retry logic.
use pokelinux_settings_client::{
    connectivity::{Status, View as RustView},
    connectivity_ffi::{self as ffi, Handle, View as ScalarView},
    ffi::{
        ABORT, CONTROLS_UNAVAILABLE, DENIED, INVALID, NOT_SENT, OK, PENDING, READY, REFUSED,
        STOPPED, UNAVAILABLE, UNCERTAIN, WAIT,
    },
};
use std::{
    cell::Cell,
    io,
    mem::{align_of, offset_of, size_of, MaybeUninit},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    ptr,
};

fn layout_and_deadline() {
    assert_eq!(size_of::<ScalarView>(), 40);
    assert_eq!(align_of::<ScalarView>(), 8);
    assert_eq!(offset_of!(ScalarView, observed_at_ns), 0);
    assert_eq!(offset_of!(ScalarView, until_ns), 8);
    assert_eq!(offset_of!(ScalarView, transport_connected), 16);
    assert_eq!(offset_of!(ScalarView, available), 20);
    assert_eq!(offset_of!(ScalarView, pending), 24);
    assert_eq!(offset_of!(ScalarView, can_control), 28);
    assert_eq!(offset_of!(ScalarView, status), 32);
    assert_eq!(offset_of!(ScalarView, configured), 36);
    let before = super::now();
    let deadline = ffi::pr_settings_connectivity_deadline_v1();
    let after = super::now();
    assert_ne!(
        deadline, 0,
        "actual C ABI BOOTTIME capture is required setup"
    );
    assert!(before.checked_add(1_000_000_000).unwrap() <= deadline);
    assert!(deadline <= after.checked_add(1_000_000_000).unwrap());
}
fn boolean(value: u32) -> bool {
    assert!(value <= 1, "non-boolean C ABI scalar {value}");
    value == 1
}
fn translate(v: ScalarView) -> RustView {
    let status = match v.status {
        UNAVAILABLE => Status::Unavailable,
        READY => Status::Ready,
        CONTROLS_UNAVAILABLE => Status::ControlsUnavailable,
        DENIED => Status::Denied,
        PENDING => Status::Pending,
        NOT_SENT => Status::NotSent,
        UNCERTAIN => Status::Uncertain,
        STOPPED => Status::Stopped,
        other => panic!("unmapped connectivity C ABI status {other}"),
    };
    let configured = match v.configured {
        -1 => None,
        0 => Some(false),
        1 => Some(true),
        other => panic!("non-tristate C ABI configured {other}"),
    };
    RustView {
        transport_connected: boolean(v.transport_connected),
        available: boolean(v.available),
        pending: boolean(v.pending),
        can_control: boolean(v.can_control),
        configured,
        observed_at_ns: v.observed_at_ns,
        until_ns: v.until_ns,
        status,
    }
}
pub struct TestEngine {
    handle: Cell<*mut Handle>,
    joined: Cell<bool>,
    first_stop_until: Cell<u64>,
}
impl TestEngine {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        layout_and_deadline();
        let bytes = path.as_os_str().as_bytes();
        let mut handle = ptr::null_mut();
        // SAFETY: complete path bytes, aligned exclusive null output slot.
        let result = unsafe {
            ffi::pr_settings_connectivity_new_v1(bytes.as_ptr(), bytes.len(), &mut handle)
        };
        if result != OK {
            assert!(handle.is_null(), "failed C ABI new transferred ownership");
            return Err(io::Error::new(
                if result == INVALID {
                    io::ErrorKind::InvalidInput
                } else {
                    io::ErrorKind::PermissionDenied
                },
                format!("actual C ABI new returned {result}"),
            ));
        }
        assert!(!handle.is_null());
        // SAFETY: returned unique live handle. Free must refuse before join.
        assert_eq!(
            unsafe { ffi::pr_settings_connectivity_free_v1(handle) },
            REFUSED
        );
        Ok(Self {
            handle: Cell::new(handle),
            joined: Cell::new(false),
            first_stop_until: Cell::new(0),
        })
    }
    pub fn view(&self) -> Option<RustView> {
        if self.handle.get().is_null() {
            return None;
        }
        let mut out = MaybeUninit::<ScalarView>::uninit();
        // SAFETY: live handle, disjoint complete aligned exclusive output.
        let result =
            unsafe { ffi::pr_settings_connectivity_view_v1(self.handle.get(), out.as_mut_ptr()) };
        match result {
            // SAFETY: C ABI OK guarantees complete initialization.
            OK => Some(translate(unsafe { out.assume_init() })),
            REFUSED => None,
            other => panic!("actual C ABI view returned {other}"),
        }
    }
    pub fn set(&self, connected: bool, original_until_ns: u64) -> bool {
        if self.handle.get().is_null() {
            return false;
        }
        // SAFETY: live handle, no concurrent free, canonical boolean. The exact
        // caller-authored absolute cutoff is forwarded without renewal.
        match unsafe {
            ffi::pr_settings_connectivity_set_v1(
                self.handle.get(),
                u32::from(connected),
                original_until_ns,
            )
        } {
            OK => true,
            REFUSED => false,
            other => panic!("actual C ABI set returned {other}"),
        }
    }
    pub fn stop(&self) -> bool {
        if self.handle.get().is_null() {
            return self.joined.get();
        }
        if self.first_stop_until.get() == 0 {
            self.first_stop_until
                .set(super::now().checked_add(200_000_000).unwrap());
        }
        // SAFETY: unique live handle; Stop is the actual local C ABI operation.
        match unsafe { ffi::pr_settings_connectivity_stop_v1(self.handle.get()) } {
            OK => true,
            ABORT => false,
            other => panic!("actual C ABI Stop returned {other}"),
        }
    }
    pub fn join(&self) -> i32 {
        if self.handle.get().is_null() {
            return if self.joined.get() { OK } else { ABORT };
        }
        // SAFETY: live uniquely retained handle, no overlapping free/calls.
        let result = unsafe { ffi::pr_settings_connectivity_join_v1(self.handle.get()) };
        if result == OK {
            // SAFETY: actual successful C ABI join precedes this unique free.
            assert_eq!(
                unsafe { ffi::pr_settings_connectivity_free_v1(self.handle.get()) },
                OK
            );
            self.handle.set(ptr::null_mut());
            self.joined.set(true);
        }
        result
    }
    pub fn joined(&self) -> bool {
        self.joined.get()
    }
}
impl Drop for TestEngine {
    fn drop(&mut self) {
        if self.handle.get().is_null() {
            return;
        }
        // Api::close normally performs this exact Stop/join/free path and the
        // original 200ms/task-count checks. An unwind cannot detach a worker or
        // renew the first Stop cutoff. Unresolved ownership aborts this child.
        if !self.stop() {
            std::process::abort();
        }
        let end = self.first_stop_until.get();
        loop {
            match self.join() {
                OK => return,
                WAIT if super::now() < end => super::pause(),
                _ => std::process::abort(),
            }
        }
    }
}
