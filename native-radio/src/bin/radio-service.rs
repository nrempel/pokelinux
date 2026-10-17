//! Installed owner for one radio transport process and private operator diagnostics.
//! Historical transport data is never a cellular/IMS state or operation receipt.
use pokelinux_process_owner::{
    bootstrap::WorkerState,
    deadline::{BootClock, Clock},
    supervisor::{ProcessIdentity, Supervisor},
};
use pokelinux_radio::{
    observation::{self, Frame, Kind},
    receiver::ReceiverFailure,
    status::{Command, Connection, Phase, Status},
};
use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
use std::{
    fs, io,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    time::Duration,
};
#[path = "radio-service/operator_control.rs"]
mod operator_control;
const TURN: Duration = Duration::from_millis(2);
const DISPOSAL: Duration = Duration::from_secs(2);
fn now() -> Duration {
    BootClock.now()
}
fn ns(value: Duration) -> u64 {
    value.as_nanos().try_into().expect("bounded Linux uptime")
}
fn event(body: String) {
    // Diagnostics cannot block the owner. Missing/partial diagnostics cannot be
    // used as test evidence; no log write grants lifecycle or retry authority.
    let bytes = format!("{{\"schema\":1,{body}}}\n");
    if bytes.len() <= 1024 {
        let _ = rustix::io::write(std::io::stdout(), bytes.as_bytes());
    }
}
fn private_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
struct Config {
    runtime: PathBuf,
    journal: PathBuf,
    worker: PathBuf,
    device: String,
    service: String,
    slot: i32,
    until: Duration,
}
impl Config {
    fn arguments() -> io::Result<Self> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 7 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let lease: u64 = args[5].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        let slot: i32 = args[6].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        if slot < 0
            || !(1_000..=86_400_000).contains(&lease)
            || args[..4].iter().any(|s| !Path::new(s).is_absolute())
            || args[3].len() > 256
            || args[4].is_empty()
            || args[4].len() > 128
            || !args[4]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            runtime: args[0].clone().into(),
            journal: args[1].clone().into(),
            worker: args[2].clone().into(),
            device: args[3].clone(),
            service: args[4].clone(),
            slot,
            until: now()
                .checked_add(Duration::from_millis(lease))
                .ok_or(io::ErrorKind::InvalidInput)?,
        })
    }
}
struct Client {
    connection: Connection,
    until: Duration,
    command: Option<Command>,
    admit_reply: Option<pokelinux_radio::operator::AdmitReply>,
}
struct Service {
    owner: Supervisor,
    identity: ProcessIdentity,
    worker_pid: u32,
    slot: i32,
    state: Status,
    input: Vec<u8>,
    cleanup_until: Option<Duration>,
    closing: bool,
    fault: bool,
    raw_seen: bool,
    control: operator_control::Control,
}
impl Service {
    fn retire(&mut self, reason: ReceiverFailure, closing: bool) {
        self.control.fence();
        self.closing |= closing;
        self.state.phase = if self.closing {
            Phase::Stopping
        } else {
            Phase::Unavailable
        };
        self.state.failure.get_or_insert(reason);
        if (self.owner.occupied() || self.control.occupied()) && self.cleanup_until.is_none() {
            self.cleanup_until = Some(now() + DISPOSAL);
            let radio = self.owner.stop();
            let writer = self.control.stop();
            if radio.is_err() || writer.is_err() {
                self.fault = true;
            }
        }
        self.input.clear();
    }
    fn reap(&mut self) {
        match self.owner.poll_reap() {
            Ok(Some(status)) => {
                let waited = now();
                let cutoff = self.cleanup_until;
                if cutoff.is_some_and(|until| waited >= until) {
                    self.fault = true;
                }
                event(format!(
                    "\"event\":\"worker-reap\",\"worker_pid\":{},\"incarnation\":\"{}\",\"wait_signal\":{},\"wait_code\":{},\"waited_at_ns\":{},\"original_cleanup_until_ns\":{},\"occupied\":{},\"admitted\":{}",
                    self.worker_pid,
                    self.identity.hex(),
                    status.signal().map_or("null".into(), |v| v.to_string()),
                    status.code().map_or("null".into(), |v| v.to_string()),
                    ns(waited),
                    cutoff.map_or("null".into(), |v| ns(v).to_string()),
                    self.owner.occupied(),
                    self.owner.admits(self.identity)
                ));
                if !self.closing {
                    self.state.phase = Phase::Unavailable;
                }
                self.state
                    .failure
                    .get_or_insert(ReceiverFailure::InternalFailure);
                self.input.clear();
                if self.cleanup_until.is_none() {
                    self.retire(ReceiverFailure::InternalFailure, false);
                }
            }
            Ok(None) => {}
            Err(_) => {
                self.fault = true;
                self.retire(ReceiverFailure::InternalFailure, true);
            }
        }
        self.reap_writer();
        if (self.owner.occupied() || self.control.occupied())
            && self.cleanup_until.is_some_and(|until| now() >= until)
        {
            // No detach and no renewed cleanup budget. The installed manager
            // owns process/cgroup fallback; this is not a native successful wait.
            std::process::abort();
        }
    }
    fn accept_frame(&mut self, frame: Frame) -> io::Result<()> {
        if !self
            .owner
            .admits_frame(self.identity, frame.identity, frame.attempt)
            || frame.revision <= self.state.revision
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if let Kind::RegistrationWire {
            slot_id,
            message_id,
            ..
        } = &frame.kind
            && (*slot_id != self.slot || *message_id != pokelinux_radio::registration::MESSAGE_ID)
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.state.revision = frame.revision;
        match &frame.kind {
            Kind::Active { client_id } if self.state.phase == Phase::Initializing => {
                self.state.phase = Phase::ObservedActive;
                self.control.client_id = Some(*client_id);
                event(format!(
                    "\"event\":\"transport-active\",\"incarnation\":\"{}\",\"attempt\":1,\"client_id\":{},\"revision\":{}",
                    self.identity.hex(),
                    client_id,
                    frame.revision
                ));
            }
            Kind::Raw { .. } if self.state.phase == Phase::ObservedActive => {
                if !self.raw_seen {
                    self.raw_seen = true;
                    event(format!(
                        "\"event\":\"raw-observed\",\"incarnation\":\"{}\",\"attempt\":1,\"revision\":{}",
                        self.identity.hex(),
                        frame.revision
                    ));
                }
                self.state.last_observation = Some(frame);
            }
            Kind::RegistrationWire {
                slot_id,
                message_id,
                ..
            } if self.state.phase == Phase::ObservedActive => {
                event(format!(
                    "\"event\":\"registration-observed\",\"incarnation\":\"{}\",\"attempt\":1,\"revision\":{},\"slot_id\":{},\"message_id\":{}",
                    self.identity.hex(),
                    frame.revision,
                    slot_id,
                    message_id
                ));
                self.state.last_observation = Some(frame);
            }
            Kind::Failed(reason) => self.retire(*reason, false),
            _ => return Err(io::ErrorKind::InvalidData.into()),
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
        for _ in 0..4 {
            if let Some((frame, size)) = observation::decode(&self.input)? {
                self.input.drain(..size);
                self.accept_frame(frame)?;
                if self.cleanup_until.is_some() {
                    break;
                }
            } else {
                let mut bytes = [0; 512];
                match self.owner.read_output(&mut bytes) {
                    Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                    Ok(size) => {
                        if self.input.len() + size > observation::MAX_FRAME_LEN * 2 {
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                        self.input.extend_from_slice(&bytes[..size]);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
}
fn run(config: Config) -> io::Result<i32> {
    operator_control::run(config)
}

fn main() {
    let output = std::io::stdout();
    let stat = fstat(&output).unwrap_or_else(|_| std::process::exit(70));
    if !matches!(
        FileType::from_raw_mode(stat.st_mode),
        FileType::Fifo | FileType::Socket
    ) {
        std::process::exit(70);
    }
    let flags = fcntl_getfl(&output).unwrap_or_else(|_| std::process::exit(70));
    fcntl_setfl(&output, flags | OFlags::NONBLOCK).unwrap_or_else(|_| std::process::exit(70));
    let config = Config::arguments().unwrap_or_else(|_| std::process::exit(64));
    let code = run(config).unwrap_or(70);
    std::process::exit(code);
}
