//! One process-owned SIGTERM latch; the caller retains its existing Stop actor.
use std::{
    fs, io,
    marker::PhantomData,
    mem::MaybeUninit,
    ptr,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

static OWNER_PID: AtomicU32 = AtomicU32::new(0);
static REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn request_termination(_: libc::c_int) {
    // Available Rust atomics are lock-free. No allocation, I/O or Stop work is
    // performed from the signal handler.
    REQUESTED.store(true, Ordering::Relaxed);
}

fn single_main_thread(pid: u32) -> io::Result<()> {
    // SAFETY: gettid takes no pointers and reads the calling Linux thread ID.
    if unsafe { libc::gettid() } != pid as libc::pid_t {
        return Err(io::ErrorKind::Unsupported.into());
    }
    let mut tasks = fs::read_dir("/proc/self/task")?;
    let only = tasks
        .next()
        .transpose()?
        .ok_or(io::ErrorKind::InvalidData)?;
    if only.file_name() != pid.to_string().as_str() || tasks.next().transpose()?.is_some() {
        return Err(io::ErrorKind::Unsupported.into());
    }
    Ok(())
}

/// Own the calling process's SIGTERM latch until the existing actor has drained.
///
/// Installation and destruction require the main thread, a single-threaded
/// process and a trusted `/proc` mount. The process owner must exclude concurrent
/// thread creation and external signal-configuration changes at both boundaries.
/// Any threads created during the lifetime must be joined before destruction.
/// These checks do not serialize unknown external code. The guard is neither
/// Send nor Sync, and must outlive all of the caller's Stop/drain custody.
///
/// The inherited SIGTERM mask must be unblocked. This API never changes a mask,
/// installs a worker or creates a second Stop authority. A caught handler resets
/// to default across exec; children must exec before running independent code.
/// The copied parent latch is not observable as a stop request in a fork child.
#[must_use = "retain the guard until the existing Stop/drain path has completed"]
pub struct TerminationGuard {
    previous: libc::sigaction,
    pid: u32,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl TerminationGuard {
    /// Install one SIGTERM handler, refusing an active guard with AlreadyExists,
    /// an unqualified thread set with Unsupported, or a blocked mask with
    /// InvalidInput. Refusals do not change the signal action or calling mask.
    pub fn install() -> io::Result<Self> {
        let pid = std::process::id();
        single_main_thread(pid)?;
        OWNER_PID
            .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| io::Error::from(io::ErrorKind::AlreadyExists))?;
        let result = Self::install_owned(pid);
        if result.is_err() {
            OWNER_PID.store(0, Ordering::Release);
        }
        result
    }

    fn install_owned(pid: u32) -> io::Result<Self> {
        let mut inherited = MaybeUninit::<libc::sigset_t>::uninit();
        // SAFETY: null input queries only; inherited is aligned writable storage.
        // pthread_sigmask returns an error number directly, not through errno.
        let status = unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, ptr::null(), inherited.as_mut_ptr())
        };
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status));
        }
        // SAFETY: the successful query initialized the complete signal set.
        let inherited = unsafe { inherited.assume_init() };
        // SAFETY: inherited is a valid initialized set and SIGTERM is fixed.
        match unsafe { libc::sigismember(&inherited, libc::SIGTERM) } {
            0 => {}
            1 => return Err(io::ErrorKind::InvalidInput.into()),
            _ => return Err(io::Error::last_os_error()),
        }
        // SAFETY: all-zero is valid storage for this C structure; sigemptyset
        // initializes its mask before it is supplied to sigaction.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = request_termination as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        // SAFETY: action owns a valid writable signal-set field.
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut previous = MaybeUninit::<libc::sigaction>::uninit();
        // Clear before installing, so a signal delivered immediately upon
        // installation cannot be erased by a later initializer.
        REQUESTED.store(false, Ordering::Relaxed);
        // SAFETY: the handler has the C signal ABI and static lifetime; both
        // structures have the precise initialized input/writable output layout.
        if unsafe { libc::sigaction(libc::SIGTERM, &action, previous.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            // SAFETY: successful sigaction initialized the previous action.
            previous: unsafe { previous.assume_init() },
            pid,
            _not_send_sync: PhantomData,
        })
    }
}

impl Drop for TerminationGuard {
    fn drop(&mut self) {
        if self.pid != std::process::id() {
            // A fork child cannot release or restore the parent process's guard.
            return;
        }
        // Other threads must have joined: no old in-flight handler can then
        // race restoration and write into a later guard's fresh lifetime.
        if single_main_thread(self.pid).is_err() {
            std::process::abort();
        }
        // SAFETY: previous is the complete action saved by successful sigaction.
        if unsafe { libc::sigaction(libc::SIGTERM, &self.previous, ptr::null_mut()) } != 0 {
            // Do not publish a free lease while our handler might remain active.
            std::process::abort();
        }
        REQUESTED.store(false, Ordering::Relaxed);
        OWNER_PID.store(0, Ordering::Release);
    }
}

/// Read the latch only in its owning process. This does not execute Stop/drain.
pub fn termination_requested() -> bool {
    OWNER_PID.load(Ordering::Acquire) == std::process::id() && REQUESTED.load(Ordering::Relaxed)
}
