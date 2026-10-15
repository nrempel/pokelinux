//! Disk operations run only in the owned storage process.
use crate::{MAX_APPS, MAX_RECEIPTS, Permission, boottime_ns, fresh_id, ipc};
use rustix::{
    event::{PollFd, PollFlags},
    fd::OwnedFd,
    net::{self, SocketFlags},
    process::{Pid, PidfdFlags},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Grant {
    pub app: String,
    pub permission: Permission,
    pub allowed: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Durable {
    pub grants: Vec<Grant>,
    pub receipts: Vec<String>,
    /// Native lock admission survives actor and display/source replacement.
    /// Only an explicitly fresh root-owned session store starts without it.
    pub lock_pending: bool,
}
pub(crate) struct Disk {
    directory: PathBuf,
    _lock: File,
    epoch: String,
    hash: String,
    closed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Live {
    epoch: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Clean {
    epoch: String,
    hash: String,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn small(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(m) => {
            if !m.is_file() || m.uid() != 0 || m.mode() & 0o077 != 0 {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let mut bytes = Vec::new();
            OpenOptions::new()
                .read(true)
                .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                .open(path)?
                .take(513)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 512 {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Ok(Some(bytes))
        }
    }
}
fn atomic(directory: &Path, pending: &str, target: &str, bytes: &[u8]) -> io::Result<()> {
    let path = directory.join(pending);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    std::fs::rename(path, directory.join(target))?;
    File::open(directory)?.sync_all()?;
    Ok(())
}
impl Disk {
    pub(crate) fn open(directory: &Path, apps: &[String]) -> io::Result<(Self, Durable)> {
        let m = std::fs::symlink_metadata(directory)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o077 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(directory.join("owner.lock"))?;
        let m = lock.metadata()?;
        if !m.is_file() || m.uid() != 0 || m.mode() & 0o077 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
        match std::fs::symlink_metadata(directory.join("policy.pending")) {
            Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        for name in ["owner.pending", "clean.pending"] {
            match std::fs::symlink_metadata(directory.join(name)) {
                Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        let live = small(&directory.join("owner.live"))?
            .map(|b| serde_json::from_slice::<Live>(&b))
            .transpose()?;
        let clean = small(&directory.join("owner.clean"))?
            .map(|b| serde_json::from_slice::<Clean>(&b))
            .transpose()?;
        if live
            .as_ref()
            .is_some_and(|l| clean.as_ref().is_none_or(|c| c.epoch != l.epoch))
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut policy_hash = hash(b"");
        let path = directory.join("policy.json");
        let durable = match std::fs::symlink_metadata(&path) {
            // This is root-owned storage bootstrap, never an unlocked Source
            // observation. Existing owner seals still bind exact policy bytes;
            // deleting an admitted policy cannot qualify a fresh session here.
            Err(e) if e.kind() == io::ErrorKind::NotFound => Durable::default(),
            Err(e) => return Err(e),
            Ok(m) => {
                if !m.is_file() || m.uid() != 0 || m.mode() & 0o077 != 0 {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                let mut bytes = Vec::new();
                OpenOptions::new()
                    .read(true)
                    .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                    .open(path)?
                    .take(32769)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > 32768 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                policy_hash = hash(&bytes);
                let state: Durable = serde_json::from_slice(&bytes)?;
                validate(&state, apps)?;
                state
            }
        };
        if live.is_some() && clean.as_ref().is_none_or(|c| c.hash != policy_hash) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let epoch = fresh_id()?;
        atomic(
            directory,
            "owner.pending",
            "owner.live",
            &serde_json::to_vec(&Live {
                epoch: epoch.clone(),
            })?,
        )?;
        Ok((
            Self {
                directory: directory.into(),
                _lock: lock,
                epoch,
                hash: policy_hash,
                closed: false,
            },
            durable,
        ))
    }
    pub(crate) fn replace(&mut self, next: &Durable) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let bytes = serde_json::to_vec(next)?;
        if bytes.len() > 32768 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let path = self.directory.join("policy.pending");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(path, self.directory.join("policy.json"))?;
        File::open(&self.directory)?.sync_all()?;
        self.hash = hash(&bytes);
        Ok(())
    }
    fn close_clean(&mut self) -> io::Result<()> {
        self.closed = true;
        atomic(
            &self.directory,
            "clean.pending",
            "owner.clean",
            &serde_json::to_vec(&Clean {
                epoch: self.epoch.clone(),
                hash: self.hash.clone(),
            })?,
        )
    }
}
fn validate(state: &Durable, apps: &[String]) -> io::Result<()> {
    if apps.len() > MAX_APPS
        || state.grants.len() > MAX_APPS * 2
        || state.receipts.len() > MAX_RECEIPTS
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    for (i, g) in state.grants.iter().enumerate() {
        if !apps.contains(&g.app)
            || state.grants[..i]
                .iter()
                .any(|p| p.app == g.app && p.permission == g.permission)
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    if state.receipts.iter().any(|s| {
        s.len() != 49
            || s.as_bytes()[32] != b'-'
            || !s
                .bytes()
                .enumerate()
                .all(|(i, b)| i == 32 || b.is_ascii_hexdigit())
    }) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source: String,
    token: u64,
    until: u64,
    job: Job,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Job {
    Load { apps: Vec<String> },
    Replace { state: Durable },
    Shutdown {},
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    source: String,
    token: u64,
    success: bool,
    state: Option<Durable>,
}
struct Pending {
    token: u64,
    until: u64,
    wire: Option<Vec<u8>>,
}
pub(crate) enum Event {
    Complete {
        token: u64,
        success: bool,
        state: Option<Durable>,
    },
    Failed {
        token: u64,
    },
    Reaped,
}
pub(crate) struct Worker {
    child: Option<Child>,
    alive: OwnedFd,
    listener: OwnedFd,
    connection: Option<OwnedFd>,
    path: PathBuf,
    identity: (u64, u64),
    source: String,
    sequence: u64,
    pending: Option<Pending>,
    terminal: bool,
}
impl Worker {
    pub(crate) fn spawn(
        runtime: &Path,
        policy: &Path,
        executable: &Path,
        test: Option<&str>,
    ) -> io::Result<Self> {
        crate::service::trusted_executable(executable)?;
        let path = runtime.join("storage.sock");
        let listener = ipc::listener(&path, 0o600)?;
        let m = std::fs::symlink_metadata(&path)?;
        let source = fresh_id()?;
        let mut command = Command::new(executable);
        command
            .arg("--storage-worker")
            .arg(&path)
            .arg(policy)
            .arg(std::process::id().to_string())
            .arg(&source)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(test) = test {
            command.arg(test).env("NATIVE_LIFECYCLE_ISOLATED", "1");
        }
        let mut child = command.spawn()?;
        let alive = match rustix::process::pidfd_open(
            Pid::from_raw(child.id() as i32).unwrap(),
            PidfdFlags::NONBLOCK,
        ) {
            Ok(fd) => fd,
            Err(e) => {
                let _ = child.kill();
                let until = boottime_ns().saturating_add(2_000_000_000);
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) if boottime_ns() < until => {
                            std::thread::sleep(std::time::Duration::from_millis(2));
                        }
                        _ => std::process::abort(),
                    }
                }
                return Err(e.into());
            }
        };
        Ok(Self {
            child: Some(child),
            alive,
            listener,
            connection: None,
            path,
            identity: (m.dev(), m.ino()),
            source,
            sequence: 0,
            pending: None,
            terminal: false,
        })
    }
    pub(crate) fn occupied(&self) -> bool {
        self.child.is_some()
    }
    /// Monotonic admission identity, including a write that has already finished.
    /// Queued acknowledgements must retain the identity at their own admission.
    pub(crate) fn admission_sequence(&self) -> u64 {
        self.sequence
    }
    pub(crate) fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn start_load(&mut self, apps: Vec<String>, until: u64) -> io::Result<u64> {
        self.start(Job::Load { apps }, until)
    }
    pub(crate) fn start_shutdown(&mut self, until: u64) -> io::Result<u64> {
        self.start(Job::Shutdown {}, until)
    }
    pub(crate) fn start_write(&mut self, state: Durable, until: u64) -> io::Result<u64> {
        self.start(Job::Replace { state }, until)
    }
    fn start(&mut self, job: Job, until: u64) -> io::Result<u64> {
        if self.terminal || self.pending.is_some() || self.child.is_none() || boottime_ns() >= until
        {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.sequence = self.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        let wire = serde_json::to_vec(&Request {
            source: self.source.clone(),
            token: self.sequence,
            until,
            job,
        })?;
        if wire.len() > 65536 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.pending = Some(Pending {
            token: self.sequence,
            until,
            wire: Some(wire),
        });
        Ok(self.sequence)
    }
    pub(crate) fn stop(&mut self) {
        self.terminal = true;
        if let Some(child) = &mut self.child {
            let _ = child.kill();
        }
    }
    pub(crate) fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(child) = &mut self.child
            && child.try_wait()?.is_some()
        {
            self.child = None;
            self.terminal = true;
            if let Some(p) = self.pending.take() {
                return Ok(Some(Event::Failed { token: p.token }));
            }
            return Ok(Some(Event::Reaped));
        }
        if let Some(p) = &self.pending
            && (self.terminal || boottime_ns() >= p.until)
        {
            let token = p.token;
            self.terminal = true;
            self.pending = None;
            return Ok(Some(Event::Failed { token }));
        }
        if self.terminal {
            return Ok(None);
        }
        if self.connection.is_none() {
            match net::accept_with(&self.listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                Ok(fd) => {
                    net::sockopt::set_socket_passcred(&fd, true)?;
                    let cred = net::sockopt::socket_peercred(&fd)?;
                    if cred.uid.as_raw() == 0
                        && Some(cred.pid.as_raw_nonzero().get() as u32)
                            == self.child.as_ref().map(Child::id)
                    {
                        self.connection = Some(fd);
                    }
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(e) => return Err(e.into()),
            }
        }
        if let (Some(fd), Some(p)) = (&self.connection, &mut self.pending) {
            if let Some(wire) = &p.wire {
                match ipc::send_storage(fd, wire) {
                    Ok(()) => p.wire = None,
                    Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => return Ok(None),
                    Err(_) => {
                        let token = p.token;
                        self.terminal = true;
                        self.pending = None;
                        return Ok(Some(Event::Failed { token }));
                    }
                }
            }
            match ipc::receive_storage(fd) {
                Ok(Some((wire, cred))) => {
                    let result = serde_json::from_slice::<Response>(&wire);
                    let Some(expected) = self.pending.take() else {
                        return Err(io::ErrorKind::InvalidData.into());
                    };
                    if cred.uid.as_raw() != 0
                        || Some(cred.pid.as_raw_nonzero().get() as u32)
                            != self.child.as_ref().map(Child::id)
                        || !crate::authority::living(&self.alive)?
                        || boottime_ns() >= expected.until
                    {
                        self.terminal = true;
                        return Ok(Some(Event::Failed {
                            token: expected.token,
                        }));
                    }
                    match result {
                        Ok(r) if r.source == self.source && r.token == expected.token => {
                            Ok(Some(Event::Complete {
                                token: r.token,
                                success: r.success,
                                state: r.state,
                            }))
                        }
                        _ => {
                            self.terminal = true;
                            Ok(Some(Event::Failed {
                                token: expected.token,
                            }))
                        }
                    }
                }
                Ok(None) => Ok(None),
                Err(e) if e.raw_os_error() == Some(4) => Ok(None),
                Err(_) => {
                    let token = self.pending.take().unwrap().token;
                    self.terminal = true;
                    Ok(Some(Event::Failed { token }))
                }
            }
        } else {
            Ok(None)
        }
    }
    #[cfg(test)]
    pub(crate) fn drain(&mut self) {
        if !self.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
            std::process::abort();
        }
    }
    pub(crate) fn drain_until(&mut self, until: u64) -> bool {
        self.drain_until_with(until, Self::poll)
    }
    fn drain_until_with(
        &mut self,
        until: u64,
        mut poll: impl FnMut(&mut Self) -> io::Result<Option<Event>>,
    ) -> bool {
        self.stop();
        while self.occupied() {
            if boottime_ns() >= until || poll(self).is_err() {
                return false;
            }
            if self.occupied() {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
        true
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if self.child.is_some() {
            std::process::abort();
        }
        if std::fs::symlink_metadata(&self.path).is_ok_and(|m| (m.dev(), m.ino()) == self.identity)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
/// Explicit isolated qualification of wait/reap failure while retaining a real child.
/// This does not create an uninterruptible filesystem syscall.
pub fn isolated_reap_error_test(runtime: &Path, policy: &Path, mode: &str) -> io::Result<()> {
    use std::io::Write;
    if std::env::var("NATIVE_LIFECYCLE_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
        || !matches!(mode, "error" | "withheld")
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut owner = Worker::spawn(runtime, policy, &std::env::current_exe()?, None)?;
    let until = boottime_ns().saturating_add(1_000_000_000);
    owner.start_load(vec!["one".into()], until)?;
    loop {
        match owner.poll()? {
            Some(Event::Complete { success: true, .. }) => break,
            Some(_) => return Err(io::ErrorKind::Other.into()),
            None if boottime_ns() < until => {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            None => return Err(io::ErrorKind::TimedOut.into()),
        }
    }
    println!("owned-child:{}", owner.child.as_ref().unwrap().id());
    std::io::stdout().flush()?;
    if !owner.drain_until_with(boottime_ns().saturating_add(200_000_000), |_| {
        if mode == "error" {
            Err(io::ErrorKind::Other.into())
        } else {
            Ok(None)
        }
    }) {
        // Retain Child until process termination: no detached success or replacement.
        std::process::abort();
    }
    Err(io::ErrorKind::Other.into())
}
pub fn worker(args: &[std::ffi::OsString]) -> io::Result<()> {
    if !(args.len() == 4
        || args.len() == 5 && std::env::var("NATIVE_LIFECYCLE_ISOLATED").as_deref() == Ok("1"))
        || rustix::process::getuid().as_raw() != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let path = Path::new(&args[0]);
    let policy = Path::new(&args[1]);
    let parent: u32 = args[2]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let source = args[3].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let test = if args.len() == 5 {
        let text = args[4].to_str().ok_or(io::ErrorKind::InvalidInput)?;
        let (mode, count) = text.split_once(':').ok_or(io::ErrorKind::InvalidInput)?;
        let count: u64 = count.parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        if !matches!(mode, "pause" | "stale") || count > 128 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Some((mode, count))
    } else {
        None
    };
    let mut writes = 0u64;
    let mut previous: Option<Vec<u8>> = None;
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    if rustix::process::getppid().map(|p| p.as_raw_nonzero().get() as u32) != Some(parent) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let fd = ipc::connect(path)?;
    let mut disk = None;
    let mut apps = Vec::new();
    let mut last = 0;
    loop {
        match ipc::receive_storage(&fd) {
            Ok(Some((wire, cred))) => {
                if cred.uid.as_raw() != 0 || cred.pid.as_raw_nonzero().get() as u32 != parent {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                let request: Request = serde_json::from_slice(&wire)?;
                if request.source != source || request.token <= last {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                last = request.token;
                let mut response = Response {
                    source: source.into(),
                    token: request.token,
                    success: false,
                    state: None,
                };
                let shutdown = matches!(request.job, Job::Shutdown {});
                if boottime_ns() < request.until {
                    match request.job {
                        Job::Load { apps: allowed } if disk.is_none() => {
                            if test == Some(("pause", 0)) {
                                rustix::process::kill_process(
                                    rustix::process::getpid(),
                                    rustix::process::Signal::STOP,
                                )?;
                            }
                            if let Ok((opened, state)) = Disk::open(policy, &allowed) {
                                apps = allowed;
                                disk = Some(opened);
                                response.success = true;
                                response.state = Some(state);
                            }
                        }
                        Job::Replace { state } => {
                            if let Some(disk) = &mut disk
                                && validate(&state, &apps).is_ok()
                                && disk.replace(&state).is_ok()
                            {
                                response.success = true;
                                writes = writes.checked_add(1).ok_or(io::ErrorKind::Other)?;
                                if test == Some(("pause", writes)) {
                                    rustix::process::kill_process(
                                        rustix::process::getpid(),
                                        rustix::process::Signal::STOP,
                                    )?;
                                }
                                if test == Some(("stale", writes))
                                    && let Some(old) = &previous
                                {
                                    ipc::send_storage(&fd, old)?;
                                }
                            }
                        }
                        Job::Shutdown {} => {
                            if let Some(disk) = &mut disk {
                                response.success = disk.close_clean().is_ok();
                            }
                        }
                        _ => {}
                    }
                }
                let bytes = serde_json::to_vec(&response)?;
                loop {
                    if boottime_ns() >= request.until {
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                    match ipc::send_storage(&fd, &bytes) {
                        Ok(()) if boottime_ns() < request.until => break,
                        Ok(()) => return Err(io::ErrorKind::TimedOut.into()),
                        Err(e)
                            if matches!(e.raw_os_error(), Some(4 | 11))
                                && boottime_ns() < request.until => {}
                        Err(e) => return Err(e),
                    };
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                previous = Some(bytes);
                if shutdown {
                    return Ok(());
                }
            }
            Ok(None) => {
                let mut poll = [PollFd::new(&fd, PollFlags::IN)];
                match rustix::event::poll(
                    &mut poll,
                    Some(&rustix::event::Timespec {
                        tv_sec: 0,
                        tv_nsec: 20_000_000,
                    }),
                ) {
                    Ok(_) | Err(rustix::io::Errno::INTR) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) if e.raw_os_error() == Some(4) => {}
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            assert_eq!(rustix::process::getuid().as_raw(), 0);
            let root =
                std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").expect("isolated kernel runner");
            let path = PathBuf::from(root).join(format!("storage-{}", fresh_id().unwrap()));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn clean_seal_binds_exact_policy_and_unclean_owner_never_reopens() {
        let dir = Temp::new();
        let apps = vec!["one".into()];
        let (mut disk, _) = Disk::open(&dir.0, &apps).unwrap();
        let state = Durable {
            grants: vec![Grant {
                app: "one".into(),
                permission: Permission::Capture,
                allowed: true,
            }],
            receipts: vec![],
            lock_pending: false,
        };
        disk.replace(&state).unwrap();
        disk.close_clean().unwrap();
        drop(disk);
        let (disk, _) = Disk::open(&dir.0, &apps).unwrap();
        drop(disk);
        assert!(
            Disk::open(&dir.0, &apps).is_err(),
            "fresh live epoch cannot inherit the previous clean seal"
        );
    }
    #[test]
    fn clean_seal_rejects_changed_policy_without_overwriting_owner() {
        let dir = Temp::new();
        let apps = vec!["one".into()];
        let (mut disk, _) = Disk::open(&dir.0, &apps).unwrap();
        disk.close_clean().unwrap();
        drop(disk);
        let original = std::fs::read(dir.0.join("owner.live")).unwrap();
        std::fs::write(
            dir.0.join("policy.json"),
            b"{\"grants\":[],\"receipts\":[]}",
        )
        .unwrap();
        assert!(Disk::open(&dir.0, &apps).is_err());
        assert_eq!(std::fs::read(dir.0.join("owner.live")).unwrap(), original);
    }
    #[test]
    fn terminated_worker_slot_remains_owned_until_actual_reap() {
        let dir = Temp::new();
        let source = PathBuf::from(
            std::env::var_os("NATIVE_LIFECYCLE_STORAGE_HELPER")
                .expect("exact recorded daemon helper"),
        );
        let executable =
            PathBuf::from("/opt").join(format!("lifecycle-storage-helper-{}", fresh_id().unwrap()));
        std::fs::copy(source, &executable).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut worker = Worker::spawn(&dir.0, &dir.0, &executable, None).unwrap();
        let token = worker
            .start_load(
                vec!["one".into()],
                boottime_ns().saturating_add(2_000_000_000),
            )
            .unwrap();
        loop {
            match worker.poll().unwrap() {
                Some(Event::Complete {
                    token: returned,
                    success: true,
                    state: Some(_),
                }) if returned == token => break,
                None => std::thread::sleep(std::time::Duration::from_millis(2)),
                _ => {
                    worker.drain();
                    panic!("worker failed initial load")
                }
            }
        }
        // No publication exists in this ownership-only test. Do not poll/reap yet.
        worker.stop();
        let until = boottime_ns().saturating_add(2_000_000_000);
        while crate::authority::living(&worker.alive).unwrap() && boottime_ns() < until {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let exited = !crate::authority::living(&worker.alive).unwrap();
        let retained = worker.occupied();
        let refused = worker.start_write(Durable::default(), until).is_err();
        worker.drain();
        assert!(
            exited && retained && refused,
            "exit notification must not relinquish Child before wait/reap"
        );
        assert!(!worker.occupied());
        assert!(
            worker
                .start_load(vec![], boottime_ns().saturating_add(1_000_000_000))
                .is_err(),
            "same supervisor never starts a successor"
        );
        drop(worker);
        std::fs::remove_file(executable).unwrap();
    }
}
