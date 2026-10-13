//! Experimental public-only owner. No vendor runtime or registration admission.
//! Every wait is WNOHANG; the retained pidfd/direct child survive all fencing.
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs, io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::{Path, PathBuf},
    time::Duration,
};

const UID: u32 = 12020;
const MEMORY: u64 = 268435456;
const TASKS: u64 = 16;
const PTRACE_GET_SYSCALL_INFO: libc::c_uint = 0x420e;
const SECCOMP_SET_MODE_FILTER: libc::c_uint = 1;
const AUDIT_ARCH: u32 = if cfg!(target_arch = "aarch64") {
    0xc00000b7
} else {
    0xc000003e
};
#[repr(C)]
struct SyscallInfo {
    op: u8,
    pad: [u8; 3],
    arch: u32,
    ip: u64,
    sp: u64,
    data: [u64; 8],
}
fn now() -> u64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
        0
    );
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn cv(s: impl AsRef<[u8]>) -> CString {
    CString::new(s.as_ref()).unwrap()
}
fn check(r: libc::c_int) -> io::Result<()> {
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn fd(r: libc::c_int) -> io::Result<OwnedFd> {
    check(r)?;
    Ok(unsafe { OwnedFd::from_raw_fd(r) })
}
fn ptrace(request: libc::c_uint, pid: i32, address: usize, data: usize) -> io::Result<()> {
    check(unsafe {
        libc::ptrace(
            request,
            pid,
            address as *mut libc::c_void,
            data as *mut libc::c_void,
        )
    } as i32)
}
fn mount(
    src: Option<&str>,
    dst: &Path,
    kind: Option<&str>,
    flags: libc::c_ulong,
    data: Option<&str>,
) -> io::Result<()> {
    let s = src.map(cv);
    let d = cv(dst.as_os_str().as_encoded_bytes());
    let k = kind.map(cv);
    let v = data.map(cv);
    check(unsafe {
        libc::mount(
            s.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            d.as_ptr(),
            k.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            flags,
            v.as_ref().map_or(std::ptr::null(), |p| p.as_ptr().cast()),
        )
    })
}
fn rlimit(resource: libc::__rlimit_resource_t, value: u64) -> io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    check(unsafe { libc::setrlimit(resource, &limit) })
}
fn socketpair() -> io::Result<[OwnedFd; 2]> {
    let mut f = [-1; 2];
    check(unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
            f.as_mut_ptr(),
        )
    })?;
    Ok([fd(f[0])?, fd(f[1])?])
}
fn pipe() -> io::Result<[OwnedFd; 2]> {
    let mut f = [-1; 2];
    check(unsafe { libc::pipe2(f.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) })?;
    Ok([fd(f[0])?, fd(f[1])?])
}
fn addr(path: &Path) -> libc::sockaddr_un {
    let mut a: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    a.sun_family = libc::AF_UNIX as u16;
    let bytes = path.as_os_str().as_encoded_bytes();
    assert!(bytes.len() < a.sun_path.len());
    for (i, b) in bytes.iter().enumerate() {
        a.sun_path[i] = *b as libc::c_char;
    }
    a
}
fn listener(path: &Path) -> io::Result<OwnedFd> {
    let s = fd(unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    })?;
    let a = addr(path);
    check(unsafe {
        libc::bind(
            s.as_raw_fd(),
            (&a as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&a) as u32,
        )
    })?;
    check(unsafe { libc::listen(s.as_raw_fd(), 8) })?;
    Ok(s)
}

