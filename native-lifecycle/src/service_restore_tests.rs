//! Isolated root integration only. Auth/Source/history are authored test inputs;
//! the service loop, app credentials/read publication and policy Worker are real.
use crate::{
    authority::{AuthenticatedCompletion, AuthenticationAttempt, Authority},
    boottime_ns, ipc, messages,
    session::{NativeFocus, OwnedFocus, ProtocolUnlock, SAMPLE_BUDGET_NS, Snapshot},
};
use rustix::{
    fd::OwnedFd,
    process::{Pid, PidfdFlags, Signal},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const APP: &str = messages::APP;
const WAIT_NS: u64 = 2_000_000_000;
const HISTORY: &[u8] = b"{\"history\":[\"restoration-fixture\"]}";

fn check(value: bool, text: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(text))
    }
}

enum ControlCommand {
    Begin {
        until: u64,
        reply: SyncSender<bool>,
    },
    Complete {
        reply: SyncSender<bool>,
    },
    SourceBarrier {
        expected_revision: u64,
        until: u64,
        reply: SyncSender<bool>,
    },
}

pub(super) struct ActorControl {
    inbox: Receiver<ControlCommand>,
    attempt: Option<(AuthenticationAttempt, u64)>,
    observed_revision: Arc<AtomicU64>,
    source_barrier: Option<(u64, u64, SyncSender<bool>)>,
    pub(super) storage_executable: PathBuf,
}

impl ActorControl {
    pub(super) fn poll(
        &mut self,
        authority: &mut Authority,
        busy: bool,
        stopping: bool,
    ) -> io::Result<()> {
        // Normal refresh_session ran immediately before this cfgtest poll.
        // A pending observation barrier never blocks the actor or mints authority.
        if let Some((expected, until, reply)) = self.source_barrier.take() {
            if !stopping
                && boottime_ns() < until
                && self.observed_revision.load(Ordering::Acquire) < expected
            {
                self.source_barrier = Some((expected, until, reply));
                return Ok(());
            }
            return reply
                .try_send(!stopping && boottime_ns() < until)
                .map_err(|_| io::ErrorKind::BrokenPipe.into());
        }
        let command = match self.inbox.try_recv() {
            Ok(command) => command,
            Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => return Ok(()),
        };
        let (reply, accepted) = match command {
            ControlCommand::SourceBarrier {
                expected_revision,
                until,
                reply,
            } => {
                let observed = self.observed_revision.load(Ordering::Acquire);
                if !stopping && boottime_ns() < until && observed < expected_revision {
                    self.source_barrier = Some((expected_revision, until, reply));
                    return Ok(());
                }
                (
                    reply,
                    !stopping && boottime_ns() < until && observed >= expected_revision,
                )
            }
            ControlCommand::Begin { until, reply } => {
                let accepted = if stopping || busy || self.attempt.is_some() {
                    false
                } else {
                    match authority.begin_authentication_attempt(boottime_ns(), until) {
                        Ok(attempt) => {
                            self.attempt = Some((attempt, until));
                            true
                        }
                        Err(_) => false,
                    }
                };
                (reply, accepted)
            }
            ControlCommand::Complete { reply } => {
                let accepted = self.attempt.take().is_some_and(|(attempt, until)| {
                    !stopping
                        && authority.arm_authentication(
                            AuthenticatedCompletion::fixture(&attempt, until),
                            boottime_ns(),
                        )
                });
                (reply, accepted)
            }
        };
        reply
            .try_send(accepted)
            .map_err(|_| io::ErrorKind::BrokenPipe.into())
    }
}

