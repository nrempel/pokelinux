//! Actual kernel controls on the production sandbox, after a real UID1000 drop.
//! Content opens are confined; permitted path metadata and inherited descriptors
//! are separate properties. These later controls are not a chronological RED.
use pokelinux_appearance::sandbox;
use rustix::process::{Pid, PidfdFlags};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::{self, File},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, PermissionsExt},
    },
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Serialize, Deserialize)]
struct Report {
    pid: i32,
    prior_filters: u32,
    selected_read: bool,
    unrelated_open_before: bool,
    inet_before: bool,
    clone_before: bool,
    exec_before: bool,
    compat_before: Option<bool>,
    open_errno: Option<i32>,
    socket_errno: Option<i32>,
    clone_errno: Option<i32>,
    exec_errno: Option<i32>,
    unexpected_clone_pid: Option<i32>,
}
fn err() -> io::Error {
    io::Error::last_os_error()
}
fn errno(result: libc::c_long) -> i32 {
    if result == -1 {
        err().raw_os_error().unwrap_or(0)
    } else {
        0
    }
}
fn status_value<'a>(status: &'a str, key: &str) -> &'a str {
    status
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .unwrap_or("")
        .trim()
}
fn direct_wait(pid: i32, duration: Duration) -> Option<i32> {
    let end = Instant::now() + duration;
    loop {
        let mut status = 0;
        let n = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if n == pid {
            return Some(status);
        }
        if n < 0 && err().raw_os_error() != Some(libc::EINTR) {
            return None;
        }
        if Instant::now() >= end {
            return None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
struct ChildOwner {
    pid: i32,
    pidfd: Option<OwnedFd>,
    reaped: bool,
    unexpected: Option<i32>,
}
impl ChildOwner {
    fn wait(&mut self) -> i32 {
        let status = direct_wait(self.pid, Duration::from_secs(2))
            .expect("owned child direct wait timed out");
        self.reaped = true;
        status
    }
}
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if !self.reaped {
            unsafe {
                libc::kill(self.pid, libc::SIGKILL);
            }
            if direct_wait(self.pid, Duration::from_secs(2)).is_none() {
                std::process::abort();
            }
            self.reaped = true;
        }
        // The parent is a subreaper. Even a mistakenly allowed clone stays owned.
        if let Some(pid) = self.unexpected {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
            if direct_wait(pid, Duration::from_secs(2)).is_none() {
                std::process::abort();
            }
        }
    }
}
fn pipe() -> [OwnedFd; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
        0
    );
    unsafe { [OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])] }
}
fn write_all(fd: i32, bytes: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let count =
            unsafe { libc::write(fd, bytes[offset..].as_ptr().cast(), bytes.len() - offset) };
        if count < 0 {
            return Err(err());
        }
        if count == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        offset += count as usize;
    }
    Ok(())
}
fn await_release(fd: i32) -> io::Result<()> {
    // The sandbox permits these actual poll/read/clock calls; no fresh FD opens.
    let end = pokelinux_appearance::boottime_ns().saturating_add(3_000_000_000);
    let mut byte = 0u8;
    while pokelinux_appearance::boottime_ns() < end {
        let n = unsafe { libc::read(fd, (&mut byte as *mut u8).cast(), 1) };
        if n == 1 {
            return if byte == 1 {
                Ok(())
            } else {
                Err(io::ErrorKind::InvalidData.into())
            };
        }
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        if !matches!(err().raw_os_error(), Some(libc::EAGAIN | libc::EINTR)) {
            return Err(err());
        }
        let mut item = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            libc::poll(&mut item, 1, 10);
        }
    }
    Err(io::ErrorKind::TimedOut.into())
}
fn clone_probe(exec: bool) -> io::Result<bool> {
    // SIGCHLD without CLONE_VM has fork semantics on both audited architectures.
    let pid = unsafe {
        libc::syscall(
            libc::SYS_clone,
            libc::SIGCHLD,
            0usize,
            0usize,
            0usize,
            0usize,
        )
    };
    if pid < 0 {
        return Err(err());
    }
    if pid == 0 {
        unsafe {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
            if exec {
                let program = c"/bin/true";
                let argv = [program.as_ptr(), std::ptr::null()];
                let envp: [*const libc::c_char; 1] = [std::ptr::null()];
                libc::execve(program.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::_exit(91);
            }
            libc::_exit(0);
        }
    }
    if let Some(status) = direct_wait(pid as i32, Duration::from_secs(1)) {
        return Ok(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
    }
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    if direct_wait(pid as i32, Duration::from_secs(1)).is_none() {
        unsafe {
            libc::abort();
        }
    }
    Err(io::ErrorKind::TimedOut.into())
}
#[cfg(target_arch = "x86_64")]
fn compatibility_getpid() -> u32 {
    let mut number = 20u32; // i386 __NR_getpid; actual AUDIT_ARCH_I386 syscall entry.
    unsafe {
        core::arch::asm!("int 0x80", inlateout("eax") number,
            lateout("rcx") _, lateout("r8") _, lateout("r9") _,
            lateout("r10") _, lateout("r11") _, options(nostack));
    }
    number
}
fn child(
    private: PathBuf,
    report_fd: i32,
    release_fd: i32,
    architecture_case: bool,
) -> io::Result<()> {
    if unsafe { libc::setgroups(0, std::ptr::null()) } != 0
        || unsafe { libc::setresgid(1000, 1000, 1000) } != 0
        || unsafe { libc::setresuid(1000, 1000, 1000) } != 0
    {
        return Err(err());
    }
    if unsafe { libc::getuid() } != 1000
        || unsafe { libc::geteuid() } != 1000
        || unsafe { libc::getgid() } != 1000
        || unsafe { libc::getegid() } != 1000
        || unsafe { libc::getgroups(0, std::ptr::null_mut()) } != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // Credential changes clear PDEATHSIG, so set it after the actual drop.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) } != 0 {
        return Err(err());
    }
    fs::create_dir(&private)?;
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700))?;
    let selected = private.join("selected");
    let unrelated = private.join("unrelated-synthetic");
    fs::write(&selected, b"selected-public-palette")?;
    fs::write(&unrelated, b"synthetic-unrelated-content")?;
    for path in [&selected, &unrelated] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        let m = path.metadata()?;
        if !m.is_file() || m.uid() != 1000 || m.gid() != 1000 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    let mut input = File::open(&selected)?;
    let mut before = String::new();
    File::open(&unrelated)?.read_to_string(&mut before)?;
    let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if socket < 0 {
        return Err(err());
    }
    unsafe {
        libc::close(socket);
    }
    let clone_before = clone_probe(false)?;
    let exec_before = clone_probe(true)?;
    let status = fs::read_to_string("/proc/self/status")?;
    let prior_filters = status_value(&status, "Seccomp_filters:")
        .parse()
        .map_err(|_| io::ErrorKind::InvalidData)?;
    let pathname = CString::new(unrelated.as_os_str().as_encoded_bytes())
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let mut report = Report {
        pid: unsafe { libc::getpid() },
        prior_filters,
        selected_read: false,
        unrelated_open_before: before == "synthetic-unrelated-content",
        inet_before: true,
        clone_before,
        exec_before,
        compat_before: None,
        open_errno: None,
        socket_errno: None,
        clone_errno: None,
        exec_errno: None,
        unexpected_clone_pid: None,
    };
    #[cfg(target_arch = "x86_64")]
    if architecture_case {
        report.compat_before = Some(compatibility_getpid() == report.pid as u32);
    }
    if !report.unrelated_open_before
        || !clone_before
        || !exec_before
        || report.compat_before == Some(false)
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    sandbox::restrict()?; // The exact production filter, no alternate test policy.
    let mut bytes = [0; 23];
    input.read_exact(&mut bytes)?;
    report.selected_read = &bytes == b"selected-public-palette";
    if !architecture_case {
        let opened = unsafe {
            libc::syscall(
                libc::SYS_openat,
                libc::AT_FDCWD,
                pathname.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC,
                0,
            )
        };
        report.open_errno = Some(errno(opened));
        if opened >= 0 {
            unsafe {
                libc::close(opened as i32);
            }
        }
        let socket = unsafe {
            libc::syscall(
                libc::SYS_socket,
                libc::AF_INET,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                0,
            )
        };
        report.socket_errno = Some(errno(socket));
        if socket >= 0 {
            unsafe {
                libc::close(socket as i32);
            }
        }
        let cloned = unsafe {
            libc::syscall(
                libc::SYS_clone,
                libc::SIGCHLD,
                0usize,
                0usize,
                0usize,
                0usize,
            )
        };
        if cloned == 0 {
            unsafe {
                libc::_exit(92);
            }
        }
        report.clone_errno = Some(errno(cloned));
        if cloned > 0 {
            // Report ownership before any later assertion/exec; parent is subreaper.
            report.unexpected_clone_pid = Some(cloned as i32);
        } else {
            let program = c"/bin/true";
            let argv = [program.as_ptr(), std::ptr::null()];
            let envp: [*const libc::c_char; 1] = [std::ptr::null()];
            let result = unsafe {
                libc::syscall(
                    libc::SYS_execve,
                    program.as_ptr(),
                    argv.as_ptr(),
                    envp.as_ptr(),
                )
            };
            report.exec_errno = Some(errno(result));
        }
    }
    let mut data = serde_json::to_vec(&report)?;
    data.push(b'\n');
    write_all(report_fd, &data)?;
    await_release(release_fd)?;
    #[cfg(target_arch = "x86_64")]
    if architecture_case {
        let _ = compatibility_getpid();
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
fn exercise(architecture_case: bool) {
    assert_eq!(
        unsafe { libc::getuid() },
        0,
        "actual root/SETUID/SETGID capability precondition"
    );
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) },
        0
    );
    let directory = PathBuf::from(format!(
        "/tmp/appearance-confinement-{}-{}",
        std::process::id(),
        pokelinux_appearance::fresh_id().unwrap()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o777)).unwrap();
    let private = directory.join("uid1000-private");
    let [parent_read, child_write] = pipe();
    let [child_read, parent_write] = pipe();
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "actual fork prerequisite");
    if pid == 0 {
        drop(parent_read);
        drop(parent_write);
        // Fork leaves one actual thread; /proc/PID/status describes this filter.
        let result = child(
            private,
            child_write.as_raw_fd(),
            child_read.as_raw_fd(),
            architecture_case,
        );
        if let Err(e) = result {
            let _ = write_all(child_write.as_raw_fd(), format!("ERROR:{e}\n").as_bytes());
            unsafe {
                libc::_exit(90);
            }
        }
        unsafe {
            libc::_exit(0);
        }
    }
    let mut owner = ChildOwner {
        pid,
        pidfd: None,
        reaped: false,
        unexpected: None,
    };
    owner.pidfd = Some(
        rustix::process::pidfd_open(Pid::from_raw(pid).unwrap(), PidfdFlags::NONBLOCK).unwrap(),
    );
    drop(child_write);
    drop(child_read);
    let end = Instant::now() + Duration::from_secs(3);
    let mut data = Vec::new();
    while Instant::now() < end && !data.ends_with(b"\n") {
        let mut buffer = [0; 2048];
        let count = unsafe {
            libc::read(
                parent_read.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        if count > 0 {
            data.extend_from_slice(&buffer[..count as usize]);
        } else if count == 0 {
            break;
        } else {
            assert!(matches!(
                err().raw_os_error(),
                Some(libc::EAGAIN | libc::EINTR)
            ));
        }
        assert!(data.len() <= 4096, "bounded test report");
        std::thread::sleep(Duration::from_millis(2));
    }
    let report: Report = serde_json::from_slice(&data).unwrap_or_else(|e| {
        panic!(
            "actual child prerequisite/report failed: {e}; {}",
            String::from_utf8_lossy(&data)
        )
    });
    owner.unexpected = report.unexpected_clone_pid;
    assert_eq!(report.pid, pid);
    assert!(
        report.selected_read
            && report.unrelated_open_before
            && report.inet_before
            && report.clone_before
            && report.exec_before
    );
    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    for field in ["Uid:", "Gid:"] {
        assert!(
            status_value(&status, field)
                .split_whitespace()
                .all(|id| id == "1000")
        );
        assert_eq!(status_value(&status, field).split_whitespace().count(), 4);
    }
    assert!(status_value(&status, "Groups:").is_empty());
    assert_eq!(status_value(&status, "NoNewPrivs:"), "1");
    assert_eq!(
        u64::from_str_radix(status_value(&status, "CapEff:"), 16).unwrap(),
        0
    );
    assert_eq!(status_value(&status, "Seccomp:"), "2");
    assert!(
        status_value(&status, "Seccomp_filters:")
            .parse::<u32>()
            .unwrap()
            > report.prior_filters,
        "actual production filter added beyond container baseline"
    );
    if architecture_case {
        assert_eq!(report.compat_before, Some(true));
    } else {
        for result in [
            report.open_errno,
            report.socket_errno,
            report.clone_errno,
            report.exec_errno,
        ] {
            assert_eq!(result, Some(libc::EPERM));
        }
        assert!(report.unexpected_clone_pid.is_none());
    }
    write_all(parent_write.as_raw_fd(), &[1]).unwrap();
    let waited = owner.wait();
    if architecture_case {
        assert!(libc::WIFSIGNALED(waited));
        assert_eq!(libc::WTERMSIG(waited), libc::SIGSYS);
    } else {
        assert!(libc::WIFEXITED(waited));
        assert_eq!(libc::WEXITSTATUS(waited), 0);
    }
    eprintln!(
        "actual confinement control: PID={pid}, direct_wait=1, filter_added=1, UID/GID1000, groups=[], CapEff=0, NNP=1, architecture_case={architecture_case}"
    );
    // UID1000-owned0700 synthetic inputs persist only in the disposable test container.
}
#[test]
fn actual_uid1000_content_open_network_clone_exec_confinement() {
    exercise(false);
}
#[cfg(target_arch = "x86_64")]
#[test]
fn actual_x86_64_filter_refuses_i386_syscall_architecture() {
    exercise(true);
}