fn filter() -> io::Result<()> {
    // seccomp_data offsets: nr0, arch4, args16; kernel authority, never string inspection.
    let mut f: Vec<libc::sock_filter> = Vec::new();
    let statement = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |code, k, jt, jf| libc::sock_filter { code, jt, jf, k };
    const LD: u16 = 0x20;
    const JEQ: u16 = 0x15;
    const JSET: u16 = 0x45;
    const RET: u16 = 0x06;
    const ALLOW: u32 = 0x7fff0000;
    const DENY: u32 = 0x00050000 | libc::EPERM as u32;
    const KILL: u32 = 0x80000000;
    f.extend([
        statement(LD, 4),
        jump(JEQ, AUDIT_ARCH, 1, 0),
        statement(RET, KILL),
        statement(LD, 0),
    ]);
    let denied = [
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_unshare,
        libc::SYS_setns,
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_bpf,
        libc::SYS_io_uring_setup,
        libc::SYS_io_uring_enter,
        libc::SYS_io_uring_register,
        libc::SYS_openat2,
        libc::SYS_memfd_create,
        libc::SYS_connect,
        libc::SYS_bind,
        libc::SYS_listen,
        libc::SYS_accept,
        libc::SYS_accept4,
        libc::SYS_sendto,
        libc::SYS_sendmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvmsg,
        libc::SYS_recvmmsg,
        libc::SYS_mkdirat,
        libc::SYS_mknodat,
        libc::SYS_unlinkat,
        libc::SYS_linkat,
        libc::SYS_symlinkat,
        libc::SYS_renameat,
        libc::SYS_renameat2,
        libc::SYS_fchmod,
        libc::SYS_fchmodat,
        libc::SYS_fchown,
        libc::SYS_fchownat,
        libc::SYS_truncate,
        libc::SYS_ftruncate,
        libc::SYS_fallocate,
        libc::SYS_utimensat,
        libc::SYS_setxattr,
        libc::SYS_lsetxattr,
        libc::SYS_fsetxattr,
        libc::SYS_removexattr,
        libc::SYS_lremovexattr,
        libc::SYS_fremovexattr,
        libc::SYS_reboot,
        libc::SYS_kexec_load,
        libc::SYS_setuid,
        libc::SYS_setgid,
        libc::SYS_setresuid,
        libc::SYS_setresgid,
        libc::SYS_setreuid,
        libc::SYS_setregid,
        libc::SYS_setgroups,
        libc::SYS_capset,
        libc::SYS_prctl,
        libc::SYS_seccomp,
    ];
    for n in denied {
        f.extend([jump(JEQ, n as u32, 0, 1), statement(RET, DENY)]);
    }
    #[cfg(target_arch = "x86_64")]
    for n in [
        libc::SYS_fork,
        libc::SYS_vfork,
        libc::SYS_open,
        libc::SYS_creat,
        libc::SYS_unlink,
        libc::SYS_mkdir,
        libc::SYS_mknod,
        libc::SYS_rename,
        libc::SYS_link,
        libc::SYS_symlink,
        libc::SYS_chmod,
        libc::SYS_chown,
        libc::SYS_lchown,
        libc::SYS_utime,
        libc::SYS_utimes,
    ] {
        f.extend([jump(JEQ, n as u32, 0, 1), statement(RET, DENY)]);
    }
    // clone3's pointed-to flags are not inspected. ENOSYS permits libc's bounded clone-thread fallback.
    f.extend([
        jump(JEQ, libc::SYS_clone3 as u32, 0, 1),
        statement(RET, 0x00050000 | libc::ENOSYS as u32),
    ]);
    for n in [libc::SYS_socket, libc::SYS_socketpair] {
        f.extend([
            jump(JEQ, n as u32, 0, 4),
            statement(LD, 16),
            jump(JEQ, libc::AF_UNIX as u32, 1, 0),
            statement(RET, DENY),
            statement(RET, ALLOW),
        ]);
    }
    let required = (libc::CLONE_VM
        | libc::CLONE_FS
        | libc::CLONE_FILES
        | libc::CLONE_SIGHAND
        | libc::CLONE_THREAD
        | libc::CLONE_SYSVSEM) as u32;
    let allowed = required
        | (libc::CLONE_SETTLS
            | libc::CLONE_PARENT_SETTID
            | libc::CLONE_CHILD_CLEARTID
            | libc::CLONE_CHILD_SETTID) as u32;
    f.extend([
        jump(JEQ, libc::SYS_clone as u32, 0, 7),
        statement(LD, 16),
        jump(JSET, !allowed, 0, 1),
        statement(RET, DENY),
        statement(0x54, required),
        jump(JEQ, required, 1, 0),
        statement(RET, DENY),
        statement(RET, ALLOW),
    ]);
    // Write-capable open flags fail even before the readonly namespace's own decision.
    let write_flags = (libc::O_WRONLY
        | libc::O_RDWR
        | libc::O_CREAT
        | libc::O_TRUNC
        | libc::O_APPEND
        | (libc::O_TMPFILE & !libc::O_DIRECTORY)) as u32;
    f.extend([
        jump(JEQ, libc::SYS_openat as u32, 0, 4),
        statement(LD, 32),
        jump(JSET, write_flags, 0, 1),
        statement(RET, DENY),
        statement(RET, ALLOW),
    ]);
    for n in [libc::SYS_write, libc::SYS_writev] {
        f.extend([
            jump(JEQ, n as u32, 0, 6),
            statement(LD, 16),
            jump(JEQ, 1, 3, 0),
            jump(JEQ, 2, 2, 0),
            jump(JEQ, 3, 1, 0),
            statement(RET, DENY),
            statement(RET, ALLOW),
        ]);
    }
    f.push(statement(RET, ALLOW));
    let program = libc::sock_fprog {
        len: f.len() as u16,
        filter: f.as_mut_ptr(),
    };
    check(unsafe { libc::syscall(libc::SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &program) } as i32)
}

