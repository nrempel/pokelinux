//! Compare already-owned Linux pidfds in one pinned, trusted procfs view.
#![forbid(unsafe_code)]
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fs::{Mode, OFlags, PROC_SUPER_MAGIC, fstatfs, open, openat, readlinkat_raw},
};
use std::{
    fs::File,
    io::{self, Read},
    os::fd::{AsRawFd, OwnedFd},
};

const MAX_FDINFO: usize = 4096;
const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::CLOEXEC)
    .union(OFlags::NOFOLLOW);

/// A procfs view opened by this process, with its own fd table pinned.
///
/// Deployment must protect the procfs mount and keep source credentials,
/// registered PIDs and this procfs view in one qualified PID namespace. NSpid
/// contains translated PIDs, never a namespace identifier. No view or descriptor
/// can be supplied through a caller path, and an inherited view is refused after
/// fork. All directory and temporary descriptors have atomic CLOEXEC.
#[derive(Debug)]
pub struct ProcessView {
    _process: OwnedFd,
    descriptors: OwnedFd,
    info: OwnedFd,
    owner_pid: u32,
}
impl ProcessView {
    /// Pin actual procfs and the current process's fd/fdinfo directories.
    pub fn open() -> io::Result<Self> {
        let root = checked_directory("/proc")?;
        let owner_pid = std::process::id();
        let mut target = [0; 32];
        let length = readlinkat_raw(&root, "self", &mut target[..])?;
        let process_name = &target[..length];
        // Kernel-generated /proc/self resolves in this mount's PID namespace.
        // Refuse a translated current PID rather than guessing a namespace.
        if parse_positive_pid(process_name)? != owner_pid {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let process = checked_directory_at(&root, process_name)?;
        let descriptors = checked_directory_at(&process, b"fd")?;
        let info = checked_directory_at(&process, b"fdinfo")?;
        Ok(Self {
            _process: process,
            descriptors,
            info,
            owner_pid,
        })
    }

    /// Check only the pinned kernel descriptor type, independent of liveness.
    ///
    /// A dead or reaped process still has a correctly typed retained pidfd. This
    /// check can validate ancillary protocol shape without treating app exit as
    /// source failure. Only `matches` establishes positive live identity.
    pub fn is_pidfd(&self, owned: &OwnedFd) -> io::Result<bool> {
        self.check_owner()?;
        let number = owned.as_raw_fd().to_string();
        let mut target = [0; 64];
        let length = readlinkat_raw(&self.descriptors, number.as_str(), &mut target[..])?;
        // Both Linux anon-inode pidfds and current pidfs pidfds deliberately
        // expose this fixed kernel type name. Regular files, sockets, eventfds
        // and other typed descriptors cannot satisfy it through actual procfs.
        Ok(&target[..length] == b"anon_inode:[pidfd]")
    }

    fn check_owner(&self) -> io::Result<()> {
        // Before allocation, path lookup or polling: never inspect the parent's
        // fd table through a view inherited by its forked child.
        if self.owner_pid != std::process::id() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(())
    }

    /// Test the two retained references against one positive expected PID.
    ///
    /// The expected PID must be expressed in the pinned procfs mount's namespace.
    /// Owned handles cannot retarget while safely borrowed. A positive result
    /// means that both references named the same live process at the final poll;
    /// it is not a reusable authorization or a future liveness guarantee.
    pub fn matches(&self, left: &OwnedFd, right: &OwnedFd, expected_pid: u32) -> io::Result<bool> {
        self.matches_after_reads(left, right, expected_pid, || {})
    }

    fn matches_after_reads(
        &self,
        left: &OwnedFd,
        right: &OwnedFd,
        expected_pid: u32,
        after_reads: impl FnOnce(),
    ) -> io::Result<bool> {
        self.check_owner()?;
        if expected_pid == 0 || expected_pid > i32::MAX as u32 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let left_pid = self.pid(left)?;
        let right_pid = self.pid(right)?;
        after_reads();
        // Both reads precede this joint final poll. A zombie can still have a
        // positive Pid; polling refuses it. Reaped/invisible Pid values already
        // fail parsing. Any readiness/error flag, including NVAL, fails closed.
        let mut fds = [
            PollFd::new(left, PollFlags::IN),
            PollFd::new(right, PollFlags::IN),
        ];
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let alive =
            poll(&mut fds, Some(&zero))? == 0 && fds.iter().all(|fd| fd.revents().is_empty());
        Ok(alive && left_pid == right_pid && left_pid == expected_pid)
    }

    fn pid(&self, owned: &OwnedFd) -> io::Result<u32> {
        // The path comes only from a safely borrowed owned descriptor number.
        let number = owned.as_raw_fd().to_string();
        if !self.is_pidfd(owned)? {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let fd = openat(
            &self.info,
            number.as_str(),
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )?;
        let mut file = File::from(fd);
        let mut bytes = [0; MAX_FDINFO + 1];
        let mut length = 0;
        loop {
            let read = file.read(&mut bytes[length..])?;
            if read == 0 {
                break;
            }
            length += read;
            if length > MAX_FDINFO {
                return Err(io::ErrorKind::InvalidData.into());
            }
        }
        parse_pid(&bytes[..length])
    }
}

fn checked_directory(path: &str) -> io::Result<OwnedFd> {
    check_procfs(open(path, DIRECTORY_FLAGS, Mode::empty())?)
}
fn checked_directory_at(parent: &OwnedFd, name: &[u8]) -> io::Result<OwnedFd> {
    check_procfs(openat(parent, name, DIRECTORY_FLAGS, Mode::empty())?)
}
fn check_procfs(owned: OwnedFd) -> io::Result<OwnedFd> {
    if fstatfs(&owned)?.f_type != PROC_SUPER_MAGIC {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(owned)
}
fn parse_positive_pid(bytes: &[u8]) -> io::Result<u32> {
    if bytes.is_empty() || bytes[0] == b'0' || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut value: u32 = 0;
    for &digit in bytes {
        value = value
            .checked_mul(10)
            .and_then(|n| n.checked_add(u32::from(digit - b'0')))
            .ok_or(io::ErrorKind::InvalidData)?;
    }
    if value > i32::MAX as u32 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(value)
}
fn parse_pid(bytes: &[u8]) -> io::Result<u32> {
    if bytes.is_empty()
        || bytes.len() > MAX_FDINFO
        || !bytes.ends_with(b"\n")
        || !bytes.is_ascii()
        || bytes.contains(&0)
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut pid = None;
    for line in bytes.split(|&byte| byte == b'\n') {
        if line.starts_with(b"Pid:") {
            if pid.is_some() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            pid = Some(parse_positive_pid(
                line.strip_prefix(b"Pid:\t")
                    .ok_or(io::ErrorKind::InvalidData)?,
            )?);
        }
    }
    pid.ok_or_else(|| io::ErrorKind::InvalidData.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::fd::AsRawFd, process::Command};
    #[test]
    fn fixture_parser_refuses_duplicate_missing_bad_nonpositive_and_overflow_pid() {
        let malformed: &[&[u8]] = &[
            b"Pid:\t7\nPid:\t7\n",
            b"NSpid:\t7\n",
            b"Pid: 7\n",
            b"Pid:\t0\n",
            b"Pid:\t-1\n",
            b"Pid:\t+7\n",
            b"Pid:\t07\n",
            b"Pid:\t7 \n",
            b"Pid:\t7\r\n",
            b"Pid:\t2147483648\n",
            b"Pid:\t4294967296\n",
            b"Pid:\t7",
            b"Pid:\t7\n\0",
        ];
        let accepted: Vec<_> = malformed
            .iter()
            .enumerate()
            .filter_map(|(i, bytes)| parse_pid(bytes).is_ok().then_some(i))
            .collect();
        assert!(
            accepted.is_empty(),
            "accepted malformed fixture cases {accepted:?}"
        );
    }
    #[test]
    fn fixture_parser_bounds_input_and_uses_pid_not_nspid_as_identity() {
        assert_eq!(
            parse_pid(b"pos:\t0\nPid:\t17\nNSpid:\t17\t1\n").unwrap(),
            17
        );
        assert_eq!(parse_pid(b"Pid:\t2147483647\n").unwrap(), i32::MAX as u32);
        let mut oversized = vec![b'x'; MAX_FDINFO];
        oversized.extend_from_slice(b"\nPid:\t17\n");
        assert!(parse_pid(&oversized).is_err());
    }
    #[test]
    fn actual_nonproc_directory_is_refused() {
        assert!(checked_directory("/tmp").is_err());
    }
    #[test]
    fn actual_exit_after_identity_reads_is_refused_by_final_liveness_poll() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let owned = rustix::process::pidfd_open(
            rustix::process::Pid::from_raw(pid as i32).unwrap(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap();
        let duplicate = rustix::io::fcntl_dupfd_cloexec(&owned, 3).unwrap();
        let result =
            ProcessView::open()
                .unwrap()
                .matches_after_reads(&owned, &duplicate, pid, || {
                    child.kill().unwrap();
                    child.wait().unwrap();
                });
        assert!(!result.unwrap_or(false));
        assert!(rustix::io::fcntl_getfd(&owned).is_ok());
        assert!(rustix::io::fcntl_getfd(&duplicate).is_ok());
        assert!(owned.as_raw_fd() >= 0);
    }
}
