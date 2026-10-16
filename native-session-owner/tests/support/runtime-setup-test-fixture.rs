//! Authored quiet compositor and exact production runtime setup-error path only.
//! No Source acceptance, Ready, GUI, audio effect or full integration claim.
use std::{io, path::Path, time::Duration};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--compositor-bootstrap" {
        return pokelinux_session_owner::bootstrap_entry();
    }
    if args.first().is_some_and(|v| v == "--messages-worker") {
        return pokelinux_messages::worker::entry(&args[1..]);
    }
    if args.first().is_some_and(|v| v == "--messages-read-worker") {
        return pokelinux_messages::worker::reader_entry(&args[1..]);
    }
    if args.len() == 2 && args[0] == "--config" {
        if rustix::process::getuid().as_raw() != 1000 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        loop {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let [path] = args.as_slice() else {
        return Err(io::ErrorKind::InvalidInput.into());
    };
    let config = pokelinux_session_owner::config::Config::load(Path::new(path))?;
    let before = std::fs::read_to_string("/proc/thread-self/children")?;
    assert!(before.trim().is_empty());
    let started = pokelinux_lifecycle::boottime_ns();
    let result = pokelinux_session_owner::runtime::run(&config);
    assert_eq!(
        result.unwrap_err().kind(),
        io::ErrorKind::NotFound,
        "missing audio setup after projector retention"
    );
    assert!(
        std::fs::read_to_string("/proc/thread-self/children")?
            .trim()
            .is_empty()
    );
    assert_eq!(
        rustix::process::waitpid(None, rustix::process::WaitOptions::NOHANG).unwrap_err(),
        rustix::io::Errno::CHILD
    );
    let pokelinux_session_owner::config::Messages::History { runtime, .. } = &config.messages
    else {
        panic!("history target required")
    };
    assert!(!runtime.join("messages-storage.sock").exists());
    assert!(!runtime.join("messages-read.sock").exists());
    println!(
        "RUNTIME_SETUP_ERROR original=NotFound Source_and_Projector_children=ECHILD history_listeners_absent=true elapsed_ns={} numeric_child_exit_status_not_exported=true",
        pokelinux_lifecycle::boottime_ns() - started
    );
    Ok(())
}
