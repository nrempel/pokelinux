//! One Rust-owned worker, one explicit intent and one nonblocking socket at a time.
use crate::{
    clock,
    ffi::{View, CONTROLS_UNAVAILABLE, DENIED, PENDING, READY, STOPPED, UNAVAILABLE, UNCERTAIN},
    protocol::{Reply, Request, Set, State, LIMIT, SECOND},
};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};
#[derive(Clone, Copy)]
struct Click {
    id: u64,
    percent: u8,
    revision: u64,
    until: u64,
    selected: bool,
    sent: bool,
}
struct Cache {
    state: State,
    can_set: bool,
}
struct Model {
    cache: Option<Cache>,
    click: Option<Click>,
    sequence: u64,
    status: u32,
}
impl Model {
    fn clear(&mut self, status: u32) {
        self.cache = None;
        self.click = None;
        self.status = status;
    }
    fn expire(&mut self, now: u64) {
        if let Some(c) = self.click {
            if now >= c.until {
                self.clear(if c.sent { UNCERTAIN } else { UNAVAILABLE });
            }
        }
        if self
            .cache
            .as_ref()
            .is_some_and(|c| c.state.available && now >= c.state.until())
        {
            self.cache = None;
            if self.click.is_none() {
                self.status = UNAVAILABLE;
            }
        }
    }
    fn usable(&self, now: u64) -> bool {
        self.cache.as_ref().is_some_and(|c| {
            c.can_set && c.state.available && !c.state.pending && c.state.valid(now)
        })
    }
    fn enqueue(&mut self, percent: u32, now: u64) -> bool {
        self.expire(now);
        if !matches!(percent, 25 | 75) || self.click.is_some() || !self.usable(now) {
            return false;
        }
        let Some(id) = self.sequence.checked_add(1) else {
            return false;
        };
        let Some(until) = now.checked_add(SECOND) else {
            return false;
        };
        self.sequence = id;
        self.click = Some(Click {
            id,
            percent: percent as u8,
            revision: self.cache.as_ref().unwrap().state.revision,
            until,
            selected: false,
            sent: false,
        });
        true
    }
    fn may_send(&mut self, click: Click, now: u64) -> bool {
        self.expire(now);
        self.click.is_some_and(|c| c.id == click.id && !c.sent)
            && now < click.until
            && self.usable(now)
            && self
                .cache
                .as_ref()
                .is_some_and(|c| c.state.revision == click.revision)
    }
    // An already accepted choice may wait for readonly custody before its first
    // send. Keep its exact revision/deadline; loss or changed state is terminal.
    fn waiting_on_readonly(&self, click: Click, now: u64) -> bool {
        self.click
            .is_some_and(|c| c.id == click.id && !c.selected && !c.sent)
            && now < click.until
            && self.cache.as_ref().is_some_and(|c| {
                c.state.available
                    && c.state.pending
                    && !c.can_set
                    && c.state.revision == click.revision
                    && c.state.requested_percent.is_none()
                    && c.state.valid(now)
            })
    }
    fn view(&mut self, now: u64, stopped: bool) -> View {
        if stopped {
            self.clear(STOPPED);
        } else {
            self.expire(now);
        }
        let mut v = View {
            status: self.status,
            ..View::default()
        };
        if let Some(c) = &self.cache {
            let s = &c.state;
            v.revision = s.revision;
            v.connected = u32::from(s.connected);
            v.pending = u32::from(s.pending);
            if s.available && s.valid(now) {
                v.available = 1;
                v.until_ns = s.until();
                v.requested_percent = s.requested_percent.map_or(-1, i32::from);
                v.observed_percent = s.observed_percent.map_or(-1, i32::from);
                v.actual_percent = s.actual_percent.map_or(-1, i32::from);
                v.can_set = u32::from(c.can_set && !s.pending);
                v.status = if s.pending {
                    PENDING
                } else if c.can_set {
                    READY
                } else {
                    CONTROLS_UNAVAILABLE
                };
            }
        }
        if self.click.is_some() {
            v.available = 0;
            v.pending = 1;
            v.can_set = 0;
            v.until_ns = 0;
            v.status = PENDING;
            v.requested_percent = -1;
            v.observed_percent = -1;
            v.actual_percent = -1;
        }
        v
    }
    fn accept(&mut self, reply: Reply, operation: Option<Click>, now: u64) -> bool {
        if let Some(click) = operation {
            if now >= click.until || !self.click.is_some_and(|c| c.id == click.id) {
                return false;
            }
        }
        match reply {
            Reply::BrightnessState { state, can_set }
                if operation.is_none()
                    && state.valid(now)
                    && (!can_set || (state.available && !state.pending)) =>
            {
                self.status = if state.pending { PENDING } else { UNAVAILABLE };
                self.cache = Some(Cache { state, can_set });
                true
            }
            Reply::BrightnessCompleted { state }
                if operation.is_some_and(|c| {
                    state.valid(now)
                        && state.available
                        && !state.pending
                        && state.revision > c.revision
                        && state.requested_percent == Some(c.percent)
                }) =>
            {
                self.click = None;
                self.cache = Some(Cache {
                    state,
                    can_set: false,
                });
                self.status = CONTROLS_UNAVAILABLE;
                true
            }
            Reply::Denied => {
                self.clear(DENIED);
                true
            }
            Reply::Unavailable => {
                self.clear(UNAVAILABLE);
                true
            }
            _ => false,
        }
    }
}
pub struct Shared {
    model: Mutex<Model>,
    pub stop: AtomicBool,
}
pub struct Engine {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stop_until: AtomicU64,
    join_status: AtomicI32,
}
impl Engine {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            model: Mutex::new(Model {
                cache: None,
                click: None,
                sequence: 0,
                status: UNAVAILABLE,
            }),
            stop: AtomicBool::new(false),
        });
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("settings-brightness".into())
            .spawn(move || run(worker, path))?;
        Ok(Self {
            shared,
            thread: Mutex::new(Some(thread)),
            stop_until: AtomicU64::new(0),
            join_status: AtomicI32::new(crate::ffi::WAIT),
        })
    }
    pub fn view(&self) -> Option<View> {
        let mut m = self.shared.model.lock().ok()?;
        match clock::now() {
            Some(now) => Some(m.view(now, self.shared.stop.load(Ordering::Acquire))),
            None => {
                m.clear(UNAVAILABLE);
                Some(View::default())
            }
        }
    }
    pub fn set(&self, percent: u32) -> bool {
        if self.shared.stop.load(Ordering::Acquire) {
            return false;
        }
        let Ok(mut m) = self.shared.model.lock() else {
            return false;
        };
        clock::now()
            .is_some_and(|now| !self.shared.stop.load(Ordering::Acquire) && m.enqueue(percent, now))
    }
    pub fn stop(&self) -> bool {
        self.shared.stop.store(true, Ordering::Release);
        if self.stop_until.load(Ordering::Acquire) != 0 {
            return true;
        }
        let Some(until) = clock::now().and_then(|n| n.checked_add(200_000_000)) else {
            return false;
        };
        let _ = self
            .stop_until
            .compare_exchange(0, until, Ordering::AcqRel, Ordering::Acquire);
        true
    }
    pub fn join(&self) -> i32 {
        use crate::ffi::{ABORT, INVALID, OK, WAIT};
        let until = self.stop_until.load(Ordering::Acquire);
        if until == 0 {
            return INVALID;
        }
        let Ok(mut owner) = self.thread.lock() else {
            return ABORT;
        };
        if owner.is_none() {
            return self.join_status.load(Ordering::Acquire);
        }
        let Some(now) = clock::now() else {
            return ABORT;
        };
        if !owner.as_ref().unwrap().is_finished() {
            if now >= until {
                self.join_status.store(ABORT, Ordering::Release);
            }
            return if self.join_status.load(Ordering::Acquire) == ABORT {
                ABORT
            } else {
                WAIT
            };
        }
        let result = owner.take().unwrap().join();
        let status = if result.is_ok()
            && clock::now().is_some_and(|n| n < until)
            && self.join_status.load(Ordering::Acquire) != ABORT
        {
            OK
        } else {
            ABORT
        };
        self.join_status.store(status, Ordering::Release);
        status
    }
    pub fn joined(&self) -> bool {
        self.thread.lock().is_ok_and(|t| t.is_none())
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        if !self.joined() {
            std::process::abort();
        }
    }
}
fn active(shared: &Shared, until: u64) -> bool {
    !shared.stop.load(Ordering::Acquire) && clock::now().is_some_and(|n| n < until)
}
fn wait(fd: &OwnedFd, events: PollFlags, shared: &Shared, until: u64) -> io::Result<()> {
    if !active(shared, until) {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let remaining = until
        .saturating_sub(clock::now().ok_or(io::ErrorKind::TimedOut)?)
        .min(10_000_000);
    let timeout = Timespec {
        tv_sec: 0,
        tv_nsec: remaining as _,
    };
    let mut fds = [PollFd::new(fd, events)];
    match poll(&mut fds, Some(&timeout)) {
        Ok(_) => Ok(()),
        Err(rustix::io::Errno::INTR) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn connect(path: &PathBuf, shared: &Shared, until: u64) -> io::Result<(OwnedFd, UCred)> {
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    let address = SocketAddrUnix::new(path)?;
    match net::connect(&fd, &address) {
        Ok(()) => {}
        Err(rustix::io::Errno::INPROGRESS) => {
            wait(&fd, PollFlags::OUT, shared, until)?;
            net::sockopt::socket_error(&fd)??;
        }
        Err(e) => return Err(e.into()),
    }
    if !active(shared, until) {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let peer = net::sockopt::socket_peercred(&fd)?;
    if peer.uid.as_raw() != 0 || peer.gid.as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok((fd, peer))
}
fn receive(fd: &OwnedFd, peer: &UCred) -> io::Result<Option<Reply>> {
    let mut bytes = [0; LIMIT];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        r => r?,
    };
    let mut credential = None;
    let mut invalid = message.bytes == 0
        || message
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(c) if credential.is_none() => credential = Some(c),
            _ => invalid = true,
        }
    }
    let c = credential.ok_or(io::ErrorKind::PermissionDenied)?;
    if invalid || c != *peer {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(serde_json::from_slice(&bytes[..message.bytes])?))
}
fn transaction(
    path: &PathBuf,
    shared: &Shared,
    operation: Option<Click>,
    until: u64,
) -> io::Result<Reply> {
    let (fd, peer) = connect(path, shared, until)?;
    loop {
        if !active(shared, until) {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let request = if let Some(click) = operation {
            let mut m = shared.model.lock().map_err(|_| io::ErrorKind::Other)?;
            let now = clock::now().ok_or(io::ErrorKind::TimedOut)?;
            if !m.may_send(click, now) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            let remaining = (click.until - now) / 1_000_000;
            if remaining == 0 {
                return Err(io::ErrorKind::TimedOut.into());
            }
            Request::BrightnessSet {
                request: Set {
                    percent: click.percent,
                    expected_revision: click.revision,
                    ttl_ms: remaining.min(1000),
                },
            }
        } else {
            Request::BrightnessStatus {}
        };
        let bytes = serde_json::to_vec(&request)?;
        if !active(shared, until) {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match net::send(&fd, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
            Ok(n) if n == bytes.len() => {
                if let Some(c) = operation {
                    if let Ok(mut m) = shared.model.lock() {
                        if let Some(current) = m.click.as_mut() {
                            if current.id == c.id {
                                current.sent = true;
                            }
                        }
                    }
                }
                break;
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                wait(&fd, PollFlags::OUT, shared, until)?
            }
            Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
            Err(e) => return Err(e.into()),
        }
    }
    // Keep both channel halves open: the actor interprets request-half EOF as cancellation.
    loop {
        if !active(shared, until) {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(reply) = receive(&fd, &peer)? {
            if !active(shared, until) {
                return Err(io::ErrorKind::TimedOut.into());
            }
            return Ok(reply);
        }
        wait(&fd, PollFlags::IN, shared, until)?;
    }
}
fn run(shared: Arc<Shared>, path: PathBuf) {
    let mut next_status = 0;
    while !shared.stop.load(Ordering::Acquire) {
        let Some(now) = clock::now() else { break };
        let operation = {
            let Ok(mut m) = shared.model.lock() else {
                break;
            };
            m.expire(now);
            if let Some(click) = m.click {
                if !click.selected {
                    if m.may_send(click, now) {
                        m.click.as_mut().unwrap().selected = true;
                        Some(Some(click))
                    } else if m.waiting_on_readonly(click, now) {
                        // Readiness is established by a new readonly status,
                        // never by resending or extending this explicit choice.
                        (now >= next_status).then_some(None)
                    } else {
                        m.click = None;
                        None
                    }
                } else {
                    None
                }
            } else if now >= next_status {
                Some(None)
            } else {
                None
            }
        };
        if let Some(operation) = operation {
            let until = operation.map_or_else(|| now.saturating_add(500_000_000), |c| c.until);
            let reply = transaction(&path, &shared, operation, until);
            let Some(finished) = clock::now() else { break };
            let Ok(mut m) = shared.model.lock() else {
                break;
            };
            if shared.stop.load(Ordering::Acquire) {
                m.clear(STOPPED);
                break;
            }
            let accepted =
                finished < until && reply.is_ok_and(|r| m.accept(r, operation, finished));
            if !accepted {
                if let Some(c) = operation {
                    if let Some(current) = m.click {
                        if current.id == c.id {
                            m.clear(if current.sent { UNCERTAIN } else { UNAVAILABLE });
                        }
                    }
                } else {
                    m.clear(UNAVAILABLE);
                }
            }
            next_status = finished.saturating_add(250_000_000);
        } else {
            thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    if let Ok(mut m) = shared.model.lock() {
        m.clear(STOPPED);
    }
}
