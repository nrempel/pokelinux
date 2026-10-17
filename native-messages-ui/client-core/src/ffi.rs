//! Only this module translates caller pointers. All application policy is safe
//! Rust. Every operation copies inputs/outputs and holds the same engine mutex.
use crate::{engine::Engine, layout::*};
use std::sync::Mutex;

#[repr(C)]
pub struct PrMessagesClient {
    state: Mutex<Engine>,
}

fn pointer_valid<T>(pointer: *const T) -> bool {
    !pointer.is_null() && pointer.is_aligned()
}

/// # Safety
/// The pointer must be a live handle, without concurrent free. The C caller owns
/// that lifetime contract; null/misalignment and poisoned state are refused.
unsafe fn with(client: *const PrMessagesClient, operation: impl FnOnce(&mut Engine) -> i32) -> i32 {
    if !pointer_valid(client) {
        return INVALID;
    }
    // SAFETY: checked basic pointer form; caller guarantees live handle lifetime.
    let client = unsafe { &*client };
    match client.state.lock() {
        Ok(mut state) => operation(&mut state),
        Err(_) => REFUSED,
    }
}
/// # Safety
/// out must be a writable aligned pointer containing null. It must not overlap
/// another access. Non-OK leaves that output unchanged.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_new_v1(out: *mut *mut PrMessagesClient) -> i32 {
    if !pointer_valid(out) {
        return INVALID;
    }
    // SAFETY: caller guarantees a valid writable out slot.
    if unsafe { !(*out).is_null() } {
        return INVALID;
    }
    let client = Box::new(PrMessagesClient {
        state: Mutex::new(Engine::default()),
    });
    // SAFETY: checked aligned slot, exclusively writable for this call.
    unsafe {
        out.write(Box::into_raw(client));
    }
    OK
}
/// # Safety
/// client must be a live handle created here, with no call/free overlap. A
/// refused free preserves the handle and never blocks for native thread exit.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_free_v1(client: *mut PrMessagesClient) -> i32 {
    if !pointer_valid(client) {
        return INVALID;
    }
    // SAFETY: caller guarantees live handle and exclusive free lifetime.
    let handle = unsafe { &*client };
    let allowed = match handle.state.lock() {
        Ok(state) => state.can_free(),
        Err(_) => false,
    };
    if !allowed {
        return REFUSED;
    }
    // SAFETY: this pointer came from Box::into_raw and is released once only;
    // the above state guard has already been dropped.
    unsafe {
        drop(Box::from_raw(client));
    }
    OK
}
/// # Safety
/// client must be a live handle with no concurrent free.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_worker_enter_v1(client: *mut PrMessagesClient) -> i32 {
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, Engine::enter) }
}
/// # Safety
/// client must be live; out must be writable/aligned and disjoint from handle
/// storage and other caller accesses. Non-OK leaves output unchanged.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_view_v1(
    client: *const PrMessagesClient,
    out: *mut View,
) -> i32 {
    if !pointer_valid(out) {
        return INVALID;
    }
    // SAFETY: same live-handle contract; out is exclusively writable on success.
    unsafe {
        with(client, |state| {
            out.write(state.view());
            OK
        })
    }
}
/// # Safety
/// client must be live. A nonnull id points to a readable aligned complete Id;
/// its contents are copied for this call and never retained.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_action_v1(
    client: *mut PrMessagesClient,
    action: u32,
    id: *const Id,
) -> i32 {
    let id = if id.is_null() {
        None
    } else {
        if !pointer_valid(id) {
            return INVALID;
        }
        // SAFETY: caller guarantees readable complete Id.
        Some(unsafe { id.read() })
    };
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, |state| state.action(action, id)) }
}
/// # Safety
/// client must be live; out is an exclusively writable aligned complete Step.
/// WAIT and all other non-OK dispositions leave it unchanged.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_step_v1(
    client: *mut PrMessagesClient,
    out: *mut Step,
) -> i32 {
    if !pointer_valid(out) {
        return INVALID;
    }
    // SAFETY: live handle and exclusive output slot, written only on success.
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
/// client must be live; plan points to a readable aligned complete copied Step.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_report_v1(
    client: *mut PrMessagesClient,
    plan: *const Step,
    outcome: u32,
    sent: u32,
) -> i32 {
    if !pointer_valid(plan) {
        return INVALID;
    }
    // SAFETY: caller guarantees a complete readable plan; no pointer retained.
    let plan = unsafe { plan.read() };
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, |state| state.report(plan, outcome, sent)) }
}
/// # Safety
/// client must be live; plan is a readable aligned Step. bytes spans length
/// readable bytes, or may be null only at length0. Packet storage is copied.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_received_v1(
    client: *mut PrMessagesClient,
    plan: *const Step,
    bytes: *const u8,
    length: u32,
) -> i32 {
    if !pointer_valid(plan) || length as usize > CAPACITY || (length != 0 && bytes.is_null()) {
        return INVALID;
    }
    // SAFETY: caller guarantees a readable complete plan.
    let plan = unsafe { plan.read() };
    let bytes = if length == 0 {
        Vec::new()
    } else {
        // SAFETY: length <=4096, caller guarantees the readable packet region.
        unsafe { std::slice::from_raw_parts(bytes, length as usize) }.to_vec()
    };
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, |state| state.received(plan, bytes)) }
}
/// # Safety
/// client must be live, without concurrent free.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_deliver_v1(
    client: *mut PrMessagesClient,
    token: u64,
) -> i32 {
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, |state| state.deliver(token)) }
}
/// # Safety
/// client must be live, without concurrent free. The native worker may invoke
/// this same terminal Stop after actual queued-invocation failure.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_stop_v1(client: *mut PrMessagesClient) -> i32 {
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, Engine::stop) }
}
/// # Safety
/// client must be live; the native edge reports this only after actual loop exit
/// and descriptor closure. It does not constitute a native join.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_worker_exited_v1(client: *mut PrMessagesClient) -> i32 {
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, Engine::worker_exited) }
}
/// # Safety
/// client must be live; actual_completed is the real native wait return, never
/// an inferred worker flag. A false ABORT is a terminal caller disposition.
#[no_mangle]
pub unsafe extern "C" fn pr_messages_client_join_result_v1(
    client: *mut PrMessagesClient,
    actual_completed: u32,
) -> i32 {
    // SAFETY: same live-handle contract is forwarded to with.
    unsafe { with(client, |state| state.join_result(actual_completed)) }
}
