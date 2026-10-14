//! Independent responsive presentation owner. No lifecycle/grant/focus mutations.
use crate::{
    Ack, COLLECTION_BUDGET_NS, Completion, Configuration, DESKTOP_UID, Job, REFRESH_NS, Snapshot,
    boottime_ns, fresh_id, ipc, style::Appearance,
};
use rustix::{
    fd::OwnedFd,
    net::{self, SocketFlags},
    process::{Pid, PidfdFlags},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::{File, OpenOptions},
    io,
    os::fd::{AsRawFd, FromRawFd},
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn request_stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Snapshot,
    Status,
    Stop,
}
#[derive(Serialize)]
struct Status<'a> {
    owner: &'a str,
    state: &'a str,
    worker_pid: Option<u32>,
    worker_until_ns: Option<String>,
    completed: u64,
    revision: u32,
    available: bool,
}
struct Worker {
    child: Option<OwnedChild>,
    watch: Option<OwnedFd>,
    channel: OwnedFd,
    job: Job,
    bootstrap: Option<Vec<u8>>,
    ack: Option<Vec<u8>>,
    accepted: bool,
    terminal: bool,
}
enum Event {
    Candidate(Box<Completion>),
    Reaped,
}

struct Executable {
    file: File,
    null: File,
}
impl Executable {
    fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        // Keep selected handles outside the standard-descriptor range even if
        // the installer's standard descriptors were closed.
        fn retain(file: File) -> io::Result<File> {
            let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(unsafe { File::from_raw_fd(fd) })
        }
        let file = retain(file)?;
        let null = retain(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/null")?,
        )?;
        Ok(Self { file, null })
    }
}

