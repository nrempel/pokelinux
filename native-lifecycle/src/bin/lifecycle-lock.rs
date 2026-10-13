//! Unprivileged installed lock entry: native intent first, unchanged original action second.
use pokelinux_lifecycle::{boottime_ns, lock, service::trusted_executable};
use std::{io, os::unix::process::CommandExt, path::Path, process::Command};

fn main() -> io::Result<()> {
    let until = boottime_ns()
        .checked_add(lock::ENTRY_BUDGET_NS)
        .ok_or(io::ErrorKind::InvalidInput)?;
    if std::env::args_os().len() != 1
        || rustix::process::getuid().as_raw() != lock::SESSION_UID
        || rustix::process::geteuid().as_raw() != lock::SESSION_UID
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    trusted_executable(Path::new(lock::ORIGINAL_ACTION))?;
    let acknowledged = lock::admit(Path::new(lock::SOCKET), until)?;
    println!(
        "{}",
        serde_json::json!({"event":"native_lock_intent_ack","acknowledged_at_ns":acknowledged,
            "until_ns":until,"secure_presentation_qualified":false})
    );
    if boottime_ns() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    // The retained exact script runs as the same session UID. Its lookup paths
    // are installer-owned, and shell startup injection is excluded.
    Err(Command::new(lock::ORIGINAL_ACTION)
        .env(
            "PATH",
            "/usr/share/omarchy/bin:/usr/local/bin:/usr/bin:/bin",
        )
        .env("OMARCHY_PATH", "/usr/share/omarchy")
        .env_remove("BASH_ENV")
        .env_remove("ENV")
        .exec())
}
