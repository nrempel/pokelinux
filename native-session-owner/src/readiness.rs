//! One-shot main-process startup notification. This grants no app permission.
//! The lifecycle actor supplies its already accepted Source cutoff; there is no
//! second Source reader, lease renewal, blocking connect or detached worker.
use crate::MAX_SOURCE_STARTUP_NS;
use pokelinux_display::SourceIdentity;
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use pokelinux_lifecycle::boottime_ns;
use rustix::{fd::OwnedFd, net};
use std::{ffi::OsString, io, os::unix::ffi::OsStrExt, path::PathBuf};

#[derive(Eq, PartialEq)]
enum Phase {
    Pending,
    Published,
    Retired,
}

pub struct Readiness {
    wayland: net::SocketAddrUnix,
    source: OwnedFd,
    identity: SourceIdentity,
    view: ProcessView,
    notification: OwnedFd,
    destination: net::SocketAddrUnix,
    startup_until: u64,
    phase: Phase,
}
impl Readiness {
    /// Both the pidfd and the original BOOTTIME deadline come from the owned
    /// compositor startup. Notification delivery failures are terminal.
    pub fn new(
        wayland: PathBuf,
        source: OwnedFd,
        identity: SourceIdentity,
        destination: OsString,
        startup_until: u64,
    ) -> io::Result<Self> {
        let now = boottime_ns();
        if rustix::process::getuid().as_raw() != 0
            || rustix::process::geteuid().as_raw() != 0
            || identity.uid != 1000
            || identity.gid != 1000
            || !wayland.is_absolute()
            || startup_until <= now
            || startup_until - now > MAX_SOURCE_STARTUP_NS
            || !rustix::io::fcntl_getfd(&source)?.contains(rustix::io::FdFlags::CLOEXEC)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let bytes = destination.as_bytes();
        let destination = match bytes.first() {
            Some(b'/') => net::SocketAddrUnix::new(destination.as_os_str())?,
            Some(b'@') if bytes.len() > 1 => net::SocketAddrUnix::new_abstract_name(&bytes[1..])?,
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        };
        let view = ProcessView::open()?;
        if !view.is_pidfd(&source)? {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            wayland: net::SocketAddrUnix::new(wayland)?,
            source,
            identity,
            view,
            notification: net::socket_with(
                net::AddressFamily::UNIX,
                net::SocketType::DGRAM,
                net::SocketFlags::NONBLOCK | net::SocketFlags::CLOEXEC,
                None,
            )?,
            destination,
            startup_until,
            phase: Phase::Pending,
        })
    }

    /// `source_until` is the existing accepted sample's cutoff, including while
    /// locked. Recheck it at publication; a previous ready bool is insufficient.
    pub fn poll(&mut self, source_until: Option<u64>, stopping: bool) -> io::Result<()> {
        if stopping {
            self.phase = Phase::Retired;
        }
        if self.phase != Phase::Pending {
            return Ok(());
        }
        let result = self.advance(source_until);
        if result.is_err() {
            self.phase = Phase::Retired;
        }
        result
    }

    fn advance(&mut self, source_until: Option<u64>) -> io::Result<()> {
        let now = boottime_ns();
        if now >= self.startup_until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let Some(source_until) = source_until.filter(|&until| now < until) else {
            return Ok(());
        };
        let stream = net::socket_with(
            net::AddressFamily::UNIX,
            net::SocketType::STREAM,
            net::SocketFlags::NONBLOCK | net::SocketFlags::CLOEXEC,
            None,
        )?;
        match net::connect(&stream, &self.wayland) {
            Ok(()) => {}
            Err(
                rustix::io::Errno::NOENT
                | rustix::io::Errno::CONNREFUSED
                | rustix::io::Errno::AGAIN,
            ) => {
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        }
        let credentials = net::sockopt::socket_peercred(&stream)?;
        let peer = socket_peer_pidfd(&stream)?;
        if credentials.pid != self.identity.pid
            || credentials.uid.as_raw() != self.identity.uid
            || credentials.gid.as_raw() != self.identity.gid
            || !self
                .view
                .matches(&self.source, &peer, self.identity.pid.as_raw_pid() as u32)?
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // No readiness survives setup work which consumed the original sample
        // or startup budget. No new validity period is manufactured here.
        let now = boottime_ns();
        if now >= self.startup_until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if now >= source_until {
            return Ok(());
        }
        if net::sendto(
            &self.notification,
            b"READY=1",
            net::SendFlags::DONTWAIT,
            &self.destination,
        )? != 7
        {
            return Err(io::ErrorKind::WriteZero.into());
        }
        self.phase = Phase::Published;
        Ok(())
    }
}

impl pokelinux_lifecycle::startup::Observer for Readiness {
    fn poll(&mut self, source_until: Option<u64>, stopping: bool) -> io::Result<()> {
        Readiness::poll(self, source_until, stopping)
    }
}
