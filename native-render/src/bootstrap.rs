//! Owned copy of the source2 AQ initial session prefix and its separate FDs.
//!
//! This validates value/descriptor shape and duplicates CLOEXEC capabilities;
//! it authenticates no DRM device, output/routing authority or native retirement.
use std::{
    fs::File,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        unix::fs::{FileTypeExt, MetadataExt},
    },
};

pub const MAX_BOOTSTRAP_DEVICES: usize = 32;
pub const MAX_BOOTSTRAP_EVENTS: usize = 34;
pub const MAX_BOOTSTRAP_FDS: usize = 64;
const EVENT_VERSION: u32 = 1;
const INIT: u32 = 1;
const DEVICE_ADD: u32 = 2;
const ENUMERATION_DONE: u32 = 3;
const F_DUPFD_CLOEXEC: i32 = 1030;
const F_GETFD: i32 = 1;
const FD_CLOEXEC: i32 = 1;

/// Exact source2 Aquamarine::SSessionRenderEvent value layout. `event_type`
/// spells C++ `type`; byte strings preserve every byte, including after NUL.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRenderEvent {
    pub version: u32,
    pub event_type: u32,
    pub session_sequence: u64,
    pub session_generation: u64,
    pub device_lease_id: u64,
    pub device_generation: u64,
    pub rdev: u64,
    pub active: u32,
    pub vt: u32,
    pub change_type: u32,
    pub connector_id: u32,
    pub prop_id: u32,
    pub boot_vga: u32,
    pub reserved: [u32; 4],
    pub seat_name: [u8; 64],
    pub device_path: [u8; 256],
}
impl Default for SessionRenderEvent {
    fn default() -> Self {
        Self {
            version: EVENT_VERSION,
            event_type: 0,
            session_sequence: 0,
            session_generation: 0,
            device_lease_id: 0,
            device_generation: 0,
            rdev: 0,
            active: 0,
            vt: 0,
            change_type: 0,
            connector_id: 0,
            prop_id: 0,
            boot_vga: 0,
            reserved: [0; 4],
            seat_name: [0; 64],
            device_path: [0; 256],
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct BorrowedBootstrapEvent<'a> {
    pub metadata: &'a SessionRenderEvent,
    pub primary: Option<BorrowedFd<'a>>,
    pub render: Option<BorrowedFd<'a>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    Invalid,
    Capacity,
    /// Actual descriptor operation failure, retaining the OS errno if available.
    Descriptor(i32),
    Allocation,
}

#[derive(Debug)]
pub struct OwnedBootstrapEvent {
    metadata: SessionRenderEvent,
    primary: Option<OwnedFd>,
    render: Option<OwnedFd>,
}
impl OwnedBootstrapEvent {
    pub fn metadata(&self) -> &SessionRenderEvent {
        &self.metadata
    }
    pub fn primary(&self) -> Option<BorrowedFd<'_>> {
        self.primary.as_ref().map(AsFd::as_fd)
    }
    pub fn render(&self) -> Option<BorrowedFd<'_>> {
        self.render.as_ref().map(AsFd::as_fd)
    }
    /// Transfers separately owned capabilities once. Dropping any returned FD
    /// performs descriptor cleanup only, not GPU/vendor-resource retirement.
    pub fn into_parts(self) -> (SessionRenderEvent, Option<OwnedFd>, Option<OwnedFd>) {
        (self.metadata, self.primary, self.render)
    }
}
/// Move-only complete INIT -> ADD* -> ENUMERATION_DONE prefix. No borrowed input
/// survives capture. Later ingress, OOB revoke and real discovery are separate.
#[derive(Debug)]
pub struct OwnedBootstrap {
    session_generation: u64,
    events: Vec<OwnedBootstrapEvent>,
}
impl OwnedBootstrap {
    pub fn capture(
        expected_session_generation: u64,
        events: &[BorrowedBootstrapEvent<'_>],
    ) -> Result<Self, BootstrapError> {
        if events.len() > MAX_BOOTSTRAP_EVENTS {
            return Err(BootstrapError::Capacity);
        }
        if expected_session_generation == 0 || events.len() < 2 {
            return Err(BootstrapError::Invalid);
        }
        let mut descriptors = 0usize;
        for (i, borrowed) in events.iter().enumerate() {
            let value = borrowed.metadata;
            let kind = if i == 0 {
                INIT
            } else if i + 1 == events.len() {
                ENUMERATION_DONE
            } else {
                DEVICE_ADD
            };
            if value.version != EVENT_VERSION
                || value.event_type != kind
                || value.session_generation != expected_session_generation
                || value.session_sequence != (i + 1) as u64
                || value.active > 1
                || value.boot_vga > 1
                || value.reserved.iter().any(|v| *v != 0)
                || !value.seat_name.contains(&0)
                || !value.device_path.contains(&0)
            {
                return Err(BootstrapError::Invalid);
            }
            if kind == DEVICE_ADD {
                if borrowed.primary.is_none()
                    || value.device_lease_id == 0
                    || value.device_generation == 0
                    || value.device_path[0] == 0
                    || events[1..i].iter().any(|prior| {
                        prior.metadata.device_lease_id == value.device_lease_id
                            || prior.metadata.rdev == value.rdev
                    })
                {
                    return Err(BootstrapError::Invalid);
                }
            } else if borrowed.primary.is_some()
                || borrowed.render.is_some()
                || (kind == INIT && value.seat_name[0] == 0)
            {
                return Err(BootstrapError::Invalid);
            }
            descriptors +=
                usize::from(borrowed.primary.is_some()) + usize::from(borrowed.render.is_some());
            if descriptors > MAX_BOOTSTRAP_FDS {
                return Err(BootstrapError::Capacity);
            }
        }
        if events.len() - 2 > MAX_BOOTSTRAP_DEVICES {
            return Err(BootstrapError::Capacity);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(events.len())
            .map_err(|_| BootstrapError::Allocation)?;
        for borrowed in events {
            let primary = borrowed
                .primary
                .map(|fd| duplicate(fd, Some(borrowed.metadata.rdev)))
                .transpose()?;
            // Source2 requires a character render FD/CLOEXEC, not the primary
            // rdev: card and render nodes may have different device numbers.
            let render = borrowed.render.map(|fd| duplicate(fd, None)).transpose()?;
            owned.push(OwnedBootstrapEvent {
                metadata: *borrowed.metadata,
                primary,
                render,
            });
        }
        Ok(Self {
            session_generation: expected_session_generation,
            events: owned,
        })
    }
    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }
    pub fn device_count(&self) -> usize {
        self.events.len() - 2
    }
    pub fn fd_count(&self) -> usize {
        self.events
            .iter()
            .map(|v| usize::from(v.primary.is_some()) + usize::from(v.render.is_some()))
            .sum()
    }
    pub fn events(&self) -> &[OwnedBootstrapEvent] {
        &self.events
    }
    pub fn into_events(self) -> Vec<OwnedBootstrapEvent> {
        self.events
    }
}
unsafe extern "C" {
    fn fcntl(fd: i32, command: i32, ...) -> i32;
}
fn descriptor_error(error: std::io::Error) -> BootstrapError {
    BootstrapError::Descriptor(error.raw_os_error().unwrap_or(0))
}
fn duplicate(fd: BorrowedFd<'_>, primary_rdev: Option<u64>) -> Result<OwnedFd, BootstrapError> {
    let result = unsafe { fcntl(fd.as_raw_fd(), F_DUPFD_CLOEXEC, 3) };
    if result < 0 {
        return Err(descriptor_error(std::io::Error::last_os_error()));
    }
    // Retain RAII custody immediately, before any fallible fd-stat/validation.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(result) });
    let metadata = file.metadata().map_err(descriptor_error)?;
    if !metadata.file_type().is_char_device()
        || primary_rdev.is_some_and(|expected| metadata.rdev() != expected)
    {
        return Err(BootstrapError::Invalid);
    }
    let flags = unsafe { fcntl(file.as_raw_fd(), F_GETFD) };
    if flags < 0 {
        return Err(descriptor_error(std::io::Error::last_os_error()));
    }
    if flags & FD_CLOEXEC == 0 {
        return Err(BootstrapError::Invalid);
    }
    Ok(file.into())
}
