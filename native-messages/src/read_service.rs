//! Sole history owner and its root-private, bounded session read connection.
//! Pages cross this boundary as authenticated immutable copies. App policy and
//! Source are checked by the session actor again before every app send.
use crate::protocol::{DataId, Page, Query};
use crate::worker::Projector;
use pokelinux_lifecycle::{
    boottime_ns, ipc,
    messages::{Event, Provider},
};
use rustix::{
    fd::OwnedFd,
    net::{self, SocketFlags, UCred},
};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
    time::Duration,
};

const VERSION: u8 = 1;
const OPEN_BUDGET_NS: u64 = 1_000_000_000;
const HANDSHAKE_BUDGET_NS: u64 = 2_000_000_000;
const MAX_CLIENTS: usize = 4;
const MAX_PRIVATE_PACKET: usize = 8192;
pub const READ_SOCKET: &str = "read-provider.sock";

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Open {
        version: u8,
    },
    Read {
        version: u8,
        generation: String,
        token: u64,
        until_ns: u64,
        query: Query,
    },
    Cancel {
        version: u8,
        generation: String,
        token: u64,
    },
    Restart {
        version: u8,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Response {
    State {
        version: u8,
        generation: String,
        available: bool,
    },
    Complete {
        version: u8,
        generation: String,
        token: u64,
        until_ns: u64,
        page: Page,
    },
    Failed {
        version: u8,
        generation: String,
        token: u64,
        until_ns: u64,
    },
    Retired {
        version: u8,
        generation: String,
        token: u64,
    },
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Control {
    Stop {},
}

struct Peer {
    credentials: UCred,
    alive: OwnedFd,
}
impl Peer {
    fn new(fd: &OwnedFd) -> io::Result<Self> {
        let credentials = net::sockopt::socket_peercred(fd)?;
        if credentials.uid.as_raw() != 0 || credentials.gid.as_raw() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let peer = Self {
            credentials,
            alive: pokelinux_kernel_identity::socket_peer_pidfd(fd)?,
        };
        peer.current()?;
        Ok(peer)
    }
    fn current(&self) -> io::Result<()> {
        if !pokelinux_lifecycle::authority::living(&self.alive)? {
            return Err(io::ErrorKind::NotConnected.into());
        }
        Ok(())
    }
    fn check(&self, credentials: UCred) -> io::Result<()> {
        self.current()?;
        if credentials.pid != self.credentials.pid
            || credentials.uid != self.credentials.uid
            || credentials.gid != self.credentials.gid
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }
}
// The existing app packet limit stays4096. The private envelope needs bounded
// space for one existing4096-byte Page plus its generation/deadline metadata.
fn receive(fd: &OwnedFd) -> io::Result<Option<(Vec<u8>, UCred)>> {
    use std::{io::IoSliceMut, mem::MaybeUninit};
    let mut bytes = [0; MAX_PRIVATE_PACKET];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = net::RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        net::RecvFlags::DONTWAIT | net::RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        result => result?,
    };
    if message.bytes == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    let mut credentials = None;
    let mut invalid = message
        .flags
        .intersects(net::ReturnFlags::TRUNC | net::ReturnFlags::CTRUNC);
    for control in ancillary.drain() {
        match control {
            net::RecvAncillaryMessage::ScmCredentials(c) if credentials.is_none() => {
                credentials = Some(c)
            }
            net::RecvAncillaryMessage::ScmRights(handles) => {
                for handle in handles {
                    drop(handle);
                }
                invalid = true;
            }
            _ => invalid = true,
        }
    }
    if invalid {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some((
        bytes[..message.bytes].to_vec(),
        credentials.ok_or(io::ErrorKind::PermissionDenied)?,
    )))
}
fn send(fd: &OwnedFd, bytes: &[u8]) -> io::Result<bool> {
    if bytes.is_empty() || bytes.len() > MAX_PRIVATE_PACKET {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    match net::send(
        fd,
        bytes,
        net::SendFlags::DONTWAIT | net::SendFlags::NOSIGNAL,
    ) {
        Ok(n) if n == bytes.len() => Ok(true),
        Ok(_) => Err(io::ErrorKind::WriteZero.into()),
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn private_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for parent in path.ancestors() {
        let m = fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.uid() != 0 || m.gid() != 0 || m.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    if fs::symlink_metadata(path)?.mode() & 0o7777 != 0o700 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
struct Listener {
    fd: OwnedFd,
    path: PathBuf,
    identity: (u64, u64),
}
impl Listener {
    fn new(path: PathBuf) -> io::Result<Self> {
        let fd = ipc::listener(&path, 0o600)?;
        let m = fs::symlink_metadata(&path)?;
        Ok(Self {
            fd,
            path,
            identity: (m.dev(), m.ino()),
        })
    }
    fn accept(&self) -> io::Result<Option<(OwnedFd, Peer)>> {
        match net::accept_with(&self.fd, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
            Ok(fd) => {
                net::sockopt::set_socket_passcred(&fd, true)?;
                let peer = match Peer::new(&fd) {
                    Ok(peer) => peer,
                    Err(_) => return Ok(None),
                };
                Ok(Some((fd, peer)))
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path).is_ok_and(|m| (m.dev(), m.ino()) == self.identity) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
struct Client {
    id: u64,
    fd: OwnedFd,
    peer: Peer,
    opened: bool,
    until: u64,
    last_state: Option<(String, bool)>,
    reply: Option<Vec<u8>>,
    reply_until: Option<u64>,
}
struct Read {
    client: u64,
    token: u64,
    provider_token: u64,
    until: u64,
    generation: String,
    retiring: bool,
}

/// Normal daemon: one Projector, no app endpoint, grant store or policy actor.
pub fn run(runtime: &Path, raw: &Path, inbox: &Path) -> io::Result<()> {
    let _termination = pokelinux_kernel_identity::TerminationGuard::install()?;
    let mut identities = std::collections::BTreeSet::new();
    for path in [runtime, raw, inbox] {
        private_directory(path)?;
        let m = fs::symlink_metadata(path)?;
        if !identities.insert((m.dev(), m.ino())) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
    }
    let read = Listener::new(runtime.join(READ_SOCKET))?;
    let admin = Listener::new(runtime.join("admin.sock"))?;
    let mut projector = Projector::spawn(runtime, raw, inbox, &std::env::current_exe()?)?;
    let outcome = serve(&mut projector, &read, &admin);
    // Stop and every error retain the same Projector until actual child waits.
    let until = boottime_ns().saturating_add(2_000_000_000);
    if !projector.drain_until(until) {
        std::process::abort();
    }
    outcome
}
fn serve(projector: &mut Projector, listener: &Listener, admin: &Listener) -> io::Result<()> {
    let mut clients: Vec<Client> = Vec::new();
    let mut reading: Option<Read> = None;
    let mut sequence = 0u64;
    let mut controls: Vec<(OwnedFd, Peer, u64, bool)> = Vec::new();
    loop {
        if pokelinux_kernel_identity::termination_requested() {
            return Ok(());
        }
        if controls.len() < 2
            && let Some((fd, peer)) = admin.accept()?
        {
            controls.push((
                fd,
                peer,
                boottime_ns().saturating_add(OPEN_BUDGET_NS),
                false,
            ));
        }
        let mut stopped = false;
        controls.retain_mut(|(fd, peer, until, ack)| {
            if boottime_ns() >= *until {
                return false;
            }
            if !*ack {
                match receive(fd) {
                    Ok(Some((bytes, cred)))
                        if peer.check(cred).is_ok()
                            && serde_json::from_slice::<Control>(&bytes).is_ok() =>
                    {
                        *ack = true
                    }
                    Ok(None) => return true,
                    _ => return false,
                }
            }
            // Existing messages-stop waits for this exact Main after ACK.
            match send(fd, br#"{"result":"ok"}"#) {
                Ok(true) => {
                    stopped = true;
                    false
                }
                Ok(false) => true,
                Err(_) => false,
            }
        });
        if stopped {
            return Ok(());
        }
        if clients.len() < MAX_CLIENTS
            && let Some((fd, peer)) = listener.accept()?
        {
            sequence = sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
            clients.push(Client {
                id: sequence,
                fd,
                peer,
                opened: false,
                until: boottime_ns().saturating_add(OPEN_BUDGET_NS),
                last_state: None,
                reply: None,
                reply_until: None,
            });
        }
        // Consume EOF/Cancel before polling a possible completed storage reply.
        let mut index = 0;
        while index < clients.len() {
            let client = &mut clients[index];
            let input = receive(&client.fd);
            let valid = match input {
                Ok(None) => client.opened || boottime_ns() < client.until,
                Ok(Some((bytes, cred))) if client.peer.check(cred).is_ok() => {
                    match serde_json::from_slice::<Request>(&bytes) {
                        Ok(Request::Open { version: VERSION }) if !client.opened => {
                            client.opened = true;
                            true
                        }
                        Ok(Request::Read {
                            version: VERSION,
                            generation,
                            token,
                            until_ns,
                            query,
                        }) if client.opened
                            && client.reply.is_none()
                            && reading.is_none()
                            && token > 0
                            && generation == projector.incarnation()
                            && projector.available()
                            && until_ns > boottime_ns()
                            && until_ns <= boottime_ns().saturating_add(OPEN_BUDGET_NS)
                            && query.validate().is_ok() =>
                        {
                            match projector.start(serde_json::to_value(query)?, until_ns) {
                                Ok(provider_token) => {
                                    reading = Some(Read {
                                        client: client.id,
                                        token,
                                        provider_token,
                                        until: until_ns,
                                        generation,
                                        retiring: false,
                                    });
                                    true
                                }
                                Err(_) => false,
                            }
                        }
                        Ok(Request::Cancel {
                            version: VERSION,
                            generation,
                            token,
                        }) if reading.as_ref().is_some_and(|r| {
                            r.client == client.id && r.token == token && r.generation == generation
                        }) =>
                        {
                            let r = reading.as_mut().unwrap();
                            r.retiring = true;
                            projector.cancel_read(r.provider_token);
                            true
                        }
                        Ok(Request::Restart { version: VERSION })
                            if client.opened && reading.is_none() =>
                        {
                            let _ = projector.restart_read_only();
                            client.last_state = None;
                            true
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
            if !valid {
                let id = client.id;
                if let Some(r) = &mut reading
                    && r.client == id
                {
                    r.retiring = true;
                    projector.cancel_read(r.provider_token);
                }
                clients.remove(index);
            } else {
                index += 1;
            }
        }
        if let Some(r) = &mut reading
            && boottime_ns() >= r.until
        {
            r.retiring = true;
            projector.cancel_read(r.provider_token);
        }
        if let Some(r) = &mut reading
            && !clients.iter().any(|c| c.id == r.client)
        {
            r.retiring = true;
            projector.cancel_read(r.provider_token);
        }
        let event = projector.poll()?;
        if let Some(r) = &reading {
            let response = match event {
                Some(Event::Complete { token, packet })
                    if token == r.provider_token
                        && !r.retiring
                        && boottime_ns() < r.until
                        && projector.incarnation() == r.generation =>
                {
                    let page: Page = serde_json::from_slice(&packet)?;
                    if boottime_ns() >= r.until {
                        None
                    } else {
                        Some(Response::Complete {
                            version: VERSION,
                            generation: r.generation.clone(),
                            token: r.token,
                            until_ns: r.until,
                            page,
                        })
                    }
                }
                Some(Event::Failed { token }) if token == r.provider_token && !r.retiring => {
                    Some(Response::Failed {
                        version: VERSION,
                        generation: r.generation.clone(),
                        token: r.token,
                        until_ns: r.until,
                    })
                }
                _ => None,
            };
            if let Some(response) = response {
                if let Some(client) = clients.iter_mut().find(|c| c.id == r.client) {
                    client.reply = Some(serde_json::to_vec(&response)?);
                    client.reply_until = Some(r.until);
                }
                reading = None;
            }
        }
        if reading
            .as_ref()
            .is_some_and(|r| r.retiring && !projector.read_occupied())
        {
            let r = reading.take().unwrap();
            if let Some(client) = clients.iter_mut().find(|c| c.id == r.client) {
                client.reply = Some(serde_json::to_vec(&Response::Retired {
                    version: VERSION,
                    generation: r.generation,
                    token: r.token,
                })?);
            }
        }
        clients.retain_mut(|client| {
            if client.peer.current().is_err() {
                return false;
            }
            if client
                .reply_until
                .is_some_and(|until| boottime_ns() >= until)
            {
                return false;
            }
            if client.reply.is_none() && client.opened {
                let state = (projector.incarnation().to_owned(), projector.available());
                if client.last_state.as_ref() != Some(&state) {
                    client.reply = serde_json::to_vec(&Response::State {
                        version: VERSION,
                        generation: state.0.clone(),
                        available: state.1,
                    })
                    .ok();
                    client.last_state = Some(state);
                }
            }
            match client.reply.as_ref().map(|bytes| send(&client.fd, bytes)) {
                Some(Ok(true)) => {
                    client.reply = None;
                    client.reply_until = None;
                    true
                }
                Some(Err(_)) => false,
                _ => true,
            }
        });
        std::thread::sleep(Duration::from_millis(2));
    }
}

struct Pending {
    token: u64,
    until: u64,
    wire: Option<Vec<u8>>,
    retiring: bool,
}
/// A session owns this connection/transaction, never the daemon or its children.
pub struct Remote {
    path: PathBuf,
    fd: Option<OwnedFd>,
    peer: Option<Peer>,
    generation: String,
    ready: bool,
    sequence: u64,
    pending: Option<Pending>,
    queued: std::collections::VecDeque<Vec<u8>>,
    opening_until: u64,
    retirement_uncertain: bool,
}
impl Remote {
    pub fn connect(path: &Path) -> io::Result<Self> {
        private_directory(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        let mut remote = Self {
            path: path.into(),
            fd: None,
            peer: None,
            generation: String::new(),
            ready: false,
            sequence: 0,
            pending: None,
            queued: std::collections::VecDeque::new(),
            opening_until: 0,
            retirement_uncertain: false,
        };
        remote.open()?;
        Ok(remote)
    }
    fn open(&mut self) -> io::Result<()> {
        let fd = ipc::connect(&self.path)?;
        let peer = Peer::new(&fd)?;
        self.fd = Some(fd);
        self.peer = Some(peer);
        self.ready = false;
        self.queued.clear();
        self.queued
            .push_back(serde_json::to_vec(&Request::Open { version: VERSION })?);
        self.opening_until = boottime_ns().saturating_add(HANDSHAKE_BUDGET_NS);
        Ok(())
    }
    fn close(&mut self) -> Option<Event> {
        self.retirement_uncertain |= self.pending.is_some();
        self.fd = None;
        self.peer = None;
        self.ready = false;
        self.queued.clear();
        self.pending
            .take()
            .map(|p| Event::Failed { token: p.token })
    }
    /// Local closure is not evidence that the server has reaped a reader.
    pub fn retirement_uncertain(&self) -> bool {
        self.retirement_uncertain
    }
}
impl Provider for Remote {
    fn incarnation(&self) -> &str {
        &self.generation
    }
    fn occupied(&self) -> bool {
        self.fd.is_some()
    }
    fn read_occupied(&self) -> bool {
        self.pending.is_some()
    }
    fn busy(&self) -> bool {
        self.pending.is_some() || !self.queued.is_empty()
    }
    fn available(&self) -> bool {
        self.ready && self.peer.as_ref().is_some_and(|p| p.current().is_ok())
    }
    fn start(&mut self, query: serde_json::Value, until: u64) -> io::Result<u64> {
        let query: Query = serde_json::from_value(query)?;
        query.validate().map_err(|_| io::ErrorKind::InvalidInput)?;
        let now = boottime_ns();
        if !self.available()
            || self.busy()
            || now >= until
            || until > now.saturating_add(OPEN_BUDGET_NS)
        {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.sequence = self.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        self.pending = Some(Pending {
            token: self.sequence,
            until,
            wire: Some(serde_json::to_vec(&Request::Read {
                version: VERSION,
                generation: self.generation.clone(),
                token: self.sequence,
                until_ns: until,
                query,
            })?),
            retiring: false,
        });
        Ok(self.sequence)
    }
    fn cancel_read(&mut self, token: u64) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.token == token && !p.retiring && p.wire.is_some())
        {
            // No send succeeded: no server read or retirement is inferred.
            self.pending = None;
            return;
        }
        if let Some(p) = &mut self.pending
            && p.token == token
            && !p.retiring
        {
            p.retiring = true;
            self.ready = false;
            match serde_json::to_vec(&Request::Cancel {
                version: VERSION,
                generation: self.generation.clone(),
                token,
            }) {
                Ok(bytes) => p.wire = Some(bytes),
                Err(_) => {
                    let _ = self.close();
                }
            }
        }
    }
    fn poll(&mut self) -> io::Result<Option<Event>> {
        let now = boottime_ns();
        if self.fd.is_none() {
            return Ok(None);
        }
        if self.peer.as_ref().is_none_or(|p| p.current().is_err())
            || self.pending.as_ref().is_some_and(|p| now >= p.until)
            || (!self.ready && self.pending.is_none() && now >= self.opening_until)
        {
            return Ok(self.close());
        }
        let fd = self.fd.as_ref().unwrap();
        if let Some(bytes) = self.queued.front() {
            match send(fd, bytes) {
                Ok(true) => {
                    self.queued.pop_front();
                }
                Ok(false) => return Ok(None),
                Err(_) => return Ok(self.close()),
            }
        }
        if let Some(p) = &mut self.pending
            && let Some(bytes) = &p.wire
        {
            match send(fd, bytes) {
                Ok(true) => p.wire = None,
                Ok(false) => return Ok(None),
                Err(_) => return Ok(self.close()),
            }
        }
        let (bytes, credentials) = match receive(fd) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(None),
            Err(_) => return Ok(self.close()),
        };
        if self.peer.as_ref().unwrap().check(credentials).is_err() {
            return Ok(self.close());
        }
        let response = match serde_json::from_slice::<Response>(&bytes) {
            Ok(v) => v,
            Err(_) => return Ok(self.close()),
        };
        // Include receive and decoding in the original absolute read deadline.
        let now = boottime_ns();
        if self.pending.as_ref().is_some_and(|p| now >= p.until)
            || (!self.ready && self.pending.is_none() && now >= self.opening_until)
        {
            return Ok(self.close());
        }
        match response {
            Response::State {
                version: VERSION,
                generation,
                available,
            } if (!available && generation.is_empty()) || DataId::parse(&generation).is_ok() => {
                if self.pending.is_some() && generation != self.generation {
                    return Ok(self.close());
                }
                self.generation = generation;
                self.ready = available;
                Ok(None)
            }
            Response::Complete {
                version: VERSION,
                generation,
                token,
                until_ns,
                page,
            } if self.pending.as_ref().is_some_and(|p| {
                !p.retiring && p.wire.is_none() && p.token == token && p.until == until_ns
            }) && generation == self.generation
                && self.ready =>
            {
                let packet = serde_json::to_vec(&page)?;
                if packet.len() > 4096 || boottime_ns() >= until_ns {
                    return Ok(self.close());
                }
                self.pending = None;
                Ok(Some(Event::Complete { token, packet }))
            }
            Response::Failed {
                version: VERSION,
                generation,
                token,
                until_ns,
            } if generation == self.generation
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.token == token && p.until == until_ns) =>
            {
                self.pending = None;
                self.ready = false;
                Ok(Some(Event::Failed { token }))
            }
            Response::Retired {
                version: VERSION,
                generation,
                token,
            } if generation == self.generation
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.token == token && p.retiring) =>
            {
                self.pending = None;
                self.retirement_uncertain = false;
                self.ready = false;
                Ok(Some(Event::Failed { token }))
            }
            _ => Ok(self.close()),
        }
    }
    fn stop(&mut self) {
        let _ = self.close();
    }
    fn restart(&mut self) -> io::Result<()> {
        if self.pending.is_some() || !self.queued.is_empty() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if self.fd.is_none() {
            self.open()?;
        }
        // Open is sent first; one explicit restart is sent in a later poll.
        self.queued
            .push_back(serde_json::to_vec(&Request::Restart { version: VERSION })?);
        self.opening_until = boottime_ns().saturating_add(HANDSHAKE_BUDGET_NS);
        self.ready = false;
        Ok(())
    }
    fn drain_until(&mut self, _until: u64) -> bool {
        self.stop();
        !self.occupied()
    }
}

#[cfg(test)]
#[path = "read_service_tests.rs"]
mod tests;

#[cfg(test)]
mod restart_admission_tests {
    use super::*;
    #[test]
    fn repeated_restart_retains_one_bounded_request() {
        let (fd, _peer) = net::socketpair(
            net::AddressFamily::UNIX,
            net::SocketType::SEQPACKET,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .unwrap();
        let mut remote = Remote {
            path: "/not-used".into(),
            fd: Some(fd),
            peer: None,
            generation: String::new(),
            ready: false,
            sequence: 0,
            pending: None,
            queued: std::collections::VecDeque::from([serde_json::to_vec(&Request::Restart {
                version: VERSION,
            })
            .unwrap()]),
            opening_until: boottime_ns() + OPEN_BUDGET_NS,
            retirement_uncertain: false,
        };
        let result = remote.restart();
        assert_eq!(
            result.unwrap_err().kind(),
            io::ErrorKind::WouldBlock,
            "queued restart must refuse another admission"
        );
        assert_eq!(remote.queued.len(), 1);
    }
}
