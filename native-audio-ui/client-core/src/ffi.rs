//! Fixed borrowed-pointer ABI. All operations run on the one native event thread.
use crate::{
    queue::Queue,
    wire::{Code, Command, Fresh, Route},
};
use pokelinux_power::clock::{BootClock, Clock};
use std::ptr;
fn now() -> Option<u64> {
    BootClock.now().elapsed().as_nanos().try_into().ok()
}

/// Allocate one empty local queue. No service operation is performed.
/// # Safety
/// out must point to a writable initialized null handle slot, exclusive for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_new_v1(out: *mut *mut Queue) -> i32 {
    // SAFETY: the caller supplies the declared initialized writable output slot.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return Code::Invalid as i32;
    };
    if !out.is_null() {
        return Code::Invalid as i32;
    }
    *out = Box::into_raw(Box::new(Queue::new()));
    Code::Ok as i32
}
/// Release a queue; this is not remote cleanup.
/// # Safety
/// queue must be null or an unreleased handle from new; no borrowed call may coexist.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_free_v1(queue: *mut Queue) {
    if !queue.is_null() {
        // SAFETY: ownership of this exact allocation is returned once by the caller.
        drop(unsafe { Box::from_raw(queue) });
    }
}
/// Copy one bounded explicit gesture and sample its original clock budget.
/// # Safety
/// queue must be a live exclusive handle; route must be null or an initialized
/// aligned Route borrowed for this call. No pointer is retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_offer_v1(
    queue: *mut Queue,
    kind: u32,
    route: *const Route,
) -> i32 {
    // SAFETY: the caller supplies the declared live handle and optional input.
    let Some(queue) = (unsafe { queue.as_mut() }) else {
        return Code::Invalid as i32;
    };
    let Some(now) = now() else {
        return Code::Invalid as i32;
    };
    // SAFETY: route is null or borrowed initialized storage for this call.
    queue.offer(kind, unsafe { route.as_ref() }, now) as i32
}
/// Read only, without dispatching or renewing a deadline.
/// # Safety
/// queue must be a live borrowed handle, without a simultaneous mutable call/free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_pending_v1(queue: *const Queue) -> i32 {
    // SAFETY: the caller retains the declared live borrowed handle.
    unsafe { queue.as_ref() }.map_or(-1, |q| i32::from(q.pending()))
}
/// Prepare a copy; failures preserve the output storage.
/// # Safety
/// queue must be a live exclusive handle; fresh and out must be initialized aligned
/// objects of their declared layouts, not aliasing the queue or one another.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_prepare_v1(
    queue: *mut Queue,
    fresh: *const Fresh,
    out: *mut Command,
) -> i32 {
    // SAFETY: all three pointers satisfy the borrowed/exclusive call contract.
    let (Some(queue), Some(fresh)) = (unsafe { queue.as_mut() }, unsafe { fresh.as_ref() }) else {
        return Code::Invalid as i32;
    };
    if out.is_null() {
        return Code::Invalid as i32;
    }
    let Some(now) = now() else {
        queue.cancel();
        return Code::Invalid as i32;
    };
    match queue.prepare(fresh, now) {
        Ok(command) => {
            // SAFETY: the caller provides the exclusive writable Command output.
            unsafe {
                ptr::write(out, command);
            }
            Code::Ok as i32
        }
        Err(code) => code as i32,
    }
}
/// Record only the native edge's actual complete-send observation.
/// # Safety
/// queue must be a live exclusive handle throughout this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_sent_v1(queue: *mut Queue, token: u64) -> i32 {
    // SAFETY: the caller retains this declared exclusive live handle.
    let Some(queue) = (unsafe { queue.as_mut() }) else {
        return Code::Invalid as i32;
    };
    let Some(now) = now() else {
        queue.cancel();
        return Code::Invalid as i32;
    };
    queue.sent(token, now) as i32
}
/// Forget queued intent without a transport operation.
/// # Safety
/// queue must be null or a live exclusive handle, without concurrent access/free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_queue_cancel_v1(queue: *mut Queue) {
    // SAFETY: the caller retains the declared optional live exclusive handle.
    if let Some(queue) = unsafe { queue.as_mut() } {
        queue.cancel();
    }
}