struct Facts {
    shared: Arc<Mutex<Snapshot>>,
    cached: Snapshot,
    observed_revision: Arc<AtomicU64>,
    stopped: bool,
}
impl crate::session::Provider for Facts {
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        if self.stopped {
            return Ok(None);
        }
        match self.shared.try_lock() {
            Ok(sample) => self.cached = sample.clone(),
            Err(std::sync::TryLockError::WouldBlock) => {}
            Err(_) => return Err(io::Error::other("authored Source mutex poisoned")),
        }
        self.observed_revision
            .store(self.cached.revision, Ordering::Release);
        Ok(Some(self.cached.clone()))
    }
    fn stop(&mut self) {
        self.stopped = true;
    }
    fn occupied(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct History {
    pending: Option<(u64, u64)>,
    sequence: u64,
    stopped: bool,
}
impl messages::Provider for History {
    fn incarnation(&self) -> &str {
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
    fn occupied(&self) -> bool {
        false
    }
    fn read_occupied(&self) -> bool {
        false
    }
    fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn available(&self) -> bool {
        !self.stopped
    }
    fn start(&mut self, query: Value, until: u64) -> io::Result<u64> {
        check(
            !self.stopped
                && self.pending.is_none()
                && boottime_ns() < until
                && query == json!({"fixture":"history"}),
            "invalid authored history request",
        )?;
        self.sequence = self.sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
        self.pending = Some((self.sequence, until));
        Ok(self.sequence)
    }
    fn poll(&mut self) -> io::Result<Option<messages::Event>> {
        Ok(self.pending.take().map(|(token, until)| {
            if boottime_ns() < until {
                messages::Event::Complete {
                    token,
                    packet: HISTORY.to_vec(),
                }
            } else {
                messages::Event::Failed { token }
            }
        }))
    }
    fn cancel_read(&mut self, token: u64) {
        if self.pending.is_some_and(|(current, _)| current == token) {
            self.pending = None;
        }
    }
    fn stop(&mut self) {
        self.stopped = true;
        self.pending = None;
    }
    fn restart(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn drain_until(&mut self, _: u64) -> bool {
        self.stop();
        true
    }
}

fn wait_until(until: u64, mut ready: impl FnMut() -> io::Result<bool>) -> io::Result<()> {
    loop {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if ready()? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn rpc_fd(fd: &OwnedFd, request: &Value, uid: u32) -> io::Result<Value> {
    let peer = rustix::net::sockopt::socket_peercred(fd)?;
    check(peer.uid.as_raw() == uid, "wrong actual socket peer uid")?;
    let until = boottime_ns().saturating_add(WAIT_NS);
    let bytes = serde_json::to_vec(request)?;
    wait_until(until, || match ipc::send(fd, &bytes) {
        Ok(()) => Ok(true),
        Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => Ok(false),
        Err(e) => Err(e),
    })?;
    let mut reply = None;
    wait_until(until, || match ipc::receive(fd) {
        Ok(Some((bytes, credentials))) => {
            check(
                credentials.uid == peer.uid
                    && credentials.gid == peer.gid
                    && credentials.pid == peer.pid,
                "wrong per-packet credentials",
            )?;
            reply = Some(serde_json::from_slice(&bytes)?);
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(e) if e.raw_os_error() == Some(4) => Ok(false),
        Err(e) => Err(e),
    })?;
    Ok(reply.unwrap())
}
fn rpc(path: &Path, request: Value, uid: u32) -> io::Result<Value> {
    rpc_fd(&ipc::connect(path)?, &request, uid)
}

fn child_wait(child: &mut Child, until: u64) -> io::Result<ExitStatus> {
    let mut result = None;
    wait_until(until, || {
        result = child.try_wait()?;
        Ok(result.is_some())
    })?;
    Ok(result.unwrap())
}
struct Locker(Child);
impl Drop for Locker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            if child_wait(&mut self.0, boottime_ns().saturating_add(WAIT_NS)).is_err() {
                std::process::abort();
            }
        }
    }
}

struct Case {
    runtime: PathBuf,
    policy: PathBuf,
    install: PathBuf,
    python: PathBuf,
    facts: Arc<Mutex<Snapshot>>,
    heartbeat_stop: Arc<AtomicBool>,
    heartbeat: Option<JoinHandle<()>>,
    heartbeat_done: Receiver<()>,
    service: Option<JoinHandle<()>>,
    service_done: Receiver<io::Result<()>>,
    commands: SyncSender<ControlCommand>,
    locker: Locker,
    app: Option<OwnedFocus>,
    worker: Option<(u32, OwnedFd)>,
    observations: Vec<Value>,
    closed: bool,
}

impl Case {
    fn new(existing_policy: Option<PathBuf>, pause: bool) -> io::Result<Self> {
        check(
            rustix::process::getuid().as_raw() == 0 && rustix::process::geteuid().as_raw() == 0,
            "isolated root fixture required",
        )?;
        check(
            std::env::var("NATIVE_LIFECYCLE_ISOLATED").as_deref() == Ok("1"),
            "isolated fixture environment required",
        )?;
        let install = PathBuf::from(
            std::env::var_os("NATIVE_LIFECYCLE_RESTORE_INSTALL").ok_or(io::ErrorKind::NotFound)?,
        );
        let python = PathBuf::from(fs::read_to_string(install.join("python-path"))?.trim());
        for name in ["app.py", "python-path"] {
            let metadata = fs::symlink_metadata(install.join(name))?;
            check(
                metadata.is_file() && metadata.uid() == 0 && metadata.mode() & 0o022 == 0,
                "authored helper inputs must be root-installed immutable files",
            )?;
        }
        for executable in [
            &python,
            &install.join("lifecycle-daemon"),
            &install.join("lifecycle-launcher"),
        ] {
            super::trusted_executable(executable)?;
        }
        let id = crate::fresh_id()?;
        let runtime = PathBuf::from("/run").join(format!("restore-{}", &id[..10]));
        fs::create_dir(&runtime)?;
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755))?;
        let policy = existing_policy
            .unwrap_or_else(|| PathBuf::from("/var/lib").join(format!("restore-{}", &id[..10])));
        if !policy.exists() {
            fs::create_dir(&policy)?;
            fs::set_permissions(&policy, fs::Permissions::from_mode(0o700))?;
            fs::write(
                policy.join("policy.json"),
                serde_json::to_vec(&json!({
                    "grants":[{"app":APP,"permission":"messages_read","allowed":true}],
                    "receipts":[format!("{}-0000000000000000", "a".repeat(32))],"lock_pending":true
                }))?,
            )?;
            fs::set_permissions(
                policy.join("policy.json"),
                fs::Permissions::from_mode(0o600),
            )?;
        }
        let driver_dir = runtime.join("app");
        fs::create_dir(&driver_dir)?;
        fs::set_permissions(&driver_dir, fs::Permissions::from_mode(0o700))?;
        std::os::unix::fs::chown(&driver_dir, Some(11010), Some(11010))?;
        let profile = install.join(format!("profile-{}.json", &id[..10]));
        fs::write(
            &profile,
            serde_json::to_vec(&json!({"apps":[{"app":APP,"uid":11010,
            "executable":python,"args":[install.join("app.py"),"--app",runtime.join("app.sock"),driver_dir.join("driver.sock")]}]}))?,
        )?;
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o600))?;
        let locker = Locker(
            Command::new(&python)
                .arg("-I")
                .arg(install.join("app.py"))
                .arg("--locker")
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        wait_until(boottime_ns().saturating_add(WAIT_NS), || {
            let status = fs::read_to_string(format!("/proc/{}/status", locker.0.id()))?;
            Ok(status.lines().any(|line| {
                line.starts_with("Uid:") && line.split_whitespace().skip(1).all(|uid| uid == "1000")
            }))
        })?;
        let sample_at = boottime_ns();
        let sample = Snapshot {
            source: crate::fresh_id()?,
            revision: 1,
            sequence: 1,
            sampled_at_ns: sample_at,
            until: sample_at + SAMPLE_BUDGET_NS,
            locked: true,
            focus: None,
            locker: Some(OwnedFocus::new(
                NativeFocus {
                    pid: locker.0.id(),
                    uid: 1000,
                    gid: 1000,
                },
                rustix::process::pidfd_open(
                    Pid::from_raw(locker.0.id() as i32).unwrap(),
                    PidfdFlags::NONBLOCK,
                )?,
            )),
            lock_generation: Some(1),
            protocol_unlock: None,
        };
        let facts = Arc::new(Mutex::new(sample.clone()));
        let heartbeat_stop = Arc::new(AtomicBool::new(false));
        let (beat_tx, heartbeat_done) = mpsc::sync_channel(1);
        let beat_facts = facts.clone();
        let beat_stop = heartbeat_stop.clone();
        let heartbeat = thread::spawn(move || {
            while !beat_stop.load(Ordering::Acquire) {
                if let Ok(mut sample) = beat_facts.lock() {
                    sample.sequence += 1;
                    sample.sampled_at_ns = boottime_ns();
                    sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
                } else {
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
            let _ = beat_tx.try_send(());
        });
        let (commands, inbox) = mpsc::sync_channel(1);
        let (service_tx, service_done) = mpsc::sync_channel(1);
        let service_runtime = runtime.clone();
        let service_policy = policy.clone();
        let service_install = install.clone();
        let service_facts = facts.clone();
        let service = thread::spawn(move || {
            let observed_revision = Arc::new(AtomicU64::new(0));
            let mut source = Facts {
                shared: service_facts,
                cached: sample,
                observed_revision: observed_revision.clone(),
                stopped: false,
            };
            let mut history = History::default();
            let mut control = ActorControl {
                inbox,
                attempt: None,
                observed_revision,
                source_barrier: None,
                storage_executable: service_install.join("lifecycle-daemon"),
            };
            let result = super::run_with_services_inner(
                &service_runtime,
                &service_policy,
                &profile,
                &service_install.join("lifecycle-launcher"),
                if pause { Some("pause:1") } else { None },
                Some(&mut history),
                Some(&mut source),
                None,
                None,
                Some(&mut control),
            );
            let _ = service_tx.try_send(result);
        });
        let mut case = Self {
            runtime,
            policy,
            install,
            python,
            facts,
            heartbeat_stop,
            heartbeat: Some(heartbeat),
            heartbeat_done,
            service: Some(service),
            service_done,
            commands,
            locker,
            app: None,
            worker: None,
            observations: vec![],
            closed: false,
        };
        let setup = (|| {
            wait_until(boottime_ns().saturating_add(WAIT_NS), || {
                match case.admin(json!({"op":"snapshot"})) {
                    Ok(value) => Ok(value["ready"] == true),
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                        ) =>
                    {
                        Ok(false)
                    }
                    Err(e) => Err(e),
                }
            })?;
            case.find_worker()?;
            check(
                case.admin(json!({"op":"launch","app":APP}))? == json!({"result":"ok"}),
                "actual app launch failed",
            )?;
            let path = case.runtime.join("app/driver.sock");
            wait_until(boottime_ns().saturating_add(WAIT_NS), || Ok(path.exists()))?;
            let fd = ipc::connect(&path)?;
            let peer = rustix::net::sockopt::socket_peercred(&fd)?;
            let identity = rpc_fd(&fd, &json!({"op":"identity"}), 11010)?;
            check(
                identity["uid"] == 11010
                    && identity["gid"] == 11010
                    && identity["groups"] == json!([])
                    && identity["cap_eff"] == "0000000000000000"
                    && identity["pid"] == peer.pid.as_raw_nonzero().get(),
                "actual app identity mismatch",
            )?;
            case.app = Some(OwnedFocus::new(
                NativeFocus {
                    pid: peer.pid.as_raw_nonzero().get() as u32,
                    uid: 11010,
                    gid: 11010,
                },
                pokelinux_kernel_identity::socket_peer_pidfd(&fd)?,
            ));
            case.observations
                .push(json!({"actual_app_identity":identity}));
            Ok(())
        })();
        if let Err(error) = setup {
            let _ = case.close();
            return Err(error);
        }
        Ok(case)
    }

    fn admin(&self, request: Value) -> io::Result<Value> {
        rpc(&self.runtime.join("admin.sock"), request, 0)
    }
    fn read(&mut self) -> io::Result<Value> {
        let reply = rpc(
            &self.runtime.join("app/driver.sock"),
            json!({"op":"read"}),
            11010,
        )?;
        self.observations.push(json!({"actual_app_read":reply}));
        Ok(reply)
    }
    fn policy(&self) -> io::Result<Value> {
        Ok(serde_json::from_slice(&fs::read(
            self.policy.join("policy.json"),
        )?)?)
    }
    fn arm(&mut self) -> io::Result<()> {
        for complete in [false, true] {
            let (reply, receive) = mpsc::sync_channel(1);
            self.commands
                .try_send(if complete {
                    ControlCommand::Complete { reply }
                } else {
                    ControlCommand::Begin {
                        until: boottime_ns() + 20_000_000_000,
                        reply,
                    }
                })
                .map_err(|_| io::Error::other("bounded test mailbox unavailable"))?;
            check(
                receive
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(io::Error::other)?,
                "synthetic begin/completion refused",
            )?;
        }
        self.observations
            .push(json!({"authored_completion_armed":true,"disk":self.policy()?}));
        check(
            self.policy()?["lock_pending"] == true,
            "arming changed disk before later Source event",
        )?;
        check(
            self.read()? == json!({"result":"denied"}),
            "arming released actual read without later Source event",
        )
    }
    fn unlock(&self) -> io::Result<()> {
        let mut sample = self
            .facts
            .lock()
            .map_err(|_| io::Error::other("Source mutex poisoned"))?;
        sample.revision += 1;
        sample.sequence += 1;
        sample.sampled_at_ns = boottime_ns();
        sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
        sample.locked = false;
        sample.locker = None;
        sample.lock_generation = None;
        sample.focus = self.app.clone();
        sample.protocol_unlock = Some(ProtocolUnlock {
            generation: 1,
            sequence: sample.sequence,
            observed_at_ns: sample.sampled_at_ns,
        });
        Ok(())
    }
    fn find_worker(&mut self) -> io::Result<()> {
        let expected = self.runtime.join("storage.sock");
        let mut children = std::collections::BTreeSet::new();
        let mut found = None;
        for task in fs::read_dir("/proc/self/task")?.flatten() {
            if let Ok(text) = fs::read_to_string(task.path().join("children")) {
                children.extend(
                    text.split_whitespace()
                        .filter_map(|pid| pid.parse::<u32>().ok()),
                );
            }
        }
        for pid in children {
            if let Ok(bytes) = fs::read(format!("/proc/{pid}/cmdline")) {
                let arguments: Vec<_> = bytes.split(|b| *b == 0).collect();
                if arguments.contains(&b"--storage-worker".as_slice())
                    && arguments.contains(&expected.as_os_str().as_encoded_bytes())
                {
                    check(found.is_none(), "more than one exact storage child")?;
                    found = Some((
                        pid,
                        rustix::process::pidfd_open(
                            Pid::from_raw(pid as i32).unwrap(),
                            PidfdFlags::NONBLOCK,
                        )?,
                    ));
                }
            }
        }
        let (pid, alive) =
            found.ok_or_else(|| io::Error::other("actual owned storage child not found"))?;
        if let Some((original_pid, original)) = &self.worker {
            check(
                *original_pid == pid
                    && pokelinux_kernel_identity::ProcessView::open()?
                        .matches(original, &alive, pid)?,
                "repair replaced actual retained storage owner",
            )
        } else {
            self.worker = Some((pid, alive));
            Ok(())
        }
    }
    fn new_lock(&mut self) -> io::Result<()> {
        let mut child = Command::new(&self.python)
            .arg("-I")
            .arg(self.install.join("app.py"))
            .arg("--lock")
            .arg(self.runtime.join("lock.sock"))
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let status = match child_wait(&mut child, boottime_ns() + WAIT_NS) {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                if child_wait(&mut child, boottime_ns() + WAIT_NS).is_err() {
                    std::process::abort();
                }
                return Err(error);
            }
        };
        let mut output = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .take(4097)
            .read_to_string(&mut output)?;
        check(
            status.success()
                && output.len() <= 4096
                && !Path::new(&format!("/proc/{}", child.id())).exists(),
            "actual UID1000 lock helper failed",
        )?;
        let value: Value = serde_json::from_str(&output)?;
        check(
            value["identity"]["uid"] == 1000
                && value["identity"]["gid"] == 1000
                && value["identity"]["groups"] == json!([])
                && value["identity"]["cap_eff"] == "0000000000000000"
                && value["reply"] == json!({"result":"unavailable"}),
            "new lock must refuse busy durability ACK",
        )?;
        self.observations.push(json!({"actual_uid1000_lock":value,"actual_parent_wait":status.code(),"stdout_eof":true,"proc_absent":true}));
        Ok(())
    }
    fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        let stopped = self.admin(json!({"op":"stop"}));
        let service_result = self.service_done.recv_timeout(Duration::from_secs(3));
        if service_result.is_err() {
            std::process::abort();
        }
        if self.service.take().unwrap().join().is_err() {
            std::process::abort();
        }
        self.heartbeat_stop.store(true, Ordering::Release);
        if self
            .heartbeat_done
            .recv_timeout(Duration::from_secs(1))
            .is_err()
        {
            std::process::abort();
        }
        if self.heartbeat.take().unwrap().join().is_err() {
            std::process::abort();
        }
        if let Some(mut input) = self.locker.0.stdin.take() {
            input.write_all(b"stop\n")?;
        }
        let locker_status = child_wait(&mut self.locker.0, boottime_ns() + WAIT_NS)?;
        self.closed = true;
        check(
            stopped? == json!({"result":"ok"}),
            "normal Stop did not acknowledge",
        )?;
        service_result.unwrap()?;
        check(
            locker_status.success(),
            "owned ordinary locker fixture did not exit0",
        )?;
        if let Some((pid, alive)) = &self.worker {
            check(
                !crate::authority::living(alive)? && !Path::new(&format!("/proc/{pid}")).exists(),
                "storage child death/proc absence missing after service-owned drain",
            )?;
        }
        if let Some(app) = &self.app {
            check(
                !crate::authority::living(app.pidfd())?
                    && !Path::new(&format!("/proc/{}", app.metadata.pid)).exists(),
                "app child closure missing after service-owned drain",
            )?;
        }
        self.observations.push(json!({"normal_service_result":true,"service_thread_joined":true,
            "storage_death_and_proc_absence":self.worker.is_some(),"storage_wait_owned_by_service":true,
            "locker_actual_parent_wait":locker_status.code(),"locker_proc_absent":!Path::new(&format!("/proc/{}",self.locker.0.id())).exists()}));
        Ok(())
    }
    fn report(&self, label: &str, body: &io::Result<()>, closed: &io::Result<()>) {
        let record = json!({"case":label,"passed":body.is_ok()&&closed.is_ok(),"body_error":body.as_ref().err().map(ToString::to_string),
            "closure_error":closed.as_ref().err().map(ToString::to_string),"observations":self.observations,
            "actual_policy_worker":true,"auth_and_Source_and_history_authored":true,"production_auth_ingress":false,"phone_accessed":false});
        if let Some(root) = std::env::var_os("NATIVE_LIFECYCLE_RESTORE_RESULTS") {
            let path = PathBuf::from(root).join(format!("{label}.json"));
            if fs::write(path, serde_json::to_vec_pretty(&record).unwrap()).is_err() {
                std::process::abort();
            }
        }
        eprintln!("RESTORE_SERVICE_BOUNDARY {record}");
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.closed && self.close().is_err() {
            std::process::abort();
        }
    }
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn authenticated_completion_then_later_source_restores_actual_service_read() {
    let mut case = Case::new(None, false).expect("ordinary restoration fixture setup");
    let body = (|| {
        check(
            case.read()? == json!({"result":"denied"}),
            "initial actual read must deny",
        )?;
        let before = case.policy()?;
        case.arm()?;
        case.unlock()?;
        wait_until(boottime_ns()+900_000_000,|| {
            let snapshot=case.admin(json!({"op":"snapshot"}))?;
            Ok(case.policy()?["lock_pending"]==false && snapshot["ready"]==true && snapshot["locked"]==false && snapshot["storage_busy"]==false)
        }).map_err(|_|io::Error::other("behavioral RED: armed exact later Source event did not restore through actual Worker"))?;
        let after = case.policy()?;
        check(
            after["grants"] == before["grants"] && after["receipts"] == before["receipts"],
            "clear changed retained policy/history",
        )?;
        check(
            case.read()? == serde_json::from_slice::<Value>(HISTORY)?,
            "actual UID11010 read was not restored after synced false",
        )
    })();
    let closed = case.close();
    case.report("completion-source-read", &body, &closed);
    assert!(closed.is_ok(), "actual fixture closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn new_lock_during_known_false_clear_restores_true_and_clean_restart_denies() {
    let mut case = Case::new(None, true).expect("ordinary restoration fixture setup");
    let policy = case.policy.clone();
    let body = (|| {
        let before = case.policy()?;
        case.arm()?;
        case.unlock()?;
        let pid = case.worker.as_ref().unwrap().0;
        wait_until(boottime_ns() + 900_000_000, || {
            let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
            Ok(stat
                .rsplit_once(") ")
                .and_then(|(_, tail)| tail.split_whitespace().next())
                .is_some_and(|s| s == "T" || s == "t")
                && case.policy()?["lock_pending"] == false)
        })
        .map_err(|_| {
            io::Error::other(
                "behavioral RED: actual false-clear worker never reached existing post-sync pause",
            )
        })?;
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        check(
            snapshot["storage_busy"] == true && snapshot["ready"] == true,
            "held false clear did not preserve responsive Snapshot",
        )?;
        check(
            case.read()? != serde_json::from_slice::<Value>(HISTORY)?,
            "read escaped while false completion held",
        )?;
        case.new_lock()?;
        rustix::process::pidfd_send_signal(&case.worker.as_ref().unwrap().1, Signal::CONT)?;
        wait_until(boottime_ns() + 900_000_000, || {
            let snapshot = case.admin(json!({"op":"snapshot"}))?;
            Ok(case.policy()?["lock_pending"] == true
                && snapshot["ready"] == true
                && snapshot["locked"] == true
                && snapshot["storage_busy"] == false)
        })
        .map_err(|_| {
            io::Error::other("known old false completion did not produce required true write")
        })?;
        let after = case.policy()?;
        check(
            after["grants"] == before["grants"] && after["receipts"] == before["receipts"],
            "repair changed retained policy/history",
        )?;
        check(
            case.read()? == json!({"result":"denied"}),
            "new lock did not retain actual read denial",
        )?;
        let original_pid = pid;
        case.find_worker()?;
        check(
            case.worker.as_ref().unwrap().0 == original_pid,
            "repair replaced actual storage owner",
        )
    })();
    let closed = case.close();
    case.report("new-lock-clear-repair", &body, &closed);
    assert!(closed.is_ok(), "actual fixture closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
    check(
        policy.join("owner.clean").is_file(),
        "normal Stop did not leave clean owner seal",
    )
    .unwrap();
    let mut restart = Case::new(Some(policy), false).expect("same-policy clean restart");
    let restored = (|| {
        restart.unlock()?;
        check(
            restart.policy()?["lock_pending"] == true,
            "restart lost durable new lock",
        )?;
        check(
            restart.read()? == json!({"result":"denied"}),
            "fresh synthetic unlocked Source cleared restored pending lock",
        )
    })();
    let closed = restart.close();
    restart.report("clean-restart-denial", &restored, &closed);
    assert!(
        closed.is_ok(),
        "actual restarted fixture closure: {closed:?}"
    );
    assert!(restored.is_ok(), "{restored:?}");
}

#[path = "service_restore_storage_tests.rs"]
mod storage_negative;

#[path = "service_restore_negative_tests.rs"]
mod negative_tests;
