//! Pointer translation only. Caller owns the one Qt event-loop lifetime and
//! native resources; all policy and copied state remain in safe Rust.
use crate::{engine::Engine, layout::*};
use std::sync::Mutex;

#[repr(C)]
pub struct PrAppearanceClient {
    state: Mutex<Engine>,
}
fn valid<T>(pointer: *const T) -> bool {
    !pointer.is_null() && pointer.is_aligned()
}
/// # Safety
/// A nonnull aligned client is a live handle, with no call/free overlap.
unsafe fn with(
    client: *const PrAppearanceClient,
    operation: impl FnOnce(&mut Engine) -> i32,
) -> i32 {
    if !valid(client) {
        return INVALID;
    }
    // SAFETY: caller guarantees the live handle lifetime after basic form checks.
    let handle = unsafe { &*client };
    match handle.state.lock() {
        Ok(mut state) => operation(&mut state),
        Err(_) => REFUSED,
    }
}
/// # Safety
/// out is a writable aligned complete slot containing null, with exclusive access.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_new_v1(out: *mut *mut PrAppearanceClient) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies a complete exclusive writable pointer slot.
    if unsafe { !(*out).is_null() } {
        return INVALID;
    }
    let handle = Box::new(PrAppearanceClient {
        state: Mutex::new(Engine::default()),
    });
    // SAFETY: same checked output slot; ownership passes to the caller once.
    unsafe {
        out.write(Box::into_raw(handle));
    }
    OK
}
/// # Safety
/// client came from new, is freed once with no overlapping calls, and caller has
/// first retired its actual fd/notifiers. This function claims no native close.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_free_v1(client: *mut PrAppearanceClient) {
    if valid(client) {
        // SAFETY: caller guarantees exclusive live Box ownership at free.
        unsafe {
            drop(Box::from_raw(client));
        }
    }
}
/// # Safety
/// client is a live handle, with no concurrent free.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_refresh_v1(client: *mut PrAppearanceClient) -> i32 {
    // SAFETY: forwards the caller's live handle lifetime.
    unsafe { with(client, Engine::refresh) }
}
/// # Safety
/// client is live, without concurrent free; generation and event are copied.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_ready_v1(
    client: *mut PrAppearanceClient,
    generation: u64,
    event: u32,
) -> i32 {
    // SAFETY: forwards the caller's live handle lifetime.
    unsafe { with(client, |state| state.ready(generation, event)) }
}
/// # Safety
/// client is live; out points to a complete exclusive writable aligned Step,
/// disjoint from the handle/other accesses. Non-OK leaves it unchanged.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_step_v1(
    client: *mut PrAppearanceClient,
    out: *mut Step,
) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies live handle and exclusive complete output storage.
    unsafe {
        with(client, |state| match state.step() {
            Ok(plan) => {
                out.write(plan);
                OK
            }
            Err(code) => code,
        })
    }
}
/// # Safety
/// client is live; plan is an aligned complete readable copied Step. Nothing is retained.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_report_v1(
    client: *mut PrAppearanceClient,
    plan: *const Step,
    outcome: u32,
    sent_bytes: u32,
) -> i32 {
    if !valid(plan) {
        return INVALID;
    }
    // SAFETY: caller supplies a complete readable aligned copied Step.
    let plan = unsafe { plan.read() };
    // SAFETY: forwards the same live handle lifetime; only copied values enter policy.
    unsafe { with(client, |state| state.report(plan, outcome, sent_bytes)) }
}
/// # Safety
/// client is live, plan readable/aligned, and bytes spans length readable bytes
/// (null allowed only at length0). Pointers cannot overlap an exclusive output.
/// Oversized packets refuse without reading the caller's byte region.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_received_v1(
    client: *mut PrAppearanceClient,
    plan: *const Step,
    bytes: *const u8,
    length: u32,
) -> i32 {
    if !valid(plan) || (length != 0 && bytes.is_null()) {
        return INVALID;
    }
    // SAFETY: caller supplies the complete readable copied plan.
    let plan = unsafe { plan.read() };
    let packet = if length == 0 || length > 4096 {
        &[][..]
    } else {
        // SAFETY: bounded readable region supplied for this call; never retained.
        unsafe { std::slice::from_raw_parts(bytes, length as usize) }
    };
    // SAFETY: policy copies every accepted field before this borrow ends.
    unsafe { with(client, |state| state.received(plan, packet)) }
}
/// # Safety
/// client is a live handle with no concurrent free.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_tick_v1(client: *mut PrAppearanceClient) -> i32 {
    // SAFETY: forwards the live handle lifetime.
    unsafe { with(client, Engine::tick) }
}
/// # Safety
/// client is a live handle with no concurrent free.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_stop_v1(client: *mut PrAppearanceClient) -> i32 {
    // SAFETY: forwards the live handle lifetime.
    unsafe { with(client, Engine::stop) }
}
/// # Safety
/// client is live, out is complete aligned writable exclusive View storage,
/// disjoint from the handle/other accesses. Non-OK leaves it unchanged.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_view_v1(
    client: *const PrAppearanceClient,
    out: *mut View,
) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies exclusive complete writable output storage.
    unsafe {
        with(client, |state| {
            out.write(state.view());
            OK
        })
    }
}
/// # Safety
/// client is live; out is aligned exclusive writable uint32 storage, disjoint
/// from the handle/other accesses. This does not mutate publication or timers.
#[no_mangle]
pub unsafe extern "C" fn pr_appearance_client_available_v1(
    client: *const PrAppearanceClient,
    out: *mut u32,
) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies exclusive complete writable output storage.
    unsafe {
        with(client, |state| {
            out.write(u32::from(state.available()));
            OK
        })
    }
}
