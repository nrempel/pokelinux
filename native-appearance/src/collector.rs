//! All source IO runs in the exact supervised child, never the service loop.
use crate::{Ack, Completion, DESKTOP_UID, Job, boottime_ns, ipc, palette, sandbox};
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, Mode, OFlags},
    process::{Gid, Pid, PidfdFlags, Uid},
};
use std::{
    fs::File,
    io::{self, Read},
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
        let local = child_dir(&home, ".local")?;
        let state = child_dir(&local, "state")?;
        let omarchy = child_dir(&state, "omarchy")?;
        let current = child_dir(&omarchy, "current")?;
        let generation = child_dir(&current, "theme")?;
        let identity = fs::fstat(&generation)?;
        let mut colors = regular(&generation, "colors.toml", OFlags::RDONLY)?;
        let mut shell = match regular(&generation, "shell.toml", OFlags::RDONLY) {
            Ok(f) => Some(f),
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => None,
            Err(e) => return Err(e),
        };
        // Ancestor handles are not needed for reading. Retain only explicit
        // generation/lock identity guards and selected regular input handles.
        drop((home, local, state, omarchy));
        sandbox::restrict()?;
        let colors = read(&mut colors)?;
        let shell = match &mut shell {
            Some(f) => read(f)?,
            None => String::new(),
        };
        let palette = palette::project(&colors, &shell)?;
        let now = boottime_ns();
        let still_current = fs::statat(&current, "theme", fs::AtFlags::SYMLINK_NOFOLLOW)?;
        let still_lock = fs::statat(
            &runtime,
            "omarchy-theme-set.lock",
            fs::AtFlags::SYMLINK_NOFOLLOW,
        )?;
        let lock_meta = lock.metadata()?;
        if now >= job.until_ns
            || (still_current.st_dev, still_current.st_ino) != (identity.st_dev, identity.st_ino)
            || (still_lock.st_dev as u64, still_lock.st_ino as u64)
                != (lock_meta.dev(), lock_meta.ino())
        {
            return Err(denied());
        }
        guards = Some((lock, current, generation, runtime));
        Ok(palette)
    })();
    let response = Completion {
        owner: job.owner.clone(),
        token: job.token,
        until_ns: job.until_ns,
        palette: collected.ok(),
    };
    let wire = serde_json::to_vec(&response)?;
    loop {
        if boottime_ns() >= job.until_ns || !ipc::living(&parent_watch)? {
            return Err(io::ErrorKind::TimedOut.into());
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
