//! Fixed, credential-free disposable-guest fixture checks. Never production.
#![forbid(unsafe_code)]

use std::{fs, io, io::Read, os::unix::fs::MetadataExt};

pub const INSTALL: &str = "/run/pokelinux-auth-containment-test";
pub const PROBE: &str = "/run/pokelinux-auth-containment-test/helper-profile-probe";
pub const SERVICE: &str = "/sys/fs/cgroup/system.slice/pokelinux-auth-containment-test.service";
pub const WORKLOAD: &str = "/system.slice/pokelinux-auth-containment-test.service/workload";
pub const ESCAPED: &str = "/system.slice/pokelinux-auth-containment-test.service/escaped";
pub const DESTINATION: &str =
    "/sys/fs/cgroup/system.slice/pokelinux-auth-containment-test.service/escaped/cgroup.procs";

pub fn require(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}

pub fn bounded(path: &str) -> io::Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(16385).read_to_end(&mut bytes)?;
    require(bytes.len() <= 16384)?;
    String::from_utf8(bytes).map_err(|_| io::ErrorKind::InvalidData.into())
}

/// No diagnostic includes the contents of an inspected file.
pub fn root_owned(path: &str, mode: u32, directory: bool) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    require(
        m.uid() == 0
            && m.gid() == 0
            && m.mode() & 0o7777 == mode
            && if directory { m.is_dir() } else { m.is_file() },
    )
}

pub fn installation() -> io::Result<()> {
    require(std::env::args_os().count() == 1)?;
    root_owned("/run", 0o755, true)?;
    root_owned(INSTALL, 0o755, true)?;
    root_owned(&format!("{INSTALL}/armed"), 0o400, false)?;
    require(bounded(&format!("{INSTALL}/armed"))? == "PKHC-VM-ONLY1\n")?;
    root_owned(PROBE, 0o6755, false)
}

pub fn output() -> io::Result<()> {
    use rustix::fs::{OFlags, fcntl_getfl, fstat};
    let fd = std::io::stdout();
    let stat = fstat(&fd)?;
    let flags = fcntl_getfl(&fd)?;
    require(
        stat.st_mode & libc::S_IFMT == libc::S_IFIFO
            && flags.contains(OFlags::NONBLOCK)
            && flags & OFlags::ACCMODE == OFlags::WRONLY,
    )
}

pub fn membership(expected: &str) -> io::Result<()> {
    require(bounded("/proc/self/cgroup")? == format!("0::{expected}\n"))
}

pub fn scope() -> io::Result<()> {
    // All paths are compiled constants. The root driver must independently bind
    // this mount/service and prohibit concurrent mount or ancestry replacement.
    require(
        rustix::fs::statfs("/sys/fs/cgroup")?.f_type as u64 == libc::CGROUP2_SUPER_MAGIC as u64,
    )?;
    root_owned("/sys/fs/cgroup/system.slice", 0o755, true)?;
    root_owned(SERVICE, 0o755, true)?;
    for leaf in ["owner", "workload", "escaped"] {
        root_owned(&format!("{SERVICE}/{leaf}"), 0o755, true)?;
        root_owned(&format!("{SERVICE}/{leaf}/cgroup.procs"), 0o644, false)?;
    }
    root_owned(&format!("{SERVICE}/cgroup.procs"), 0o644, false)
}

#[cfg(not(feature = "contained"))]
fn field<'a>(status: &'a str, key: &str) -> io::Result<&'a str> {
    let mut matches = status.lines().filter_map(|line| line.strip_prefix(key));
    let value = matches.next().ok_or(io::ErrorKind::InvalidData)?;
    require(matches.next().is_none())?;
    Ok(value.trim())
}

#[cfg(not(feature = "contained"))]
pub fn profile() -> io::Result<()> {
    use std::path::Path;
    let status = bounded("/proc/self/status")?;
    for key in ["Uid:", "Gid:"] {
        let ids: Vec<_> = field(&status, key)?.split_whitespace().collect();
        require(ids == ["1000", "0", "0", "0"])?;
    }
    require(field(&status, "Groups:")?.is_empty())?;
    for (key, expected) in [
        ("CapInh:", 0),
        ("CapPrm:", 1 << 29),
        ("CapEff:", 1 << 29),
        ("CapBnd:", 1 << 29),
        ("CapAmb:", 0),
    ] {
        let mask = u64::from_str_radix(field(&status, key)?, 16)
            .map_err(|_| io::ErrorKind::InvalidData)?;
        require(mask == expected)?;
    }
    require(
        field(&status, "NoNewPrivs:")? == "0"
            && field(&status, "Threads:")? == "1"
            && field(&status, "TracerPid:")? == "0",
    )?;
    let core = rustix::process::getrlimit(rustix::process::Resource::Core);
    require(core.current == Some(0) && core.maximum == Some(0))?;
    // Verify the protected file was really exec'd, rather than a process merely
    // imitating the expected metadata. Root still independently qualifies it.
    require(fs::read_link("/proc/self/exe")? == Path::new(PROBE))?;
    let installed = fs::metadata(PROBE)?;
    let running = fs::metadata("/proc/self/exe")?;
    require(installed.dev() == running.dev() && installed.ino() == running.ino())
}

/// Direct syscall subset; the outside retained-PID observer must independently
/// qualify saved/fs IDs, threads, tracer, executable and membership. The private
/// root deliberately does not provide fabricated status or cgroup files.
#[cfg(feature = "contained")]
pub fn profile() -> io::Result<()> {
    use rustix::{process, thread};
    require(process::getuid().as_raw() == 1000 && process::geteuid().as_raw() == 0)?;
    require(process::getgid().as_raw() == 1000 && process::getegid().as_raw() == 0)?;
    require(process::getgroups()?.is_empty() && !thread::no_new_privs()?)?;
    let sets = thread::capabilities(None)?;
    require(
        sets.effective.bits() == 1 << 29
            && sets.permitted.bits() == 1 << 29
            && sets.inheritable.is_empty(),
    )?;
    for cap in 0u32..=40 {
        let capability = thread::CapabilitySet::from_bits_retain(1u64 << cap);
        require(thread::capability_is_in_bounding_set(capability)? == (cap == 29))?;
        require(!thread::capability_is_in_ambient_set(capability)?)?;
    }
    let core = process::getrlimit(process::Resource::Core);
    require(core.current == Some(0) && core.maximum == Some(0))
}