// Keep each inherited setup handle explicit in the audited pre-exec boundary.
#[allow(clippy::too_many_arguments)]
fn child_setup(
    root: &Path,
    parent_pidfd: i32,
    release: i32,
    checkpoint: i32,
    stdout: i32,
    stderr: i32,
    mode: &CString,
    nonce: &CString,
    until: &CString,
) -> io::Result<()> {
    check(unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) })?;
    // Setup hold is child-only; parent retains pidfd and never waits here.
    let mut byte = [0u8];
    loop {
        let r = unsafe { libc::read(release, byte.as_mut_ptr().cast(), 1) };
        if r == 1 {
            break;
        }
        if r < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN) {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        return Err(io::Error::last_os_error());
    }
    mount(
        None,
        Path::new("/"),
        None,
        libc::MS_REC | libc::MS_PRIVATE,
        None,
    )?;
    mount(
        Some(root.to_str().unwrap()),
        root,
        None,
        libc::MS_BIND | libc::MS_REC,
        None,
    )?;
    mount(
        Some("proc"),
        &root.join("proc"),
        Some("proc"),
        libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        None,
    )?;
    mount(
        None,
        &root.join("proc"),
        None,
        libc::MS_REMOUNT | libc::MS_RDONLY | libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        None,
    )?;
    mount(
        Some("tmpfs"),
        &root.join("dev"),
        Some("tmpfs"),
        libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        Some("size=4096,mode=0555"),
    )?;
    fs::create_dir(root.join("dev/__properties__"))?;
    mount(
        None,
        &root.join("dev"),
        None,
        libc::MS_REMOUNT | libc::MS_RDONLY | libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        None,
    )?;
    mount(
        None,
        root,
        None,
        libc::MS_REMOUNT | libc::MS_BIND | libc::MS_RDONLY | libc::MS_NOSUID | libc::MS_NODEV,
        None,
    )?;
    check(unsafe { libc::chdir(cv(root.as_os_str().as_encoded_bytes()).as_ptr()) })?;
    check(unsafe { libc::chroot(cv(".").as_ptr()) })?;
    check(unsafe { libc::chdir(cv("/").as_ptr()) })?;
    unsafe { libc::umask(0o077) };
    rlimit(libc::RLIMIT_AS, MEMORY)?;
    rlimit(libc::RLIMIT_NOFILE, 64)?;
    rlimit(libc::RLIMIT_NPROC, TASKS)?;
    rlimit(libc::RLIMIT_CORE, 0)?;
    // Clear capability bounding/ambient before final UID transition; setuid clears all active sets.
    check(unsafe {
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        )
    })?;
    for cap in 0..=40 {
        check(unsafe { libc::prctl(libc::PR_CAPBSET_DROP, cap, 0, 0, 0) })?;
    }
    check(unsafe { libc::setgroups(0, std::ptr::null()) })?;
    check(unsafe { libc::setresgid(UID, UID, UID) })?;
    check(unsafe { libc::setresuid(UID, UID, UID) })?;
    // setresuid resets PDEATHSIG: establish it only after final credentials and then read it back.
    check(unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) })?;
    let mut death = 0;
    check(unsafe { libc::prctl(libc::PR_GET_PDEATHSIG, &mut death) })?;
    if death != libc::SIGKILL {
        return Err(io::Error::other("parent-death readback"));
    }
    let mut parent = libc::pollfd {
        fd: parent_pidfd,
        events: libc::POLLIN,
        revents: 0,
    };
    check(unsafe { libc::poll(&mut parent, 1, 0) })?;
    if parent.revents != 0 {
        return Err(io::Error::other("parent died across credentials"));
    }
    check(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) })?;
    if unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) } != 1 {
        return Err(io::Error::other("no-new-privileges readback"));
    }
    // Closing original duplicates occurs after dup2. Only the three controlled FDs survive.
    check(unsafe { libc::dup2(stdout, 1) })?;
    check(unsafe { libc::dup2(stderr, 2) })?;
    check(unsafe { libc::dup2(checkpoint, 3) })?;
    unsafe { libc::close(0) };
    check(unsafe { libc::syscall(libc::SYS_close_range, 4u32, u32::MAX, 0u32) } as i32)?;
    check(unsafe {
        libc::ptrace(
            libc::PTRACE_TRACEME,
            0,
            std::ptr::null_mut::<libc::c_void>(),
            std::ptr::null_mut::<libc::c_void>(),
        )
    } as i32)?;
    filter()?;
    // Signal self using tgkill; private PID namespace PID1 is protected from ordinary self kill.
    check(unsafe {
        libc::syscall(
            libc::SYS_tgkill,
            libc::getpid(),
            libc::gettid(),
            libc::SIGSTOP,
        )
    } as i32)?;
    let executable = cv("/bin/radio-harmless-child");
    let arg0 = cv("/experiment/bin/radio-harmless-child");
    let args = [
        arg0.as_ptr(),
        mode.as_ptr(),
        nonce.as_ptr(),
        until.as_ptr(),
        std::ptr::null(),
    ];
    let lang = cv("LANG=C");
    let lc = cv("LC_ALL=C");
    let tz = cv("TZ=UTC");
    let env = [lang.as_ptr(), lc.as_ptr(), tz.as_ptr(), std::ptr::null()];
    check(unsafe { libc::execve(executable.as_ptr(), args.as_ptr(), env.as_ptr()) })
}

