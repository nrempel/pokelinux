use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, Mode, OFlags, ResolveFlags, SeekFrom},
};
use std::fs as stdfs;

const CGROUP2_MAGIC: u64 = 0x6367_7270;
const MAX_CONTROL_BYTES: usize = 4096;
const RESOLVE: ResolveFlags = ResolveFlags::BENEATH
    .union(ResolveFlags::NO_SYMLINKS)
    .union(ResolveFlags::NO_MAGICLINKS);

pub(crate) struct Leaf {
    // Keep both anchored directories and controls for the entire occupied slot.
    _delegated: OwnedFd,
    directory: OwnedFd,
    identity: (u64, u64),
    kill: OwnedFd,
    events: OwnedFd,
    procs: OwnedFd,
    pub(crate) membership: String,
}

fn denied() -> std::io::Error {
    std::io::ErrorKind::PermissionDenied.into()
}

fn protected(fd: &impl AsFd, directory: bool) -> std::io::Result<()> {
    let metadata = fs::fstat(fd)?;
    let kind = fs::FileType::from_raw_mode(metadata.st_mode);
    if metadata.st_uid != 0
        || metadata.st_mode & 0o022 != 0
        || (directory && kind != fs::FileType::Directory)
        || (!directory && kind != fs::FileType::RegularFile)
        || fs::fstatfs(fd)?.f_type as u64 != CGROUP2_MAGIC
    {
        return Err(denied());
    }
    Ok(())
}

fn control(directory: &OwnedFd, name: &str, write: bool) -> std::io::Result<OwnedFd> {
    let flags = if write {
        OFlags::WRONLY
    } else {
        OFlags::RDONLY
    };
    let fd = fs::openat2(
        directory,
        name,
        flags | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
        RESOLVE,
    )?;
    protected(&fd, false)?;
    Ok(fd)
}

fn read(fd: &OwnedFd) -> std::io::Result<String> {
    fs::seek(fd, SeekFrom::Start(0))?;
    let mut bytes = [0; MAX_CONTROL_BYTES + 1];
    let count = rustix::io::read(fd, &mut bytes)?;
    if count > MAX_CONTROL_BYTES {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    String::from_utf8(bytes[..count].to_vec()).map_err(|_| std::io::ErrorKind::InvalidData.into())
}

pub(crate) fn unified_membership(pid: u32) -> std::io::Result<String> {
    let text = stdfs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    // The private fixture selects a single unified hierarchy, never a hybrid
    // ambiguity or an inferred filesystem path from a caller's request.
    let mut lines = text.lines();
    let value = lines
        .next()
        .and_then(|line| line.strip_prefix("0::"))
        .filter(|path| path.starts_with('/') && !path.contains(".."))
        .ok_or(std::io::ErrorKind::InvalidData)?;
    if lines.next().is_some() {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(value.into())
}

impl Leaf {
    pub(crate) fn open_isolated() -> std::io::Result<Self> {
        let member = unified_membership(std::process::id())?;
        let delegated_path = member
            .strip_suffix("/owner")
            .filter(|path| path.ends_with("/pokelinux-auth-scope-test.service"))
            .ok_or_else(denied)?;
        let mount = fs::open(
            "/sys/fs/cgroup",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )?;
        protected(&mount, true)?;
        // Validate each root-owned ancestor through the anchored mount. The
        // inherited descriptors are CLOEXEC and never passed to the workload.
        let mut current = mount;
        for component in delegated_path.trim_start_matches('/').split('/') {
            if component.is_empty()
                || !component
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.:@".contains(&b))
            {
                return Err(denied());
            }
            current = fs::openat2(
                &current,
                component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
                RESOLVE,
            )?;
            protected(&current, true)?;
        }
        let delegated = current;
        if !read(&control(&delegated, "cgroup.procs", false)?)?
            .trim()
            .is_empty()
        {
            return Err(denied());
        }
        let directory = fs::openat2(
            &delegated,
            "workload",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            RESOLVE,
        )?;
        protected(&directory, true)?;
        if read(&control(&directory, "cgroup.type", false)?)?.trim() != "domain" {
            return Err(denied());
        }
        let metadata = fs::fstat(&directory)?;
        let leaf = Self {
            kill: control(&directory, "cgroup.kill", true)?,
            events: control(&directory, "cgroup.events", false)?,
            procs: control(&directory, "cgroup.procs", true)?,
            directory,
            identity: (metadata.st_dev, metadata.st_ino),
            _delegated: delegated,
            membership: format!("{delegated_path}/workload"),
        };
        if leaf.populated()? || !leaf.processes()?.is_empty() {
            return Err(denied());
        }
        Ok(leaf)
    }

    fn check_identity(&self) -> std::io::Result<()> {
        let value = fs::fstat(&self.directory)?;
        if (value.st_dev, value.st_ino) != self.identity {
            return Err(denied());
        }
        Ok(())
    }

    pub(crate) fn admit(&self, pid: u32) -> std::io::Result<()> {
        self.check_identity()?;
        let value = format!("{pid}\n");
        if rustix::io::write(&self.procs, value.as_bytes())? != value.len() {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        if unified_membership(pid)? != self.membership || self.processes()? != [pid] {
            return Err(denied());
        }
        Ok(())
    }

    pub(crate) fn populated(&self) -> std::io::Result<bool> {
        self.check_identity()?;
        parse_populated(&read(&self.events)?)
    }

    pub(crate) fn processes(&self) -> std::io::Result<Vec<u32>> {
        let fd = control(&self.directory, "cgroup.procs", false)?;
        read(&fd)?
            .lines()
            .map(|line| {
                line.parse()
                    .map_err(|_| std::io::ErrorKind::InvalidData.into())
            })
            .collect()
    }

    /// Terminate the entire anchored workload, including session-detached
    /// descendants. Completion is only the write acknowledgement: actual waits,
    /// EOF and populated0 must still close the independently owned scope.
    pub(crate) fn kill_tree(&self) -> std::io::Result<()> {
        self.check_identity()?;
        if rustix::io::write(&self.kill, b"1\n")? != 2 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        Ok(())
    }
}

fn parse_populated(text: &str) -> std::io::Result<bool> {
    let mut value = None;
    for line in text.lines() {
        let (key, raw) = line
            .split_once(' ')
            .ok_or(std::io::ErrorKind::InvalidData)?;
        if key == "populated" {
            if value.is_some() {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            value = match raw {
                "0" => Some(false),
                "1" => Some(true),
                _ => return Err(std::io::ErrorKind::InvalidData.into()),
            };
        }
    }
    value.ok_or_else(|| std::io::ErrorKind::InvalidData.into())
}

#[cfg(test)]
mod tests {
    use super::parse_populated;

    #[test]
    fn populated_is_exact_and_unique_even_with_other_kernel_fields() {
        assert!(!parse_populated("populated 0\nfrozen 0\n").unwrap());
        assert!(parse_populated("frozen 0\npopulated 1\n").unwrap());
        for invalid in [
            "",
            "populated 0\npopulated 1\n",
            "populated 2\n",
            "populated 00\n",
        ] {
            assert!(parse_populated(invalid).is_err());
        }
    }
}
