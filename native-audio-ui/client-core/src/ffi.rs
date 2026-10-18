//! Borrowed-pointer ABI only; all state and parsing live in safe Rust modules.
use crate::{
    engine::{Client, Packet, View},
    wire::{Code, Route},
};
use std::{ptr, slice};
/// Allocate one local client without a transport operation.
/// # Safety
/// out must be an exclusive initialized null pointer slot, aligned and writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_new_v1(out: *mut *mut Client) -> i32 {
    // SAFETY: caller owns the declared initialized output slot for this call.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return Code::Invalid as i32;
    };
    if !out.is_null() {
        return Code::Invalid as i32;
    }
    *out = Box::into_raw(Box::new(Client::new()));
    Code::Ok as i32
}
/// Dispose local state only, not remote cleanup.
/// # Safety
/// client is null or an unreleased allocation from new; no call/free may overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_free_v1(client: *mut Client) {
    if !client.is_null() {
        // SAFETY: the caller returns this exact allocation's ownership once.
        drop(unsafe { Box::from_raw(client) });
    }
}
/// Start the one initial connection intent.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_start_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::start) as i32
}
/// Report a native authenticated connection, never a fabricated peer.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_connected_v1(client: *mut Client, epoch: u64) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, |c| c.connected(epoch)) as i32
}
/// Copy one explicit route choice.
/// # Safety
/// client is exclusive/live; route is a valid aligned borrowed Route not aliasing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_choose_v1(
    client: *mut Client,
    route: *const Route,
) -> i32 {
    // SAFETY: both pointers satisfy the declared exclusive/borrowed call contract.
    let (Some(c), Some(r)) = (unsafe { client.as_mut() }, unsafe { route.as_ref() }) else {
        return Code::Invalid as i32;
    };
    c.choose(r) as i32
}
/// Explicit user reconnect only, without automatic replay.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_reconnect_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::reconnect) as i32
}
/// Request an idle read-only status poll.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_refresh_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::refresh) as i32
}
/// Copy an unsent packet without native I/O. Non-OK preserves out.
/// # Safety
/// client is exclusive/live; out is aligned writable Packet storage not aliasing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_prepare_v1(client: *mut Client, out: *mut Packet) -> i32 {
    // SAFETY: caller retains this declared live exclusive handle.
    let Some(c) = (unsafe { client.as_mut() }) else {
        return Code::Invalid as i32;
    };
    if out.is_null() {
        return Code::Invalid as i32;
    }
    match c.prepare() {
        Ok(p) => {
            // SAFETY: caller supplies the exclusive aligned output storage.
            unsafe { ptr::write(out, p) };
            Code::Ok as i32
        }
        Err(e) => e as i32,
    }
}
/// Report only the actual current native send result.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_sent_v1(
    client: *mut Client,
    epoch: u64,
    token: u64,
    outcome: u32,
    count: u32,
) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, |c| c.sent(epoch, token, outcome, count))
        as i32
}
/// Parse copied bytes from one already authenticated native packet.
/// # Safety
/// client is exclusive/live; bytes points to length readable initialized bytes,
/// not aliasing client. The native edge has already checked credentials/ancillary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_reply_v1(
    client: *mut Client,
    epoch: u64,
    bytes: *const u8,
    length: u32,
) -> i32 {
    // SAFETY: caller retains the declared live exclusive handle.
    let Some(c) = (unsafe { client.as_mut() }) else {
        return Code::Invalid as i32;
    };
    if bytes.is_null() || length == 0 || length > 4096 {
        return Code::Invalid as i32;
    }
    // SAFETY: finite length and the declared caller borrow cover this exact slice.
    c.reply(epoch, unsafe {
        slice::from_raw_parts(bytes, length as usize)
    }) as i32
}
/// Report a native failure for the matching channel only.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_disconnected_v1(
    client: *mut Client,
    epoch: u64,
    reason: u32,
) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, |c| c.disconnected(epoch, reason)) as i32
}
/// Deliver the native watch tick; no deadline is supplied or renewed.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_tick_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::tick) as i32
}
/// Copy a pure observation with no clock/state transition. Non-OK preserves out.
/// # Safety
/// client is live and borrowed without mutation/free; out is aligned exclusive
/// View storage not aliasing the client.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_view_v1(client: *const Client, out: *mut View) -> i32 {
    // SAFETY: caller retains the declared live immutable borrow.
    let Some(c) = (unsafe { client.as_ref() }) else {
        return Code::Invalid as i32;
    };
    if out.is_null() {
        return Code::Invalid as i32;
    }
    // SAFETY: caller supplies the exclusive aligned writable output storage.
    unsafe { ptr::write(out, c.view()) };
    Code::Ok as i32
}

/// Atomically select next-preview level/mute locally, without I/O or deadline changes.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_preview_level_v1(
    client: *mut Client,
    percent: u32,
    muted: u32,
) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, |c| c.preview_level(percent, muted)) as i32
}

/// Explicit Play input; never queues behind another request or opens a socket.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_preview_play_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::preview_play) as i32
}
/// Explicit cleanup input; fences an in-flight effect by closing local channel state.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_preview_stop_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::preview_stop) as i32
}

/// One explicit mute action; the Rust engine chooses local or current-Play scope.
/// # Safety
/// client is an exclusive live handle on its single caller thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_sounds_client_preview_toggle_mute_v1(client: *mut Client) -> i32 {
    // SAFETY: caller owns the declared live exclusive handle.
    unsafe { client.as_mut() }.map_or(Code::Invalid, Client::toggle_preview_mute) as i32
}
