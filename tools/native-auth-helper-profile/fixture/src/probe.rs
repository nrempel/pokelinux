//! Authored safe Rust profile/migration probe, never a PAM helper replacement.
#![forbid(unsafe_code)]
use pokelinux_auth_helper_profile_fixture as fixed;
use std::{fs::OpenOptions, io, io::Write, os::unix::fs::OpenOptionsExt};

// PKHCPR01 | outcome:u8 | stage:u8 | errno_class:u8 | reserved0:u8 | flags:u32le.
// No variable-length data, raw text, credential, account or PID leaves the probe.
const OUTPUT: u32 = 1;
const INSTALLED: u32 = 2;
const PROFILE_BEFORE: u32 = 4;
#[cfg(not(feature = "contained"))]
const WORKLOAD_BEFORE: u32 = 8;
#[cfg(not(feature = "contained"))]
const ROOT_SCOPE: u32 = 16;
const WRITE_COMPLETE: u32 = 32;
#[cfg(not(feature = "contained"))]
const ESCAPED_AFTER: u32 = 64;
const PROFILE_AFTER: u32 = 128;

fn errno_class(error: &io::Error) -> u8 {
    match error.raw_os_error() {
        Some(libc::EPERM) => 1,
        Some(libc::EACCES) => 2,
        Some(libc::EROFS) => 3,
        Some(libc::ENOENT) => 5,
        _ => 4,
    }
}

#[cfg(not(feature = "contained"))]
fn observe(packet: &mut [u8; 16], flags: &mut u32) -> io::Result<()> {
    packet[9] = 1;
    fixed::output()?;
    *flags |= OUTPUT;
    packet[9] = 2;
    fixed::installation()?;
    *flags |= INSTALLED;
    packet[9] = 3;
    fixed::profile()?;
    *flags |= PROFILE_BEFORE;
    packet[9] = 4;
    fixed::membership(fixed::WORKLOAD)?;
    *flags |= WORKLOAD_BEFORE;
    packet[9] = 5;
    fixed::scope()?;
    *flags |= ROOT_SCOPE;
    // Fixed, already-qualified private sibling. Never writes any supplied PID:
    // cgroup v2 interprets 0 as the caller, so only this process can be moved.
    // This handle is dropped before the independent observation/teardown hold.
    {
        packet[9] = 6;
        let mut target = match OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(fixed::DESTINATION)
        {
            Ok(file) => file,
            Err(error) => {
                packet[8] = 1; // MoveOpenDenied; an errno class is not a RED.
                return Err(error);
            }
        };
        packet[9] = 7;
        match target.write(b"0\n") {
            Ok(2) => *flags |= WRITE_COMPLETE,
            Ok(_) => {
                packet[8] = 2;
                return Err(io::ErrorKind::WriteZero.into());
            }
            Err(error) => {
                packet[8] = 2;
                return Err(error);
            }
        }
    }
    packet[8] = 3; // MoveUncertain until both postconditions hold.
    packet[9] = 8;
    fixed::membership(fixed::ESCAPED)?;
    *flags |= ESCAPED_AFTER;
    packet[9] = 9;
    fixed::profile()?;
    *flags |= PROFILE_AFTER;
    packet[8] = 4; // Moved; qualification still belongs to the independent driver.
    packet[9] = 10;
    Ok(())
}

#[cfg(feature = "contained")]
fn observe(packet: &mut [u8; 16], flags: &mut u32) -> io::Result<()> {
    packet[9] = 1;
    fixed::output()?;
    *flags |= OUTPUT;
    packet[9] = 2;
    fixed::installation()?;
    *flags |= INSTALLED;
    packet[9] = 3;
    fixed::profile()?;
    *flags |= PROFILE_BEFORE;
    // The same fixed self-only target as the actual uncontained RED. Membership
    // and full profile are independently checked outside this filesystem view.
    packet[9] = 6;
    let error = match OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(fixed::DESTINATION)
    {
        Err(error) => error,
        Ok(mut target) => {
            packet[8] = 3;
            packet[9] = 7;
            if target.write(b"0\n")? == 2 {
                *flags |= WRITE_COMPLETE;
            }
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    };
    packet[8] = 1;
    packet[10] = errno_class(&error);
    if error.raw_os_error() != Some(libc::ENOENT) {
        return Err(error);
    }
    packet[9] = 9;
    fixed::profile()?;
    *flags |= PROFILE_AFTER;
    packet[9] = 10;
    Ok(())
}

fn main() {
    let mut packet = [0u8; 16];
    packet[..8].copy_from_slice(b"PKHCPR01");
    let mut flags = 0;
    if let Err(error) = observe(&mut packet, &mut flags) {
        packet[10] = errno_class(&error);
    }
    packet[12..].copy_from_slice(&flags.to_le_bytes());
    if flags & OUTPUT != 0 {
        // A single nonblocking <=PIPE_BUF write, without retries or raw errors.
        // Missing/short output is a driver failure, never implicit success.
        let _ = rustix::io::write(std::io::stdout(), &packet);
    }
    // No extra thread, fork, exec, signal, cgroup write or implicit self-cleanup.
    // Root owns the original deadline and retained pidfd across launcher exec.
    loop {
        std::thread::park();
    }
}