struct OwnedChild {
    pid: libc::pid_t,
    reaped: bool,
}
impl OwnedChild {
    fn id(&self) -> u32 {
        self.pid as u32
    }
    fn kill(&mut self) -> io::Result<()> {
        if unsafe { libc::kill(self.pid, libc::SIGKILL) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        let mut status = 0;
        let waited = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
        if waited < 0 {
            return Err(io::Error::last_os_error());
        }
        if waited == self.pid {
            self.reaped = true;
            return Ok(Some(status));
        }
        Ok(None)
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            std::process::abort();
        }
    }
}
impl Worker {
    fn spawn(executable: &Executable, job: Job) -> io::Result<Self> {
        let (channel, peer) = net::socketpair(
            net::AddressFamily::UNIX,
            net::SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        for fd in [&channel, &peer] {
            net::sockopt::set_socket_passcred(fd, true)?;
        }
        let bootstrap = serde_json::to_vec(&job)?;
        if bootstrap.len() > crate::MAX_PACKET {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let parent_pid = std::process::id();
        let parent = CString::new(parent_pid.to_string())?;
        let argv = [
            c"pokelinux-appearance".as_ptr(),
            c"--collector".as_ptr(),
            parent.as_ptr(),
            std::ptr::null(),
        ];
        let env = [std::ptr::null::<libc::c_char>()];
        let input = peer.as_raw_fd();
        let selected = executable.file.as_raw_fd();
        let null = executable.null.as_raw_fd();
        // The owner is single threaded. Allocate everything before fork; the
        // child uses only libc/kernel operations and never Rust destructors.
        // No exec handshake is waited on by the responsive owner.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            unsafe {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0
                    || libc::getppid() != parent_pid as libc::pid_t
                    || libc::chdir(c"/".as_ptr()) != 0
                    || libc::dup2(input, 0) < 0
                    || libc::dup2(null, 1) < 0
                    || libc::dup2(null, 2) < 0
                    || libc::dup2(selected, 3) < 0
                    || libc::fcntl(3, libc::F_SETFD, libc::FD_CLOEXEC) < 0
                    || libc::syscall(libc::SYS_close_range, 4u32, u32::MAX, 0u32) != 0
                {
                    libc::_exit(125);
                }
                libc::syscall(
                    libc::SYS_execveat,
                    3,
                    c"".as_ptr(),
                    argv.as_ptr(),
                    env.as_ptr(),
                    libc::AT_EMPTY_PATH,
                );
                libc::_exit(127);
            }
        }
        let child = OwnedChild { pid, reaped: false };
        // Retain the exact child before fallible pidfd work; even an exec-held
        // or exec-failed child occupies the slot through actual waitpid.
        let watch = Pid::from_raw(child.id() as i32)
            .and_then(|p| rustix::process::pidfd_open(p, PidfdFlags::NONBLOCK).ok());
        let mut owner = Self {
            child: Some(child),
            watch,
            channel,
            job,
            bootstrap: Some(bootstrap),
            ack: None,
            accepted: false,
            terminal: false,
        };
        if owner.watch.is_none() || boottime_ns() >= owner.job.until_ns {
            owner.fence();
        }
        Ok(owner)
    }
    fn fence(&mut self) {
        self.terminal = true;
        self.bootstrap = None;
        self.ack = None;
        if let Some(watch) = &self.watch {
            let _ = rustix::process::pidfd_send_signal(watch, rustix::process::Signal::KILL);
        } else if let Some(child) = &mut self.child {
            // SIGCHLD is normalized before fork, so this directly owned child
            // cannot have its PID reused before our own successful waitpid.
            let _ = child.kill();
        }
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(child) = &mut self.child
            && child.try_wait()?.is_some()
        {
            self.child = None;
            return Ok(Some(Event::Reaped));
        }
        if boottime_ns() >= self.job.until_ns {
            self.fence();
        }
        if self.terminal {
            return Ok(None);
        }
        for pending in [&mut self.bootstrap, &mut self.ack] {
            if let Some(bytes) = pending {
                match ipc::send(&self.channel, bytes) {
                    Ok(()) => *pending = None,
                    Err(e) if matches!(e.raw_os_error(), Some(libc::EINTR | libc::EAGAIN)) => {
                        return Ok(None);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        if self.accepted {
            return Ok(None);
        }
        if let Some((wire, cred)) = ipc::receive(&self.channel)? {
            let completion: Completion = serde_json::from_slice(&wire)?;
            if cred.uid.as_raw() != DESKTOP_UID
                || cred.gid.as_raw() != DESKTOP_UID
                || Some(cred.pid.as_raw_pid() as u32) != self.child.as_ref().map(OwnedChild::id)
                || !self
                    .watch
                    .as_ref()
                    .is_some_and(|fd| ipc::living(fd).unwrap_or(false))
                || completion.owner != self.job.owner
                || completion.token != self.job.token
                || completion.until_ns != self.job.until_ns
                || boottime_ns() >= self.job.until_ns
                || completion.appearance.as_ref().is_some_and(|p| !p.valid())
            {
                self.fence();
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            return Ok(Some(Event::Candidate(Box::new(completion))));
        }
        Ok(None)
    }
    fn accept(&mut self) -> io::Result<()> {
        if self.terminal
            || boottime_ns() >= self.job.until_ns
            || !self
                .watch
                .as_ref()
                .is_some_and(|fd| ipc::living(fd).unwrap_or(false))
        {
            self.fence();
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.ack = Some(serde_json::to_vec(&Ack {
            owner: self.job.owner.clone(),
            token: self.job.token,
        })?);
        self.accepted = true;
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // Normal stop never drops this slot until actual parent try_wait.
        if self.child.is_some() {
            std::process::abort();
        }
    }
}
struct Client {
    fd: OwnedFd,
    reply: Option<(Vec<u8>, u64)>,
    idle_until: u64,
    control: bool,
}
impl Client {
    /// Returns false when the owning loop must close this client. Revalidate
    /// immediately before every send, including retries after real backpressure.
    fn flush_reply(
        &mut self,
        owner: &str,
        revision: u32,
        current: Option<&(Appearance, u64)>,
        stopping: bool,
    ) -> io::Result<bool> {
        if let Some((wire, until)) = &self.reply {
            if boottime_ns() >= *until {
                return Ok(false);
            }
            // A queued available publication never outlives its collection.
            if let Ok(snapshot) = serde_json::from_slice::<Snapshot>(wire) {
                let now = boottime_ns();
                let available = !stopping && current.is_some_and(|(_, until)| now < *until);
                if snapshot.owner != owner
                    || snapshot.revision != revision
                    || snapshot.available != available
                    || (snapshot.available
                        && (snapshot.appearance.as_ref() != current.map(|(p, _)| p)
                            || !snapshot.until_ns.parse::<u64>().is_ok_and(|t| now < t)))
                {
                    return Ok(false);
                }
            }
            match ipc::send(&self.fd, wire) {
                Ok(()) => {
                    self.reply = None;
                    self.idle_until = boottime_ns().saturating_add(500_000_000);
                }
                Err(e) if matches!(e.raw_os_error(), Some(libc::EAGAIN | libc::EINTR)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(true)
    }
}

pub fn trusted_path(path: &Path, executable: bool) -> io::Result<()> {
    if !path.is_absolute()
        || path.components().any(|p| {
            matches!(
                p,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file()
        || meta.uid() != 0
        || meta.mode() & 0o6022 != 0
        || (executable && meta.mode() & 0o111 == 0)
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    trusted_directory(path.parent().ok_or(io::ErrorKind::InvalidInput)?)
}
fn trusted_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for ancestor in path.ancestors() {
        let m = std::fs::symlink_metadata(ancestor)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}

fn owner_lock(parent: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(parent.join("appearance-owner.lock"))?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != 0 || m.gid() != 0 || m.nlink() != 1 || m.mode() & 0o077 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
    Ok(file)
}

fn retire_socket(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_socket() && m.uid() == 0 && m.gid() == 0 => {
            std::fs::remove_file(path)
        }
        Ok(_) => Err(io::ErrorKind::PermissionDenied.into()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn run(path: &Path, config: Configuration, executable: &Path) -> io::Result<()> {
    if rustix::process::getuid().as_raw() != 0
        || rustix::process::getgid().as_raw() != 0
        || config.home.len() > 1024
        || config.runtime.len() > 1024
        || !Path::new(&config.home).is_absolute()
        || !Path::new(&config.runtime).is_absolute()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    trusted_directory(parent)?;
    trusted_path(executable, true)?;
    // Exec preserves an inherited SIG_IGN disposition. Explicitly establish
    // waitable children and clear SA_NOCLDWAIT before creating any worker.
    let mut child_action: libc::sigaction = unsafe { std::mem::zeroed() };
    child_action.sa_sigaction = libc::SIG_DFL;
    if unsafe { libc::sigemptyset(&mut child_action.sa_mask) } != 0
        || unsafe { libc::sigaction(libc::SIGCHLD, &child_action, std::ptr::null_mut()) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let executable = Executable::open(executable)?;
    // Only an exclusive owner may retire stale socket nodes. A live successor
    // attempt must fail before touching either endpoint. This filesystem lock
    // does not establish a dead predecessor's process-tree closure.
    let _owner_lock = owner_lock(parent)?;
    let control_path = path.with_file_name("appearance-control.sock");
    retire_socket(path)?;
    retire_socket(&control_path)?;
    let listener = ipc::listener(path, 0o666)?;
    let control_listener = ipc::listener(&control_path, 0o600)?;
    let control_identity = std::fs::symlink_metadata(&control_path)?;
    let socket_identity = std::fs::symlink_metadata(path)?;
    for signal in [libc::SIGTERM, libc::SIGINT] {
        if unsafe { libc::signal(signal, request_stop as *const () as libc::sighandler_t) }
            == libc::SIG_ERR
        {
            return Err(io::Error::last_os_error());
        }
    }
    let owner = fresh_id()?;
    let mut revision = 0u32;
    let mut token = 0u64;
    let mut completed = 0u64;
    let mut current: Option<(Appearance, u64)> = None;
    let mut worker: Option<Worker> = None;
    let mut clients: Vec<Client> = Vec::new();
    let mut refresh = 0;
    let mut stopping = false;
    loop {
        let now = boottime_ns();
        if STOP.load(Ordering::Relaxed) {
            stopping = true;
        }
        if stopping {
            current = None;
            if let Some(w) = &mut worker
                && !w.terminal
            {
                w.fence();
            }
        }
        if let Some(w) = &mut worker {
            match w.poll() {
                Ok(Some(Event::Candidate(candidate))) => {
                    let new_revision =
                        if current.as_ref().map(|(p, _)| p) != candidate.appearance.as_ref() {
                            revision.checked_add(1)
                        } else {
                            Some(revision)
                        };
                    if let Some(new_revision) = new_revision
                        && w.accept().is_ok()
                        && boottime_ns() < candidate.until_ns
                    {
                        revision = new_revision;
                        current = candidate
                            .appearance
                            .map(|palette| (palette, candidate.until_ns));
                        completed += 1;
                    } else {
                        w.fence();
                        current = None;
                    }
                }
                Ok(Some(Event::Reaped)) => {
                    worker = None;
                    refresh = now.saturating_add(REFRESH_NS);
                }
                Ok(None) => {}
                Err(_) => {
                    w.fence();
                    current = None;
                }
            }
        }
        if !stopping && worker.is_none() && now >= refresh {
            if let Some(next) = token.checked_add(1) {
                token = next;
                let job = Job {
                    owner: owner.clone(),
                    token,
                    until_ns: boottime_ns().saturating_add(COLLECTION_BUDGET_NS),
                    config: config.clone(),
                };
                match Worker::spawn(&executable, job) {
                    Ok(w) => worker = Some(w),
                    Err(_) => {
                        current = None;
                        refresh = now.saturating_add(REFRESH_NS);
                    }
                }
            } else {
                stopping = true;
            }
        }
        // Bounded work per client/turn; no filesystem traversal or helper waits.
        for (server, control, limit) in [(&control_listener, true, 8), (&listener, false, 16)] {
            if clients.iter().filter(|c| c.control == control).count() < limit {
                match net::accept_with(server, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
                    Ok(fd) => {
                        if net::sockopt::set_socket_passcred(&fd, true).is_ok() {
                            clients.push(Client {
                                fd,
                                reply: None,
                                idle_until: boottime_ns().saturating_add(500_000_000),
                                control,
                            });
                        }
                    }
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                    Err(_) => {
                        stopping = true;
                    }
                }
            }
        }
        clients.retain_mut(|client| {
            let result = (|| -> io::Result<bool> {
                let now = boottime_ns();
                if client.reply.is_none() && now >= client.idle_until {
                    return Ok(false);
                }
                if let Some((_, until)) = &client.reply
                    && now >= *until
                {
                    return Ok(false);
                }
                if client.reply.is_none()
                    && let Some((wire, cred)) = ipc::receive(&client.fd)?
                {
                    let request: Request = serde_json::from_slice(&wire)?;
                    let available =
                        !stopping && current.as_ref().is_some_and(|(_, until)| now < *until);
                    let reply = match request {
                        Request::Snapshot => serde_json::to_vec(&Snapshot {
                            schema: 2,
                            owner: owner.clone(),
                            revision,
                            until_ns: if available {
                                current.as_ref().unwrap().1
                            } else {
                                now.saturating_add(COLLECTION_BUDGET_NS)
                            }
                            .to_string(),
                            available,
                            appearance: if available {
                                Some(current.as_ref().unwrap().0.clone())
                            } else {
                                None
                            },
                        })?,
                        Request::Status
                            if client.control
                                && cred.uid.as_raw() == 0
                                && cred.gid.as_raw() == 0 =>
                        {
                            serde_json::to_vec(&Status {
                                owner: &owner,
                                state: if stopping {
                                    "stopping"
                                } else if worker.as_ref().is_some_and(|w| w.terminal) {
                                    "reaping"
                                } else if worker.is_some() {
                                    "collecting"
                                } else {
                                    "idle"
                                },
                                worker_pid: worker
                                    .as_ref()
                                    .and_then(|w| w.child.as_ref().map(OwnedChild::id)),
                                worker_until_ns: worker
                                    .as_ref()
                                    .map(|w| w.job.until_ns.to_string()),
                                completed,
                                revision,
                                available,
                            })?
                        }
                        Request::Stop
                            if client.control
                                && cred.uid.as_raw() == 0
                                && cred.gid.as_raw() == 0 =>
                        {
                            stopping = true;
                            current = None;
                            if let Some(w) = &mut worker {
                                w.fence();
                            }
                            serde_json::to_vec(&serde_json::json!({"result":"stopping"}))?
                        }
                        _ => return Ok(false),
                    };
                    client.reply = Some((reply, now.saturating_add(500_000_000)));
                }
                client.flush_reply(&owner, revision, current.as_ref(), stopping)
            })();
            result.unwrap_or(false)
        });
        if stopping && worker.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    drop((clients, listener, control_listener));
    if std::fs::symlink_metadata(&control_path)
        .is_ok_and(|m| (m.dev(), m.ino()) == (control_identity.dev(), control_identity.ino()))
    {
        std::fs::remove_file(&control_path)?;
    }
    if std::fs::symlink_metadata(path)
        .is_ok_and(|m| (m.dev(), m.ino()) == (socket_identity.dev(), socket_identity.ino()))
    {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod queued_reply_controls {
    //! Real Linux SEQPACKET backpressure on the production flush path, with
    //! same-process owner state setup. These are not whole-service, original
    //! Omarchy, emulator, or physical-device controls.
    use super::*;
    use serde_json::json;

    const OWNER: &str = "0123456789abcdef0123456789abcdef";
    const REVISION: u32 = 7;

    struct HeldReply {
        client: Client,
        peer: OwnedFd,
        current: (Appearance, u64),
        wire: Vec<u8>,
        filler: Vec<u8>,
        filled: usize,
        queued_ns: u64,
        eagain_ns: u64,
        retained_ns: u64,
        transport_until_ns: u64,
    }
    impl HeldReply {
        fn new(publication_lifetime_ns: u64) -> Self {
            let (fd, peer) = net::socketpair(
                net::AddressFamily::UNIX,
                net::SocketType::SEQPACKET,
                SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
                None,
            )
            .unwrap();
            net::sockopt::set_socket_passcred(&peer, true).unwrap();
            net::sockopt::set_socket_send_buffer_size(&fd, 4096).unwrap();
            let queued_ns = boottime_ns();
            let current = (
                crate::style::project(
                    "background = '#1a1b26'\nforeground = '#a9b1d6'\naccent = '#7aa2f7'\nred = '#f7768e'\n",
                    "",
                    "",
                )
                .unwrap(),
                queued_ns + publication_lifetime_ns,
            );
            let wire = serde_json::to_vec(&Snapshot {
                schema: 2,
                owner: OWNER.into(),
                revision: REVISION,
                until_ns: current.1.to_string(),
                available: true,
                appearance: Some(current.0.clone()),
            })
            .unwrap();
            let transport_until_ns = queued_ns + 500_000_000;
            let client = Client {
                fd,
                reply: Some((wire.clone(), transport_until_ns)),
                idle_until: transport_until_ns,
                control: false,
            };
            // Equal-size non-JSON packets occupy the actual sender queue. The
            // upper bound makes failure to obtain kernel backpressure explicit.
            let filler = vec![b'x'; wire.len()];
            let mut filled = 0;
            loop {
                match ipc::send(&client.fd, &filler) {
                    Ok(()) => {
                        filled += 1;
                        assert!(filled < 4096, "SEQPACKET queue did not reach EAGAIN");
                    }
                    Err(error) => {
                        assert_eq!(error.raw_os_error(), Some(libc::EAGAIN));
                        break;
                    }
                }
            }
            assert!(filled > 0, "queue must contain actual kernel packets");
            // Prove this exact serialized snapshot also gets actual EAGAIN;
            // inferring pressure merely from the filler is insufficient.
            let error = ipc::send(&client.fd, &wire).unwrap_err();
            assert_eq!(error.raw_os_error(), Some(libc::EAGAIN));
            let eagain_ns = boottime_ns();
            let mut held = Self {
                client,
                peer,
                current,
                wire,
                filler,
                filled,
                queued_ns,
                eagain_ns,
                retained_ns: 0,
                transport_until_ns,
            };
            assert!(
                held.client
                    .flush_reply(OWNER, REVISION, Some(&held.current), false)
                    .unwrap()
            );
            assert_eq!(
                held.client.reply.as_ref().map(|(wire, _)| wire),
                Some(&held.wire),
                "the production flush must retain the actual backpressured reply"
            );
            held.retained_ns = boottime_ns();
            assert!(held.retained_ns < held.current.1);
            held
        }
        fn release_and_prove_writable(&self) -> u64 {
            for _ in 0..self.filled {
                let (packet, credentials) = ipc::receive(&self.peer).unwrap().unwrap();
                assert_eq!(packet, self.filler, "only preexisting filler may drain");
                assert_eq!(credentials.pid.as_raw_pid() as u32, std::process::id());
            }
            assert!(ipc::receive(&self.peer).unwrap().is_none());
            let probe = b"writable-after-invalidation";
            ipc::send(&self.client.fd, probe).unwrap();
            let (packet, _) = ipc::receive(&self.peer).unwrap().unwrap();
            assert_eq!(packet, probe);
            boottime_ns()
        }
        fn invalidated(mut self, cause: &str) {
            let mut revision = REVISION;
            let mut stopping = false;
            let mut current = Some(self.current.clone());
            let mut renewed_ns = None;
            match cause {
                "publication_expiry" => {
                    // An accepted same-color collection refreshes availability
                    // without changing revision. It cannot renew an already
                    // serialized snapshot's original deadline.
                    current.as_mut().unwrap().1 = self.transport_until_ns + 500_000_000;
                    renewed_ns = Some(boottime_ns());
                    assert!(
                        self.client
                            .flush_reply(OWNER, revision, current.as_ref(), stopping)
                            .unwrap()
                    );
                    assert!(self.client.reply.is_some());
                    while boottime_ns() < self.current.1 {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    assert!(boottime_ns() < current.as_ref().unwrap().1);
                }
                "revision_change" => {
                    revision += 1;
                    current.as_mut().unwrap().0.palette.background = "#eff1f5".into();
                }
                "stop" => {
                    stopping = true;
                    current = None;
                }
                _ => panic!("unknown invalidation control"),
            }
            let invalidated_ns = boottime_ns();
            assert!(self.queued_ns <= self.eagain_ns);
            assert!(self.eagain_ns <= self.retained_ns);
            assert!(self.retained_ns <= invalidated_ns);
            let writable_ns = self.release_and_prove_writable();
            assert!(invalidated_ns <= writable_ns);
            assert!(
                writable_ns < self.transport_until_ns,
                "test must invalidate before the independent reply timeout"
            );
            assert!(
                !self
                    .client
                    .flush_reply(OWNER, revision, current.as_ref(), stopping)
                    .unwrap(),
                "invalidated snapshot must be refused even on a writable socket"
            );
            assert!(self.client.reply.is_some());
            let rejected_ns = boottime_ns();
            assert!(rejected_ns < self.transport_until_ns);
            // This is the owning retain loop's close action. An erroneous send
            // would be visible before EOF even if the flush reported refusal.
            drop(self.client);
            let error = ipc::receive(&self.peer).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
            eprintln!(
                "queued_reply_control {}",
                json!({
                    "cause":cause, "boundary":"same_process_real_seqpacket",
                    "queued_ns":self.queued_ns, "actual_eagain_ns":self.eagain_ns,
                    "retained_ns":self.retained_ns, "invalidated_ns":invalidated_ns,
                    "writable_ns":writable_ns, "rejected_ns":rejected_ns,
                    "renewed_ns":renewed_ns,
                    "snapshot_until_ns":self.current.1,
                    "current_until_ns":current.as_ref().map(|(_, until)| until),
                    "current_revision":revision, "current_stopping":stopping,
                    "transport_until_ns":self.transport_until_ns,
                    "kernel_filler_packets":self.filled,
                    "snapshot_packets_after_invalidation":0, "actual_eof":true
                })
            );
        }
    }

    #[test]
    fn real_seqpacket_eagain_queued_reply_expires_before_transport_timeout() {
        HeldReply::new(50_000_000).invalidated("publication_expiry");
    }

    #[test]
    fn real_seqpacket_eagain_queued_reply_rejects_new_revision() {
        HeldReply::new(1_000_000_000).invalidated("revision_change");
    }

    #[test]
    fn real_seqpacket_eagain_queued_reply_rejects_stop() {
        HeldReply::new(1_000_000_000).invalidated("stop");
    }

    #[test]
    fn real_seqpacket_eagain_unchanged_queued_reply_sends_exactly_once() {
        let mut held = HeldReply::new(1_000_000_000);
        let writable_ns = held.release_and_prove_writable();
        assert!(writable_ns < held.transport_until_ns);
        assert!(
            held.client
                .flush_reply(OWNER, REVISION, Some(&held.current), false)
                .unwrap()
        );
        assert!(held.client.reply.is_none());
        let (packet, _) = ipc::receive(&held.peer).unwrap().unwrap();
        assert_eq!(packet, held.wire);
        assert!(
            held.client
                .flush_reply(OWNER, REVISION, Some(&held.current), false)
                .unwrap()
        );
        assert!(ipc::receive(&held.peer).unwrap().is_none());
        drop(held.client);
        assert_eq!(
            ipc::receive(&held.peer).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        eprintln!(
            "queued_reply_control {}",
            json!({
                "cause":"unchanged_positive", "boundary":"same_process_real_seqpacket",
                "queued_ns":held.queued_ns, "actual_eagain_ns":held.eagain_ns,
                "retained_ns":held.retained_ns, "writable_ns":writable_ns,
                "kernel_filler_packets":held.filled, "snapshot_packets":1,
                "duplicate_snapshot_packets":0, "actual_eof":true
            })
        );
    }
}
