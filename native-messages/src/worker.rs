//! One persistent storage child. Local SQLite recovery never replays a modem effect.
use crate::{
    projection::Store,
    protocol::{Page, Query, StorageStatus},
};
use pokelinux_lifecycle::{
    authority::living,
    boottime_ns, fresh_id, ipc,
    messages::{Event, Provider},
};
use rustix::{
    fd::OwnedFd,
    net::{self, SocketFlags},
    process::{Pid, PidfdFlags},
};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source: String,
    token: u64,
    until: u64,
    query: Option<Query>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    source: String,
    token: u64,
    status: StorageStatus,
    page: Option<Page>,
}
struct Pending {
    token: u64,
    until: u64,
    wire: Option<Vec<u8>>,
    init: bool,
}
struct Worker {
    child: Option<Child>,
    alive: OwnedFd,
    listener: OwnedFd,
    fd: Option<OwnedFd>,
    source: String,
    pending: Option<Pending>,
    sequence: u64,
    terminal: bool,
    ready: bool,
    path: PathBuf,
    identity: (u64, u64),
}
impl Worker {
    fn spawn(runtime: &Path, raw: &Path, inbox: &Path, exe: &Path) -> io::Result<Self> {
        pokelinux_lifecycle::service::trusted_executable(exe)?;
        let path = runtime.join("messages-storage.sock");
        let listener = ipc::listener(&path, 0o600)?;
        let m = fs::symlink_metadata(&path)?;
        let source = fresh_id()?;
        let launch = Command::new(exe)
            .arg("--messages-worker")
            .arg(&path)
            .arg(raw)
            .arg(inbox)
            .arg(std::process::id().to_string())
            .arg(&source)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match launch {
            Ok(c) => c,
            Err(e) => {
                if fs::symlink_metadata(&path)
                    .is_ok_and(|meta| (meta.dev(), meta.ino()) == (m.dev(), m.ino()))
                {
                    let _ = fs::remove_file(&path);
                }
                return Err(e);
            }
        };
        let alive = match rustix::process::pidfd_open(
            Pid::from_raw(child.id() as i32).ok_or(io::ErrorKind::Other)?,
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
                            std::thread::sleep(std::time::Duration::from_millis(2))
                        }
                        _ => std::process::abort(),
                    }
                }
                return Err(e.into());
            }
        };
        let until = boottime_ns().saturating_add(1_000_000_000);
        let wire = serde_json::to_vec(&Request {
            source: source.clone(),
            token: 1,
            until,
            query: None,
        })?;
        Ok(Self {
            child: Some(child),
            alive,
            listener,
            fd: None,
            source,
            pending: Some(Pending {
                token: 1,
                until,
                wire: Some(wire),
                init: true,
            }),
            sequence: 1,
            terminal: false,
            ready: false,
            path,
            identity: (m.dev(), m.ino()),
        })
    }
    fn stop(&mut self) {
        self.terminal = true;
        self.ready = false;
        if let Some(c) = &mut self.child {
            let _ = c.kill();
        }
    }
    fn fail(&mut self) -> Option<Event> {
        self.stop();
        self.pending
            .take()
            .filter(|p| !p.init)
            .map(|p| Event::Failed { token: p.token })
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(c) = &mut self.child
            && c.try_wait()?.is_some()
        {
            self.child = None;
            return Ok(self.fail());
        }
        if self.terminal {
            return Ok(None);
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| boottime_ns() >= p.until)
        {
            return Ok(self.fail());
        }
        if self.fd.is_none() {
            match net::accept_with(&self.listener, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
                Ok(fd) => {
                    net::sockopt::set_socket_passcred(&fd, true)?;
                    let c = net::sockopt::socket_peercred(&fd)?;
                    if c.uid.as_raw() == 0
                        && Some(c.pid.as_raw_nonzero().get() as u32)
                            == self.child.as_ref().map(Child::id)
                    {
                        self.fd = Some(fd)
                    }
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let (Some(fd), Some(p)) = (&self.fd, &mut self.pending) else {
            return Ok(None);
        };
        if let Some(bytes) = &p.wire {
            match ipc::send(fd, bytes) {
                Ok(()) => p.wire = None,
                Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => return Ok(None),
                Err(_) => return Ok(self.fail()),
            }
        }
        match ipc::receive(fd) {
            Ok(Some((bytes, c))) => {
                let p = self.pending.take().ok_or(io::ErrorKind::Other)?;
                let response = serde_json::from_slice::<Response>(&bytes);
                if c.uid.as_raw() != 0
                    || Some(c.pid.as_raw_nonzero().get() as u32)
                        != self.child.as_ref().map(Child::id)
                    || !living(&self.alive)?
                    || boottime_ns() >= p.until
                {
                    self.stop();
                    return Ok((!p.init).then_some(Event::Failed { token: p.token }));
                }
                match response {
                    Ok(r) if r.source == self.source && r.token == p.token => {
                        if r.status != StorageStatus::Ready {
                            self.stop();
                            return Ok((!p.init).then_some(Event::Failed { token: p.token }));
                        }
                        if p.init && r.page.is_none() {
                            self.ready = true;
                            Ok(None)
                        } else if !p.init && r.page.is_some() {
                            let packet =
                                serde_json::to_vec(&r.page.ok_or(io::ErrorKind::InvalidData)?)?;
                            if packet.len() > 4096 {
                                self.stop();
                                Ok(Some(Event::Failed { token: p.token }))
                            } else {
                                Ok(Some(Event::Complete {
                                    token: p.token,
                                    packet,
                                }))
                            }
                        } else {
                            self.stop();
                            Ok((!p.init).then_some(Event::Failed { token: p.token }))
                        }
                    }
                    _ => {
                        self.stop();
                        Ok((!p.init).then_some(Event::Failed { token: p.token }))
                    }
                }
            }
            Ok(None) => Ok(None),
            Err(e) if e.raw_os_error() == Some(4) => Ok(None),
            Err(_) => Ok(self.fail()),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if self.child.is_some() {
            std::process::abort()
        }
        if fs::symlink_metadata(&self.path).is_ok_and(|m| (m.dev(), m.ino()) == self.identity) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
pub struct Projector {
    worker: Option<Worker>,
    runtime: PathBuf,
    raw: PathBuf,
    inbox: PathBuf,
    exe: PathBuf,
}
impl Projector {
    pub fn spawn(runtime: &Path, raw: &Path, inbox: &Path, exe: &Path) -> io::Result<Self> {
        Ok(Self {
            worker: Some(Worker::spawn(runtime, raw, inbox, exe)?),
            runtime: runtime.into(),
            raw: raw.into(),
            inbox: inbox.into(),
            exe: exe.into(),
        })
    }
}
impl Provider for Projector {
    fn incarnation(&self) -> &str {
        self.worker.as_ref().map_or("", |w| w.source.as_str())
    }
    fn occupied(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.child.is_some())
    }
    fn busy(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.pending.is_some())
    }
    fn available(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| {
            w.ready && !w.terminal && w.child.is_some() && living(&w.alive).unwrap_or(false)
        })
    }
    fn start(&mut self, query: serde_json::Value, until: u64) -> io::Result<u64> {
        let q: Query = serde_json::from_value(query)?;
        q.validate().map_err(|_| io::ErrorKind::InvalidInput)?;
        let w = self.worker.as_mut().ok_or(io::ErrorKind::NotConnected)?;
        if !w.ready || w.terminal || w.pending.is_some() || boottime_ns() >= until {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        w.sequence = w.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        let wire = serde_json::to_vec(&Request {
            source: w.source.clone(),
            token: w.sequence,
            until,
            query: Some(q),
        })?;
        if wire.len() > 4096 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        w.pending = Some(Pending {
            token: w.sequence,
            until,
            wire: Some(wire),
            init: false,
        });
        Ok(w.sequence)
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        match self.worker.as_mut() {
            Some(w) => w.poll(),
            None => Ok(None),
        }
    }
    fn stop(&mut self) {
        if let Some(w) = &mut self.worker {
            w.stop()
        }
    }
    fn restart(&mut self) -> io::Result<()> {
        if self.occupied() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.worker = None;
        self.worker = Some(Worker::spawn(
            &self.runtime,
            &self.raw,
            &self.inbox,
            &self.exe,
        )?);
        Ok(())
    }
    fn drain_until(&mut self, until: u64) -> bool {
        self.stop();
        while self.occupied() {
            if boottime_ns() >= until || self.poll().is_err() {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        true
    }
}
/// Private installed child entry. No incoming PDU, ACK or send endpoint exists.
pub fn entry(args: &[std::ffi::OsString]) -> io::Result<()> {
    if args.len() != 5 || rustix::process::getuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let parent: u32 = args[3]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    if rustix::process::getppid().map(|p| p.as_raw_nonzero().get() as u32) != Some(parent) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let fd = ipc::connect(Path::new(&args[0]))?;
    let source = args[4].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let mut store = Store::open(Path::new(&args[1]), Path::new(&args[2]))
        .map_err(|_| io::ErrorKind::InvalidData)?;
    let mut sequence = 0;
    loop {
        let Some((bytes, c)) = ipc::receive(&fd)? else {
            std::thread::sleep(std::time::Duration::from_millis(2));
            continue;
        };
        if c.uid.as_raw() != 0 || c.pid.as_raw_nonzero().get() as u32 != parent {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let r: Request = serde_json::from_slice(&bytes)?;
        if r.source != source || r.token != sequence + 1 || boottime_ns() >= r.until {
            return Err(io::ErrorKind::InvalidData.into());
        }
        sequence = r.token;
        // Every retry/restart only reconciles exact pre-existing local receipt IDs.
        let status = store.reconcile().map_err(|_| io::ErrorKind::InvalidData)?;
        let page = if let Some(q) = r.query {
            Some(store.query(&q).map_err(|_| io::ErrorKind::InvalidData)?)
        } else {
            None
        };
        let response = serde_json::to_vec(&Response {
            source: source.into(),
            token: r.token,
            status,
            page,
        })?;
        loop {
            if boottime_ns() >= r.until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match ipc::send(&fd, &response) {
                Ok(()) => break,
                Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                    std::thread::sleep(std::time::Duration::from_millis(2))
                }
                Err(e) => return Err(e),
            }
        }
    }
}
