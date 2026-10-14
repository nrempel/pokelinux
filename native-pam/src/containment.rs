//! Fixed bootstrap for an already admitted, single-threaded root worker.
//!
//! The outside owner retains all process/cgroup handles. This code changes only
//! the worker's mount namespace; Google's matched kernel has no PID namespaces.
//! The fixed root is provisioned by the trusted installer, never by an app.
use std::{ffi::CStr, fs, io, os::unix::fs::MetadataExt, ptr};

pub const ROOT: &str = "/run/pokelinux-auth-worker-root";

fn checked(value: libc::c_int) -> io::Result<()> {
    if value == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn protected(path: &str, directory: bool) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if m.uid() != 0
        || m.gid() != 0
        || m.mode() & 0o022 != 0
        || (if directory { !m.is_dir() } else { !m.is_file() })
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

fn mount(source: Option<&CStr>, target: &CStr, flags: libc::c_ulong) -> io::Result<()> {
    // SAFETY: all paths are fixed, NUL-terminated and live for this syscall.
    // No data or filesystem type is passed for bind/propagation operations.
    unsafe {
        checked(libc::mount(
            source.map_or(ptr::null(), CStr::as_ptr),
            target.as_ptr(),
            ptr::null(),
            flags,
            ptr::null(),
        ))
    }
}

fn bind_readonly(source: &CStr, target: &CStr, directory: bool) -> io::Result<()> {
    let path = target.to_str().map_err(|_| io::ErrorKind::InvalidInput)?;
    protected(path, directory)?;
    mount(Some(source), target, libc::MS_BIND)?;
    mount(
        None,
        target,
        libc::MS_BIND
            | libc::MS_REMOUNT
            | libc::MS_RDONLY
            | libc::MS_NOSUID
            | libc::MS_NODEV
            | libc::MS_NOEXEC,
    )
}

fn filter() -> io::Result<()> {
    // This is an inherited restriction on direct outside-process acquisition
    // and namespace entry, not an exec-path allowlist or a network sandbox.
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    compile_error!("qualify the native authentication ABI for this architecture");
    fn ins(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
        libc::sock_filter { code, jt, jf, k }
    }
    let mut program = vec![
        ins(0x20, 0, 0, 4), // seccomp_data.arch
        ins(0x15, 1, 0, ARCH),
        ins(0x06, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
        ins(0x20, 0, 0, 0), // seccomp_data.nr
    ];
    for nr in [
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_pidfd_getfd,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_setns,
        libc::SYS_unshare,
        libc::SYS_open_by_handle_at,
        libc::SYS_name_to_handle_at,
        libc::SYS_fsopen,
        libc::SYS_fsmount,
        libc::SYS_fsconfig,
        libc::SYS_open_tree,
        libc::SYS_move_mount,
        libc::SYS_mount_setattr,
    ] {
        program.push(ins(0x15, 0, 1, nr as u32));
        program.push(ins(
            0x06,
            0,
            0,
            libc::SECCOMP_RET_ERRNO | libc::EPERM as u32,
        ));
    }
    // Reject x32 syscall numbers on x86_64 as well as a different audit arch.
    #[cfg(target_arch = "x86_64")]
    {
        program.push(ins(0x45, 0, 1, 0x40000000));
        program.push(ins(0x06, 0, 0, libc::SECCOMP_RET_KILL_PROCESS));
    }
    program.push(ins(0x06, 0, 0, libc::SECCOMP_RET_ALLOW));
    let descriptor = libc::sock_fprog {
        len: program.len() as u16,
        filter: program.as_mut_ptr(),
    };
    // SAFETY: valid in-memory classic BPF for the duration of this synchronous
    // call. Root still has SYS_ADMIN; do not set NNP, which would disable the
    // original set-ID PAM helper. The kernel copies and validates the program.
    unsafe {
        checked(libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &descriptor as *const libc::sock_fprog,
            0,
            0,
        ))
    }
}

pub fn enter() -> io::Result<()> {
    for path in ["/", "/run", ROOT] {
        protected(path, true)?;
    }
    if fs::read_dir("/proc/self/task")?.count() != 1 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // SAFETY: dedicated worker before NSS/PAM/credential input. No other thread
    // can retain the former mount view or descriptors. Errors are terminal.
    unsafe {
        if libc::getuid() != 0
            || libc::geteuid() != 0
            || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 0
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        checked(libc::unshare(libc::CLONE_NEWNS))?;
    }
    mount(None, c"/", libc::MS_REC | libc::MS_PRIVATE)?;
    mount(
        Some(c"/run/pokelinux-auth-worker-root"),
        c"/run/pokelinux-auth-worker-root",
        libc::MS_BIND,
    )?;
    // The real worker task directory is the only proc task view. These are real
    // sysctls needed by entry.rs, not fabricated proc/security readbacks.
    bind_readonly(
        c"/proc/self/task",
        c"/run/pokelinux-auth-worker-root/proc/self/task",
        true,
    )?;
    bind_readonly(
        c"/proc/sys/fs/suid_dumpable",
        c"/run/pokelinux-auth-worker-root/proc/sys/fs/suid_dumpable",
        false,
    )?;
    bind_readonly(
        c"/proc/sys/kernel/cap_last_cap",
        c"/run/pokelinux-auth-worker-root/proc/sys/kernel/cap_last_cap",
        false,
    )?;
    // Keep set-ID execution enabled on the trusted root. There are no device
    // nodes here except explicitly provisioned dependencies on separate mounts.
    mount(
        None,
        c"/run/pokelinux-auth-worker-root",
        libc::MS_BIND | libc::MS_REMOUNT | libc::MS_RDONLY,
    )?;
    // SAFETY: documented pivot_root(".", ".") sequence on a private mount.
    // Afterwards no former-root cwd remains. close_range removes any descriptor
    // accidentally opened during bootstrap before any unprivileged code runs.
    unsafe {
        checked(libc::chdir(c"/run/pokelinux-auth-worker-root".as_ptr()))?;
        checked(libc::syscall(libc::SYS_pivot_root, c".".as_ptr(), c".".as_ptr()) as i32)?;
        checked(libc::umount2(c".".as_ptr(), libc::MNT_DETACH))?;
        checked(libc::chdir(c"/".as_ptr()))?;
        checked(libc::syscall(
            libc::SYS_close_range,
            3u32,
            u32::MAX,
            libc::CLOSE_RANGE_UNSHARE,
        ) as i32)?;
    }
    filter()?;
    // SAFETY: process-local readback; NNP must stay zero for the helper control.
    if unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) } != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
