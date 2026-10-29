//! C ABI for one GUI-thread-owned client. No policy or revision enters through Qt.
use crate::Client;
use std::{os::unix::ffi::OsStrExt, path::Path};
pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const REFUSED: i32 = 2;
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct View {
    pub allowed: i32,
    pub pending: u32,
    pub can_set: u32,
    pub unconfirmed: u32,
}
pub struct Handle(Client);
fn valid<T>(p: *const T) -> bool {
    !p.is_null() && p.is_aligned()
}
/// # Safety
/// Path and out are live disjoint storage; out is an exclusive null pointer slot.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_new_v1(
    path: *const u8,
    len: usize,
    out: *mut *mut Handle,
) -> i32 {
    if !valid(path) || !valid(out) || len == 0 || len > 107 {
        return INVALID;
    }
    // SAFETY: checked form; caller supplies live storage of the stated size.
    let bytes = unsafe { std::slice::from_raw_parts(path, len) };
    if bytes.contains(&0) || unsafe { !(*out).is_null() } {
        return INVALID;
    }
    let Ok(client) = Client::new(Path::new(std::ffi::OsStr::from_bytes(bytes))) else {
        return REFUSED;
    };
    // SAFETY: exclusive checked output receives unique ownership.
    unsafe { out.write(Box::into_raw(Box::new(Handle(client)))) };
    OK
}
/// # Safety
/// Handle is live and exclusively used on its owning GUI thread.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_active_v1(h: *mut Handle, active: u32) -> i32 {
    if !valid(h) || active > 1 {
        return INVALID;
    }
    // SAFETY: caller retains exclusive live handle.
    unsafe { (*h).0.active(active == 1) };
    OK
}
/// # Safety
/// Handle is live and exclusively used on its owning GUI thread.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_tick_v1(h: *mut Handle) -> i32 {
    if !valid(h) {
        return INVALID;
    }
    // SAFETY: caller retains exclusive live handle.
    unsafe { (*h).0.tick() };
    OK
}
/// # Safety
/// Handle and output are disjoint live storage, exclusively owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_view_v1(h: *mut Handle, out: *mut View) -> i32 {
    if !valid(h) || !valid(out) {
        return INVALID;
    }
    // SAFETY: caller retains exclusive live handle and output.
    let v = unsafe { (*h).0.view() };
    unsafe {
        out.write(View {
            allowed: v.allowed.map_or(-1, i32::from),
            pending: u32::from(v.pending),
            can_set: u32::from(v.can_set),
            unconfirmed: u32::from(v.unconfirmed),
        })
    };
    OK
}
/// # Safety
/// Handle is live and exclusively used on its owning GUI thread.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_choose_v1(h: *mut Handle, allowed: u32) -> i32 {
    if !valid(h) || allowed > 1 {
        return INVALID;
    }
    // SAFETY: caller retains exclusive live handle.
    if unsafe { (*h).0.choose(allowed == 1) } {
        OK
    } else {
        REFUSED
    }
}
/// # Safety
/// Handle is uniquely owned and live, with no concurrent or subsequent use.
#[no_mangle]
pub unsafe extern "C" fn pr_permissions_free_v1(h: *mut Handle) -> i32 {
    if !valid(h) {
        return INVALID;
    }
    // SAFETY: ownership returns exactly once; dropping closes the sole socket.
    unsafe { drop(Box::from_raw(h)) };
    OK
}
