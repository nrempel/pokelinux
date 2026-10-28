//! One receipt/projection writer and an independently supervised readonly child.
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
    collections::BTreeMap,
    ffi::OsString,
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source: String,
    token: u64,
    until: u64,
    query: Option<Query>,
    upper: Option<u64>,
    projection: Option<ProjectionPermit>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionPermit {
    occurrence: u64,
    binding: Binding,
    until: u64,
    receipt: String,
}
struct ReservedProjection {
    occurrence: u64,
    wire: Option<Vec<u8>>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    pub slot: u8,
    pub incarnation: String,
    pub generation: String,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Response {
    Read {
        source: String,
        token: u64,
        status: StorageStatus,
        page: Option<Page>,
        receiving: bool,
        upper: u64,
    },
    SourceBound {
        source: String,
        binding: Binding,
    },
    ArrivalStarted {
        source: String,
        occurrence: u64,
        binding: Binding,
        until: u64,
    },
    RawCommitted {
        source: String,
        occurrence: u64,
        binding: Binding,
        until: u64,
        receipt: String,
    },
    ArrivalComplete {
        upper: u64,
        source: String,
        occurrence: u64,
        binding: Binding,
        until: u64,
        receipt: String,
    },
    SourceFailed {
        source: String,
        reason: String,
    },
}
impl Response {
    fn source(&self) -> &str {
        match self {
            Self::Read { source, .. }
            | Self::SourceBound { source, .. }
            | Self::ArrivalStarted { source, .. }
            | Self::RawCommitted { source, .. }
            | Self::ArrivalComplete { source, .. }
            | Self::SourceFailed { source, .. } => source,
        }
    }
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
    incoming_sequence: u64,
    incoming: BTreeMap<u64, (Binding, u64, Option<String>)>,
    raw_committed: u64,
    incoming_completed: u64,
    bindings: Vec<Binding>,
    generation: Option<String>,
    receiving: bool,
    failure: Option<String>,
    terminal: bool,
    ready: bool,
    path: PathBuf,
    identity: (u64, u64),
    reader: bool,
    upper: u64,
    projection: Option<ReservedProjection>,
}
impl Worker {
    fn spawn(
        runtime: &Path,
        raw: &Path,
        inbox: &Path,
        exe: &Path,
        incoming_args: &[OsString],
        read_upper: Option<u64>,
        reader_hold: bool,
    ) -> io::Result<Self> {
        pokelinux_lifecycle::service::trusted_executable(exe)?;
        let path = runtime.join(if read_upper.is_some() {
            "messages-read.sock"
        } else {
            "messages-storage.sock"
        });
        let listener = ipc::listener(&path, 0o600)?;
        let m = fs::symlink_metadata(&path)?;
        let source = fresh_id()?;
        let launch = Command::new(exe)
            .arg(if read_upper.is_some() && reader_hold {
                "--messages-held-read-fixture-worker"
            } else if read_upper.is_some() {
                "--messages-read-worker"
            } else if incoming_args.is_empty() {
                "--messages-worker"
            } else {
                "--messages-incoming-worker"
            })
            .arg(&path)
            .arg(raw)
            .arg(inbox)
            .arg(std::process::id().to_string())
            .arg(&source)
            .args(incoming_args)
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
        let until = boottime_ns()
            .checked_add(if incoming_args.is_empty() {
                1_000_000_000
            } else {
                3_000_000_000
            })
            .ok_or(io::ErrorKind::Other)?;
        let wire = serde_json::to_vec(&Request {
            source: source.clone(),
            token: 1,
            until,
            query: None,
            upper: read_upper,
            projection: None,
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
            incoming_sequence: 0,
            incoming: BTreeMap::new(),
            raw_committed: 0,
            incoming_completed: 0,
            bindings: Vec::new(),
            generation: incoming_args
                .first()
                .and_then(|v| v.to_str())
                .map(str::to_owned),
            receiving: false,
            failure: None,
            terminal: false,
            ready: false,
            path,
            identity: (m.dev(), m.ino()),
            reader: read_upper.is_some(),
            upper: read_upper.unwrap_or(0),
            projection: None,
        })
    }
    fn stop(&mut self) {
        self.terminal = true;
        self.ready = false;
        self.receiving = false;
        self.projection = None;
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
    // Reserve the exact active occurrence before transmitting its grant. Reader
    // admission remains fenced until this occurrence's authenticated completion.
    fn reserve_projection(&mut self) -> io::Result<()> {
        if self.reader || !self.ready || self.terminal || self.projection.is_some() {
            return Ok(());
        }
        let Some((&occurrence, (binding, until, Some(receipt)))) = self.incoming.first_key_value()
        else {
            return Ok(());
        };
        if boottime_ns() >= *until {
            return Ok(());
        }
        let token = self.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        let wire = serde_json::to_vec(&Request {
            source: self.source.clone(),
            token,
            until: *until,
            query: None,
            upper: None,
            projection: Some(ProjectionPermit {
                occurrence,
                binding: binding.clone(),
                until: *until,
                receipt: receipt.clone(),
            }),
        })?;
        if wire.len() > 4096 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.sequence = token;
        self.projection = Some(ReservedProjection {
            occurrence,
            wire: Some(wire),
        });
        Ok(())
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(c) = &mut self.child
            && let Some(status) = c.try_wait()?
        {
            self.child = None;
            if self.failure.is_none() {
                self.failure = status.code().map(|code| format!("child_exit_{code}"));
            }
            return Ok(self.fail());
        }
        if self.terminal {
            return Ok(None);
        }
        let now = boottime_ns();
        if self.pending.as_ref().is_some_and(|p| now >= p.until)
            || self.incoming.values().any(|(_, until, _)| now >= *until)
        {
            self.failure = Some("worker_or_arrival_deadline".into());
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
                        self.fd = Some(fd);
                    }
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let Some(fd) = &self.fd else {
            return Ok(None);
        };
        if let Some(p) = &mut self.pending
            && let Some(wire) = &p.wire
        {
            match ipc::send(fd, wire) {
                Ok(()) => p.wire = None,
                Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => return Ok(None),
                Err(_) => return Ok(self.fail()),
            }
        }
        if let Some(reserved) = &mut self.projection
            && let Some(wire) = &reserved.wire
        {
            match ipc::send(fd, wire) {
                Ok(()) => reserved.wire = None,
                Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => return Ok(None),
                Err(_) => return Ok(self.fail()),
            }
        }
        // Always drain authenticated child packets, including with no app/read.
        let received = match ipc::receive(fd) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(None),
            Err(e) if e.raw_os_error() == Some(4) => return Ok(None),
            Err(_) => return Ok(self.fail()),
        };
        let (bytes, c) = received;
        if c.uid.as_raw() != 0
            || Some(c.pid.as_raw_nonzero().get() as u32) != self.child.as_ref().map(Child::id)
            || !living(&self.alive)?
        {
            return Ok(self.fail());
        }
        let response = match serde_json::from_slice::<Response>(&bytes) {
            Ok(r) if r.source() == self.source => r,
            _ => return Ok(self.fail()),
        };
        let now = boottime_ns();
        let valid_binding = |b: &Binding| {
            b.slot < 8
                && crate::protocol::DataId::parse(&b.incarnation).is_ok()
                && crate::protocol::DataId::parse(&b.generation).is_ok()
                && self.generation.as_deref() == Some(b.generation.as_str())
        };
        match response {
            Response::SourceBound { binding, .. } => {
                if self.ready
                    || !self.pending.as_ref().is_some_and(|p| p.init)
                    || !valid_binding(&binding)
                    || self.bindings.len() >= 8
                    || self.bindings.iter().any(|b| b.slot == binding.slot)
                {
                    return Ok(self.fail());
                }
                self.bindings.push(binding);
                Ok(None)
            }
            Response::ArrivalStarted {
                occurrence,
                binding,
                until,
                ..
            } => {
                if !valid_binding(&binding)
                    || !self.bindings.contains(&binding)
                    || self.incoming_sequence.checked_add(1) != Some(occurrence)
                    || self.incoming.len() > crate::incoming::MAX_QUEUED
                    || now >= until
                    || until > now.saturating_add(crate::incoming::ARRIVAL_TTL_NS)
                {
                    return Ok(self.fail());
                }
                self.incoming_sequence = occurrence;
                self.incoming.insert(occurrence, (binding, until, None));
                Ok(None)
            }
            Response::RawCommitted {
                occurrence,
                binding,
                until,
                receipt,
                ..
            } => {
                let Some(record) = self.incoming.get_mut(&occurrence) else {
                    return Ok(self.fail());
                };
                if now >= until
                    || record.0 != binding
                    || record.1 != until
                    || record.2.is_some()
                    || crate::protocol::DataId::parse(&receipt).is_err()
                {
                    return Ok(self.fail());
                }
                record.2 = Some(receipt);
                self.raw_committed = self
                    .raw_committed
                    .checked_add(1)
                    .ok_or(io::ErrorKind::Other)?;
                Ok(None)
            }
            Response::ArrivalComplete {
                upper,
                occurrence,
                binding,
                until,
                receipt,
                ..
            } => {
                if self.reader
                    || !self
                        .projection
                        .as_ref()
                        .is_some_and(|p| p.occurrence == occurrence && p.wire.is_none())
                    || self.upper.checked_add(1) != Some(upper)
                    || upper > 4096
                    || now >= until
                    || self.incoming.remove(&occurrence)
                        != Some((binding, until, Some(receipt.clone())))
                    || crate::protocol::DataId::parse(&receipt).is_err()
                {
                    return Ok(self.fail());
                }
                self.projection = None;
                self.upper = upper;
                self.incoming_completed = self
                    .incoming_completed
                    .checked_add(1)
                    .ok_or(io::ErrorKind::Other)?;
                Ok(None)
            }
            Response::SourceFailed { reason, .. } => {
                if !reason.is_empty()
                    && reason.len() <= 64
                    && reason.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                {
                    self.failure = Some(reason);
                }
                Ok(self.fail())
            }
            Response::Read {
                token,
                status,
                page,
                receiving,
                upper,
                ..
            } => {
                let Some(p) = self.pending.take() else {
                    return Ok(self.fail());
                };
                if upper > 4096
                    || (self.reader && upper != self.upper)
                    || token != p.token
                    || now >= p.until
                    || status != StorageStatus::Ready
                    || (receiving && self.bindings.is_empty())
                    || (!receiving && !self.bindings.is_empty())
                {
                    self.stop();
                    return Ok((!p.init).then_some(Event::Failed { token: p.token }));
                }
                if p.init && page.is_none() {
                    self.ready = true;
                    self.receiving = receiving;
                    self.upper = upper;
                    Ok(None)
                } else if !p.init
                    && let Some(page) = page
                {
                    let packet = serde_json::to_vec(&page)?;
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
    reader: Option<Worker>,
    reader_started: bool,
    reader_hold: bool,
    runtime: PathBuf,
    raw: PathBuf,
    inbox: PathBuf,
    exe: PathBuf,
}
impl Projector {
    pub fn spawn(runtime: &Path, raw: &Path, inbox: &Path, exe: &Path) -> io::Result<Self> {
        Ok(Self {
            worker: Some(Worker::spawn(runtime, raw, inbox, exe, &[], None, false)?),
            reader: None,
            reader_started: false,
            reader_hold: false,
            runtime: runtime.into(),
            raw: raw.into(),
            inbox: inbox.into(),
            exe: exe.into(),
        })
    }
}
impl Projector {
    /// Explicit trusted synthetic entry only; restart intentionally discards these arguments.
    pub fn spawn_receiving(
        runtime: &Path,
        raw: &Path,
        inbox: &Path,
        exe: &Path,
        args: &[OsString],
    ) -> io::Result<Self> {
        if args.len() != 2
            || args[0]
                .to_str()
                .is_none_or(|s| crate::protocol::DataId::parse(s).is_err())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            worker: Some(Worker::spawn(runtime, raw, inbox, exe, args, None, false)?),
            reader: None,
            reader_started: false,
            reader_hold: args[1] == "hold-read-snapshot",
            runtime: runtime.into(),
            raw: raw.into(),
            inbox: inbox.into(),
            exe: exe.into(),
        })
    }
}
impl Projector {
    /// Explicit local read recovery never replaces or restarts the writer.
    /// A cancelled reader retains its slot until this owner actually reaps it.
    pub fn restart_read_only(&mut self) -> io::Result<()> {
        if self.read_occupied() || self.writer_busy() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if !self.writer_available() {
            return Err(io::ErrorKind::NotConnected.into());
        }
        self.reader = None;
        self.spawn_reader()
    }
    fn writer_available(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| {
            w.ready && !w.terminal && w.child.is_some() && living(&w.alive).unwrap_or(false)
        })
    }
    fn writer_busy(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| {
            w.pending.is_some() || !w.incoming.is_empty() || w.projection.is_some()
        })
    }
    fn spawn_reader(&mut self) -> io::Result<()> {
        if self.writer_busy() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let upper = self
            .worker
            .as_ref()
            .ok_or(io::ErrorKind::NotConnected)?
            .upper;
        self.reader_started = true;
        self.reader = Some(Worker::spawn(
            &self.runtime,
            &self.raw,
            &self.inbox,
            &self.exe,
            &[],
            Some(upper),
            self.reader_hold,
        )?);
        Ok(())
    }
}
impl Provider for Projector {
    fn incarnation(&self) -> &str {
        self.reader.as_ref().map_or("", |w| w.source.as_str())
    }
    fn occupied(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.child.is_some()) || self.read_occupied()
    }
    fn read_occupied(&self) -> bool {
        self.reader.as_ref().is_some_and(|w| w.child.is_some())
    }
    fn busy(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|w| w.child.is_some() && (w.pending.is_some() || !w.incoming.is_empty()))
            || self
                .reader
                .as_ref()
                .is_some_and(|w| w.child.is_some() && w.pending.is_some())
    }
    fn incoming_progress(&self) -> (u64, u64, u64) {
        self.worker.as_ref().map_or((0, 0, 0), |w| {
            (w.incoming_sequence, w.raw_committed, w.incoming_completed)
        })
    }
    fn receiving(&self) -> bool {
        self.writer_available() && self.worker.as_ref().is_some_and(|w| w.receiving)
    }
    fn incoming_failure(&self) -> Option<&str> {
        self.worker.as_ref().and_then(|w| w.failure.as_deref())
    }
    fn available(&self) -> bool {
        self.writer_available()
            && self.reader.as_ref().is_some_and(|w| {
                w.ready && !w.terminal && w.child.is_some() && living(&w.alive).unwrap_or(false)
            })
    }
    fn start(&mut self, query: serde_json::Value, until: u64) -> io::Result<u64> {
        let q: Query = serde_json::from_value(query)?;
        q.validate().map_err(|_| io::ErrorKind::InvalidInput)?;
        if !self.available() || self.writer_busy() || boottime_ns() >= until {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let upper = self.worker.as_ref().unwrap().upper;
        let reader = self.reader.as_mut().ok_or(io::ErrorKind::NotConnected)?;
        if reader.pending.is_some() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        reader.sequence = reader.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        let wire = serde_json::to_vec(&Request {
            source: reader.source.clone(),
            token: reader.sequence,
            until,
            query: Some(q),
            upper: Some(upper),
            projection: None,
        })?;
        if wire.len() > 4096 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        reader.upper = upper;
        reader.pending = Some(Pending {
            token: reader.sequence,
            until,
            wire: Some(wire),
            init: false,
        });
        Ok(reader.sequence)
    }
    fn cancel_read(&mut self, token: u64) {
        if let Some(reader) = &mut self.reader
            && reader
                .pending
                .as_ref()
                .is_some_and(|p| !p.init && p.token == token)
        {
            reader.stop();
        }
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        if let Some(writer) = &mut self.worker {
            let _ = writer.poll()?;
        }
        if !self.writer_available() {
            if let Some(reader) = &mut self.reader {
                reader.stop();
            }
        } else if !self.reader_started && !self.writer_busy() {
            // A failed readonly bootstrap is local; only an explicit later
            // launch may try again. It must never stop or reattach receiving.
            let _ = self.spawn_reader();
        }
        let event = match &mut self.reader {
            Some(reader) => match reader.poll() {
                Ok(event) => event,
                Err(_) => {
                    reader.failure = Some("reader_poll_unavailable".into());
                    reader.fail()
                }
            },
            None => None,
        };
        // A successful authenticated read/init reply certifies that its SQLite
        // work finished. Otherwise every owned child remains a gate until reap.
        let reads_released = self
            .reader
            .as_ref()
            .is_none_or(|r| r.child.is_none() || (!r.terminal && r.ready && r.pending.is_none()));
        if self.writer_available()
            && reads_released
            && let Some(writer) = &mut self.worker
        {
            writer.reserve_projection()?;
        }
        Ok(event)
    }
    fn stop(&mut self) {
        if let Some(w) = &mut self.worker {
            w.stop();
        }
        if let Some(w) = &mut self.reader {
            w.stop();
        }
    }
    fn restart(&mut self) -> io::Result<()> {
        if self.read_occupied() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if self.writer_available() {
            if self.writer_busy() {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            self.reader = None;
            return self.spawn_reader();
        }
        if self.occupied() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.reader = None;
        self.worker = None;
        self.reader_started = false;
        self.worker = Some(Worker::spawn(
            &self.runtime,
            &self.raw,
            &self.inbox,
            &self.exe,
            &[],
            None,
            false,
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
/// Private normal installed child: no source is selected or attached.
pub fn entry(args: &[OsString]) -> io::Result<()> {
    entry_with_backend(args, None)
}
/// Only separately built trusted fixture binaries supply a receive backend.
pub fn entry_with_backend(
    args: &[OsString],
    mut backend: Option<(String, Box<dyn crate::incoming::Backend>)>,
) -> io::Result<()> {
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
    let fd = Arc::new(ipc::connect(Path::new(&args[0]))?);
    let source = args[4].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    crate::protocol::DataId::parse(source).map_err(|_| io::ErrorKind::InvalidInput)?;
    let mut store = Store::open(Path::new(&args[1]), Path::new(&args[2]))
        .map_err(|_| io::ErrorKind::InvalidData)?;
    let mut sequence = 0u64;
    let mut incoming: Option<crate::incoming::Incoming> = None;
    let mut request: Option<Request> = None;
    let mut active: Option<(
        crate::incoming::Arrival,
        pokelinux_messaging::receipt::ReceiptId,
        bool,
    )> = None;
    loop {
        if request.as_ref().is_some_and(|r| boottime_ns() >= r.until) {
            if let Some(i) = &incoming {
                i.control.fail("worker_request_deadline");
            }
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(i) = &incoming {
            if i.control.failed() {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            if active
                .as_ref()
                .is_some_and(|(a, _, _)| boottime_ns() >= a.until)
            {
                i.control.fail("arrival_deadline");
                return Err(io::ErrorKind::TimedOut.into());
            }
            if active.is_none()
                && request.is_none()
                && let Ok(arrival) = i.queue.try_recv()
            {
                if boottime_ns() >= arrival.until {
                    i.control.fail("arrival_deadline");
                    return Err(io::ErrorKind::TimedOut.into());
                }
                let receipt = match store.admit_incoming(arrival.source, arrival.kind, &arrival.raw)
                {
                    Ok(id) => id,
                    Err(_) => {
                        i.control.fail("raw_storage_unavailable");
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                };
                if i.control
                    .raw_committed(&arrival, receipt.to_string())
                    .is_err()
                {
                    i.control.fail("raw_commit_ipc_failed");
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
                if let Some((_, b)) = &mut backend {
                    b.after_raw()?;
                }
                active = Some((arrival, receipt, false));
            }
        }
        if request.is_none()
            && let Some((bytes, c)) = ipc::receive(&*fd)?
        {
            if c.uid.as_raw() != 0 || c.pid.as_raw_nonzero().get() as u32 != parent {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let r: Request = serde_json::from_slice(&bytes)?;
            if r.query.is_some()
                || r.upper.is_some()
                || r.source != source
                || sequence.checked_add(1) != Some(r.token)
                || boottime_ns() >= r.until
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            sequence = r.token;
            if let Some(permit) = r.projection {
                let Some((arrival, receipt, granted)) = &mut active else {
                    return Err(io::ErrorKind::InvalidData.into());
                };
                let generation = backend.as_ref().map(|(generation, _)| generation.as_str());
                if *granted
                    || r.token == 1
                    || permit.occurrence != arrival.occurrence
                    || permit.until != arrival.until
                    || r.until != arrival.until
                    || permit.receipt != receipt.to_string()
                    || permit.binding.slot != arrival.source.slot().0
                    || permit.binding.incarnation
                        != arrival.source.incarnation().simple().to_string()
                    || generation != Some(permit.binding.generation.as_str())
                {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                *granted = true;
            } else {
                if r.token != 1 || active.is_some() {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                request = Some(r);
            }
        }
        if active.as_ref().is_some_and(|(_, _, granted)| *granted) || request.is_some() {
            // At most one bounded import page per turn. No full-history loop
            // holds the worker; every next turn rechecks original deadlines.
            let status = match store.reconcile() {
                Ok(status) => status,
                Err(_) => {
                    if let Some(i) = &incoming {
                        i.control.fail("projection_storage_unavailable");
                    }
                    return Err(io::ErrorKind::InvalidData.into());
                }
            };
            if status == StorageStatus::Ready {
                if let Some((arrival, receipt, _)) = active.take() {
                    if let Some((_, b)) = &mut backend {
                        b.before_complete()?;
                    }
                    let i = incoming.as_ref().ok_or(io::ErrorKind::InvalidData)?;
                    if i.control
                        .complete(
                            &arrival,
                            receipt.to_string(),
                            store
                                .completed_upper()
                                .map_err(|_| io::ErrorKind::InvalidData)?,
                        )
                        .is_err()
                    {
                        i.control.fail("completion_ipc_failed");
                        return Err(io::ErrorKind::BrokenPipe.into());
                    }
                }
                if let Some(r) = request.take() {
                    if boottime_ns() >= r.until {
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                    if r.token == 1
                        && r.query.is_none()
                        && let Some((generation, b)) = &mut backend
                    {
                        if store
                            .claim_test_source(generation)
                            .map_err(|_| io::ErrorKind::InvalidData)?
                        {
                            let i = crate::incoming::Incoming::new(fd.clone(), source, generation);
                            b.initialize(&mut store, &i.control, r.until)?;
                            if i.control.failed() || boottime_ns() >= r.until {
                                return Err(io::ErrorKind::TimedOut.into());
                            }
                            incoming = Some(i);
                        } else {
                            backend = None;
                        } // A service restart remains history-only.
                    }
                    let page = r
                        .query
                        .as_ref()
                        .map(|q| store.query(q).map_err(|_| io::ErrorKind::InvalidData))
                        .transpose()?;
                    let response = serde_json::to_vec(&Response::Read {
                        source: source.into(),
                        token: r.token,
                        status,
                        page,
                        receiving: incoming.is_some(),
                        upper: store
                            .completed_upper()
                            .map_err(|_| io::ErrorKind::InvalidData)?,
                    })?;
                    loop {
                        if boottime_ns() >= r.until {
                            return Err(io::ErrorKind::TimedOut.into());
                        }
                        match ipc::send(&*fd, &response) {
                            Ok(()) => break,
                            Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                                std::thread::sleep(std::time::Duration::from_millis(2))
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// Independent supervised readonly child. It owns no raw journal, writer lock or backend.
pub fn reader_entry(args: &[OsString]) -> io::Result<()> {
    reader_entry_impl(args, false)
}
#[cfg(feature = "isolated-fixture")]
pub fn held_reader_entry(args: &[OsString]) -> io::Result<()> {
    reader_entry_impl(args, true)
}
fn reader_entry_impl(args: &[OsString], _held: bool) -> io::Result<()> {
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
    crate::protocol::DataId::parse(source).map_err(|_| io::ErrorKind::InvalidInput)?;
    let mut reader = None;
    let mut sequence = 0u64;
    loop {
        let Some((bytes, cred)) = ipc::receive(&fd)? else {
            std::thread::sleep(std::time::Duration::from_millis(2));
            continue;
        };
        if cred.uid.as_raw() != 0 || cred.pid.as_raw_nonzero().get() as u32 != parent {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let request: Request = serde_json::from_slice(&bytes)?;
        let upper = request.upper.ok_or(io::ErrorKind::InvalidData)?;
        if request.projection.is_some()
            || request.source != source
            || sequence.checked_add(1) != Some(request.token)
            || upper > 4096
            || boottime_ns() >= request.until
            || (request.query.is_none() && request.token != 1)
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        sequence = request.token;
        if reader.is_none() {
            reader = Some(
                crate::projection::Reader::open(Path::new(&args[2]))
                    .map_err(|_| io::ErrorKind::InvalidData)?,
            );
        }
        #[cfg(feature = "isolated-fixture")]
        if _held && request.token == 2 {
            reader.as_mut().unwrap().hold_next_snapshot();
        }
        let page = request
            .query
            .as_ref()
            .map(|q| {
                reader
                    .as_mut()
                    .unwrap()
                    .query(q, upper)
                    .map_err(|_| io::ErrorKind::InvalidData)
            })
            .transpose()?;
        if boottime_ns() >= request.until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let reply = serde_json::to_vec(&Response::Read {
            source: source.into(),
            token: request.token,
            status: StorageStatus::Ready,
            page,
            receiving: false,
            upper,
        })?;
        if reply.len() > 4096 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        loop {
            if boottime_ns() >= request.until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match ipc::send(&fd, &reply) {
                Ok(()) => break,
                Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                    std::thread::sleep(std::time::Duration::from_millis(2))
                }
                Err(e) => return Err(e),
            }
        }
    }
}
