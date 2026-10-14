//! Root starts the fixed account directly, without an exec after changing UID.
use std::{ffi::CString, fs, io, ptr};

// The original setuid unix_chkpwd helper reports successful verification through
// audit. Actual original-helper controls reject the matching response with Bnd0
// and accept it with only this bit. The Rust worker's active sets remain empty.
const HELPER_AUDIT_WRITE: u32 = 29;

fn checked(result: libc::c_int) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
fn single_thread() -> io::Result<()> {
    if fs::read_dir("/proc/self/task")?
        .collect::<Result<Vec<_>, _>>()?
        .len()
        != 1
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
fn parent_guard(parent: u32) -> io::Result<()> {
    // SAFETY: fixed prctl operations and an initialized integer output, in this
    // dedicated worker. Parent identity is supplied by its trusted launcher.
    unsafe {
        if parent < 2 || libc::getppid() as u32 != parent {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        checked(libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0))?;
        let mut signal = 0;
        checked(libc::prctl(libc::PR_GET_PDEATHSIG, &mut signal, 0, 0, 0))?;
        if signal != libc::SIGKILL || libc::getppid() as u32 != parent {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}
pub fn initial(parent: u32) -> io::Result<()> {
    single_thread()?;
    // SAFETY: fixed process-local operations, before any private response.
    unsafe {
        if libc::getuid() != 0 || libc::geteuid() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        parent_guard(parent)?;
        checked(libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0))?;
        checked(libc::syscall(
            libc::SYS_close_range,
            3u32,
            u32::MAX,
            libc::CLOSE_RANGE_UNSHARE,
        ) as i32)?;
        for fd in 0..=2 {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }
            checked(libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC))?;
        }
    }
    Ok(())
}
pub fn enter(parent: u32) -> io::Result<CString> {
    // SAFETY: the process is a dedicated single-threaded worker. Scalars and
    // initialized output buffers below use the installed public libc ABI.
    unsafe {
        if libc::getuid() != 0
            || libc::geteuid() != 0
            || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 0
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // setresuid resets dumpability to this kernel setting. Require zero so
        // no same-UID ptrace interval opens at the credential transition.
        if fs::read_to_string("/proc/sys/fs/suid_dumpable")?.trim() != "0" {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        if libc::getppid() as u32 != parent || libc::prctl(libc::PR_GET_SECUREBITS, 0, 0, 0, 0) != 0
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        checked(libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0))?;
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        checked(libc::setrlimit(libc::RLIMIT_CORE, &limit))?;
        let mut record: libc::passwd = std::mem::zeroed();
        let mut buffer = vec![0 as libc::c_char; 16384];
        let mut result = ptr::null_mut();
        let status = libc::getpwuid_r(
            1000,
            &mut record,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        );
        if status != 0
            || result.is_null()
            || record.pw_uid != 1000
            || record.pw_gid != 1000
            || record.pw_name.is_null()
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let account = std::ffi::CStr::from_ptr(record.pw_name).to_owned();
        if account.as_bytes().is_empty()
            || account.as_bytes().len() > 64
            || !account
                .as_bytes()
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(b))
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut reverse: libc::passwd = std::mem::zeroed();
        let mut reverse_buffer = vec![0 as libc::c_char; 16384];
        let mut reverse_result = ptr::null_mut();
        let reverse_status = libc::getpwnam_r(
            account.as_ptr(),
            &mut reverse,
            reverse_buffer.as_mut_ptr(),
            reverse_buffer.len(),
            &mut reverse_result,
        );
        if reverse_status != 0
            || reverse_result.is_null()
            || reverse.pw_uid != 1000
            || reverse.pw_gid != 1000
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        single_thread()?;
        checked(libc::syscall(
            libc::SYS_close_range,
            3u32,
            u32::MAX,
            libc::CLOSE_RANGE_UNSHARE,
        ) as i32)?;
        let last: u32 = fs::read_to_string("/proc/sys/kernel/cap_last_cap")?
            .trim()
            .parse()
            .map_err(|_| io::ErrorKind::InvalidData)?;
        if !(HELPER_AUDIT_WRITE..=63).contains(&last) {
            return Err(io::ErrorKind::Unsupported.into());
        }
        checked(libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        ))?;
        for cap in 0..=last {
            if cap != HELPER_AUDIT_WRITE {
                checked(libc::prctl(libc::PR_CAPBSET_DROP, cap, 0, 0, 0))?;
            }
            let expected = i32::from(cap == HELPER_AUDIT_WRITE);
            if libc::prctl(libc::PR_CAPBSET_READ, cap, 0, 0, 0) != expected {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
        checked(libc::setgroups(0, ptr::null()))?;
        checked(libc::setresgid(1000, 1000, 1000))?;
        checked(libc::setresuid(1000, 1000, 1000))?;
        checked(libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0))?;
        rustix::thread::set_capabilities(
            None,
            rustix::thread::CapabilitySets {
                effective: rustix::thread::CapabilitySet::empty(),
                permitted: rustix::thread::CapabilitySet::empty(),
                inheritable: rustix::thread::CapabilitySet::empty(),
            },
        )?;
        let caps = rustix::thread::capabilities(None)?;
        if !caps.effective.is_empty()
            || !caps.permitted.is_empty()
            || !caps.inheritable.is_empty()
            || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 0
            || libc::prctl(libc::PR_GET_KEEPCAPS, 0, 0, 0, 0) != 0
            || libc::setfsuid(u32::MAX) != 1000
            || libc::setfsgid(u32::MAX) != 1000
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        for cap in 0..=last {
            let expected = i32::from(cap == HELPER_AUDIT_WRITE);
            if libc::prctl(libc::PR_CAPBSET_READ, cap, 0, 0, 0) != expected
                || libc::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_IS_SET, cap, 0, 0) != 0
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
        let mut core: libc::rlimit = std::mem::zeroed();
        checked(libc::getrlimit(libc::RLIMIT_CORE, &mut core))?;
        if core.rlim_cur != 0 || core.rlim_max != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        parent_guard(parent)?;
        let mut r = 0;
        let mut e = 0;
        let mut s = 0;
        checked(libc::getresuid(&mut r, &mut e, &mut s))?;
        if [r, e, s] != [1000; 3]
            || libc::getgroups(0, ptr::null_mut()) != 0
            || libc::getppid() as u32 != parent
            || libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) != 0
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        checked(libc::getresgid(&mut r, &mut e, &mut s))?;
        if [r, e, s] != [1000; 3] {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(account)
    }
}

pub fn account_matches(account: &std::ffi::CStr) -> io::Result<bool> {
    // SAFETY: fixed previously resolved account, initialized caller-owned NSS
    // buffer and outputs; this checks identity again after PAM handle teardown.
    unsafe {
        let mut record: libc::passwd = std::mem::zeroed();
        let mut buffer = vec![0 as libc::c_char; 16384];
        let mut result = ptr::null_mut();
        let status = libc::getpwnam_r(
            account.as_ptr(),
            &mut record,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        );
        Ok(status == 0
            && !result.is_null()
            && record.pw_uid == 1000
            && record.pw_gid == 1000
            && !record.pw_name.is_null()
            && std::ffi::CStr::from_ptr(record.pw_name) == account)
    }
}
