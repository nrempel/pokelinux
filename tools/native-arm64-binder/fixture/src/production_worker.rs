//! Actual selected production executable, public named Binder service, and owned output.
use crate::*;
use pokelinux_radio::{
    bootstrap::{WorkerCommand, WorkerState},
    observation::{self, Frame, Kind},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use std::{
    fs,
    io::{self, Write},
    os::unix::process::ExitStatusExt,
    path::PathBuf,
};
fn now() -> u64 {
    BootClock.now().as_nanos().try_into().unwrap()
}
fn require(ok: bool, text: &str) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(text).into())
    }
}
#[derive(Default)]
struct Reader {
    buffer: Vec<u8>,
    revision: u64,
}
impl Reader {
    fn next(
        &mut self,
        owner: &mut Supervisor,
        identity: ProcessIdentity,
    ) -> Result<Option<Frame>, Box<dyn std::error::Error>> {
        for _ in 0..2 {
            if let Some((frame, length)) = observation::decode(&self.buffer)? {
                require(
                    owner.admits_frame(identity, frame.identity, frame.attempt),
                    "unadmitted production frame",
                )?;
                require(frame.revision > self.revision, "old production revision")?;
                self.revision = frame.revision;
                self.buffer.drain(..length);
                return Ok(Some(frame));
            }
            let mut bytes = [0; 512];
            match owner.read_output(&mut bytes) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
                Ok(length) => {
                    require(
                        self.buffer.len() + length <= 2 * observation::MAX_FRAME_LEN,
                        "unbounded worker output",
                    )?;
                    self.buffer.extend_from_slice(&bytes[..length]);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(None)
    }
}
/// Parent is test infrastructure; the executable it owns is production Rust.
pub fn run() -> TestResult {
    let worker = PathBuf::from(std::env::args().nth(2).ok_or("missing production worker")?);
    let journal = PathBuf::from(std::env::args().nth(3).ok_or("missing admission journal")?);
    let path = worker.to_str().ok_or("non-UTF8 worker")?;
    require(
        path.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b)),
        "unsafe JSON worker path",
    )?;
    let ack = journal.parent().ok_or("journal parent")?.join("active-ack");
    let peer = peer(true)?;
    require(
        report(&peer)?[0] == 0,
        "production worker needs fresh named peer",
    )?;
    let setup_until = now() + 10_000_000_000;
    let until = now() + 30_000_000_000;
    let mut command = WorkerCommand::new(&worker)?;
    let until_text = until.to_string();
    for argument in [
        "--device",
        "/dev/binderfs/radio",
        "--service",
        "org.pokelinux.fixture.arm64-radio",
        "--until-ns",
        &until_text,
    ] {
        command.arg(argument);
    }
    let mut owner = Supervisor::new(journal.clone());
    let identity = match owner.start_worker(&mut command) {
        Ok(identity) => identity,
        Err(error) => {
            // A post-spawn setup fault retains custody. It is never the desired
            // callback RED, but still receives the original bounded disposal.
            let cleanup_until = setup_until + 2_000_000_000;
            if owner.occupied() {
                owner.stop()?;
                loop {
                    if owner.poll_reap()?.is_some() {
                        require(
                            now() < cleanup_until,
                            "setup-failure actual wait crossed cutoff",
                        )?;
                        break;
                    }
                    require(now() < cleanup_until, "setup-failure child still owned")?;
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            return Err(format!("start production worker {error:?}").into());
        }
    };
    let mut reader = Reader::default();
    let mut cleanup_until = setup_until + 2_000_000_000;
    let mut setup_complete = false;
    let mut raw_received = false;
    let mut peer_death_observed = false;
    let result = (|| -> TestResult {
        loop {
            require(
                now() < setup_until,
                "production registration setup deadline",
            )?;
            require(
                owner.poll_worker()? != WorkerState::Terminal,
                "production bootstrap terminal",
            )?;
            if let Some(frame) = reader.next(&mut owner, identity)? {
                require(
                    frame.kind == Kind::Active { client_id: 42 },
                    "production registration did not activate",
                )?;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let registers = report(&peer)?;
        require(
            registers[0] == 1 && registers[4] == 1000 && registers[5] == 0,
            "actual production registration count/caller",
        )?;
        let worker_pid = registers[3];
        require(worker_pid > 1, "missing actual registering worker PID")?;
        println!(
            "{{\"schema\":1,\"phase\":\"active\",\"supervisor_pid\":{},\"worker_pid\":{},\"incarnation\":\"{}\",\"attempt\":1,\"until_ns\":{},\"registration_count\":1,\"client_id\":42,\"device\":\"/dev/binderfs/radio\",\"service\":\"org.pokelinux.fixture.arm64-radio\",\"lookup\":\"IServiceManager.checkService\",\"registration_active\":true,\"worker_argv\":[\"{}\",\"--pokelinux-radio-worker\",\"--device\",\"/dev/binderfs/radio\",\"--service\",\"org.pokelinux.fixture.arm64-radio\",\"--until-ns\",\"{}\"]}}",
            std::process::id(),
            worker_pid,
            identity.hex(),
            until,
            path,
            until
        );
        std::io::stdout().flush()?;
        loop {
            require(
                now() < setup_until,
                "independent active inspection exceeded original setup cutoff",
            )?;
            if fs::read_to_string(&ack).ok().as_deref() == Some(identity.hex().as_str()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        setup_complete = true;
        let t0 = now();
        let raw_until = t0 + 1_000_000_000;
        let death_until = t0 + 2_000_000_000;
        cleanup_until = t0 + 3_000_000_000;
        loop {
            if let Some(frame) = reader.next(&mut owner, identity)? {
                require(
                    now() < raw_until,
                    "raw observation consumed beyond original oracle cutoff",
                )?;
                require(
                    frame.kind
                        == Kind::Raw {
                            slot_id: 0,
                            serial: 7,
                            rilc_msg_id: 137,
                            declared_length: 1,
                            error: 0,
                            payload: vec![7],
                        },
                    "unexpected raw production observation",
                )?;
                raw_received = true;
                break;
            }
            if now() >= raw_until {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        if raw_received {
            // The guest still independently owns and waits the peer process.
            let _ = control(&peer, 107);
            loop {
                require(
                    now() < death_until,
                    "retained production death watch missed original observation cutoff",
                )?;
                if let Some(frame) = reader.next(&mut owner, identity)? {
                    require(
                        now() < death_until,
                        "death frame consumed beyond original cutoff",
                    )?;
                    require(
                        frame.kind == Kind::Failed(ReceiverFailure::ServiceDied),
                        "production peer death did not withdraw state",
                    )?;
                    peer_death_observed = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        Ok(())
    })();
    // The same pre-fixed disposal cutoff applies to successful and failed RAW.
    owner.stop()?;
    require(
        !owner.admits(identity) && owner.occupied(),
        "Stop failed to revoke/retain production child",
    )?;
    let status = loop {
        if let Some(status) = owner.poll_reap()? {
            require(
                now() < cleanup_until,
                "actual production child wait crossed original disposal cutoff",
            )?;
            break status;
        }
        require(
            now() < cleanup_until,
            "production child still owned beyond original disposal cutoff",
        )?;
        std::thread::sleep(Duration::from_millis(1));
    };
    require(
        status.signal() == Some(9),
        "production worker needs actual SIGKILL wait",
    )?;
    require(
        !owner.occupied() && !owner.admits_frame(identity, identity, 1),
        "old production connection admitted",
    )?;
    require(
        fs::read_to_string(&journal)? == format!("admission-v1\n{}\n1\n", identity.hex()),
        "production journal changed",
    )?;
    require(
        owner.start_worker(&mut command) == Err(StartError::RegistrationUncertain),
        "same owner retried production registration",
    )?;
    let mut restarted = Supervisor::new(journal);
    require(
        restarted.start_worker(&mut command) == Err(StartError::RegistrationUncertain)
            && !restarted.occupied(),
        "recreated owner retried production registration",
    )?;
    result?;
    let desired_red = setup_complete && !raw_received;
    let passed = setup_complete && raw_received && peer_death_observed;
    println!(
        "{{\"schema\":1,\"phase\":\"result\",\"setup_complete\":{},\"raw_received\":{},\"peer_death_observed\":{},\"worker_actual_wait_signal\":9,\"durable_refusal\":true,\"old_frame_fenced\":true,\"passed\":{},\"desired_boundary_red\":{}}}",
        setup_complete, raw_received, peer_death_observed, passed, desired_red
    );
    std::io::stdout().flush()?;
    require(
        passed,
        "production raw callback publication missing after successful setup",
    )
}
