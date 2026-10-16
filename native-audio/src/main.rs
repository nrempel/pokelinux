use pokelinux_audio::pipewire::{ObservedLink, Profile};
use pokelinux_audio::{Authority, Identity, Role};
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Duration;

#[derive(Clone, PartialEq, Eq, Serialize)]
struct Observation {
    epoch: u64,
    revision: u64,
    available: bool,
    links: Vec<ObservedLink>,
}

struct State {
    authority: Authority,
    profile: Profile,
    profile_path: String,
    process: Option<pokelinux_audio::backend_process::Process>,
    cookie: Option<u64>,
    observed: Observation,
    intent: u64,
    token: u64,
    sent: Option<(u64, u64, bool)>,
    applied: Option<u64>,
    needs_apply: bool,
    recovery_required: bool,
    stopping: bool,
    last_poll: BootTime,
}
impl State {
    fn invalidate(&mut self) {
        self.observed.epoch += 1;
        self.observed.revision += 1;
        self.observed.available = false;
        self.observed.links.clear();
        self.cookie = None;
        self.applied = None;
        self.sent = None;
    }
    fn retire(&mut self) {
        // Fence buffered output and current application before signaling the worker.
        self.invalidate();
        if let Some(process) = &mut self.process {
            process.retire();
        }
    }
    fn change(&mut self) {
        self.intent = self
            .intent
            .checked_add(1)
            .expect("audio intent revision overflow");
        self.applied = None;
        self.needs_apply = true;
    }
    fn poll(&mut self) {
        if let Some(process) = &mut self.process {
            match process.poll() {
                Ok(Some(reply)) => {
                    if self.cookie != reply.cookie {
                        self.observed.epoch += 1;
                        self.applied = None;
                        self.cookie = reply.cookie;
                    }
                    self.observed.revision += 1;
                    self.observed.available = reply.cookie.is_some() && reply.error.is_none();
                    self.observed.links = reply.links;
                    let matching =
                        pokelinux_audio::pipewire::wanted(&self.profile, &self.authority.desired())
                            .is_ok_and(|wanted| {
                                let actual: std::collections::BTreeSet<_> = self
                                    .observed
                                    .links
                                    .iter()
                                    .filter(|link| link.state == "active")
                                    .map(|link| (link.source.clone(), link.target.clone()))
                                    .collect();
                                self.observed.links.len() == wanted.len() && actual == wanted
                            });
                    self.applied = (self.sent.is_some_and(|(token, intent, _)| {
                        token == reply.token && intent == self.intent
                    }) && self.observed.available
                        && matching)
                        .then_some(self.intent);
                    self.sent = None;
                }
                Ok(None) => {}
                Err(_) => {
                    self.retire();
                    self.recovery_required = true;
                    self.needs_apply = false;
                }
            }
        }
        if self
            .process
            .as_ref()
            .is_some_and(|process| process.reaped())
        {
            self.process.take(); // Already reaped; never releases occupied live capacity.
        }
        if self.stopping || self.recovery_required {
            return;
        }
        if self.process.is_none() {
            match pokelinux_audio::backend_process::Process::start(&self.profile_path) {
                Ok(process) => self.process = Some(process),
                Err(_) => {
                    self.invalidate();
                    self.recovery_required = true;
                    return;
                }
            }
        }
        let now = BootClock.now();
        if self.process.as_ref().is_some_and(|process| process.idle())
            && (self.needs_apply
                || now.elapsed().saturating_sub(self.last_poll.elapsed())
                    >= Duration::from_millis(150))
        {
            self.token = self
                .token
                .checked_add(1)
                .expect("audio source token overflow");
            let applying = self.needs_apply;
            let job = pokelinux_audio::backend_process::Job {
                token: self.token,
                desired: applying.then(|| self.authority.desired()),
            };
            if self.process.as_mut().unwrap().submit(job).is_err() {
                self.retire();
                self.recovery_required = true;
                self.needs_apply = false;
                return;
            }
            self.sent = Some((self.token, self.intent, applying));
            self.needs_apply = false;
            self.last_poll = now;
        }
    }
    fn request(&mut self, peer: Identity, value: Value) -> Value {
        if !self.authority.is_allowed(peer) {
            return json!({"ok":false,"error":"denied"});
        }
        let mut lease = None;
        let operation = (|| -> Result<(), &'static str> {
            let op = value["op"].as_str().ok_or("invalid_request")?;
            if self.stopping && op != "status" {
                return Err("service_stopping");
            }
            match op {
                "status" => {}
                "stop" => {
                    if !self.authority.is_privileged(peer) {
                        return Err("denied");
                    }
                    self.stopping = true;
                    self.authority.cancel_all();
                    self.change();
                    self.needs_apply = false;
                    self.retire();
                }
                "acquire" => {
                    if self.recovery_required {
                        return Err("explicit_recovery_required");
                    }
                    let role: Role = serde_json::from_value(value["role"].clone())
                        .map_err(|_| "invalid_role")?;
                    let route = value["route"].as_str().ok_or("invalid_route")?;
                    if !self.profile.supports(role, route) {
                        return Err("unsupported_route");
                    }
                    lease = Some(self.authority.acquire(peer, role, route)?);
                    self.change();
                }
                "release" => {
                    let id = value["lease"].as_u64().ok_or("invalid_lease")?;
                    let capture = self.authority.role(peer, id)? == Role::Capture;
                    self.authority.release(peer, id)?;
                    self.change();
                    if capture {
                        self.retire();
                    }
                }
                "route" => {
                    if self.recovery_required {
                        return Err("explicit_recovery_required");
                    }
                    let id = value["lease"].as_u64().ok_or("invalid_lease")?;
                    let route = value["route"].as_str().ok_or("invalid_route")?;
                    let role = self.authority.role(peer, id)?;
                    if !self.profile.supports(role, route) {
                        return Err("unsupported_route");
                    }
                    self.authority.set_route(peer, id, route)?;
                    self.change();
                }
                "grant_capture" | "revoke_capture" => {
                    self.authority
                        .set_capture_permission(peer, op == "grant_capture")?;
                    self.change();
                    if op == "revoke_capture" {
                        self.retire();
                    }
                }
                "reconcile" => {
                    if !self.authority.is_privileged(peer) {
                        return Err("denied");
                    }
                    self.recovery_required = false;
                    self.change();
                }
                _ => return Err("invalid_request"),
            }
            Ok(())
        })();
        if let Err(error) = operation {
            return json!({"ok":false,"error":error});
        }
        let mut response = json!({"ok":true,"lease":lease,"peer_uid":peer.uid,"desired":self.authority.desired(),
            "observed":self.observed,"intent_revision":self.intent,
            "application":if self.recovery_required { "uncertain" } else if self.applied == Some(self.intent) { "observed" } else { "pending" },
            "phase":if self.stopping { "stopping" } else { "running" },
            "cleanup":if self.stopping { "pending" } else { "none" }});
        if value["op"] == "status" {
            response["media_routes"] = json!(self.profile.media_routes());
        }
        response
    }
}

