//! Only existing process-owner API. Actual owned-process helper pattern from
//! tools/native-arm64-binder/fixture/{bootstrap_controls,supervised}.rs.
#![allow(dead_code)]
use pokelinux_process_owner::{
    bootstrap::{WorkerCommand, WorkerState},
    deadline::{BootClock, Clock, Deadline},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use std::{
    fs, io,
    os::unix::{fs::DirBuilderExt, fs::PermissionsExt, process::ExitStatusExt},
    path::Path,
    process::ExitStatus,
    time::Duration,
};

pub fn require(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
pub fn now() -> u64 {
    BootClock
        .now()
        .as_nanos()
        .try_into()
        .expect("bounded real BOOTTIME")
}
pub fn plus(at: u64, delta: u64) -> io::Result<u64> {
    at.checked_add(delta)
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}
pub fn deadline(until: u64) -> Deadline {
    Deadline::at(Duration::from_nanos(until))
}
pub fn before(until: u64) -> io::Result<()> {
    if now() < until {
        Ok(())
    } else {
        Err(io::ErrorKind::TimedOut.into())
    }
}
pub fn pause() {
    std::thread::sleep(Duration::from_millis(1));
}
pub fn until(at: u64, bound: u64) -> io::Result<()> {
    while now() < at {
        before(bound)?;
        pause();
    }
    before(bound)
}
pub fn private_dir(path: &Path) -> io::Result<()> {
    fs::DirBuilder::new().mode(0o700).create(path)
}
pub fn command(mode: &str, args: &[String]) -> io::Result<WorkerCommand> {
    let mut value = WorkerCommand::new(&std::env::current_exe()?)?;
    value.arg(mode);
    for arg in args {
        value.arg(arg);
    }
    Ok(value)
}
pub fn start(
    owner: &mut Supervisor,
    command: &mut WorkerCommand,
    cleanup: u64,
) -> io::Result<ProcessIdentity> {
    match owner.start_worker(command) {
        Ok(identity) => Ok(identity),
        Err(error) => {
            dispose(owner, cleanup)?;
            Err(io::Error::other(format!(
                "actual cooperative start: {error:?}"
            )))
        }
    }
}
pub fn ready(owner: &mut Supervisor, identity: ProcessIdentity, until: u64) -> io::Result<()> {
    loop {
        before(until)?;
        match owner.poll_worker()? {
            WorkerState::Ready => {
                return require(owner.admits(identity), "READY without admission");
            }
            WorkerState::Initializing => (),
            WorkerState::Terminal => return Err(io::Error::other("terminal before READY")),
        }
        require(owner.occupied(), "child custody lost before READY")?;
        pause();
    }
}
pub fn line(owner: &mut Supervisor, buffered: &mut Vec<u8>, until: u64) -> io::Result<String> {
    loop {
        before(until)?;
        if let Some(end) = buffered.iter().position(|&byte| byte == b'\n') {
            let bytes: Vec<_> = buffered.drain(..=end).collect();
            return String::from_utf8(bytes).map_err(|_| io::ErrorKind::InvalidData.into());
        }
        let mut part = [0; 256];
        match owner.read_output(&mut part) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => buffered.extend_from_slice(&part[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
            Err(error) => return Err(error),
        }
        require(buffered.len() <= 4096, "bounded diagnostic output exceeded")?;
        pause();
    }
}
pub fn reap(owner: &mut Supervisor, until: u64) -> io::Result<ExitStatus> {
    let child_pid = owner.child_pid().ok_or(io::ErrorKind::NotConnected)?;
    // poll_reap drops Supervisor's stdout handle. Observe actual pipe EOF first,
    // with no more than 4096 remaining output bytes and the original cutoff.
    let mut remaining_output = 0usize;
    loop {
        before(until)?;
        let mut bytes = [0; 256];
        let capacity = bytes.len().min(4097 - remaining_output);
        match owner.read_output(&mut bytes[..capacity]) {
            Ok(0) => break,
            Ok(count) => {
                remaining_output += count;
                require(
                    remaining_output <= 4096,
                    "remaining child stdout exceeded 4096 bytes",
                )?;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => pause(),
            Err(error) => return Err(error),
        }
    }
    before(until)?;
    println!("PIPE-EOF supervisor child_pid={child_pid} remaining_bytes={remaining_output}");
    loop {
        before(until)?;
        if let Some(status) = owner.poll_reap()? {
            before(until)?;
            println!("CHILD-WAIT supervisor child_pid={child_pid} stdout_eof=true status={status}");
            return Ok(status);
        }
        require(owner.occupied(), "unreaped child capacity released")?;
        pause();
    }
}
pub fn dispose(owner: &mut Supervisor, until: u64) -> io::Result<()> {
    if owner.occupied() {
        let _ = owner.stop();
        // Error disposal has the same EOF-before-wait rule and original bound.
        // Never discard the owned child on a failed closure observation.
        if reap(owner, until).is_err() {
            std::process::abort();
        }
    }
    Ok(())
}
pub fn refusal<T: std::fmt::Debug>(
    value: io::Result<T>,
    kind: io::ErrorKind,
    why: &str,
) -> io::Result<()> {
    match value {
        Err(error) if error.kind() == kind => Ok(()),
        other => Err(io::Error::other(format!(
            "{why}: expected {kind:?}, got {other:?}"
        ))),
    }
}
pub fn journal_unchanged(
    owner: &mut Supervisor,
    command: &mut WorkerCommand,
    runtime: &Path,
    identity: ProcessIdentity,
) -> io::Result<()> {
    let journal = runtime.join("admission");
    require(
        !owner.occupied() && !owner.admits(identity),
        "post-wait custody/admission wrong",
    )?;
    require(
        fs::read_to_string(&journal)? == format!("admission-v1\n{}\n1\n", identity.hex()),
        "durable uncertainty marker changed",
    )?;
    require(
        fs::metadata(&journal)?.permissions().mode() & 0o777 == 0o600,
        "durable uncertainty mode changed",
    )?;
    require(
        owner.start_worker(command) == Err(StartError::AdmissionUncertain),
        "same owner retried",
    )?;
    let mut recreated = Supervisor::new(journal);
    require(
        recreated.start_worker(command) == Err(StartError::AdmissionUncertain),
        "recreated owner retried",
    )?;
    require(!recreated.occupied(), "recreated owner spawned a child")
}
pub fn killed(status: ExitStatus) -> io::Result<()> {
    require(
        status.signal() == Some(9),
        "expected exact actual signal9 wait",
    )
}
