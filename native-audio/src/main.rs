use pokelinux_audio::pipewire::{self, ObservedLink, Profile};
use pokelinux_audio::{Authority, Identity, Role};
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
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Eq, Serialize)]
struct Observation {
    epoch: u64,
    revision: u64,
    available: bool,
    links: Vec<ObservedLink>,
}

struct State {
    authority: Authority,
    backend: pipewire::Backend,
    profile: Profile,
    cookie: Option<u64>,
    observed: Observation,
}

impl State {
    fn refresh(&mut self) {
        let graph = pipewire::read_graph();
        let (available, cookie, links) = match graph {
            Ok(graph) => (true, Some(graph.cookie()), graph.observed(&self.profile)),
            Err(_) => (false, None, Vec::new()),
        };
        if self.cookie != cookie {
            self.backend.stop_all();
            self.observed.epoch += 1;
            self.cookie = cookie;
        }
        if self.observed.available != available || self.observed.links != links {
            self.observed.revision += 1;
        }
        self.observed.available = available;
        self.observed.links = links;
    }

    fn reconcile(&mut self) -> Result<(), &'static str> {
        let graph = pipewire::read_graph()?;
        let desired = self.authority.desired();
        self.backend.apply(&self.profile, &desired, &graph)?;
        let wanted = pipewire::wanted(&self.profile, &desired)?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            self.refresh();
            let actual: std::collections::BTreeSet<_> = self
                .observed
                .links
                .iter()
                .filter(|link| link.state == "active")
                .map(|link| (link.source.clone(), link.target.clone()))
                .collect();
            if self.observed.available
                && wanted == actual
                && self.observed.links.len() == wanted.len()
            {
                return Ok(());
            }
            if Instant::now() >= deadline || !self.observed.available {
                return Err("application_unobserved");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn request(&mut self, peer: Identity, value: Value) -> Value {
        if !self.authority.is_allowed(peer) {
            return json!({"ok": false, "error": "denied"});
        }
        let mut lease = None;
        let mut changed = false;
        let operation = (|| -> Result<(), &'static str> {
            match value["op"].as_str().ok_or("invalid_request")? {
                "status" => {}
                "acquire" => {
                    let role: Role = serde_json::from_value(value["role"].clone())
                        .map_err(|_| "invalid_role")?;
                    let route = value["route"].as_str().ok_or("invalid_route")?;
                    if !self.profile.supports(role, route) {
                        return Err("unsupported_route");
                    }
                    lease = Some(self.authority.acquire(peer, role, route)?);
                    changed = true;
                }
                "release" => {
                    self.authority
                        .release(peer, value["lease"].as_u64().ok_or("invalid_lease")?)?;
                    changed = true;
                }
                "route" => {
                    let id = value["lease"].as_u64().ok_or("invalid_lease")?;
                    let route = value["route"].as_str().ok_or("invalid_route")?;
                    // Validate the lease's actual role, including interrupted leases.
                    let role = self.authority.role(peer, id)?;
                    if !self.profile.supports(role, route) {
                        return Err("unsupported_route");
                    }
                    self.authority.set_route(peer, id, route)?;
                    changed = true;
                }
                "grant_capture" | "revoke_capture" => {
                    self.authority
                        .set_capture_permission(peer, value["op"] == "grant_capture")?;
                    changed = true;
                }
                "reconcile" => {
                    if !self.authority.is_privileged(peer) {
                        return Err("denied");
                    }
                    changed = true;
                }
                _ => return Err("invalid_request"),
            }
            Ok(())
        })();
        if let Err(error) = operation {
            return json!({"ok": false, "error": error});
        }
        let result = if changed { self.reconcile() } else { Ok(()) };
        let mut response = json!({"ok": result.is_ok(), "lease": lease, "peer_uid": peer.uid,
            "desired": self.authority.desired(), "observed": self.observed });
        if let Err(error) = result {
            self.refresh();
            response["error"] = json!(error);
            response["observed"] = json!(self.observed);
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

fn serve(mut stream: UnixStream, sender: SyncSender<Message>, peer: Identity) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(read_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_stream);
    let mut pending = Vec::new();
    loop {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
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
        backend: pipewire::Backend::default(),
        profile,
        cookie: None,
        observed: Observation {
            epoch: 0,
            revision: 0,
            available: false,
            links: Vec::new(),
        },
    };
    state.refresh();
    let listener = UnixListener::bind(&args[1])?;
    std::fs::set_permissions(&args[1], std::fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    let (sender, receiver) = mpsc::sync_channel::<Message>(64);
    let mut connection = 0_u64;
    let mut workers = Vec::<(u32, thread::JoinHandle<()>)>::new();
    let mut last_observation = Instant::now();
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
                    workers.push((uid, thread::spawn(move || serve(stream, sender, peer))));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        // All original-stack children are launched by this stable main thread.
        while let Ok(message) = receiver.try_recv() {
            match message {
                Message::Request {
                    peer,
                    value,
                    cancelled,
                    reply,
                } => {
                    if !cancelled.load(Ordering::Acquire) {
                        let _ = reply.send(state.request(peer, value));
                    }
                }
                Message::Disconnect(peer) => {
                    if state.authority.disconnect(peer) {
                        let _ = state.reconcile();
                    }
                }
            }
        }
        if last_observation.elapsed() >= Duration::from_millis(150) {
            state.refresh();
            last_observation = Instant::now();
        }
        thread::sleep(Duration::from_millis(10));
    }
}