struct Owner {
    pid: i32,
    pidfd: OwnedFd,
    tasks: BTreeSet<i32>,
    root_reaped: bool,
    waits_empty: bool,
    until: u64,
    cleanup_until: Option<u64>,
    fenced: Option<String>,
    setup_verified: bool,
    execs: u64,
    trace_records: u64,
    recent_syscalls: std::collections::VecDeque<Value>,
    checkpoints: Vec<Value>,
    events: Vec<Value>,
    nonce: String,
    stdout_bytes: u64,
    stderr_bytes: u64,
    cgroup: PathBuf,
}
impl Owner {
    fn kill(&mut self, reason: &str) {
        if self.fenced.is_none() {
            self.fenced = Some(reason.into());
            self.cleanup_until = Some(now() + 2_000_000_000);
            self.events
                .push(json!({"event":"fenced","reason":reason,"boottime_ns":now()}));
        }
        if !self.root_reaped {
            let r = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pidfd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            if r < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                self.events.push(json!({"event":"pidfd-kill-error","errno":io::Error::last_os_error().raw_os_error()}));
            }
        }
    }
    fn occupied(&self) -> bool {
        !self.root_reaped || !self.tasks.is_empty() || !self.waits_empty
    }
    fn status(&self) -> Value {
        json!({"occupied":self.occupied(),"fenced":self.fenced,"setup_verified":self.setup_verified,"child_pid":self.pid,"until_boottime_ns":self.until,"now_boottime_ns":now(),"execs":self.execs,"trace_records":self.trace_records,"checkpoints":self.checkpoints,"task_count":self.tasks.len(),"registration_admission":false,"vendor_cleanup":false})
    }
    fn event(&mut self, value: Value) {
        if self.events.len() >= 4096 {
            self.kill("event-overflow");
        } else {
            self.events.push(value);
        }
    }
    fn setup_readback(&mut self) -> io::Result<Value> {
        let p = PathBuf::from(format!("/proc/{}", self.pid));
        let status = fs::read_to_string(p.join("status"))?;
        let field = |key: &str| {
            status
                .lines()
                .find_map(|l| l.strip_prefix(key))
                .unwrap_or("")
                .trim()
                .to_owned()
        };
        for key in ["Uid:", "Gid:"] {
            if field(key).split_whitespace().collect::<Vec<_>>() != ["12020"; 4] {
                return Err(io::Error::other(format!("credential mismatch {key}")));
            }
        }
        for key in ["CapInh:", "CapPrm:", "CapEff:", "CapBnd:", "CapAmb:"] {
            if field(key) != "0000000000000000" {
                return Err(io::Error::other(format!("capability mismatch {key}")));
            }
        }
        if !field("Groups:").is_empty()
            || field("NoNewPrivs:") != "1"
            || field("Seccomp:") != "2"
            || field("NSpid:").split_whitespace().last() != Some("1")
        {
            return Err(io::Error::other(
                "groups, NNP, seccomp or private PID readback",
            ));
        }
        let mut namespaces = serde_json::Map::new();
        for kind in ["mnt", "pid", "net", "ipc"] {
            let child = fs::read_link(p.join("ns").join(kind))?;
            let parent = fs::read_link(Path::new("/proc/self/ns").join(kind))?;
            if child == parent {
                return Err(io::Error::other("namespace not isolated"));
            }
            namespaces.insert(kind.into(), json!({"parent":parent,"child":child}));
        }
        let mut fds = fs::read_dir(p.join("fd"))?
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        fds.sort();
        if fds != [1, 2, 3] {
            return Err(io::Error::other(format!("inherited FD mismatch {fds:?}")));
        }
        let mounts = fs::read_to_string(p.join("mountinfo"))?;
        for point in ["/", "/proc", "/dev"] {
            let row = mounts
                .lines()
                .find(|line| line.split_whitespace().nth(4) == Some(point))
                .ok_or_else(|| io::Error::other("required mount absent"))?;
            let options = row.split_whitespace().nth(5).unwrap();
            if !options.split(',').any(|v| v == "ro") {
                return Err(io::Error::other("mount not readonly"));
            }
            if point != "/"
                && !["nosuid", "nodev", "noexec"]
                    .iter()
                    .all(|wanted| options.split(',').any(|v| v == *wanted))
            {
                return Err(io::Error::other("proc/dev mount flags"));
            }
        }
        for (name, want) in [
            ("memory.max", "268435456"),
            ("pids.max", "16"),
            ("memory.swap.max", "0"),
        ] {
            if fs::read_to_string(self.cgroup.join(name))?.trim() != want {
                return Err(io::Error::other("cgroup readback"));
            }
        }
        let limits = fs::read_to_string(p.join("limits"))?;
        for (heading, value) in [
            ("Max address space", "268435456"),
            ("Max open files", "64"),
            ("Max processes", "16"),
            ("Max core file size", "0"),
        ] {
            let row = limits
                .lines()
                .find(|l| l.starts_with(heading))
                .ok_or_else(|| io::Error::other("rlimit absent"))?;
            if !row
                .strip_prefix(heading)
                .unwrap()
                .split_whitespace()
                .take(2)
                .all(|v| v == value)
            {
                return Err(io::Error::other("rlimit readback"));
            }
        }
        self.setup_verified = true;
        Ok(
            json!({"event":"containment-readback","boottime_ns":now(),"status":status,"namespaces":namespaces,"fds":fds,"mountinfo":mounts,"limits":limits,"pdeathsig_after_final_credentials":9,"cgroup_memory_max":MEMORY,"cgroup_pids_max":TASKS}),
        )
    }
    fn drain_waits(&mut self) {
        for _ in 0..64 {
            let mut status = 0;
            let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG | libc::__WALL) };
            if pid == 0 {
                self.waits_empty = false;
                break;
            }
            if pid < 0 {
                if io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) {
                    self.waits_empty = self.root_reaped && self.tasks.is_empty();
                } else {
                    self.kill("wait-error");
                }
                break;
            }
            self.waits_empty = false;
            if !self.tasks.contains(&pid) {
                // An unexpected traced wait is still retained until its real
                // terminal wait. Never erase uncertainty by omitting its task.
                self.tasks.insert(pid);
                self.kill("unowned-wait");
            }
            if libc::WIFEXITED(status) || libc::WIFSIGNALED(status) {
                self.tasks.remove(&pid);
                if pid == self.pid {
                    self.root_reaped = true;
                    self.event(json!({"event":"actual-root-wait","pid":pid,"status":status,"boottime_ns":now()}));
                    if self.fenced.is_none() {
                        self.kill("child-loss");
                    }
                }
                continue;
            }
            if !libc::WIFSTOPPED(status) {
                self.kill("unexpected-wait-state");
                continue;
            }
            let signal = libc::WSTOPSIG(status);
            let event = (status as u32) >> 16;
            if !self.setup_verified && pid == self.pid && signal == libc::SIGSTOP {
                match self.setup_readback() {
                    Ok(v) => self.event(v),
                    Err(e) => {
                        self.event(json!({"event":"containment-failure","error":e.to_string()}));
                        self.kill("containment-readback-failed");
                    }
                }
                let flags = libc::PTRACE_O_TRACESYSGOOD
                    | libc::PTRACE_O_TRACECLONE
                    | libc::PTRACE_O_TRACEEXEC
                    | libc::PTRACE_O_TRACEEXIT
                    | libc::PTRACE_O_EXITKILL;
                if ptrace(libc::PTRACE_SETOPTIONS, pid, 0, flags as usize).is_err() {
                    self.kill("ptrace-options-failed");
                }
            } else if event == libc::PTRACE_EVENT_CLONE as u32 {
                let mut child: libc::c_ulong = 0;
                if ptrace(
                    libc::PTRACE_GETEVENTMSG,
                    pid,
                    0,
                    &mut child as *mut _ as usize,
                )
                .is_err()
                {
                    self.kill("clone-event-failed");
                } else {
                    self.tasks.insert(child as i32);
                    self.event(json!({"event":"thread-clone","pid":child,"boottime_ns":now()}));
                    if self.tasks.len() > 16 {
                        self.kill("task-overflow");
                    }
                }
            } else if signal == libc::SIGTRAP | 0x80 {
                self.trace_records += 1;
                if self.trace_records > 131072 {
                    self.kill("trace-overflow");
                }
                let mut info: SyscallInfo = unsafe { std::mem::zeroed() };
                let count = unsafe {
                    libc::ptrace(
                        PTRACE_GET_SYSCALL_INFO,
                        pid,
                        std::mem::size_of::<SyscallInfo>() as *mut libc::c_void,
                        (&mut info as *mut SyscallInfo).cast::<libc::c_void>(),
                    )
                };
                if count < 24 || info.arch != AUDIT_ARCH {
                    self.kill("syscall-info-failed");
                }
                // Bounded syscall identities/results only; never read buffers,
                // pathname pointers, property bodies or credentials.
                if self.recent_syscalls.len() == 128 {
                    self.recent_syscalls.pop_front();
                }
                self.recent_syscalls.push_back(json!({"pid":pid,"op":info.op,"nr_or_result":info.data[0] as i64,"boottime_ns":now()}));
                if info.op == 1
                    && [libc::SYS_execve as u64, libc::SYS_execveat as u64].contains(&info.data[0])
                {
                    self.execs += 1;
                    if self.execs > 1 {
                        self.event(
                            json!({"event":"second-exec-denied","pid":pid,"boottime_ns":now()}),
                        );
                        self.kill("second-exec");
                    }
                }
            } else if signal != libc::SIGTRAP && signal != libc::SIGSTOP && self.fenced.is_none() {
                self.event(json!({"event":"signal","pid":pid,"signal":signal,"boottime_ns":now()}));
                self.kill("unexpected-signal");
            }
            let request = if self.fenced.is_some() {
                libc::PTRACE_CONT
            } else {
                libc::PTRACE_SYSCALL
            };
            let delivery = if self.fenced.is_some() {
                libc::SIGKILL
            } else {
                0
            };
            if ptrace(request, pid, 0, delivery as usize).is_err() && !self.root_reaped {
                self.kill("ptrace-resume-failed");
            }
        }
    }
    fn checkpoint(&mut self, socket: i32) {
        for _ in 0..8 {
            let mut packet = [0u8; 257];
            let mut ancillary = [0u64; 16];
            let mut iov = libc::iovec {
                iov_base: packet.as_mut_ptr().cast(),
                iov_len: packet.len(),
            };
            let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = ancillary.as_mut_ptr().cast();
            msg.msg_controllen = std::mem::size_of_val(&ancillary);
            let len = unsafe {
                libc::recvmsg(
                    socket,
                    &mut msg,
                    libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
                )
            };
            if len < 0 {
                if io::Error::last_os_error().raw_os_error() != Some(libc::EAGAIN) {
                    self.kill("checkpoint-read-error");
                }
                break;
            }
            if len == 0 {
                if self.fenced.is_none() {
                    self.kill("checkpoint-EOF");
                }
                break;
            }
            let mut credentials = None;
            let mut count = 0;
            let mut unexpected = false;
            unsafe {
                let mut c = libc::CMSG_FIRSTHDR(&msg);
                while !c.is_null() {
                    count += 1;
                    if (*c).cmsg_level == libc::SOL_SOCKET
                        && (*c).cmsg_type == libc::SCM_CREDENTIALS
                        && (*c).cmsg_len
                            == libc::CMSG_LEN(std::mem::size_of::<libc::ucred>() as u32) as usize
                    {
                        credentials = Some(std::ptr::read_unaligned(
                            libc::CMSG_DATA(c).cast::<libc::ucred>(),
                        ));
                    } else {
                        unexpected = true;
                        if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS
                        {
                            let bytes = (*c).cmsg_len.saturating_sub(libc::CMSG_LEN(0) as usize);
                            for i in 0..bytes / std::mem::size_of::<i32>() {
                                libc::close(std::ptr::read_unaligned(
                                    libc::CMSG_DATA(c).cast::<i32>().add(i),
                                ));
                            }
                        }
                    }
                    c = libc::CMSG_NXTHDR(&msg, c);
                }
            }
            let observed = now();
            let valid_creds =
                credentials.is_some_and(|c| c.pid == self.pid && c.uid == UID && c.gid == UID);
            if len > 256
                || msg.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
                || count != 1
                || unexpected
                || !valid_creds
            {
                self.kill("checkpoint-authentication-failed");
                continue;
            }
            let value = serde_json::from_slice::<Value>(&packet[..len as usize]);
            match value {
                Ok(v) if v["nonce"] == self.nonce && v["event"].as_str().is_some() => {
                    let admitted = self.fenced.is_none()
                        && observed < self.until
                        && !self.root_reaped
                        && self.setup_verified;
                    self.event(json!({"event":"authenticated-checkpoint","packet":v,"outer_pid":self.pid,"uid":UID,"gid":UID,"boottime_ns":observed,"admitted":admitted}));
                    if admitted {
                        if self.checkpoints.len() >= 16 {
                            self.kill("checkpoint-overflow");
                        } else {
                            self.checkpoints.push(v);
                        }
                    } else {
                        self.kill("late-checkpoint");
                    }
                }
                _ => self.kill("malformed-checkpoint"),
            }
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.occupied() {
            self.kill("unresolved-owner-drop");
            std::process::abort();
        }
    }
}

