//! C pointer translation; caller guarantees live storage and no free/call overlap.
use crate::{Client, View};
use std::sync::Mutex;
pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const REFUSED: i32 = 2;
#[repr(C)]
pub struct PrNetworkStatusClient {
    state: Mutex<Client>,
}
fn valid<T>(pointer: *const T) -> bool {
    !pointer.is_null() && pointer.is_aligned()
}
/// # Safety
/// A nonnull aligned handle is live, without overlapping free.
unsafe fn with(client: *mut PrNetworkStatusClient, operation: impl FnOnce(&mut Client)) -> i32 {
    if !valid(client) {
        return INVALID;
    }
    // SAFETY: caller guarantees the live handle lifetime after form checks.
    match unsafe { &*client }.state.lock() {
        Ok(mut state) => {
            operation(&mut state);
            OK
        }
        Err(_) => REFUSED,
    }
}
/// # Safety
/// out is complete aligned exclusive writable pointer storage initially null.
#[no_mangle]
pub unsafe extern "C" fn pr_network_status_client_new_v1(
    out: *mut *mut PrNetworkStatusClient,
) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies complete exclusive writable pointer storage.
    if unsafe { !(*out).is_null() } {
        return INVALID;
    }
    let handle = Box::new(PrNetworkStatusClient {
        state: Mutex::new(Client::new()),
    });
    // SAFETY: ownership is transferred into the same checked exclusive slot.
    unsafe {
        out.write(Box::into_raw(handle));
    }
    OK
}
/// # Safety
/// client was returned by new and is freed once, without overlapping calls.
/// Null is allowed. Free also closes any Rust-owned pending socket.
#[no_mangle]
pub unsafe extern "C" fn pr_network_status_client_free_v1(client: *mut PrNetworkStatusClient) {
    if valid(client) {
        // SAFETY: caller guarantees unique live Box ownership for free.
        unsafe {
            drop(Box::from_raw(client));
        }
    }
}
/// # Safety
/// client is live, without overlapping free. Work and socket I/O are bounded.
#[no_mangle]
pub unsafe extern "C" fn pr_network_status_client_tick_v1(
    client: *mut PrNetworkStatusClient,
) -> i32 {
    // SAFETY: forwards the caller's live handle lifetime.
    unsafe { with(client, Client::tick) }
}
/// # Safety
/// client is live; out is complete aligned exclusive writable View storage,
/// disjoint from the handle. Non-OK leaves output unchanged. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn pr_network_status_client_view_v1(
    client: *mut PrNetworkStatusClient,
    out: *mut View,
) -> i32 {
    if !valid(out) {
        return INVALID;
    }
    // SAFETY: caller supplies the same live handle and complete exclusive output.
    unsafe { with(client, |state| out.write(state.view())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_abi_checks_pointer_form_preserves_refused_outputs_and_owns_lifetime() {
        // SAFETY: all accepted slots below are complete live exclusive storage;
        // deliberately invalid forms must be rejected before dereference.
        unsafe {
            assert_eq!(
                pr_network_status_client_new_v1(std::ptr::null_mut()),
                INVALID
            );
            let mut client = std::ptr::null_mut();
            assert_eq!(pr_network_status_client_new_v1(&mut client), OK);
            let original = client;
            assert_eq!(pr_network_status_client_new_v1(&mut client), INVALID);
            assert_eq!(client, original);
            let sentinel = View {
                available: 9,
                local_interfaces: 42,
                address_records: 8,
                revision: 7,
            };
            let mut view = sentinel;
            assert_eq!(
                pr_network_status_client_view_v1(std::ptr::null_mut(), &mut view),
                INVALID
            );
            assert_eq!(view, sentinel);
            assert_eq!(
                pr_network_status_client_tick_v1(std::ptr::null_mut()),
                INVALID
            );
            assert_eq!(
                pr_network_status_client_view_v1(client, std::ptr::null_mut()),
                INVALID
            );
            let mut storage = [0u64; 8];
            let misaligned = storage.as_mut_ptr().cast::<u8>().add(1).cast::<View>();
            assert_eq!(
                pr_network_status_client_view_v1(client, misaligned),
                INVALID
            );
            assert_eq!(storage, [0u64; 8]);
            assert_eq!(pr_network_status_client_view_v1(client, &mut view), OK);
            assert_eq!(view, View::default());
            pr_network_status_client_free_v1(client);
            pr_network_status_client_free_v1(std::ptr::null_mut());
        }
    }
}
