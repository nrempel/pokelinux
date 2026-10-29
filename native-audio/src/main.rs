use pokelinux_audio::pipewire::{MediaSource, ObservedLink, Profile, SourceIdentity};
use pokelinux_audio::{Authority, Identity, PreviewLevel, PreviewState, Role};
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

fn preview_level(value: &Value) -> Result<PreviewLevel, &'static str> {
    PreviewLevel::new(
        value["level_percent"]
            .as_u64()
            .ok_or("invalid_preview_level")?,
        value["muted"].as_bool().ok_or("invalid_preview_level")?,
    )
    .ok_or("invalid_preview_level")
}

fn preview_deadlines(
    now: BootTime,
    until_ns: u64,
    startup_until_ns: u64,
) -> Result<(BootTime, BootTime, BootTime), &'static str> {
    let admission = BootTime::from_elapsed(Duration::from_nanos(until_ns));
    let start = BootTime::from_elapsed(Duration::from_nanos(startup_until_ns));
    if admission <= now
        || start < admission
        || start
            > now
                .checked_after(Duration::from_secs(1))
                .ok_or("deadline_overflow")?
    {
        return Err("preview_expired");
    }
    Ok((
        admission,
        start,
        now.checked_after(Duration::from_secs(6))
            .ok_or("deadline_overflow")?,
    ))
}

