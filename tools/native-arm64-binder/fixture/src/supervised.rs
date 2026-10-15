//! Production Supervisor owns a child permanently held in actual guest-kernel Binder.
use crate::*;
use pokelinux_radio::{
    bootstrap::{WorkerCommand, WorkerState},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use std::{
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    process::Command,
};
pub fn child(peer: &SIBinder) -> TestResult {
    let id = ProcessIdentity::from_hex(&std::env::var("POKELINUX_OWNER_INCARNATION")?)
        .ok_or("bad incarnation")?;
    let mut session = TransportSession::new(n(2));
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "permanent".into(),
            n(8),
            deadline(),
        )
        .unwrap();
    latch(peer, 110)?;
    session.stop(&key).unwrap();
    let before = Instant::now();
    for _ in 0..16 {
        assert!(session.worker_occupied());
        assert!(session.poll_registration().is_none());
        assert_eq!(
            session.snapshot().phase,
            Phase::Failed(ReceiverFailure::Cancelled)
        );
        assert!(session.snapshot().response.is_none());
        assert_eq!(
            session.start(
                RadioTransport::from_binder(peer.clone())?,
                "retry".into(),
                n(8),
                deadline()
            ),
            Err(LifecycleError::WorkerOccupied)
        );
        session.stop(&key).unwrap();
    }
    assert!(before.elapsed() < Duration::from_secs(2));
    println!("WORKER-PID {}", std::process::id());
    println!("OWNED-READY {} 1", id.hex());
    std::io::stdout().flush()?;
    // Keep session, worker JoinHandle and death watch owned until parent SIGKILL/reap.
    loop {
        std::thread::park();
    }
}
pub fn parent() -> TestResult {
    let path = std::env::temp_dir().join(format!("arm64-binder-supervisor-{}", std::process::id()));
    fs::create_dir(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    let journal = path.join("admission");
    let mut supervisor = Supervisor::new(journal.clone());
    let mut command = WorkerCommand::new(&std::env::current_exe()?)?;
    command.arg("permanent-child");
    let identity = supervisor
        .start_worker(&mut command)
        .map_err(|e| format!("start {e:?}"))?;
    assert!(
        !supervisor.admits(identity),
        "initializing bootstrap admitted output"
    );
    let mut output = String::new();
    let limit = end();
    let expected = format!("OWNED-READY {} 1", identity.hex());
    loop {
        assert_ne!(supervisor.poll_worker()?, WorkerState::Terminal);
        let mut bytes = [0u8; 1024];
        match supervisor.read_output(&mut bytes) {
            Ok(0) => {}
            Ok(length) => {
                output.push_str(std::str::from_utf8(&bytes[..length])?);
                assert!(output.len() < 4096);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        if output.lines().any(|line| line == expected) {
            break;
        }
        assert!(Instant::now() < limit, "permanent child readiness timeout");
        std::thread::yield_now();
    }
    assert!(supervisor.admits_frame(identity, identity, 1));
    assert!(supervisor.occupied());
    assert_eq!(
        supervisor.start_worker(&mut command),
        Err(StartError::Occupied)
    );
    marker("permanent-binder-registration-child-stop-snapshot-responsive");
    supervisor.stop()?;
    assert!(supervisor.occupied());
    assert!(!supervisor.admits(identity));
    marker("supervisor-revokes-before-reap-keeps-child-occupied");
    let limit = end();
    let status = loop {
        if let Some(status) = supervisor.poll_reap()? {
            break status;
        }
        assert!(supervisor.occupied());
        assert!(Instant::now() < limit);
        std::thread::yield_now();
    };
    assert_eq!(status.signal(), Some(9));
    assert!(!supervisor.occupied());
    assert!(!supervisor.admits_frame(identity, identity, 1));
    assert!(journal.exists());
    assert_eq!(fs::metadata(&journal)?.permissions().mode() & 0o777, 0o600);
    assert_eq!(
        supervisor.start_worker(&mut command),
        Err(StartError::RegistrationUncertain)
    );
    let mut restarted = Supervisor::new(journal);
    assert_eq!(
        restarted.start_worker(&mut command),
        Err(StartError::RegistrationUncertain)
    );
    marker("permanent-binder-registration-sigkill-actual-reap-old-output-fenced");
    marker("durable-uncertainty-refuses-restart-after-local-reap");
    Ok(())
}

/// Outer verified subreaper owns the parent-death test and eventual orphan wait.
/// This infrastructure body deliberately does not call Stop or release Binder.
pub fn parent_death_supervisor() -> TestResult {
    let journal = std::path::PathBuf::from(std::env::args().nth(2).ok_or("missing journal")?);
    let executable = std::env::current_exe()?;
    let text = executable.to_str().ok_or("non-ASCII executable")?;
    assert!(
        text.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    );
    let mut supervisor = Supervisor::new(journal);
    let mut command = WorkerCommand::new(&executable)?;
    command.arg("permanent-child");
    let identity = supervisor
        .start_worker(&mut command)
        .map_err(|e| format!("start {e:?}"))?;
    let expected = format!("OWNED-READY {} 1", identity.hex());
    assert!(
        !supervisor.admits(identity),
        "initializing bootstrap admitted output"
    );
    let mut output = String::new();
    let limit = end();
    let worker = loop {
        assert_ne!(supervisor.poll_worker()?, WorkerState::Terminal);
        let mut bytes = [0; 1024];
        match supervisor.read_output(&mut bytes) {
            Ok(length) => {
                output.push_str(std::str::from_utf8(&bytes[..length])?);
                assert!(output.len() < 4096);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        if output.lines().any(|line| line == expected) {
            break output
                .lines()
                .find_map(|line| line.strip_prefix("WORKER-PID "))
                .ok_or("missing actual child PID")?
                .parse::<u32>()?;
        }
        assert!(Instant::now() < limit, "permanent child readiness timeout");
        std::thread::yield_now();
    };
    assert!(supervisor.occupied());
    assert!(supervisor.admits_frame(identity, identity, 1));
    println!(
        "{{\"schema\":1,\"supervisor_pid\":{},\"worker_pid\":{},\"incarnation\":\"{}\",\"attempt\":1,\"binder_hold_confirmed\":true,\"worker_argv\":[\"{}\",\"--pokelinux-radio-worker\",\"permanent-child\"]}}",
        std::process::id(),
        worker,
        identity.hex(),
        text
    );
    std::io::stdout().flush()?;
    loop {
        std::thread::park();
    }
}

pub fn parent_death_refuse() -> TestResult {
    let journal = std::path::PathBuf::from(std::env::args().nth(2).ok_or("missing journal")?);
    let mut supervisor = Supervisor::new(journal);
    assert_eq!(
        supervisor.start(&mut Command::new("unreachable")),
        Err(StartError::RegistrationUncertain)
    );
    assert!(!supervisor.occupied());
    let mut worker = WorkerCommand::new(&std::env::current_exe()?)?;
    worker.arg("permanent-child");
    assert_eq!(
        supervisor.start_worker(&mut worker),
        Err(StartError::RegistrationUncertain)
    );
    assert!(!supervisor.occupied());
    println!(
        "{{\"schema\":1,\"durable_uncertainty_refused\":true,\"binder_initialized\":false,\"child_spawned\":false}}"
    );
    Ok(())
}
