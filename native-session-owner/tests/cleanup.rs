use pokelinux_lifecycle::{boottime_ns, session::Provider};
use pokelinux_session_owner::{Compositor, SourceOwner, runtime::drain_after};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    time::Duration,
};
struct Guard {
    owner: SourceOwner,
    until: u64,
}
impl Guard {
    fn new() -> Self {
        let fixture = PathBuf::from(
            std::env::var_os("SOURCE_OWNER_FIXTURE")
                .expect("isolated root-installed Source fixture"),
        );
        let start = boottime_ns();
        let owner = SourceOwner::spawn(
            Compositor {
                helper: fixture.clone(),
                executable: fixture,
                arguments: vec!["valid".into()],
                environment: BTreeMap::new(),
                source: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            },
            start + 1_000_000_000,
        )
        .unwrap();
        let mut g = Self {
            owner,
            until: start + 3_000_000_000,
        };
        loop {
            match g.owner.poll() {
                Ok(Some(_)) => break,
                Ok(None) if boottime_ns() < g.until => std::thread::sleep(Duration::from_millis(1)),
                other => panic!("real Source setup failed: {other:?}"),
            }
        }
        g
    }
    fn waited(&self) {
        assert!(
            !self.owner.occupied(),
            "operation completion must actually reap Source before return"
        );
        assert!(self.owner.exit_status().is_some());
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.owner.stop();
        while self.owner.occupied() {
            let _ = self.owner.poll();
            if boottime_ns() >= self.until {
                std::process::abort();
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
#[test]
#[ignore = "requires isolated root, UID transition and built Source fixture"]
fn actual_lifecycle_early_setup_error_reaps_same_source_before_return() {
    let mut g = Guard::new();
    let pid = g.owner.pid();
    let result = pokelinux_lifecycle::service::run_with_services(
        Path::new("/usr/bin/true"),
        Path::new("/absent-policy"),
        Path::new("/absent-profile"),
        Path::new("/absent-entry"),
        None,
        None,
        Some(&mut g.owner),
        None,
        None,
        None,
        None,
    );
    assert_eq!(
        result.as_ref().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    let result = drain_after(&mut g.owner, result, g.until);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(g.owner.pid(), pid);
    g.waited();
}
#[test]
#[ignore = "requires isolated root, UID transition and built Source fixture"]
fn successful_completion_also_reaps_and_repeated_drain_preserves_status() {
    let mut g = Guard::new();
    let pid = g.owner.pid();
    assert_eq!(drain_after(&mut g.owner, Ok(7u8), g.until).unwrap(), 7);
    g.waited();
    let status = g.owner.exit_status();
    assert_eq!(drain_after(&mut g.owner, Ok(9u8), g.until).unwrap(), 9);
    assert_eq!(g.owner.pid(), pid);
    assert_eq!(g.owner.exit_status(), status);
    g.waited();
}
#[test]
#[ignore = "requires isolated root, UID transition and built Source fixture"]
fn expired_cleanup_keeps_custody_and_cannot_report_success() {
    let mut g = Guard::new();
    let result = drain_after(&mut g.owner, Ok(()), boottime_ns());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    assert!(g.owner.occupied());
}
