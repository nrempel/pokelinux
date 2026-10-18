//! Pointer/scalar translation for the connectivity engine; no socket or policy edge.
use crate::{
    connectivity::{Engine, Status},
    ffi,
};
use std::{os::unix::ffi::OsStrExt, path::Path};
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub observed_at_ns: u64,
    pub until_ns: u64,
    pub transport_connected: u32,
    pub available: u32,
    pub pending: u32,
    pub can_control: u32,
    pub status: u32,
    pub configured: i32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            observed_at_ns: 0,
            until_ns: 0,
            transport_connected: 0,
            available: 0,
            pending: 0,
            can_control: 0,
            status: ffi::UNAVAILABLE,
            configured: -1,
        }
    }
}
impl From<crate::connectivity::View> for View {
    fn from(v: crate::connectivity::View) -> Self {
        Self {
            observed_at_ns: v.observed_at_ns,
            until_ns: v.until_ns,
            transport_connected: u32::from(v.transport_connected),
            available: u32::from(v.available),
            pending: u32::from(v.pending),
            can_control: u32::from(v.can_control),
            configured: v.configured.map_or(-1, i32::from),
            status: match v.status {
                Status::Unavailable => ffi::UNAVAILABLE,
                Status::Ready => ffi::READY,
                Status::ControlsUnavailable => ffi::CONTROLS_UNAVAILABLE,
                Status::Denied => ffi::DENIED,
                Status::Pending => ffi::PENDING,
                Status::NotSent => ffi::NOT_SENT,
                Status::Uncertain => ffi::UNCERTAIN,
                Status::Stopped => ffi::STOPPED,
            },
        }
    }
}
#[repr(C)]
pub struct Handle {
    engine: Engine,
}
fn valid<T>(p: *const T) -> bool {
    !p.is_null() && p.is_aligned()
}
/// Capture once at the originating gesture, before any control transport.
/// Zero means BOOTTIME/deadline capture failed; no action may be submitted.
#[no_mangle]
pub extern "C" fn pr_settings_connectivity_deadline_v1() -> u64 {
    crate::clock::now()
        .and_then(|n| n.checked_add(crate::connectivity::ACTION_BUDGET))
        .unwrap_or(0)
}
/// # Safety
/// Path is complete readable bytes; out is exclusive aligned null pointer storage.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_new_v1(
    path: *const u8,
    length: usize,
    out: *mut *mut Handle,
) -> i32 {
    if !valid(path) || !valid(out) || length == 0 || length > 107 {
        return ffi::INVALID;
    }
    // SAFETY: caller supplies complete live storage after pointer-form checks.
    let bytes = unsafe { std::slice::from_raw_parts(path, length) };
    // SAFETY: the checked output slot is complete aligned readable storage.
    if unsafe { !(*out).is_null() } {
        return ffi::INVALID;
    }
    let path = Path::new(std::ffi::OsStr::from_bytes(bytes));
    if bytes.contains(&0)
        || !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return ffi::INVALID;
    }
    let Ok(engine) = Engine::new(path.to_path_buf()) else {
        return ffi::REFUSED;
    };
    // SAFETY: ownership transfers only through the caller's exclusive checked slot.
    unsafe {
        out.write(Box::into_raw(Box::new(Handle { engine })));
    }
    ffi::OK
}
/// # Safety
/// Handle is live; out is complete exclusive aligned View storage disjoint from it.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_view_v1(
    handle: *mut Handle,
    out: *mut View,
) -> i32 {
    if !valid(handle) || !valid(out) {
        return ffi::INVALID;
    }
    // SAFETY: caller guarantees a live handle and no overlapping free.
    let Some(view) = (unsafe { &*handle }).engine.view() else {
        return ffi::REFUSED;
    };
    // SAFETY: complete exclusive checked output; non-OK never writes it.
    unsafe {
        out.write(view.into());
    }
    ffi::OK
}
/// # Safety
/// Handle is live without an overlapping free. Connected must be zero or one.
/// Until is the original absolute BOOTTIME cutoff, never a renewed retry budget.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_set_v1(
    handle: *mut Handle,
    connected: u32,
    original_until_ns: u64,
) -> i32 {
    if !valid(handle) || connected > 1 {
        return ffi::INVALID;
    }
    // SAFETY: caller guarantees a live handle and no overlapping free.
    if (unsafe { &*handle })
        .engine
        .set(connected == 1, original_until_ns)
    {
        ffi::OK
    } else {
        ffi::REFUSED
    }
}
/// # Safety
/// Handle is live without an overlapping free. Stop is local withdrawal only.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_stop_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return ffi::INVALID;
    }
    // SAFETY: caller guarantees a live handle.
    if (unsafe { &*handle }).engine.stop() {
        ffi::OK
    } else {
        ffi::ABORT
    }
}
/// # Safety
/// Handle is live without an overlapping free. OK requires actual worker join.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_join_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return ffi::INVALID;
    }
    // SAFETY: caller guarantees a live handle.
    (unsafe { &*handle }).engine.join()
}
/// # Safety
/// Handle came from new, is uniquely owned, and has no overlapping calls.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_connectivity_free_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return ffi::INVALID;
    }
    // SAFETY: caller guarantees live unique Box ownership after form checks.
    unsafe {
        if !(*handle).engine.joined() {
            return ffi::REFUSED;
        }
        drop(Box::from_raw(handle));
    }
    ffi::OK
}
