//! Test-only x86_64 guest-thread fault: return EPERM for FUTEX_WAKE_BITSET.
//! The immutable filter only adds a restriction; no TSYNC, device access or host execution.
//! Other syscalls, including mutex wake and thread exit, remain permitted.
#[cfg(target_arch = "x86_64")]
#[allow(
    unsafe_code,
    reason = "Audited test-only libc prctl ABI; pointers reference live initialized kernel POD filter arrays for the synchronous syscall, no mutation/retention by Rust or the kernel after return."
)]
pub fn install_thread_wake_failure() -> std::io::Result<()> {
    let instruction = |code: u16, jt: u8, jf: u8, k: u32| libc::sock_filter { code, jt, jf, k };
    // seccomp_data: nr offset0, architecture offset4, args[1] low32 offset24.
    // This function only compiles for x86_64, whose futex syscall is202.
    // Mask PRIVATE/CLOCK flags; operation10 is FUTEX_WAKE_BITSET.
    let mut filters = [
        instruction(0x20, 0, 0, 0),
        instruction(0x15, 0, 4, libc::SYS_futex as u32),
        instruction(0x20, 0, 0, 24),
        instruction(0x54, 0, 0, 0x7f),
        instruction(0x15, 0, 1, 10),
        instruction(0x06, 0, 0, libc::SECCOMP_RET_ERRNO | libc::EPERM as u32),
        instruction(0x06, 0, 0, libc::SECCOMP_RET_ALLOW),
    ];
    let program = libc::sock_fprog {
        len: filters.len() as u16,
        filter: filters.as_mut_ptr(),
    };
    // SAFETY: integer prctl arguments are valid; no_new_privs1 may only remove privilege.
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: program and its initialized filters stay live through this synchronous call.
    // The kernel copies/verifies them, and the filter applies only to this owned worker thread.
    if unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &program as *const libc::sock_fprog,
            0,
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
