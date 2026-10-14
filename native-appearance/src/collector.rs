//! All source IO runs in the exact supervised child, never the service loop.
use crate::{Ack, Completion, DESKTOP_UID, Job, boottime_ns, ipc, sandbox, style};
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, Mode, OFlags},
    process::{Gid, Pid, PidfdFlags, Uid},
};
use std::{
    fs::File,
    io::{self, Read, Seek},
    os::unix::fs::MetadataExt,
    path::{Component, Path},
    time::Duration,
};

fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}
fn directory(path: &Path) -> io::Result<OwnedFd> {
    if !path.is_absolute() {
        return Err(denied());
    }
    let mut fd = fs::open(
        "/",
        OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    for part in path.components() {
        match part {
            Component::RootDir => {}
            Component::Normal(p) => {
                fd = fs::openat(
                    &fd,
                    p,
                    OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )?;
                let m = fs::fstat(&fd)?;
                if !matches!(m.st_uid, 0 | DESKTOP_UID) || m.st_mode & 0o022 != 0 {
                    return Err(denied());
                }
            }
            _ => return Err(denied()),
        }
    }
    Ok(fd)
}
fn child_dir(parent: impl AsFd, name: &str) -> io::Result<OwnedFd> {
    let fd = fs::openat(
        parent,
        name,
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let m = fs::fstat(&fd)?;
    if m.st_uid != DESKTOP_UID || m.st_mode & 0o022 != 0 {
        return Err(denied());
    }
    Ok(fd)
}
fn regular(parent: impl AsFd, name: &str, flags: OFlags) -> io::Result<File> {
    let fd = fs::openat(
        parent,
        name,
        flags | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )?;
    let f = File::from(fd);
    let m = f.metadata()?;
    if !m.is_file()
        || m.uid() != DESKTOP_UID
        || m.nlink() != 1
        || m.mode() & 0o022 != 0
        || m.len() > 64 * 1024
    {
        return Err(denied());
    }
    Ok(f)
}
fn read(f: &mut File) -> io::Result<String> {
    let before = f.metadata()?;
    let mut bytes = Vec::new();
    f.by_ref().take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    let after = f.metadata()?;
    if bytes.len() > 64 * 1024
        || (
            before.dev(),
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    String::from_utf8(bytes).map_err(|_| io::ErrorKind::InvalidData.into())
}

fn same(a: &fs::Stat, b: &fs::Stat) -> bool {
    a.st_dev == b.st_dev
        && a.st_ino == b.st_ino
        && a.st_mode == b.st_mode
        && a.st_uid == b.st_uid
        && a.st_gid == b.st_gid
        && a.st_nlink == b.st_nlink
        && a.st_size == b.st_size
        && a.st_mtime == b.st_mtime
        && a.st_mtime_nsec == b.st_mtime_nsec
        && a.st_ctime == b.st_ctime
        && a.st_ctime_nsec == b.st_ctime_nsec
}
// Fixed chains retain each component so replacing an ancestor cannot make an
// obsolete opened generation look current. Missing optional components are
// observations too; creation before completion invalidates that collection.
struct Chain {
    fds: Vec<OwnedFd>,
    names: Vec<&'static str>,
    missing: Option<&'static str>,
}
impl Chain {
    fn acquire(home: impl AsFd, names: &[&'static str], optional: bool) -> io::Result<Self> {
        let mut value = Self {
            fds: vec![home.as_fd().try_clone_to_owned()?],
            names: Vec::new(),
            missing: None,
        };
        for &name in names {
            match child_dir(value.fds.last().unwrap(), name) {
                Ok(fd) => {
                    value.fds.push(fd);
                    value.names.push(name);
                }
                Err(e) if optional && e.raw_os_error() == Some(libc::ENOENT) => {
                    value.missing = Some(name);
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(value)
    }
    fn leaf(&self) -> &OwnedFd {
        self.fds.last().unwrap()
    }
    fn verify(&self) -> io::Result<()> {
        for fd in &self.fds {
            let retained = fs::fstat(fd)?;
            if retained.st_uid != DESKTOP_UID || retained.st_mode & 0o022 != 0 {
                return Err(denied());
            }
        }
        for (index, name) in self.names.iter().enumerate() {
            let observed = fs::statat(&self.fds[index], *name, fs::AtFlags::SYMLINK_NOFOLLOW)?;
            let retained = fs::fstat(&self.fds[index + 1])?;
            if (
                observed.st_dev,
                observed.st_ino,
                observed.st_mode,
                observed.st_uid,
                observed.st_gid,
            ) != (
                retained.st_dev,
                retained.st_ino,
                retained.st_mode,
                retained.st_uid,
                retained.st_gid,
            ) || retained.st_uid != DESKTOP_UID
                || retained.st_mode & 0o022 != 0
            {
                return Err(denied());
            }
        }
        if let Some(name) = self.missing {
            match fs::statat(self.leaf(), name, fs::AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => {}
                _ => return Err(denied()),
            }
        }
        Ok(())
    }
}
struct Input {
    parent: OwnedFd,
    name: &'static str,
    file: Option<File>,
    stamp: Option<fs::Stat>,
    text: String,
}
impl Input {
    fn acquire(parent: impl AsFd, name: &'static str, optional: bool) -> io::Result<Self> {
        let file = match regular(&parent, name, OFlags::RDONLY) {
            Ok(file) => Some(file),
            Err(e) if optional && e.raw_os_error() == Some(libc::ENOENT) => None,
            Err(e) => return Err(e),
        };
        let stamp = file.as_ref().map(fs::fstat).transpose()?;
        Ok(Self {
            parent: parent.as_fd().try_clone_to_owned()?,
            name,
            file,
            stamp,
            text: String::new(),
        })
    }
    fn verify_node(&self) -> io::Result<()> {
        match (&self.file, &self.stamp) {
            (Some(file), Some(stamp)) => {
                if !same(stamp, &fs::fstat(file)?)
                    || !same(
                        stamp,
                        &fs::statat(&self.parent, self.name, fs::AtFlags::SYMLINK_NOFOLLOW)?,
                    )
                {
                    return Err(denied());
                }
            }
            (None, None) => {
                match fs::statat(&self.parent, self.name, fs::AtFlags::SYMLINK_NOFOLLOW) {
                    Err(rustix::io::Errno::NOENT) => {}
                    _ => return Err(denied()),
                }
            }
            _ => return Err(denied()),
        }
        Ok(())
    }
    fn load(&mut self) -> io::Result<()> {
        self.verify_node()?;
        if let Some(file) = &mut self.file {
            self.text = read(file)?;
        }
        self.verify_node()
    }
    fn verify(&mut self) -> io::Result<()> {
        self.verify_node()?;
        if let Some(file) = &mut self.file {
            file.rewind()?;
            if read(file)? != self.text {
                return Err(denied());
            }
        }
        self.verify_node()
    }
}
struct Guards {
    lock: File,
    runtime: OwnedFd,
    theme: Chain,
    user: Chain,
    inputs: Vec<Input>,
}
impl Guards {
    fn verify(&mut self) -> io::Result<()> {
        self.theme.verify()?;
        self.user.verify()?;
        let runtime = fs::fstat(&self.runtime)?;
        if runtime.st_uid != DESKTOP_UID || runtime.st_mode & 0o022 != 0 {
            return Err(denied());
        }
        let lock = fs::fstat(&self.lock)?;
        if !same(
            &lock,
            &fs::statat(
                &self.runtime,
                "omarchy-theme-set.lock",
                fs::AtFlags::SYMLINK_NOFOLLOW,
            )?,
        ) || lock.st_uid != DESKTOP_UID
            || lock.st_mode & 0o022 != 0
            || lock.st_nlink != 1
        {
            return Err(denied());
        }
        for input in &mut self.inputs {
            input.verify()?;
        }
        Ok(())
    }
}
pub fn run(parent: u32) -> io::Result<()> {
    let channel = std::io::stdin();
    let peer_pid = Pid::from_raw(parent as i32).ok_or_else(denied)?;
    let parent_watch = rustix::process::pidfd_open(peer_pid, PidfdFlags::NONBLOCK)?;
    let bootstrap_until = boottime_ns().saturating_add(crate::COLLECTION_BUDGET_NS);
    let job: Job = loop {
        if boottime_ns() >= bootstrap_until || !ipc::living(&parent_watch)? {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some((wire, cred)) = ipc::receive(&channel)? {
            if cred.uid.as_raw() != 0
                || cred.gid.as_raw() != 0
                || cred.pid.as_raw_pid() as u32 != parent
            {
                return Err(denied());
            }
            break serde_json::from_slice(&wire)?;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let started = boottime_ns();
    if job.until_ns <= started || job.until_ns - started > crate::COLLECTION_BUDGET_NS {
        return Err(denied());
    }
    rustix::thread::set_thread_groups(&[])?;
    rustix::thread::set_thread_res_gid(
        Gid::from_raw(DESKTOP_UID),
        Gid::from_raw(DESKTOP_UID),
        Gid::from_raw(DESKTOP_UID),
    )?;
    rustix::thread::set_thread_res_uid(
        Uid::from_raw(DESKTOP_UID),
        Uid::from_raw(DESKTOP_UID),
        Uid::from_raw(DESKTOP_UID),
    )?;
    if rustix::process::getuid().as_raw() != DESKTOP_UID
        || rustix::process::getgid().as_raw() != DESKTOP_UID
    {
        return Err(denied());
    }
    // Set this after credential changes, which clear PDEATHSIG. The retained
    // parent pidfd also closes the fork/parent-death race.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if !ipc::living(&parent_watch)? || boottime_ns() >= job.until_ns {
        return Err(io::ErrorKind::NotConnected.into());
    }
    let mut guards = None;
    let collected = (|| {
        let home = directory(Path::new(&job.config.home))?;
        if fs::fstat(&home)?.st_uid != DESKTOP_UID {
            return Err(denied());
        }
        let runtime = directory(Path::new(&job.config.runtime))?;
        if fs::fstat(&runtime)?.st_uid != DESKTOP_UID {
            return Err(denied());
        }
        let lock = regular(
            &runtime,
            "omarchy-theme-set.lock",
            OFlags::RDWR | OFlags::CREATE,
        )?;
        loop {
            if boottime_ns() >= job.until_ns || !ipc::living(&parent_watch)? {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match fs::flock(&lock, fs::FlockOperation::NonBlockingLockShared) {
                Ok(()) => break,
                Err(rustix::io::Errno::AGAIN) => std::thread::sleep(Duration::from_millis(2)),
                Err(e) => return Err(e.into()),
            }
        }
        let theme = Chain::acquire(
            &home,
            &[".local", "state", "omarchy", "current", "theme"],
            false,
        )?;
        let user = Chain::acquire(&home, &[".config", "omarchy"], true)?;
        let mut inputs = vec![
            Input::acquire(theme.leaf(), "colors.toml", false)?,
            Input::acquire(theme.leaf(), "shell.toml", true)?,
        ];
        if user.missing.is_none() {
            inputs.push(Input::acquire(user.leaf(), "shell.toml", true)?);
        }
        drop(home);
        sandbox::restrict()?;
        for input in &mut inputs {
            input.load()?;
        }
        let appearance = style::project(
            &inputs[0].text,
            &inputs[1].text,
            inputs.get(2).map_or("", |v| v.text.as_str()),
        )?;
        let mut retained = Guards {
            lock,
            runtime,
            theme,
            user,
            inputs,
        };
        retained.verify()?;
        if boottime_ns() >= job.until_ns {
            return Err(denied());
        }
        guards = Some(retained);
        Ok(appearance)
    })();
    let response = Completion {
        owner: job.owner.clone(),
        token: job.token,
        until_ns: job.until_ns,
        appearance: collected.ok(),
    };
    let wire = serde_json::to_vec(&response)?;
    loop {
        if boottime_ns() >= job.until_ns || !ipc::living(&parent_watch)? {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(retained) = &mut guards {
            retained.verify()?;
        }
        match ipc::send(&channel, &wire) {
            Ok(()) => break,
            Err(e) if matches!(e.raw_os_error(), Some(libc::EAGAIN | libc::EINTR)) => {
                std::thread::sleep(Duration::from_millis(2))
            }
            Err(e) => return Err(e),
        }
    }
    loop {
        if boottime_ns() >= job.until_ns || !ipc::living(&parent_watch)? {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(retained) = &mut guards {
            retained.verify()?;
        }
        if let Some((wire, cred)) = ipc::receive(&channel)? {
            let ack: Ack = serde_json::from_slice(&wire)?;
            if cred.uid.as_raw() != 0
                || cred.gid.as_raw() != 0
                || cred.pid.as_raw_pid() as u32 != parent
                || ack.owner != job.owner
                || ack.token != job.token
                || boottime_ns() >= job.until_ns
            {
                return Err(denied());
            }
            // Original shared lock and directory generation live through ACK.
            drop(guards);
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