fn drain_output(owner: &mut Owner, fd: i32, stderr: bool) {
    for _ in 0..4 {
        let mut buf = [0u8; 1024];
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            break;
        }
        let bytes = if stderr {
            &mut owner.stderr_bytes
        } else {
            &mut owner.stdout_bytes
        };
        *bytes += n as u64;
        if *bytes > 65536 {
            owner.kill("stdio-overflow");
            break;
        }
        // This executable accepts public authored children only. Preserve bounded
        // setup diagnostics; original-runtime raw output is separately prohibited.
        owner.event(json!({"event":"public-stdio","stderr":stderr,"bytes":n,"text":String::from_utf8_lossy(&buf[..n as usize])}));
    }
}
fn run() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 5);
    let root = PathBuf::from(&args[1]);
    let cgroup = PathBuf::from(&args[2]);
    let mode = cv(&args[3]);
    let control = PathBuf::from(&args[4]);
    assert_eq!(unsafe { libc::geteuid() }, 0);
    let server = listener(&control)?;
    fs::create_dir(&cgroup)?;
    for (name, value) in [
        ("memory.max", "268435456"),
        ("memory.swap.max", "0"),
        ("pids.max", "16"),
        ("cgroup.max.depth", "0"),
        ("cgroup.max.descendants", "0"),
    ] {
        fs::write(cgroup.join(name), value)?;
    }
    let [checkpoint_parent, checkpoint_child] = socketpair()?;
    let on = 1;
    check(unsafe {
        libc::setsockopt(
            checkpoint_parent.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PASSCRED,
            (&on as *const i32).cast(),
            4,
        )
    })?;
    let [release_read, release_write] = pipe()?;
    let [output_read, output_write] = pipe()?;
    let [error_read, error_write] = pipe()?;
    let parent_pidfd =
        fd(unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0) } as i32)?;
    let mut random = [0u8; 16];
    assert_eq!(
        unsafe { libc::getrandom(random.as_mut_ptr().cast(), random.len(), 0) },
        16
    );
    let nonce = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let nonce_c = cv(&nonce);
    let until = now() + 5_000_000_000;
    let until_c = cv(until.to_string());
    let pid = unsafe {
        libc::syscall(
            libc::SYS_clone,
            libc::CLONE_NEWNS
                | libc::CLONE_NEWPID
                | libc::CLONE_NEWNET
                | libc::CLONE_NEWIPC
                | libc::SIGCHLD,
            0,
            0,
            0,
            0,
        )
    } as i32;
    if pid < 0 {
        return Err(io::Error::last_os_error());
    }
    if pid == 0 {
        let r = child_setup(
            &root,
            parent_pidfd.as_raw_fd(),
            release_read.as_raw_fd(),
            checkpoint_child.as_raw_fd(),
            output_write.as_raw_fd(),
            error_write.as_raw_fd(),
            &mode,
            &nonce_c,
            &until_c,
        );
        if let Err(error) = r {
            eprintln!("public-child-setup-failed: {error}");
        }
        unsafe { libc::_exit(125) }
    }
    // No Child is ever constructed or detached. Raw direct wait occupancy is held from clone.
    let child_pidfd = match fd(unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32) {
        Ok(f) => f,
        Err(_) => {
            unsafe { libc::kill(pid, libc::SIGKILL) };
            std::process::abort();
        }
    };
    let mut owner = Owner {
        pid,
        pidfd: child_pidfd,
        tasks: BTreeSet::from([pid]),
        root_reaped: false,
        waits_empty: false,
        until,
        cleanup_until: None,
        fenced: None,
        setup_verified: false,
        execs: 0,
        trace_records: 0,
        recent_syscalls: std::collections::VecDeque::new(),
        checkpoints: Vec::new(),
        events: Vec::new(),
        nonce,
        stdout_bytes: 0,
        stderr_bytes: 0,
        cgroup: cgroup.clone(),
    };
    drop(checkpoint_child);
    drop(output_write);
    drop(error_write);
    drop(release_read);
    drop(parent_pidfd);
    if let Err(error) = fs::write(cgroup.join("cgroup.procs"), pid.to_string()) {
        owner.event(json!({"event":"cgroup-assignment-failure","error":error.to_string()}));
        owner.kill("cgroup-assignment-failed");
    }
    let byte = [1u8];
    if unsafe { libc::write(release_write.as_raw_fd(), byte.as_ptr().cast(), 1) } != 1 {
        owner.kill("setup-release-failed");
    }
    drop(release_write);
    println!(
        "READY {}",
        json!({"child_pid":pid,"until_boottime_ns":until,"control":control})
    );
    let mut clients: Vec<(OwnedFd, u64)> = Vec::new();
    let mut finish = false;
    loop {
        if now() >= owner.until && owner.fenced.is_none() {
            owner.kill("absolute-deadline");
        }
        owner.drain_waits();
        if args[3] != "queued-late" || now() >= until + 10_000_000 {
            owner.checkpoint(checkpoint_parent.as_raw_fd());
        }
        drain_output(&mut owner, output_read.as_raw_fd(), false);
        drain_output(&mut owner, error_read.as_raw_fd(), true);
        for _ in 0..8 {
            let accepted = unsafe {
                libc::accept4(
                    server.as_raw_fd(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                )
            };
            if accepted < 0 {
                break;
            }
            if clients.len() >= 8 {
                unsafe { libc::close(accepted) };
                owner.kill("control-overflow");
                break;
            }
            clients.push((fd(accepted)?, now() + 250_000_000));
        }
        let mut index = 0;
        while index < clients.len() {
            let (f, until) = &clients[index];
            let mut packet = [0u8; 257];
            let n = unsafe {
                libc::recv(
                    f.as_raw_fd(),
                    packet.as_mut_ptr().cast(),
                    packet.len(),
                    libc::MSG_DONTWAIT,
                )
            };
            if n < 0 && now() < *until {
                index += 1;
                continue;
            }
            let mut creds: libc::ucred = unsafe { std::mem::zeroed() };
            let mut size = std::mem::size_of::<libc::ucred>() as u32;
            let auth = unsafe {
                libc::getsockopt(
                    f.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PEERCRED,
                    (&mut creds as *mut libc::ucred).cast(),
                    &mut size,
                )
            } == 0
                && creds.uid == 0;
            let request = if n > 0 && n <= 256 {
                std::str::from_utf8(&packet[..n as usize]).unwrap_or("")
            } else {
                ""
            };
            let answer = if !auth {
                json!({"error":"denied"})
            } else {
                match request {
                    "status" => owner.status(),
                    "stop" => {
                        owner.kill("stop");
                        json!({"ack":"fenced","occupied":owner.occupied(),"registration_admission":false})
                    }
                    "finish" if !owner.occupied() => {
                        finish = true;
                        owner.status()
                    }
                    _ => json!({"error":"invalid-or-pending"}),
                }
            };
            let response = serde_json::to_vec(&answer).unwrap();
            if response.len() > 8192
                || unsafe {
                    libc::send(
                        f.as_raw_fd(),
                        response.as_ptr().cast(),
                        response.len(),
                        libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
                    )
                } != response.len() as isize
            {
                owner.kill("control-response-failed");
            }
            clients.swap_remove(index);
        }
        if finish {
            break;
        }
        if owner.occupied() && owner.cleanup_until.is_some_and(|u| now() >= u) {
            owner.event(json!({"event":"cleanup-deadline-unresolved","boottime_ns":now()}));
            println!("UNRESOLVED {}", owner.status());
            owner.cleanup_until = Some(u64::MAX);
        }
        // Bounded 1ms loop pause; no blocking SDK, wait, trace read or child Drop.
        std::thread::sleep(Duration::from_millis(1));
    }
    let count = fs::read_to_string(cgroup.join("pids.current"))?;
    if count.trim() != "0" {
        return Err(io::Error::other(
            "actual waits completed but cgroup not empty",
        ));
    }
    println!(
        "RESULT {}",
        json!({"status":owner.status(),"events":owner.events,"recent_syscalls":owner.recent_syscalls,"actual_root_wait":owner.root_reaped,"all_traced_tasks_waited":owner.tasks.is_empty()&&owner.waits_empty,"cgroup_pids_current":count.trim(),"stdout_bytes":owner.stdout_bytes,"stderr_bytes":owner.stderr_bytes,"registration_admission":false,"vendor_cleanup":false})
    );
    fs::remove_dir(&cgroup)?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("public-owner-error: {e}");
        std::process::exit(2);
    }
}
