//! Linux-PAM's public C ABI, confined to the dedicated worker process.
//! No account/session establishment, raw messages, caller policy or unlock API.
use crate::wire::{self, Outcome};
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    fs::{self, File},
    io,
    os::unix::fs::MetadataExt,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use zeroize::Zeroize;

const CONVERSATION_ERROR: c_int = 19;
#[derive(Clone, Copy)]
pub(crate) enum Policy {
    Production,
    #[cfg(feature = "ownership-fixture")]
    OwnershipFixture,
}
impl Policy {
    fn service(self) -> &'static CStr {
        match self {
            Self::Production => c"omarchy-lock-password",
            #[cfg(feature = "ownership-fixture")]
            Self::OwnershipFixture => c"pokelinux-auth-ownership-test",
        }
    }
    fn directory(self) -> &'static CStr {
        match self {
            Self::Production => c"/etc/pam.d",
            #[cfg(feature = "ownership-fixture")]
            Self::OwnershipFixture => c"/run/pokelinux-auth-scope-test/pam.d",
        }
    }
    fn configuration(self) -> &'static Path {
        match self {
            Self::Production => Path::new("/etc/pam.d/omarchy-lock-password"),
            #[cfg(feature = "ownership-fixture")]
            Self::OwnershipFixture => {
                Path::new("/run/pokelinux-auth-scope-test/pam.d/pokelinux-auth-ownership-test")
            }
        }
    }
}
#[repr(C)]
struct Message {
    style: c_int,
    text: *const c_char,
}
#[repr(C)]
struct Response {
    text: *mut c_char,
    code: c_int,
}
#[repr(C)]
struct Conversation {
    callback: unsafe extern "C" fn(
        c_int,
        *const *const Message,
        *mut *mut Response,
        *mut c_void,
    ) -> c_int,
    data: *mut c_void,
}
type Start = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const Conversation,
    *const c_char,
    *mut *mut c_void,
) -> c_int;
type Authenticate = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type GetItem = unsafe extern "C" fn(*const c_void, c_int, *mut *const c_void) -> c_int;
type End = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
struct Library {
    handle: *mut c_void,
    start: Start,
    authenticate: Authenticate,
    get_item: GetItem,
    end: End,
}
fn trusted(path: &Path) -> io::Result<()> {
    let actual = path.canonicalize()?;
    let file = fs::metadata(&actual)?;
    if !file.is_file() || file.uid() != 0 || file.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for name in [path, actual.as_path()] {
        for ancestor in name.ancestors().skip(1) {
            let m = fs::metadata(ancestor)?;
            if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
    }
    Ok(())
}
impl Library {
    fn load(policy: Policy) -> io::Result<Self> {
        trusted(Path::new("/usr/lib/libpam.so.0"))?;
        trusted(policy.configuration())?;
        trusted(Path::new("/usr/bin/unix_chkpwd"))?;
        // SAFETY: fixed trusted installed library and symbols. Function types
        // match the pinned installed Linux-PAM public headers, not private data.
        unsafe {
            let handle = libc::dlopen(
                c"/usr/lib/libpam.so.0".as_ptr(),
                libc::RTLD_NOW | libc::RTLD_LOCAL,
            );
            if handle.is_null() {
                return Err(io::ErrorKind::NotFound.into());
            }
            let names = [
                c"pam_start_confdir",
                c"pam_authenticate",
                c"pam_get_item",
                c"pam_end",
            ];
            let mut symbols = [ptr::null_mut(); 4];
            for (symbol, name) in symbols.iter_mut().zip(names) {
                libc::dlerror();
                *symbol = libc::dlsym(handle, name.as_ptr());
                if symbol.is_null() || !libc::dlerror().is_null() {
                    libc::dlclose(handle);
                    return Err(io::ErrorKind::NotFound.into());
                }
            }
            Ok(Self {
                handle,
                start: std::mem::transmute::<*mut c_void, Start>(symbols[0]),
                authenticate: std::mem::transmute::<*mut c_void, Authenticate>(symbols[1]),
                get_item: std::mem::transmute::<*mut c_void, GetItem>(symbols[2]),
                end: std::mem::transmute::<*mut c_void, End>(symbols[3]),
            })
        }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: this is our successful dlopen reference, after pam_end and
        // destruction of its handle; no callback or symbol outlives this owner.
        unsafe {
            libc::dlclose(self.handle);
        }
    }
}
struct Context {
    input: File,
    output: File,
    until: u64,
    prompts: u16,
    messages: usize,
}
struct Shared {
    failed: AtomicBool,
    context: Mutex<Context>,
}
/// Retains every libc allocation until a complete response array is transferred.
struct Replies {
    values: *mut Response,
    count: usize,
    lengths: [usize; 32],
}
impl Replies {
    fn new(count: usize) -> Option<Self> {
        // SAFETY: caller has checked 1..=32; calloc initializes every pointer and
        // return code. PAM requires libc allocation and frees successful replies.
        let values = unsafe { libc::calloc(count, size_of::<Response>()).cast::<Response>() };
        if values.is_null() {
            None
        } else {
            Some(Self {
                values,
                count,
                lengths: [0; 32],
            })
        }
    }
    fn set(&mut self, index: usize, bytes: &[u8]) -> Result<(), ()> {
        // SAFETY: checked batch/index and bounded response. Allocate the NUL
        // explicitly; rollback owns it before any further fallible operation.
        unsafe {
            let text = libc::calloc(bytes.len() + 1, 1).cast::<c_char>();
            if text.is_null() {
                return Err(());
            }
            ptr::copy_nonoverlapping(bytes.as_ptr(), text.cast::<u8>(), bytes.len());
            (*self.values.add(index)).text = text;
            self.lengths[index] = bytes.len() + 1;
        }
        Ok(())
    }
    fn release(&mut self) -> *mut Response {
        std::mem::replace(&mut self.values, ptr::null_mut())
    }
}
impl Drop for Replies {
    fn drop(&mut self) {
        if self.values.is_null() {
            return;
        }
        // SAFETY: unreleased allocations belong only to this guard. Lengths
        // were recorded from their actual allocations. Scrub before libc free.
        unsafe {
            for index in 0..self.count {
                let text = (*self.values.add(index)).text;
                if !text.is_null() {
                    std::slice::from_raw_parts_mut(text.cast::<u8>(), self.lengths[index])
                        .zeroize();
                    libc::free(text.cast());
                }
            }
            libc::free(self.values.cast());
        }
    }
}
unsafe extern "C" fn converse(
    count: c_int,
    messages: *const *const Message,
    responses: *mut *mut Response,
    data: *mut c_void,
) -> c_int {
    if data.is_null() {
        return CONVERSATION_ERROR;
    }
    // SAFETY: appdata is our stable Box<Shared>, alive through pam_end. Mutex
    // protects owned Send I/O even if a module invokes a callback on another thread.
    let shared = unsafe { &*data.cast::<Shared>() };
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), ()> {
        if responses.is_null() {
            return Err(());
        }
        // SAFETY: Linux-PAM supplies an initialized writable output pointer.
        unsafe {
            *responses = ptr::null_mut();
        }
        if !(1..=32).contains(&count) || messages.is_null() {
            return Err(());
        }
        let Ok(mut context) = shared.context.try_lock() else {
            return Err(());
        };
        if shared.failed.load(Ordering::SeqCst) {
            return Err(());
        }
        let count = count as usize;
        context.messages += count;
        if context.messages > 32 {
            return Err(());
        }
        let mut replies = Replies::new(count).ok_or(())?;
        for index in 0..count {
            wire::current(context.until).map_err(|_| ())?;
            // SAFETY: the public Linux ABI is an array of count message pointers,
            // not a contiguous array of messages. Foreign pointers are trusted
            // only for this documented ABI/lifetime; their text is never read.
            let message = unsafe { *messages.add(index) };
            if message.is_null() || unsafe { (*message).text.is_null() } {
                return Err(());
            }
            match unsafe { (*message).style } {
                1 => {
                    context.prompts = context.prompts.checked_add(1).ok_or(())?;
                    let sequence = context.prompts;
                    let until = context.until;
                    let Context { input, output, .. } = &mut *context;
                    let value = wire::response(input, output, sequence, until).map_err(|_| ())?;
                    replies.set(index, &value)?;
                }
                3 | 4 => {}          // No response; raw PAM text is not exported.
                _ => return Err(()), // Fixed password policy never echoes a response.
            }
        }
        // SAFETY: all replies are complete; ownership now transfers to PAM,
        // which releases strings and the array with libc free.
        unsafe {
            *responses = replies.release();
        }
        Ok(())
    }));
    if matches!(result, Ok(Ok(()))) {
        0
    } else {
        shared.failed.store(true, Ordering::SeqCst);
        CONVERSATION_ERROR
    }
}
pub fn authenticate(
    account: &CStr,
    input: File,
    output: File,
    until: u64,
    policy: Policy,
) -> (Outcome, File, File) {
    let Ok(library) = Library::load(policy) else {
        return (Outcome::Unavailable, input, output);
    };
    let shared = Box::new(Shared {
        failed: AtomicBool::new(false),
        context: Mutex::new(Context {
            input,
            output,
            until,
            prompts: 0,
            messages: 0,
        }),
    });
    let conversation = Conversation {
        callback: converse,
        data: (&*shared as *const Shared).cast_mut().cast(),
    };
    let mut handle = ptr::null_mut();
    let mut authenticated = None;
    // SAFETY: fixed service/directory and trusted fixed-account C string. Opaque
    // handles never escape; the boxed callback state remains alive through end.
    unsafe {
        if wire::current(until).is_ok()
            && (library.start)(
                policy.service().as_ptr(),
                account.as_ptr(),
                &conversation,
                policy.directory().as_ptr(),
                &mut handle,
            ) == 0
            && !handle.is_null()
        {
            let status = (library.authenticate)(handle, 0);
            let mut selected_user = ptr::null();
            let fixed_user = (library.get_item)(handle, 2, &mut selected_user) == 0
                && !selected_user.is_null()
                && CStr::from_ptr(selected_user.cast::<c_char>()) == account;
            // Exactly once after successful start, including auth/callback failure.
            let ended = (library.end)(handle, status);
            authenticated = Some((status, ended, fixed_user));
        }
    }
    let mut failed = shared.failed.load(Ordering::SeqCst);
    let Shared { context, .. } = *shared;
    let context = match context.into_inner() {
        Ok(context) => context,
        Err(poison) => {
            failed = true;
            poison.into_inner()
        }
    };
    let outcome = match authenticated {
        Some((0, 0, true))
            if !failed
                && context.prompts > 0
                && wire::current(until).is_ok()
                && crate::entry::account_matches(account).unwrap_or(false) =>
        {
            Outcome::Authenticated
        }
        Some((7 | 11, 0, true)) if !failed && wire::current(until).is_ok() => Outcome::Denied,
        _ => Outcome::Unavailable,
    };
    (outcome, context.input, context.output)
}
