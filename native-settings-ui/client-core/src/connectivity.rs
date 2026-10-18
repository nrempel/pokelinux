//! Settings connectivity model with one owned transport worker.
//! Explicit actions keep the caller deadline and are never reconnected or retried.
use crate::{clock, ffi};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    path::{Component, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};
const LEASE: u64 = 250_000_000;
const STATUS_BUDGET: u64 = 500_000_000;
const ACTION_BUDGET: u64 = 1_000_000_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Unavailable,
    Ready,
    ControlsUnavailable,
    Denied,
    Pending,
    NotSent,
    Uncertain,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct View {
    pub transport_connected: bool,
    pub available: bool,
    pub pending: bool,
    pub can_control: bool,
    pub configured: Option<bool>,
    pub observed_at_ns: u64,
    pub until_ns: u64,
    pub status: Status,
}
impl Default for View {
    fn default() -> Self {
        Self {
            transport_connected: false,
            available: false,
            pending: false,
            can_control: false,
            configured: None,
            observed_at_ns: 0,
            until_ns: 0,
            status: Status::Unavailable,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    connected: bool,
    available: bool,
    #[serde(deserialize_with = "required_option")]
    configured: Option<bool>,
    pending: bool,
    observed_at_ns: String,
    until_ns: String,
}
fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
fn decimal(text: &str) -> Option<u64> {
    if text.is_empty()
        || text.len() > 20
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|v| v.is_ascii_digit())
    {
        return None;
    }
    text.parse().ok()
}
impl State {
    fn valid(&self, now: u64) -> bool {
        let Some(at) = decimal(&self.observed_at_ns) else {
            return false;
        };
        let Some(until) = decimal(&self.until_ns) else {
            return false;
        };
        if !self.available {
            return self.configured.is_none() && at == 0 && until == 0;
        }
        self.connected
            && self.configured.is_some()
            && at > 0
            && at <= now
            && now < until
            && at.checked_add(LEASE) == Some(until)
    }
}
#[derive(Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    ConnectivityState { state: State, can_control: bool },
    ConnectivityCompleted { state: State },
    Denied,
    Unavailable,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Queued,
    Attempting,
    Sent,
}
#[derive(Clone, Copy)]
struct Choice {
    id: u64,
    generation: u64,
    connected: bool,
    until: u64,
    attempted_at: u64,
    phase: Phase,
    retired: bool,
}
struct Model {
    cache: Option<(State, bool)>,
    status: Status,
    outcome: Option<Status>,
    choice: Option<Choice>,
    next_id: u64,
    generation: u64,
}
impl Model {
    fn clear(&mut self, status: Status) {
        self.cache = None;
        self.status = status;
    }
    fn permitted(&self, now: u64) -> bool {
        self.generation != 0
            && self
                .cache
                .as_ref()
                .is_some_and(|(s, can)| *can && s.valid(now) && s.available && !s.pending)
    }
    fn retire(&mut self) {
        let Some(c) = &mut self.choice else {
            return;
        };
        if c.retired {
            return;
        }
        if c.phase == Phase::Queued {
            self.choice = None;
            self.outcome = Some(Status::NotSent);
        } else {
            // Retired syscall/reply custody stays occupied until the worker
            // actually returns. A getter never releases an unresolved send.
            c.retired = true;
            self.outcome = Some(Status::Uncertain);
        }
    }
    fn expire(&mut self, now: u64, stopped: bool) {
        if self.choice.is_some_and(|c| {
            stopped
                || now >= c.until
                || (c.phase == Phase::Queued
                    && (!self.permitted(now) || c.generation != self.generation))
        }) {
            self.retire();
        }
        if self.cache.as_ref().is_some_and(|(s, _)| !s.valid(now)) {
            self.clear(Status::Unavailable);
        }
        if stopped {
            self.clear(Status::Stopped);
        }
    }
    fn lost(&mut self) {
        self.retire();
        self.generation = 0;
        self.clear(Status::Unavailable);
    }
    fn view(&mut self, now: u64, stopped: bool) -> View {
        self.expire(now, stopped);
        let mut v = View {
            status: self.outcome.unwrap_or(self.status),
            ..View::default()
        };
        if stopped {
            v.status = Status::Stopped;
            return v;
        }
        if let Some((s, can)) = &self.cache {
            v.transport_connected = s.connected;
            v.pending = s.pending;
            if s.available && s.valid(now) {
                v.available = true;
                v.configured = s.configured;
                v.observed_at_ns = decimal(&s.observed_at_ns).unwrap();
                v.until_ns = decimal(&s.until_ns).unwrap();
                v.can_control = *can && !s.pending;
                v.status = self.outcome.unwrap_or(if s.pending {
                    Status::Pending
                } else if *can {
                    Status::Ready
                } else {
                    Status::ControlsUnavailable
                });
            }
        }
        if self.choice.is_some_and(|c| !c.retired) {
            v.available = false;
            v.configured = None;
            v.observed_at_ns = 0;
            v.until_ns = 0;
            v.can_control = false;
            v.pending = true;
            v.status = Status::Pending;
        }
        v
    }
    fn accept_status(&mut self, reply: Reply, now: u64) -> bool {
        // Expire the OLD observation before a new status can replace it. An
        // expired or revoked queued click must never revive on fresh facts.
        self.expire(now, false);
        match reply {
            Reply::ConnectivityState { state, can_control }
                if state.valid(now) && (!can_control || (state.available && !state.pending)) =>
            {
                self.cache = Some((state, can_control));
                self.status = Status::Unavailable;
            }
            Reply::Denied => self.clear(Status::Denied),
            Reply::Unavailable => self.clear(Status::Unavailable),
            _ => return false,
        }
        self.expire(now, false);
        true
    }
    fn commit(&mut self, generation: u64, now: u64, stopped: bool) -> Option<Choice> {
        self.expire(now, stopped);
        let c = self.choice.as_mut()?;
        if c.retired || c.phase != Phase::Queued || c.generation != generation {
            return None;
        }
        c.phase = Phase::Attempting;
        c.attempted_at = now;
        Some(*c)
    }
    fn finish(
        &mut self,
        original: Choice,
        reply: io::Result<Reply>,
        now: u64,
        stopped: bool,
    ) -> bool {
        self.expire(now, stopped);
        let Some(c) = self.choice.take() else {
            return false;
        };
        if c.id != original.id || c.generation != original.generation {
            self.outcome = Some(Status::Uncertain);
            self.clear(Status::Unavailable);
            return false;
        }
        if !c.retired && !stopped && now < c.until {
            if let Ok(Reply::ConnectivityCompleted { state }) = reply {
                if state.valid(now)
                    && state.available
                    && !state.pending
                    && state.configured == Some(c.connected)
                    && decimal(&state.observed_at_ns).is_some_and(|at| at >= c.attempted_at)
                {
                    // Completion confirms this click, never a permission grant.
                    self.cache = Some((state, false));
                    self.outcome = None;
                    self.status = Status::ControlsUnavailable;
                    return true;
                }
            }
        }
        // Even Denied may follow an effect whose publication was revoked.
        // Readonly recovery can show current facts, never confirm this click.
        self.outcome = Some(Status::Uncertain);
        self.clear(if stopped {
            Status::Stopped
        } else {
            Status::Unavailable
        });
        false
    }
}
struct Shared {
    model: Mutex<Model>,
    stop: AtomicBool,
}
/// One worker stays owned until an actual join; Drop never detaches it.
pub struct Engine {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stop_until: AtomicU64,
    join_status: AtomicI32,
}
impl Engine {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        if !path.is_absolute()
            || bytes.len() > 107
            || bytes.contains(&0)
            || path.components().any(|v| matches!(v, Component::ParentDir))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let shared = Arc::new(Shared {
            model: Mutex::new(Model {
                cache: None,
                status: Status::Unavailable,
                outcome: None,
                choice: None,
                next_id: 0,
                generation: 0,
            }),
            stop: AtomicBool::new(false),
        });
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("settings-connectivity".into())
            .spawn(move || run(worker, path))?;
        Ok(Self {
            shared,
            thread: Mutex::new(Some(thread)),
            stop_until: AtomicU64::new(0),
            join_status: AtomicI32::new(ffi::WAIT),
        })
    }
    pub fn view(&self) -> Option<View> {
        let mut m = self.shared.model.lock().ok()?;
        if let Some(now) = clock::now() {
            Some(m.view(now, self.shared.stop.load(Ordering::Acquire)))
        } else {
            m.clear(Status::Unavailable);
            Some(View::default())
        }
    }
    /// Admit one explicit desired state; true is local acceptance, not success.
    /// The caller's absolute CLOCK_BOOTTIME deadline is never extended.
    pub fn set(&self, connected: bool, original_until_ns: u64) -> bool {
        let Ok(mut m) = self.shared.model.lock() else {
            return false;
        };
        let Some(now) = clock::now() else {
            return false;
        };
        let stopped = self.shared.stop.load(Ordering::Acquire);
        m.expire(now, stopped);
        if stopped
            || m.choice.is_some()
            || !m.permitted(now)
            || original_until_ns <= now
            || original_until_ns - now > ACTION_BUDGET
        {
            return false;
        }
        let Some(id) = m.next_id.checked_add(1) else {
            return false;
        };
        m.next_id = id;
        m.choice = Some(Choice {
            id,
            generation: m.generation,
            connected,
            until: original_until_ns,
            attempted_at: 0,
            phase: Phase::Queued,
            retired: false,
        });
        m.outcome = None;
        true
    }
    /// Withdraw local work. A send already committed under the model lock may
    /// still reach the actor; Stop is never a remote Disconnect acknowledgement.
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
        let until = self.stop_until.load(Ordering::Acquire);
        if until == 0 {
            return ffi::INVALID;
        }
        let Ok(mut owner) = self.thread.lock() else {
            return ffi::ABORT;
        };
        if owner.is_none() {
            return self.join_status.load(Ordering::Acquire);
        }
        let Some(now) = clock::now() else {
            return ffi::ABORT;
        };
        if !owner.as_ref().unwrap().is_finished() {
            if now >= until {
                self.join_status.store(ffi::ABORT, Ordering::Release);
            }
            return if self.join_status.load(Ordering::Acquire) == ffi::ABORT {
                ffi::ABORT
            } else {
                ffi::WAIT
            };
        }
        let result = owner.take().unwrap().join();
        let code = if result.is_ok()
            && clock::now().is_some_and(|n| n < until)
            && self.join_status.load(Ordering::Acquire) != ffi::ABORT
        {
            ffi::OK
        } else {
            ffi::ABORT
        };
        self.join_status.store(code, Ordering::Release);
        code
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
fn active(s: &Shared, until: u64) -> bool {
    !s.stop.load(Ordering::Acquire) && clock::now().is_some_and(|n| n < until)
}
fn wait(fd: &OwnedFd, events: PollFlags, s: &Shared, until: u64) -> io::Result<()> {
    if !active(s, until) {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let remaining = until
        .saturating_sub(clock::now().ok_or(io::ErrorKind::TimedOut)?)
        .min(10_000_000);
    let mut fds = [PollFd::new(fd, events)];
    match poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: remaining as _,
        }),
    ) {
        Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn receive(fd: &OwnedFd, peer: &UCred) -> io::Result<Option<Reply>> {
    let mut bytes = [0; 4096];
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
    if invalid || credential.as_ref() != Some(peer) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(serde_json::from_slice(&bytes[..message.bytes])?))
}
struct Transport {
    fd: OwnedFd,
    peer: UCred,
    generation: u64,
}
impl Transport {
    fn connect(path: &PathBuf, s: &Shared, until: u64, generation: u64) -> io::Result<Self> {
        let fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        net::sockopt::set_socket_passcred(&fd, true)?;
        match net::connect(&fd, &SocketAddrUnix::new(path)?) {
            Ok(()) => {}
            Err(rustix::io::Errno::INPROGRESS) => {
                wait(&fd, PollFlags::OUT, s, until)?;
                net::sockopt::socket_error(&fd)??;
            }
            Err(e) => return Err(e.into()),
        }
        let peer = net::sockopt::socket_peercred(&fd)?;
        if peer.uid.as_raw() != 0 || peer.gid.as_raw() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self {
            fd,
            peer,
            generation,
        })
    }
    fn reply(&self, s: &Shared, until: u64) -> io::Result<Reply> {
        loop {
            if !active(s, until) {
                return Err(io::ErrorKind::TimedOut.into());
            }
            if let Some(reply) = receive(&self.fd, &self.peer)? {
                if !active(s, until) {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                return Ok(reply);
            }
            wait(&self.fd, PollFlags::IN, s, until)?;
        }
    }
    fn status(&self, s: &Shared, until: u64) -> io::Result<Reply> {
        let bytes = br#"{"op":"connectivity_status"}"#;
        loop {
            if !active(s, until) {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match net::send(&self.fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
                Ok(n) if n == bytes.len() => break,
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                    wait(&self.fd, PollFlags::OUT, s, until)?
                }
                Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
                Err(e) => return Err(e.into()),
            }
        }
        self.reply(s, until)
    }
    fn set(&self, s: &Shared, c: Choice) -> io::Result<Reply> {
        let bytes = serde_json::to_vec(&serde_json::json!({"op":"connectivity_set",
            "request":{"connected":c.connected,"until_ns":c.until.to_string()}}))?;
        if !active(s, c.until) {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Exactly one nonblocking send, outside the model lock. There is no
        // effect reconnect, backpressure wait, interrupted-send retry or replay.
        let n = net::send(&self.fd, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)?;
        if n != bytes.len() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        {
            let mut m = s.model.lock().map_err(|_| io::ErrorKind::Other)?;
            let now = clock::now().ok_or(io::ErrorKind::TimedOut)?;
            m.expire(now, s.stop.load(Ordering::Acquire));
            let current = m.choice.as_mut().ok_or(io::ErrorKind::Other)?;
            if current.id != c.id || current.generation != self.generation || current.retired {
                return Err(io::ErrorKind::TimedOut.into());
            }
            current.phase = Phase::Sent;
        }
        self.reply(s, c.until)
    }
}
fn run(shared: Arc<Shared>, path: PathBuf) {
    let mut next = 0;
    let mut generation: u64 = 0;
    let mut transport: Option<Transport> = None;
    while !shared.stop.load(Ordering::Acquire) {
        let Some(now) = clock::now() else {
            break;
        };
        let action = {
            let Ok(mut m) = shared.model.lock() else {
                break;
            };
            let Some(current) = clock::now() else {
                break;
            };
            m.expire(current, shared.stop.load(Ordering::Acquire));
            transport
                .as_ref()
                .and_then(|t| m.commit(t.generation, current, shared.stop.load(Ordering::Acquire)))
        };
        if let Some(c) = action {
            // Attempting is the local dispatch commitment. Neither expiry nor
            // Stop releases its custody while this send/reply is unresolved.
            let reply = transport.as_ref().unwrap().set(&shared, c);
            let Ok(mut m) = shared.model.lock() else {
                break;
            };
            let Some(finished) = clock::now() else {
                break;
            };
            if !m.finish(c, reply, finished, shared.stop.load(Ordering::Acquire)) {
                m.lost();
                transport = None;
            }
            next = 0;
            continue;
        }
        if now < next {
            thread::sleep(std::time::Duration::from_millis(2));
            continue;
        }
        let until = now.saturating_add(STATUS_BUDGET);
        if transport.is_none() {
            let Some(g) = generation.checked_add(1) else {
                break;
            };
            generation = g;
            // Only readonly recovery opens a new connection. A queued click is
            // already retired on loss and cannot migrate to this generation.
            match Transport::connect(&path, &shared, until, generation) {
                Ok(t) => {
                    let Ok(mut m) = shared.model.lock() else {
                        break;
                    };
                    m.lost();
                    m.generation = generation;
                    transport = Some(t);
                }
                Err(_) => {
                    let Ok(mut m) = shared.model.lock() else {
                        break;
                    };
                    m.lost();
                    next = now.saturating_add(50_000_000);
                    continue;
                }
            }
        }
        let reply = transport.as_ref().unwrap().status(&shared, until);
        let Ok(mut m) = shared.model.lock() else {
            break;
        };
        let Some(finished) = clock::now() else {
            break;
        };
        if shared.stop.load(Ordering::Acquire) {
            m.expire(finished, true);
            break;
        }
        if !(finished < until && reply.is_ok_and(|r| m.accept_status(r, finished))) {
            m.lost();
            transport = None;
        }
        next = finished.saturating_add(50_000_000);
    }
    // The worker owns its socket until this actual return; no detached task or
    // assertion that peer closure cleaned up a transferred vendor/backend job.
    drop(transport);
    if let Ok(mut m) = shared.model.lock() {
        if let Some(now) = clock::now() {
            m.expire(now, true);
        }
        m.choice = None;
        m.generation = 0;
        m.clear(Status::Stopped);
    }
}
