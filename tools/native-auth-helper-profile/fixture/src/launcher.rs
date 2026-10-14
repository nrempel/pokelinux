//! Fixed root launcher; the only unsafe code is the unchanged entry dependency.
use pokelinux_auth_helper_profile_fixture as fixed;
use std::{io, os::unix::process::CommandExt, process::Command};

// Immutable copy of Main78e9d46 native-pam/src/entry.rs. account_matches is unused.
#[allow(dead_code)]
#[path = "../entry-copy.rs"]
mod entry;

#[cfg(feature = "contained")]
#[path = "../../../../native-pam/src/containment.rs"]
mod containment;

fn launch() -> io::Result<()> {
    fixed::installation()?;
    fixed::root_owned(
        "/run/pokelinux-auth-containment-test/helper-profile-launcher",
        0o755,
        false,
    )?;
    fixed::output()?;
    fixed::membership(fixed::WORKLOAD)?;
    fixed::scope()?;
    let parent = rustix::process::getppid()
        .ok_or(io::ErrorKind::PermissionDenied)?
        .as_raw_pid() as u32;
    entry::initial(parent)?;
    #[cfg(feature = "contained")]
    containment::enter()?;
    drop(entry::enter(parent)?);
    // entry::initial marks stdio CLOEXEC. Preserve only this already-qualified
    // private nonblocking observation pipe; FD0/2 remain CLOEXEC. The real worker
    // also installs explicit helper stdio; no authentication is performed here.
    rustix::io::fcntl_setfd(std::io::stdout(), rustix::io::FdFlags::empty())?;
    Err(Command::new(fixed::PROBE).env_clear().exec())
}

fn main() {
    if launch().is_err() {
        // Typed launch failure, with no account, NSS or OS error text.
        std::process::exit(125);
    }
}