enum Message {
    Request {
        peer: Identity,
        value: Value,
        cancelled: Arc<AtomicBool>,
        reply: SyncSender<Value>,
    },
    Disconnect(Identity),
}

fn serve(
    mut stream: UnixStream,
    sender: SyncSender<Message>,
    peer: Identity,
    shutting: Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(read_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_stream);
    let mut pending = Vec::new();
    while !shutting.load(Ordering::Acquire) {
        let read = reader
            .by_ref()
            .take((4097 - pending.len()) as u64)
            .read_until(b'\n', &mut pending);
        if pending.len() > 4096 {
            break;
        }
        match read {
            Ok(0) => break,
            Ok(_) if pending.last() == Some(&b'\n') => {
                let response = match serde_json::from_slice(&pending) {
                    Ok(value) => {
                        let (reply, receiver) = mpsc::sync_channel(1);
                        let cancelled = Arc::new(AtomicBool::new(false));
                        if sender
                            .send(Message::Request {
                                peer,
                                value,
                                cancelled: cancelled.clone(),
                                reply,
                            })
                            .is_err()
                        {
                            break;
                        }
                        match receiver.recv_timeout(Duration::from_secs(10)) {
                            Ok(response) => response,
                            Err(_) => {
                                cancelled.store(true, Ordering::Release);
                                break;
                            }
                        }
                    }
                    Err(_) => json!({"ok": false, "error": "invalid_request"}),
                };
                pending.clear();
                if writeln!(stream, "{response}").is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => break,
        }
    }
    let _ = sender.send(Message::Disconnect(peer));
}

// Retain every frontend until joining it. Unexpected actor exit cannot detach
// a live request owner; abort also triggers the backend's kernel parent guard.
struct Frontends(Vec<(u32, thread::JoinHandle<()>)>);
impl std::ops::Deref for Frontends {
    type Target = Vec<(u32, thread::JoinHandle<()>)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Frontends {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for Frontends {
    fn drop(&mut self) {
        if self.0.iter().any(|(_, handle)| !handle.is_finished()) {
            eprintln!("audio actor exited with an owned frontend still running");
            std::process::abort();
        }
        for (_, handle) in self.0.drain(..) {
            let _ = handle.join();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--backend-worker") {
        if args.len() != 4 {
            return Err("invalid backend worker".into());
        }
        rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
        let expected: u32 = args[2].parse()?;
        if rustix::process::getppid().is_none_or(|parent| parent.as_raw_pid() as u32 != expected) {
            return Err("owned backend parent already died".into());
        }
        return pokelinux_audio::backend_process::run(serde_json::from_slice(&std::fs::read(
            &args[3],
        )?)?);
    }
    if args.get(1).is_some_and(|arg| arg == "--tool-worker") {
        if args.len() < 4 || !matches!(args[3].as_str(), "/usr/bin/pw-link" | "/usr/bin/pw-dump") {
            return Err("invalid owned worker".into());
        }
        // Set the kernel guard before checking the parent: catches death between fork and prctl.
        rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
        let expected: u32 = args[2].parse()?;
        if rustix::process::getppid().is_none_or(|parent| parent.as_raw_pid() as u32 != expected) {
            return Err("owned worker parent already died".into());
        }
        return Err(Command::new(&args[3]).args(&args[4..]).exec().into());
    }
    if args.len() != 5 {
        return Err("usage: pokelinux-audio SOCKET PROFILE_JSON CALL_UID APP_UID".into());
    }
    let call_uid: u32 = args[3].parse()?;
    let app_uid: u32 = args[4].parse()?;
    if !pokelinux_audio::validate_service_uids(
        rustix::process::getuid().as_raw(),
        call_uid,
        app_uid,
    ) {
        return Err("audio owner, call owner and app UIDs must be distinct".into());
    }
    let profile: Profile = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let mut state = State {
        authority: Authority::new(call_uid, app_uid, false),
        profile,
        profile_path: args[2].clone(),
        process: None,
        intent: 0,
        token: 0,
        sent: None,
        applied: None,
        needs_apply: false,
        recovery_required: false,
        stopping: false,
        last_poll: BootTime::default(),
        cookie: None,
        observed: Observation {
            epoch: 0,
            revision: 0,
            available: false,
            links: Vec::new(),
        },
    };
    let listener = UnixListener::bind(&args[1])?;
    std::fs::set_permissions(&args[1], std::fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    let (sender, receiver) = mpsc::sync_channel::<Message>(64);
    let mut connection = 0_u64;
    let mut workers = Frontends(Vec::new());
    let shutting = Arc::new(AtomicBool::new(false));
    let mut stop_deadline = None;
    loop {
        let mut i = 0;
        while i < workers.len() {
            if workers[i].1.is_finished() {
                let _ = workers.swap_remove(i).1.join();
            } else {
                i += 1;
            }
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let credentials = rustix::net::sockopt::socket_peercred(&stream)?;
                let uid = credentials.uid.as_raw();
                // Reserve independent authenticated budgets within the 32-worker bound.
                let (used, limit) = if uid == call_uid {
                    (
                        workers.iter().filter(|(uid, _)| *uid == call_uid).count(),
                        8,
                    )
                } else if uid == app_uid {
                    (
                        workers.iter().filter(|(uid, _)| *uid == app_uid).count(),
                        16,
                    )
                } else {
                    (
                        workers
                            .iter()
                            .filter(|(uid, _)| *uid != call_uid && *uid != app_uid)
                            .count(),
                        8,
                    )
                };
                if used < limit {
                    connection = connection.checked_add(1).ok_or("connection_limit")?;
                    let peer = Identity { uid, connection };
                    let sender = sender.clone();
                    let shutting = shutting.clone();
                    workers.push((
                        uid,
                        thread::spawn(move || serve(stream, sender, peer, shutting)),
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        // Backend IO is confined to one owned process; the authority only polls its pipes.
        while let Ok(message) = receiver.try_recv() {
            match message {
                Message::Request {
                    peer,
                    value,
                    cancelled,
                    reply,
                } => {
                    if !cancelled.load(Ordering::Acquire) {
                        let _ = reply.try_send(state.request(peer, value));
                    }
                }
                Message::Disconnect(peer) => {
                    if state.authority.disconnect(peer) && !state.stopping {
                        state.change();
                        // Disconnect revokes any capture intent before old work can continue.
                        state.retire();
                    }
                }
            }
        }
        state.poll();
        if state.stopping {
            shutting.store(true, Ordering::Release);
            let end = *stop_deadline.get_or_insert_with(|| {
                BootClock
                    .now()
                    .checked_after(Duration::from_secs(3))
                    .expect("stop deadline")
            });
            if state.process.is_none() && workers.is_empty() {
                return Ok(());
            }
            if BootClock.now() >= end {
                eprintln!("audio stop owners did not finish");
                std::process::abort();
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}
