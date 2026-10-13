//! After acquisition, content reads use retained input/socket handles.
//! Architecture-checked default-deny seccomp prevents new content opens, process
//! spawn, namespace changes, io_uring and new networking. Path metadata syscalls
//! remain available for generation checks; this is not filesystem invisibility.
//! UID alone is not confinement.
use std::io;

pub fn restrict() -> io::Result<()> {
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    compile_error!("appearance confinement requires an audited syscall architecture");
    let statement = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k, jt, jf| libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    };
    let mut code = vec![
        statement(0x20, 4),
        jump(ARCH, 1, 0),
        statement(0x06, 0x80000000),
        statement(0x20, 0),
    ];
    let allowed = [
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_close,
        libc::SYS_fstat,
        libc::SYS_newfstatat,
        libc::SYS_statx,
        libc::SYS_lseek,
        libc::SYS_fcntl,
        libc::SYS_flock,
        libc::SYS_recvmsg,
        libc::SYS_sendto,
        libc::SYS_ppoll,
        libc::SYS_clock_gettime,
        libc::SYS_clock_nanosleep,
        libc::SYS_brk,
        libc::SYS_mmap,
        libc::SYS_munmap,
        libc::SYS_mremap,
        libc::SYS_mprotect,
        libc::SYS_madvise,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_sigaltstack,
        libc::SYS_futex,
        libc::SYS_sched_yield,
        libc::SYS_getpid,
        libc::SYS_getppid,
        libc::SYS_getuid,
        libc::SYS_geteuid,
        libc::SYS_getgid,
        libc::SYS_getegid,
        libc::SYS_exit,
        libc::SYS_exit_group,
    ];
    for number in allowed {
        code.extend([jump(number as u32, 0, 1), statement(0x06, 0x7fff0000)]);
    }
    #[cfg(target_arch = "x86_64")]
    code.extend([
        jump(libc::SYS_poll as u32, 0, 1),
        statement(0x06, 0x7fff0000),
    ]);
    code.push(statement(0x06, 0x00050000 | libc::EPERM as u32));
    let program = libc::sock_fprog {
        len: code.len() as u16,
        filter: code.as_mut_ptr(),
    };
    // No privilege is gained by exec, and filter installation requires NNP.
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::syscall(libc::SYS_seccomp, 1, 0, &program) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
