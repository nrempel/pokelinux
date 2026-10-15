//! Main owns one cooperative Data worker and private operator diagnostics.
use pokelinux_cellular_service::{
    config::Config,
    ipc::{Connection, Listener, Request},
    now_ns, output,
    projection::Projection,
    status::{Phase, Status},
    wire::{self, Frame},
};
use pokelinux_process_owner::{
    bootstrap::{WorkerCommand, WorkerState},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use serde_json::json;
use std::{
    fs::{self, File},
    io,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    time::Duration,
};
const DISPOSAL_NS: u64 = 2_000_000_000;
const TURN: Duration = Duration::from_millis(2);
const ASSOCIATION_FRESH_NS: u64 = 1_000_000_000;
struct Args {
    runtime: PathBuf,
    journal: PathBuf,
    worker: PathBuf,
    config: PathBuf,
    until: u64,
}
impl Args {
    fn read() -> io::Result<Self> {
        let a: Vec<_> = std::env::args().skip(1).collect();
        if a.len() != 10
            || a[0] != "--runtime"
            || a[2] != "--journal"
            || a[4] != "--worker"
            || a[6] != "--config"
            || a[8] != "--until-ns"
            || [1, 3, 5, 7]
                .iter()
                .any(|i| !Path::new(&a[*i]).is_absolute() || a[*i].len() > 256)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let until = a[9].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        let now = now_ns();
        if until <= now || until - now > 86_400_000_000_000 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            runtime: a[1].clone().into(),
            journal: a[3].clone().into(),
            worker: a[5].clone().into(),
            config: a[7].clone().into(),
            until,
        })
    }
}
fn private(path: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir()
        || m.uid() != rustix::process::geteuid().as_raw()
        || m.permissions().mode() & 0o777 != 0o700
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
struct Client {
    connection: Connection,
    until: u64,
    command: Option<Request>,
}
struct Service {
    owner: Supervisor,
    identity: ProcessIdentity,
    worker_pid: u32,
    until: u64,
    serial: i32,
    namespace: (u64, u64),
    state: Status,
    input: Vec<u8>,
    cleanup_until: Option<u64>,
    closing: bool,
    fault: bool,
    ready_announced: bool,
    last_kind: Option<&'static str>,
}
impl Service {
    fn retire(&mut self, reason: &str, closing: bool) {
        self.closing |= closing;
        self.state.phase = if self.closing {
            Phase::Stopping
        } else {
            Phase::Unavailable
        };
        self.state.fault.get_or_insert_with(|| reason.into());
        self.state.observation.withdraw();
        if self.owner.occupied() && self.cleanup_until.is_none() {
            self.cleanup_until = Some(now_ns() + DISPOSAL_NS);
            // Publication revocation occurs inside Stop before the signal.
            if self.owner.stop().is_err() {
                self.fault = true;
            }
        }
        self.input.clear();
    }
    fn reap(&mut self) {
        match self.owner.poll_reap() {
            Ok(Some(status)) => {
                let waited = now_ns();
                if self.cleanup_until.is_some_and(|until| waited >= until) {
                    self.fault = true;
                }
                self.state.observation.withdraw();
                if !self.closing {
                    self.state.phase = Phase::Unavailable;
                }
                self.state
                    .fault
                    .get_or_insert_with(|| "worker-exited".into());
                self.input.clear();
                output::event(
                    json!({"event":"worker-reap","worker_pid":self.worker_pid,"incarnation":self.identity.hex(),
                    "wait_signal":status.signal(),"wait_code":status.code(),"actual_wait":true,"waited_at_ns":waited,
                    "original_cleanup_until_ns":self.cleanup_until,"occupied":self.owner.occupied(),"admitted":self.owner.admits(self.identity)}),
                );
            }
            Ok(None) => {}
            Err(_) => {
                self.fault = true;
                self.retire("reap-error", true);
            }
        }
        if self.owner.occupied() && self.cleanup_until.is_some_and(|until| now_ns() >= until) {
            // No detachment or renewed budget. A deployed manager must reap the cgroup.
            std::process::abort();
        }
    }
    fn accept(&mut self, frame: Frame) -> io::Result<()> {
        let identity =
            ProcessIdentity::from_hex(&frame.incarnation).ok_or(io::ErrorKind::InvalidData)?;
        if !self
            .owner
            .admits_frame(self.identity, identity, frame.attempt)
            || self.cleanup_until.is_some()
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let was_associated = self.state.observation.association.is_some();
        let previous_disposition = self.state.observation.provider_disposition.clone();
        self.state
            .observation
            .accept(&frame, self.serial, self.until, now_ns(), self.namespace)?;
        self.state.phase = Phase::Observing;
        let kind = if frame.association.is_some() {
            "Association"
        } else if was_associated {
            "Withdrawn"
        } else if frame.provider_report.is_some() {
            "ProviderReport"
        } else if matches!(frame.disposition, wire::Disposition::OutcomeUnknown) {
            "Pending"
        } else {
            "ObserverReady"
        };
        // Emit state transitions only; a fresh association is still copied every turn.
        if self.last_kind != Some(kind) || frame.disposition != previous_disposition {
            output::event(
                json!({"event":"worker-frame-admitted","incarnation":self.identity.hex(),"attempt":1,
                "source_revision":frame.revision,"kind":kind,"parent_admitted_at_ns":now_ns(),"payload":frame}),
            );
            self.last_kind = Some(kind);
        }
        if frame.fault.is_some() {
            self.retire("worker-observation-fault", false);
        }
        Ok(())
    }
    fn poll_output(&mut self) -> io::Result<()> {
        if !self.owner.occupied() || self.cleanup_until.is_some() {
            return Ok(());
        }
        if self.owner.poll_worker()? != WorkerState::Ready {
            return Ok(());
        }
        if !self.ready_announced {
            self.ready_announced = true;
            output::event(
                json!({"event":"worker-ready","worker_pid":self.worker_pid,"incarnation":self.identity.hex(),
                "attempt":1,"cooperative_GO_completed":true,"publication_admitted":true}),
            );
        }
        for _ in 0..8 {
            if let Some(end) = self.input.iter().position(|b| *b == b'\n') {
                if end + 1 > wire::MAX_RECORD_BYTES {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let frame = wire::decode_exact(&self.input[..=end])?;
                self.input.drain(..=end);
                self.accept(frame)?;
                if self.cleanup_until.is_some() {
                    break;
                }
            } else {
                if self.input.len() >= wire::MAX_RECORD_BYTES {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let mut bytes = [0u8; 4096];
                match self.owner.read_output(&mut bytes) {
                    Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                    Ok(n) => {
                        if self.input.len() + n > wire::MAX_RECORD_BYTES * 2 {
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                        self.input.extend_from_slice(&bytes[..n]);
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
    fn current(&mut self) {
        self.reap();
        if now_ns() >= self.until && self.cleanup_until.is_none() {
            self.retire("deadline", false);
        }
        if self
            .state
            .observation
            .association
            .as_ref()
            .is_some_and(|a| {
                now_ns().saturating_sub(a.observed_at_boottime_ns) >= ASSOCIATION_FRESH_NS
            })
        {
            // Local freshness withdrawal does not assert a kernel link disappeared.
            self.state.observation.withdraw();
        }
        self.state.sampled_at_ns = now_ns();
    }
}
fn run(args: Args) -> io::Result<i32> {
    private(&args.runtime)?;
    private(args.journal.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    let (config, config_identity) = Config::load_identified(&args.config)?;
    // Retain this namespace identity for every observed association admission.
    let namespace_file = File::open("/proc/self/ns/net")?;
    let m = namespace_file.metadata()?;
    let namespace = (m.dev(), m.ino());
    let listener = Listener::bind(&args.runtime.join("status.sock"))?;
    let mut owner = Supervisor::new(args.journal.clone());
    let mut worker = WorkerCommand::new(&args.worker)?;
    let config_identity_hex: String = config_identity.iter().map(|b| format!("{b:02x}")).collect();
    worker
        .arg("--config")
        .arg(&args.config)
        .arg("--until-ns")
        .arg(args.until.to_string())
        .arg("--config-sha256")
        .arg(config_identity_hex);
    let started = owner.start_worker(&mut worker);
    if let Some(receipt) = owner.admission_receipt() {
        output::event(
            json!({"event":"admission-synced","parent_pid":std::process::id(),"incarnation":receipt.identity.hex(),
            "attempt":1,"file_sync_and_directory_sync_returned":true,
            "sync_completed_before_spawn_at_boottime_ns":receipt.sync_completed_before_spawn_at_boottime_ns,
            "journal_device":receipt.journal_device,"journal_inode":receipt.journal_inode,"journal_uid":receipt.journal_uid,
            "journal_mode":receipt.journal_mode,"event_emitted_at_ns":now_ns()}),
        );
    }
    let identity = match started {
        Ok(id) => id,
        Err(StartError::AdmissionUncertain) if !owner.occupied() => {
            output::event(
                json!({"event":"start-refused","reason":"AdmissionUncertain","journal_retained":true,"child_spawned":false}),
            );
            return Ok(78);
        }
        Err(_) => {
            let until = now_ns() + DISPOSAL_NS;
            if owner.occupied() {
                let _ = owner.stop();
            }
            while owner.occupied() {
                let _ = owner.poll_reap();
                if now_ns() >= until {
                    std::process::abort();
                }
                std::thread::sleep(TURN);
            }
            return Ok(70);
        }
    };
    let worker_pid = owner.child_pid().expect("retained worker");
    output::event(
        json!({"event":"worker-spawned","parent_pid":std::process::id(),"worker_pid":worker_pid,
        "incarnation":identity.hex(),"attempt":1,"original_until_ns":args.until}),
    );
    let mut service = Service {
        owner,
        identity,
        worker_pid,
        until: args.until,
        serial: config.setup.serial,
        namespace,
        state: Status {
            schema: 1,
            server_pid: std::process::id(),
            worker_pid,
            phase: Phase::Initializing,
            until_ns: args.until,
            sampled_at_ns: now_ns(),
            fault: None,
            observation: Projection::new(identity.hex()),
        },
        input: Vec::new(),
        cleanup_until: None,
        closing: false,
        fault: false,
        ready_announced: false,
        last_kind: None,
    };
    let mut clients: Vec<Client> = Vec::new();
    loop {
        service.current();
        if service.poll_output().is_err() {
            service.retire("worker-frame-or-bootstrap", false);
        }
        if !service.closing {
            for _ in 0..2 {
                match listener.accept() {
                    Ok(Some(connection)) if clients.len() < 8 => clients.push(Client {
                        connection,
                        until: now_ns() + 500_000_000,
                        command: None,
                    }),
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                    Err(_) => {
                        service.fault = true;
                        service.retire("operator-listener", true);
                        break;
                    }
                }
            }
        }
        clients.retain_mut(|client| {
            if now_ns() >= client.until {
                return false;
            }
            if client.command.is_none() {
                match client.connection.receive() {
                    Ok(Some(request)) => {
                        if request == Request::Stop {
                            service.retire("stop", true);
                        }
                        client.command = Some(request);
                    }
                    Ok(None) => return true,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => return true,
                    Err(_) => return false,
                }
            }
            // Current revocation/deadline/reap and fresh serialization on EVERY EAGAIN retry.
            service.current();
            let bytes = if client.command == Some(Request::Stop) {
                Ok(b"{\"schema\":1,\"stop_ack\":true}\n".to_vec())
            } else {
                service.state.encode()
            };
            let Ok(bytes) = bytes else {
                service.fault = true;
                service.retire("status-size", false);
                return false;
            };
            match client.connection.send(&bytes) {
                Ok(()) => false,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    true
                }
                Err(_) => false,
            }
        });
        if service.closing
            && !service.owner.occupied()
            && !clients.iter().any(|c| c.command == Some(Request::Stop))
        {
            return Ok(if service.fault { 70 } else { 0 });
        }
        std::thread::sleep(TURN);
    }
}
fn main() {
    if output::configure().is_err() {
        std::process::exit(70);
    }
    let args = Args::read().unwrap_or_else(|_| std::process::exit(64));
    std::process::exit(run(args).unwrap_or(70));
}