struct Preview {
    peer: Identity,
    generation: u64,
    lease: u64,
    route: String,
    process: pokelinux_audio::preview_process::Process,
    start_until: BootTime,
    work_until: BootTime,
    state: PreviewState,
    terminal: PreviewState,
    natural_drain: bool,
    identity: Option<SourceIdentity>,
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
    policy_settled: bool,
    media_present: bool,
    preview: Option<Preview>,
    preview_sequence: u64,
    last_preview: Option<(Identity, PreviewState)>,
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
        self.policy_settled = false;
        self.sent = None;
    }
    fn retire(&mut self) {
        self.invalidate();
        self.retire_preview(PreviewState::Failed);
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
        self.policy_settled = false;
        self.needs_apply = true;
    }
    #[track_caller]
    fn retire_preview(&mut self, terminal: PreviewState) {
        if let Some(preview) = &mut self.preview
            && (preview.state != PreviewState::Retiring || preview.natural_drain)
        {
            // Fixed local lifecycle facts only: no app requests, tokens or PCM.
            eprintln!(
                "preview retirement: state={:?} terminal={:?} caller_line={}",
                preview.state,
                terminal,
                std::panic::Location::caller().line()
            );
            preview.state = PreviewState::Retiring;
            preview.terminal = terminal;
            preview.natural_drain = false;
            preview.process.retire();
            self.change();
        }
    }
    fn stop(&mut self) {
        if self.stopping {
            return;
        }
        self.stopping = true;
        self.authority.cancel_all();
        self.change();
        self.retire();
        self.needs_apply = false;
    }
    fn media_source(&self) -> MediaSource {
        match &self.preview {
            Some(p) if p.state == PreviewState::Retiring => MediaSource::Retiring,
            Some(p) => MediaSource::Owned {
                pid: p.process.pid(),
                uid: rustix::process::getuid().as_raw(),
                gid: rustix::process::getgid().as_raw(),
                cookie: self.cookie.unwrap_or(0),
                identity: p.identity.clone(),
            },
            None => MediaSource::Automatic,
        }
    }
    fn preview_for(&self, peer: Identity) -> PreviewState {
        self.preview
            .as_ref()
            .filter(|p| p.peer == peer)
            .map(|p| p.state)
            .or_else(|| {
                self.last_preview
                    .filter(|(owner, _)| *owner == peer)
                    .map(|(_, state)| state)
            })
            .unwrap_or_default()
    }
    fn current_preview_for(&self, peer: Identity) -> Option<&Preview> {
        self.preview.as_ref().filter(|p| {
            p.peer == peer && matches!(p.state, PreviewState::Starting | PreviewState::Playing)
        })
    }
    fn mute_preview(&mut self, peer: Identity, value: &Value) -> Result<(), &'static str> {
        if value.as_object().is_none_or(|object| object.len() != 5) {
            return Err("invalid_request");
        }
        let id = value["lease"].as_u64().ok_or("invalid_lease")?;
        if self.authority.role(peer, id)? != Role::Media {
            return Err("invalid_role");
        }
        let generation = value["generation"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("invalid_preview_generation")?;
        let muted = value["muted"].as_bool().ok_or("invalid_preview_mute")?;
        let until_ns = value["until_boottime_ns"]
            .as_u64()
            .ok_or("invalid_deadline")?;
        let now = BootClock.now();
        let until = BootTime::from_elapsed(Duration::from_nanos(until_ns));
        if until <= now
            || now
                .checked_after(Duration::from_secs(1))
                .is_none_or(|max| until > max)
        {
            return Err("preview_expired");
        }
        if self.recovery_required || !self.observed.available || !self.policy_settled {
            return Err("backend_unavailable");
        }
        self.enforce_preview_route();
        let p = self
            .preview
            .as_mut()
            .filter(|p| {
                p.peer == peer
                    && p.lease == id
                    && p.generation == generation
                    && p.state == PreviewState::Playing
            })
            .ok_or("preview_not_current")?;
        p.process
            .set_live_muted(muted, until)
            .map_err(|_| "preview_mute_refused")?;
        // This control changes only the exact producer's future PCM. It does not
        // revise route intent, relink the graph, restart a producer or renew work.
        Ok(())
    }
    fn enforce_preview_route(&mut self) {
        if self.preview.as_ref().is_some_and(|p| {
            matches!(p.state, PreviewState::Starting | PreviewState::Playing)
                && !self.authority.desired().iter().any(|(id, role, route)| {
                    *id == p.lease && *role == Role::Media && route == &p.route
                })
        }) {
            self.retire_preview(PreviewState::Failed);
        }
    }
    fn poll_preview(&mut self) {
        let Some(p) = &mut self.preview else {
            return;
        };
        let now = BootClock.now();
        let finished = p.process.poll();
        let mut changed = false;
        let mut failure = false;
        let mut failure_predicate = "none";
        let mut starting_alive = None;
        let mut finished_successful = None;
        // Retain work/start/alive short-circuit order; labels never probe again.
        if let Some(done) = finished {
            if p.state == PreviewState::Playing || p.natural_drain {
                eprintln!(
                    "preview producer completion: successful={}",
                    done.successful
                );
                p.terminal = if done.successful {
                    PreviewState::Completed
                } else {
                    PreviewState::Failed
                };
                p.state = PreviewState::Retiring;
                p.natural_drain = false;
                changed = true;
            } else if p.state == PreviewState::Starting {
                eprintln!("preview producer completed before playback admission");
                failure_predicate = "producer_completed_while_starting";
                finished_successful = Some(done.successful);
                failure = true;
            }
        } else {
            if now >= p.work_until {
                failure_predicate = "work_cutoff";
                failure = true;
            } else if p.state == PreviewState::Starting {
                if now >= p.start_until {
                    failure_predicate = "startup_cutoff";
                    failure = true;
                } else {
                    let alive = p.process.alive();
                    starting_alive = Some(alive);
                    if !alive {
                        failure_predicate = "starting_producer_not_alive";
                        failure = true;
                    }
                }
            }
            if !failure && p.state == PreviewState::Playing && !p.process.alive() {
                // Death fences Playing immediately; retain natural completion until
                // actual wait and stderr EOF decide Completed versus Failed.
                eprintln!("preview producer no longer alive; completion still pending");
                p.state = PreviewState::Retiring;
                p.terminal = PreviewState::Failed;
                p.natural_drain = true;
                changed = true;
            }
        }
        let sampled_state = p.state;
        let sampled_start_until = p.start_until;
        let sampled_work_until = p.work_until;
        if changed {
            self.change();
        }
        if failure {
            // `now` precedes the existing process.poll(); None means the
            // original short-circuit did not evaluate that liveness check.
            if sampled_state != PreviewState::Retiring {
                eprintln!(
                    "preview retirement predicate: path=poll_preview predicate={} state={:?} poll_sampled_at_ns={} forwarded_start_until_ns={} work_until_ns={} starting_alive={:?} finished_successful={:?}",
                    failure_predicate,
                    sampled_state,
                    now.elapsed().as_nanos(),
                    sampled_start_until.elapsed().as_nanos(),
                    sampled_work_until.elapsed().as_nanos(),
                    starting_alive,
                    finished_successful
                );
            }
            self.retire_preview(PreviewState::Failed);
        }
        if self.stopping && self.preview.as_ref().is_some_and(|p| p.process.settled()) {
            self.preview.take();
        }
    }
    fn poll(&mut self) {
        self.enforce_preview_route();
        self.poll_preview();
        if let Some(process) = &mut self.process {
            match process.poll() {
                Ok(Some(reply)) => {
                    if self.preview.is_some()
                        && let Some(error) = &reply.error
                    {
                        // Backend errors are fixed local protocol reasons, not app data.
                        eprintln!("preview backend job failed: {error}");
                    }
                    let cookie_changed = self.cookie != reply.cookie;
                    if cookie_changed {
                        self.observed.epoch += 1;
                        self.applied = None;
                        self.cookie = reply.cookie;
                        if self.preview.is_some() {
                            self.retire_preview(PreviewState::Failed);
                        }
                    }
                    self.observed.revision += 1;
                    self.observed.available = reply.cookie.is_some() && reply.error.is_none();
                    self.media_present = reply.media_present;
                    self.observed.links = reply.links;
                    let current = self.sent.is_some_and(|(token, intent, _)| {
                        token == reply.token && intent == self.intent
                    });
                    self.policy_settled =
                        current && self.observed.available && reply.policy_settled;
                    self.applied = (current && self.observed.available && reply.applied)
                        .then_some(self.intent);
                    self.sent = None;
                    if let Some(p) = &mut self.preview {
                        if p.state == PreviewState::Starting && current {
                            if self.policy_settled
                                && BootClock.now() < p.start_until
                                && p.process.alive()
                                && reply.preview_identity.is_some()
                            {
                                p.identity = reply.preview_identity;
                                p.state = PreviewState::Playing;
                            } else {
                                self.retire_preview(PreviewState::Failed);
                            }
                        } else if p.state == PreviewState::Playing
                            && (!self.observed.available
                                || reply.preview_identity.as_ref() != p.identity.as_ref())
                        {
                            self.retire_preview(PreviewState::Failed);
                        }
                    }
                    // Only a later successful absent-source graph can release a retiring tombstone.
                    if current
                        && self.policy_settled
                        && self.observed.available
                        && !self.media_present
                        && self.preview.as_ref().is_some_and(|p| {
                            p.state == PreviewState::Retiring && p.process.settled()
                        })
                    {
                        let p = self.preview.take().unwrap();
                        self.last_preview = Some((p.peer, p.terminal));
                        self.change();
                    }
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
            self.process.take();
        }
        if self.stopping || self.recovery_required {
            return;
        }
        if self.process.is_none() {
            match pokelinux_audio::backend_process::Process::start(&self.profile_path) {
                Ok(process) => self.process = Some(process),
                Err(_) => {
                    self.invalidate();
                    self.retire_preview(PreviewState::Failed);
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
                desired: self.authority.desired(),
                apply: applying,
                media: self.media_source(),
                until_boottime_ns: self
                    .preview
                    .as_ref()
                    .filter(|p| p.state == PreviewState::Starting)
                    .map(|p| {
                        p.start_until
                            .elapsed()
                            .as_nanos()
                            .try_into()
                            .unwrap_or(u64::MAX)
                    }),
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
    fn start_preview(&mut self, peer: Identity, value: &Value) -> Result<u64, &'static str> {
        let object = value.as_object().ok_or("invalid_request")?;
        if object.len() != 7 || !object.contains_key("lease") {
            return Err("invalid_request");
        }
        let level = preview_level(value)?;
        let start_sampled_at = BootClock.now();
        let (admission_until, start_until, work_until) = preview_deadlines(
            start_sampled_at,
            value["until_boottime_ns"]
                .as_u64()
                .ok_or("invalid_deadline")?,
            value["startup_until_boottime_ns"]
                .as_u64()
                .ok_or("invalid_deadline")?,
        )?;
        let route = value["route"].as_str().ok_or("invalid_route")?;
        if !self.profile.supports(Role::Media, route) {
            return Err("unsupported_route");
        }
        if self.preview.is_some() {
            return Err("preview_busy");
        }
        if self.recovery_required || !self.observed.available || self.cookie.is_none() {
            return Err("backend_unavailable");
        }
        if self.media_present {
            return Err("external_media_busy");
        }
        if self
            .authority
            .desired()
            .iter()
            .any(|(_, role, _)| *role == Role::Call)
        {
            return Err("call_active");
        }
        let existing = if value["lease"].is_null() {
            None
        } else {
            let id = value["lease"].as_u64().ok_or("invalid_lease")?;
            if self.authority.role(peer, id)? != Role::Media {
                return Err("invalid_role");
            }
            Some(id)
        };
        let source = self.profile.source(Role::Media)?.to_owned();
        if existing.is_none() {
            self.authority.can_acquire(peer, Role::Media)?;
        }
        let generation = self
            .preview_sequence
            .checked_add(1)
            .ok_or("preview_generation_limit")?;
        self.preview_sequence = generation;
        let process = pokelinux_audio::preview_process::Process::start(&source, level, work_until)
            .map_err(|_| "preview_start_failed")?;
        self.preview = Some(Preview {
            peer,
            generation,
            lease: existing.unwrap_or(0),
            route: route.into(),
            process,
            start_until,
            work_until,
            state: PreviewState::Starting,
            terminal: PreviewState::Failed,
            natural_drain: false,
            identity: None,
        });
        self.last_preview = None;
        let admission_ready = self.preview.as_ref().unwrap().process.admission_ready();
        if !admission_ready {
            eprintln!(
                "preview startup: predicate=admission_not_ready start_sampled_at_ns={} admission_until_ns={} startup_until_ns={} work_until_ns={} admission_ready=false",
                start_sampled_at.elapsed().as_nanos(),
                admission_until.elapsed().as_nanos(),
                start_until.elapsed().as_nanos(),
                work_until.elapsed().as_nanos()
            );
            self.retire_preview(PreviewState::Failed);
            return Err("preview_start_failed");
        }
        let start_checked_at = BootClock.now();
        if start_checked_at >= admission_until {
            eprintln!(
                "preview startup: predicate=admission_cutoff start_sampled_at_ns={} start_checked_at_ns={} admission_until_ns={} startup_until_ns={} work_until_ns={} admission_ready=true",
                start_sampled_at.elapsed().as_nanos(),
                start_checked_at.elapsed().as_nanos(),
                admission_until.elapsed().as_nanos(),
                start_until.elapsed().as_nanos(),
                work_until.elapsed().as_nanos()
            );
            self.retire_preview(PreviewState::Failed);
            return Err("preview_expired");
        }
        let selected = match existing {
            Some(id) => self.authority.set_route(peer, id, route).map(|()| id),
            None => self.authority.acquire(peer, Role::Media, route),
        };
        let lease = match selected {
            Ok(id) => id,
            Err(error) => {
                self.retire_preview(PreviewState::Failed);
                return Err(error);
            }
        };
        self.preview.as_mut().unwrap().lease = lease;
        self.change();
        eprintln!(
            "preview startup: predicate=admitted start_sampled_at_ns={} start_checked_at_ns={} admission_until_ns={} startup_until_ns={} work_until_ns={} admission_ready=true",
            start_sampled_at.elapsed().as_nanos(),
            start_checked_at.elapsed().as_nanos(),
            admission_until.elapsed().as_nanos(),
            start_until.elapsed().as_nanos(),
            work_until.elapsed().as_nanos()
        );
        Ok(lease)
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
                    self.stop();
                }
                "preview_play" => {
                    lease = Some(self.start_preview(peer, &value)?);
                }
                "preview_mute" => self.mute_preview(peer, &value)?,
                "preview_stop" => {
                    if value.as_object().is_none_or(|object| object.len() != 2) {
                        return Err("invalid_request");
                    }
                    let id = value["lease"].as_u64().ok_or("invalid_lease")?;
                    if self.authority.role(peer, id)? != Role::Media {
                        return Err("invalid_role");
                    }
                    if self
                        .preview
                        .as_ref()
                        .is_some_and(|p| p.peer != peer || p.lease != id)
                    {
                        return Err("preview_busy");
                    }
                    self.retire_preview(PreviewState::Idle);
                    self.change();
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
                    if role == Role::Call {
                        self.retire_preview(PreviewState::Failed);
                    }
                    self.change();
                }
                "release" => {
                    let id = value["lease"].as_u64().ok_or("invalid_lease")?;
                    let capture = self.authority.role(peer, id)? == Role::Capture;
                    self.authority.release(peer, id)?;
                    if self
                        .preview
                        .as_ref()
                        .is_some_and(|p| p.peer == peer && p.lease == id)
                    {
                        self.retire_preview(PreviewState::Idle);
                    }
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
                    if self
                        .preview
                        .as_ref()
                        .is_some_and(|p| p.peer == peer && p.lease == id)
                    {
                        self.retire_preview(PreviewState::Idle);
                    }
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
        self.enforce_preview_route();
        let mut response = json!({"ok":true,"lease":lease,"peer_uid":peer.uid,"desired":self.authority.desired(),
            "observed":self.observed,"intent_revision":self.intent,
            "policy_settled":self.policy_settled,"preview":self.preview_for(peer),
            "preview_generation":self.current_preview_for(peer).map(|p| p.generation),
            "preview_muted":self.current_preview_for(peer).map(|p| p.process.muted()),
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
        if args.len() < 4
            || !matches!(
                args[3].as_str(),
                "/usr/bin/pw-link" | "/usr/bin/pw-dump" | "/usr/bin/pw-cat"
            )
        {
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
    // Own SIGTERM before starting workers and retain it until all Stop owners drain.
    // Worker entry points above keep their independent default signal behavior.
    let _termination = pokelinux_kernel_identity::TerminationGuard::install()?;
    let mut state = State {
        authority: Authority::new(call_uid, app_uid, false),
        profile,
        profile_path: args[2].clone(),
        process: None,
        intent: 0,
        token: 0,
        sent: None,
        applied: None,
        policy_settled: false,
        media_present: false,
        preview: None,
        preview_sequence: 0,
        last_preview: None,
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
        if pokelinux_kernel_identity::termination_requested() {
            state.stop();
        }
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
            if state.process.is_none() && state.preview.is_none() && workers.is_empty() {
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

#[cfg(test)]
#[path = "preview_main_tests.rs"]
mod preview_main_tests;
