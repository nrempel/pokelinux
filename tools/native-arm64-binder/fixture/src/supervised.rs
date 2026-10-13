//! Production Supervisor owns a child permanently held in actual guest-kernel Binder.
use crate::*;
use pokelinux_radio::supervisor::{ProcessIdentity, StartError, Supervisor};
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
    let mut command = Command::new(std::env::current_exe()?);
    command.arg("permanent-child");
    let identity = supervisor
        .start(&mut command)
        .map_err(|e| format!("start {e:?}"))?;
    let mut output = String::new();
    let limit = end();
    let expected = format!("OWNED-READY {} 1", identity.hex());
    loop {
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
    assert_eq!(supervisor.start(&mut command), Err(StartError::Occupied));
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
        supervisor.start(&mut command),
        Err(StartError::RegistrationUncertain)
    );
    let mut restarted = Supervisor::new(journal);
    assert_eq!(
        restarted.start(&mut command),
        Err(StartError::RegistrationUncertain)
    );
    marker("permanent-binder-registration-sigkill-actual-reap-old-output-fenced");
    marker("durable-uncertainty-refuses-restart-after-local-reap");
    Ok(())
}
