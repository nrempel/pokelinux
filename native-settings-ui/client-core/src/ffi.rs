//! C pointer translation. Rust owns all model, transport and worker lifetime decisions.
use std::path::Path;
pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const WAIT: i32 = 2;
pub const ABORT: i32 = 3;
pub const REFUSED: i32 = 4;
pub const UNAVAILABLE: u32 = 0;
pub const READY: u32 = 1;
pub const DENIED: u32 = 2;
pub const PENDING: u32 = 3;
pub const UNCERTAIN: u32 = 4;
pub const STOPPED: u32 = 5;
pub const CONTROLS_UNAVAILABLE: u32 = 6;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub revision: u64,
    pub until_ns: u64,
    pub connected: u32,
    pub available: u32,
    pub pending: u32,
    pub can_set: u32,
    pub status: u32,
    pub requested_percent: i32,
    pub observed_percent: i32,
    pub actual_percent: i32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            revision: 0,
            until_ns: 0,
            connected: 0,
            available: 0,
            pending: 0,
            can_set: 0,
            status: UNAVAILABLE,
            requested_percent: -1,
            observed_percent: -1,
            actual_percent: -1,
        }
    }
}
#[repr(C)]
pub struct Handle {
    engine: crate::engine::Engine,
}
fn valid<T>(p: *const T) -> bool {
    !p.is_null() && p.is_aligned()
}
/// # Safety
/// Path is readable bytes. Out is complete aligned exclusive null pointer storage.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_new_v1(
    path: *const u8,
    length: usize,
    out: *mut *mut Handle,
) -> i32 {
    if !valid(path) || !valid(out) || length == 0 || length > 107 {
        return INVALID;
    }
    // SAFETY: the caller provides live complete storage after form checks.
    let bytes = unsafe { std::slice::from_raw_parts(path, length) };
    use std::os::unix::ffi::OsStrExt;
    let path = Path::new(std::ffi::OsStr::from_bytes(bytes));
    if unsafe { !(*out).is_null() }
        || bytes.contains(&0)
        || !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return INVALID;
    }
    let Ok(engine) = crate::engine::Engine::new(path.to_path_buf()) else {
        return REFUSED;
    };
    // SAFETY: ownership transfers to the exclusive checked output slot.
    unsafe {
        out.write(Box::into_raw(Box::new(Handle { engine })));
    }
    OK
}
/// # Safety
/// Handle is live; output is exclusive aligned View storage, disjoint from handle.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_view_v1(handle: *mut Handle, out: *mut View) -> i32 {
    if !valid(handle) || !valid(out) {
        return INVALID;
    }
    // SAFETY: caller guarantees a live handle after pointer form checks.
    let Some(view) = (unsafe { &*handle }).engine.view() else {
        return REFUSED;
    };
    // SAFETY: the checked output is complete live exclusive storage.
    unsafe {
        out.write(view);
    }
    OK
}
/// # Safety
/// Handle is live without overlapping free.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_set_v1(handle: *mut Handle, percent: u32) -> i32 {
    if !valid(handle) {
        return INVALID;
    }
    // SAFETY: caller guarantees live handle lifetime.
    if (unsafe { &*handle }).engine.set(percent) {
        OK
    } else {
        REFUSED
    }
}
/// # Safety
/// Handle is live without overlapping free.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_stop_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return INVALID;
    }
    // SAFETY: caller guarantees live handle lifetime.
    if (unsafe { &*handle }).engine.stop() {
        OK
    } else {
        ABORT
    }
}
/// # Safety
/// Handle is live without overlapping free. OK requires actual worker join.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_join_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return INVALID;
    }
    // SAFETY: caller guarantees live handle lifetime.
    (unsafe { &*handle }).engine.join()
}
/// # Safety
/// Handle is returned by new and uniquely owned, without overlapping calls.
#[no_mangle]
pub unsafe extern "C" fn pr_settings_client_free_v1(handle: *mut Handle) -> i32 {
    if !valid(handle) {
        return INVALID;
    }
    // SAFETY: the caller guarantees live unique Box ownership.
    unsafe {
        if !(*handle).engine.joined() {
            return REFUSED;
        }
        drop(Box::from_raw(handle));
    }
    OK
}
