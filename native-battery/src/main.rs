//! Public readonly observation socket. All power_supply IO is child-owned.
#![forbid(unsafe_code)]
use pokelinux_battery::{BatteryModel, Collection, read_supply};
use pokelinux_kernel_identity::{TerminationGuard, termination_requested};
use pokelinux_phone_state::{BatteryObservation, BatteryView, ChargingStatus};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    pipe::{PipeFlags, pipe_with},
    process::{self, Signal},
    time::{ClockId, clock_gettime},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::Path,
    process::{Child, Command, Stdio},
};

fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    (t.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(t.tv_nsec as u64)
}
fn status(value: ChargingStatus) -> &'static str {
    match value {
        ChargingStatus::Unknown => "unknown",
        ChargingStatus::Charging => "charging",
        ChargingStatus::Discharging => "discharging",
        ChargingStatus::NotCharging => "not_charging",
        ChargingStatus::Full => "full",
    }
}
fn encoded(value: Option<BatteryObservation>) -> Value {
    value.map_or(
        Value::Null,
        |v| json!({"capacity_percent":v.capacity_percent,"status":status(v.status)}),
    )
}
fn decoded(value: &Value) -> Option<BatteryObservation> {
    let capacity = u8::try_from(value.get("capacity_percent")?.as_u64()?).ok()?;
    if capacity > 100 {
        return None;
    }
    let status = match value.get("status")?.as_str()? {
        "unknown" => ChargingStatus::Unknown,
        "charging" => ChargingStatus::Charging,
        "discharging" => ChargingStatus::Discharging,
        "not_charging" => ChargingStatus::NotCharging,
        "full" => ChargingStatus::Full,
        _ => return None,
    };
    Some(BatteryObservation {
        capacity_percent: capacity,
        status,
    })
}
fn snapshot(view: BatteryView) -> Vec<u8> {
    let value = view.observation;
    let mut bytes = serde_json::to_vec(&json!({"schema":1,"revision":view.revision,"until_ns":view.until_ns.to_string(),"available":value.is_some(),"capacity_percent":value.map(|v|v.capacity_percent),"status":value.map(|v|status(v.status))})).expect("finite battery snapshot");
    bytes.push(b'\n');
    bytes
}
struct Reader {
    child: Child,
    output: fs::File,
    collection: Collection,
    bytes: Vec<u8>,
    reaped: bool,
    success: bool,
    eof: bool,
    retired: bool,
}
impl Reader {
    fn start(collection: Collection, root: &str, supply: &str) -> io::Result<Self> {
        // Atomic nonblocking/CLOEXEC pipe creation precedes spawn. There is no
        // post-spawn fcntl failure that could put a blocking read in Main.
        let (output, writer) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK)?;
        let child = Command::new(std::env::current_exe()?)
            .args([
                "--read-supply",
                &std::process::id().to_string(),
                &collection.until_ns.to_string(),
                root,
                supply,
            ])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            output: fs::File::from(output),
            collection,
            bytes: Vec::new(),
            reaped: false,
            success: false,
            eof: false,
            retired: false,
        })
    }

    fn fence(&mut self) {
        self.retired = true;
        if !self.reaped {
            let _ = self.child.kill();
        }
    }
    fn poll(&mut self) -> io::Result<bool> {
        if !self.reaped
            && let Some(code) = self.child.try_wait()?
        {
            self.reaped = true;
            self.success = code.success();
        }
        // One bounded nonblocking read each turn, retaining EOF independently
        // from wait. A completion cannot be published before both are observed.
        if !self.eof {
            let mut buffer = [0; 513];
            match self.output.read(&mut buffer) {
                Ok(0) => self.eof = true,
                Ok(n) if self.bytes.len() + n <= 512 => self.bytes.extend_from_slice(&buffer[..n]),
                Ok(_) => self.fence(),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => {
                    self.eof = true;
                    self.fence();
                }
            }
        }
        Ok(self.reaped && self.eof)
    }
    fn value(&self) -> Option<BatteryObservation> {
        if self.retired || !self.success || self.bytes.last() != Some(&b'\n') {
            return None;
        }
        decoded(&serde_json::from_slice::<Value>(&self.bytes).ok()?)
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            std::process::abort();
        }
    }
}
fn worker(parent: &str, until: &str, root: &str, supply: &str) -> io::Result<()> {
    let parent = parent
        .parse::<i32>()
        .ok()
        .and_then(process::Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let until: u64 = until.parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    if process::getppid() != Some(parent) || now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let value = read_supply(Path::new(root), supply).ok();
    if process::getppid() != Some(parent) || now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let mut bytes = serde_json::to_vec(&encoded(value))?;
    bytes.push(b'\n');
    std::io::stdout().lock().write_all(&bytes)
}
fn serve(socket: &str, root: &str, supply: &str) -> io::Result<()> {
    if process::getuid().as_raw() != 0
        || process::geteuid().as_raw() != 0
        || process::getgid().as_raw() != 0
        || process::getegid().as_raw() != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let _termination = TerminationGuard::install()?;
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    let mut model = BatteryModel::default();
    let mut reader: Option<Reader> = None;
    let mut next = 0;
    let mut stopping = false;
    let mut fatal = None;
    loop {
        let sample = now();
        if termination_requested() && !stopping {
            stopping = true;
            model.withdraw();
            if let Some(r) = &mut reader {
                r.fence();
            }
        }
        model.view(sample);
        if let Some(r) = &mut reader {
            if sample >= r.collection.until_ns && !r.retired {
                model.withdraw();
                r.fence();
            }
            match r.poll() {
                Ok(true) => {
                    model.complete(r.collection, now(), r.value());
                    reader.take();
                }
                Ok(false) => {}
                Err(e) => {
                    model.withdraw();
                    r.fence();
                    stopping = true;
                    fatal = Some(e);
                }
            }
        }
        if stopping && reader.is_none() {
            break;
        }
        if !stopping
            && reader.is_none()
            && sample >= next
            && let Some(collection) = model.begin(sample)
        {
            next = sample.saturating_add(1_000_000_000);
            match Reader::start(collection, root, supply) {
                Ok(r) => reader = Some(r),
                Err(_) => {
                    model.complete(collection, now(), None);
                }
            }
        }
        // Snapshot-only connect→reply protocol: no request, refresh, path or
        // mutation verb. Bounded work prevents clients starving expiry/Stop.
        for _ in 0..16 {
            match listener.accept() {
                Ok((mut peer, _)) => {
                    if peer.set_nonblocking(true).is_ok() {
                        let _ = peer.write(&snapshot(model.view(now())));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => {
                    stopping = true;
                    model.withdraw();
                    if let Some(r) = &mut reader {
                        r.fence();
                    }
                    fatal = Some(e);
                    break;
                }
            }
        }
        // Idle collections run at1Hz. Socket readiness wakes Main immediately;
        // a live reader adds bounded reap/deadline polling, never a busy loop.
        let timeout_ns = if reader.is_some() {
            50_000_000
        } else {
            next.saturating_sub(now()).clamp(1_000_000, 1_000_000_000)
        };
        let waited = {
            let mut descriptors = vec![PollFd::new(&listener, PollFlags::IN)];
            if let Some(r) = &reader
                && !r.eof
            {
                descriptors.push(PollFd::new(&r.output, PollFlags::IN));
            }
            poll(
                &mut descriptors,
                Some(&Timespec {
                    tv_sec: (timeout_ns / 1_000_000_000) as i64,
                    tv_nsec: (timeout_ns % 1_000_000_000) as i64,
                }),
            )
        };
        if let Err(error) = waited
            && error != rustix::io::Errno::INTR
        {
            stopping = true;
            model.withdraw();
            if let Some(r) = &mut reader {
                r.fence();
            }
            fatal = Some(error.into());
        }
    }
    drop(listener);
    fs::remove_file(socket)?;
    fatal.map_or(Ok(()), Err)
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, parent, until, root, supply] if mode == "--read-supply" => {
            worker(parent, until, root, supply)
        }
        [_, mode, socket, root, supply] if mode == "--serve" => serve(socket, root, supply),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
