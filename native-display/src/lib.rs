#![forbid(unsafe_code)]
//! Owned native Wayland security-context setup. Metadata is not app permission.
//! This library consumes an established source socket and a prepared listener;
//! its caller must qualify protected procfs/namespaces and listener filesystem ACLs.
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use rustix::{event, fd::OwnedFd, net, pipe, process::Pid};
use std::{io, os::fd::AsFd, os::unix::net::UnixStream};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, delegate_noop,
    protocol::{wl_callback, wl_registry},
};
use wayland_protocols::wp::security_context::v1::client::{
    wp_security_context_manager_v1::WpSecurityContextManagerV1,
    wp_security_context_v1::WpSecurityContextV1,
};

pub mod supervisor;

const MAX_GLOBALS: usize = 256;
pub const MAX_INITIALIZATION_NS: u64 = 5_000_000_000;

pub fn boottime_ns() -> u64 {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(time.tv_sec)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|seconds| seconds.checked_add(time.tv_nsec as u64))
        .unwrap_or(u64::MAX)
}
fn living(fd: &OwnedFd) -> io::Result<bool> {
    let mut watched = [event::PollFd::new(fd, event::PollFlags::IN)];
    event::poll(
        &mut watched,
        Some(&rustix::time::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    Ok(watched[0].revents().is_empty())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Initializing,
    Ready,
    Failed,
    Stopped,
}
#[derive(Clone, Copy)]
pub struct SourceIdentity {
    pub pid: Pid,
    pub uid: u32,
    pub gid: u32,
}
pub struct Metadata {
    pub app: String,
    pub instance: String,
}
impl Metadata {
    fn valid(&self) -> bool {
        !self.app.is_empty()
            && self.app.len() <= 128
            && self.app.contains('.')
            && self
                .app
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
            && self.instance.len() == 32
            && self
                .instance
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

#[derive(Default)]
struct Setup {
    registry_complete: bool,
    commit_complete: bool,
    manager_name: Option<u32>,
    globals: usize,
    invalid: bool,
}
impl Dispatch<wl_registry::WlRegistry, ()> for Setup {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                state.globals += 1;
                if state.globals > MAX_GLOBALS || interface.len() > 256 || version == 0 {
                    state.invalid = true;
                }
                if interface == "wp_security_context_manager_v1" {
                    if state.manager_name.is_some() {
                        state.invalid = true;
                    }
                    state.manager_name = Some(name);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if state.manager_name == Some(name) {
                    state.invalid = true;
                }
            }
            _ => state.invalid = true,
        }
    }
}
#[derive(Clone, Copy)]
enum Barrier {
    Registry,
    Commit,
}
impl Dispatch<wl_callback::WlCallback, Barrier> for Setup {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        barrier: &Barrier,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if !matches!(event, wl_callback::Event::Done { .. }) {
            state.invalid = true;
            return;
        }
        match barrier {
            Barrier::Registry => state.registry_complete = true,
            Barrier::Commit => state.commit_complete = true,
        }
    }
}
delegate_noop!(Setup: ignore WpSecurityContextManagerV1);
delegate_noop!(Setup: ignore WpSecurityContextV1);

struct Wire {
    connection: Connection,
    queue: EventQueue<Setup>,
    registry: wl_registry::WlRegistry,
    setup: Setup,
    installing: bool,
}
/// Protocol driver for an owned resource-limited child, not the root actor.
/// The pinned SDK can drain unbounded input inside read/dispatch; external
/// BOOTTIME checks cannot interrupt that work. A responsive parent supervisor
/// must fence launch, terminate and actually reap the child on timeout/stop.
/// No background thread, blocking roundtrip, proxy client or implicit retry.
pub struct Context {
    wire: Option<Wire>,
    source: OwnedFd,
    listener: Option<OwnedFd>,
    close_read: Option<OwnedFd>,
    close_signal: Option<OwnedFd>,
    metadata: Metadata,
    until: u64,
    phase: Phase,
}
impl Context {
    /// Consumes the listener; after transfer it is used only for closure.
    /// The fixed pure-Rust Wayland backend owns the sole queue/read guard.
    pub fn new(
        stream: UnixStream,
        expected: &OwnedFd,
        identity: SourceIdentity,
        listener: OwnedFd,
        metadata: Metadata,
        until: u64,
    ) -> io::Result<Self> {
        let now = boottime_ns();
        if rustix::process::getuid().as_raw() != 0
            || !metadata.valid()
            || until <= now
            || until - now > MAX_INITIALIZATION_NS
            || net::sockopt::socket_domain(&listener)? != net::AddressFamily::UNIX
            || net::sockopt::socket_type(&listener)? != net::SocketType::STREAM
            || !net::sockopt::socket_acceptconn(&listener)?
            || !rustix::io::fcntl_getfd(&listener)?.contains(rustix::io::FdFlags::CLOEXEC)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let flags = rustix::io::fcntl_getfd(&stream)?;
        rustix::io::fcntl_setfd(&stream, flags | rustix::io::FdFlags::CLOEXEC)?;
        let credentials = net::sockopt::socket_peercred(&stream)?;
        let source = socket_peer_pidfd(&stream)?;
        let view = ProcessView::open()?;
        if credentials.pid != identity.pid
            || credentials.uid.as_raw() != identity.uid
            || credentials.gid.as_raw() != identity.gid
            || !view.matches(expected, &source, identity.pid.as_raw_pid() as u32)?
            || boottime_ns() >= until
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        stream.set_nonblocking(true)?;
        let connection = Connection::from_socket(stream).map_err(io::Error::other)?;
        let queue = connection.new_event_queue();
        let handle = queue.handle();
        let registry = connection.display().get_registry(&handle, ());
        connection.display().sync(&handle, Barrier::Registry);
        let (close_read, close_signal) =
            pipe::pipe_with(pipe::PipeFlags::CLOEXEC | pipe::PipeFlags::NONBLOCK)?;
        Ok(Self {
            wire: Some(Wire {
                connection,
                queue,
                registry,
                setup: Setup::default(),
                installing: false,
            }),
            source,
            listener: Some(listener),
            close_read: Some(close_read),
            close_signal: Some(close_signal),
            metadata,
            until,
            phase: Phase::Initializing,
        })
    }
    pub fn snapshot(&self) -> Phase {
        if matches!(self.phase, Phase::Ready | Phase::Initializing)
            && (!living(&self.source).unwrap_or(false)
                || (self.phase == Phase::Initializing && boottime_ns() >= self.until))
        {
            Phase::Failed
        } else {
            self.phase
        }
    }
    fn release(&mut self, phase: Phase) {
        self.phase = phase;
        self.wire = None;
        self.listener = None;
        self.close_read = None;
        self.close_signal = None;
    }
    pub fn stop(&mut self) {
        self.release(Phase::Stopped);
    }
    pub fn poll(&mut self) -> io::Result<Phase> {
        let result = self.advance();
        if result.is_err() {
            self.release(Phase::Failed);
        }
        result
    }
    fn advance(&mut self) -> io::Result<Phase> {
        if matches!(self.phase, Phase::Stopped | Phase::Failed) {
            return Ok(self.phase);
        }
        if !living(&self.source)? {
            return Err(io::ErrorKind::NotConnected.into());
        }
        if self.phase == Phase::Ready {
            return Ok(self.phase);
        }
        if boottime_ns() >= self.until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let wire = self.wire.as_mut().ok_or(io::ErrorKind::NotConnected)?;
        match wire.connection.flush() {
            Ok(()) => {}
            Err(wayland_client::backend::WaylandError::Io(e))
                if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(io::Error::other(e)),
        }
        wire.queue
            .dispatch_pending(&mut wire.setup)
            .map_err(io::Error::other)?;
        if let Some(read) = wire.queue.prepare_read() {
            match read.read() {
                Ok(_) => {}
                Err(wayland_client::backend::WaylandError::Io(e))
                    if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(io::Error::other(e)),
            }
        }
        wire.queue
            .dispatch_pending(&mut wire.setup)
            .map_err(io::Error::other)?;
        if wire.setup.invalid {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if boottime_ns() >= self.until || !living(&self.source)? {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if wire.setup.registry_complete && !wire.installing {
            let name = wire.setup.manager_name.ok_or(io::ErrorKind::Unsupported)?;
            let handle = wire.queue.handle();
            let manager: WpSecurityContextManagerV1 = wire.registry.bind(name, 1, &handle, ());
            let context = manager.create_listener(
                self.listener
                    .as_ref()
                    .ok_or(io::ErrorKind::NotConnected)?
                    .as_fd(),
                self.close_read
                    .as_ref()
                    .ok_or(io::ErrorKind::NotConnected)?
                    .as_fd(),
                &handle,
                (),
            );
            context.set_sandbox_engine("org.pokelinux.native".into());
            context.set_app_id(self.metadata.app.clone());
            context.set_instance_id(self.metadata.instance.clone());
            context.commit();
            context.destroy();
            manager.destroy();
            wire.connection.display().sync(&handle, Barrier::Commit);
            wire.installing = true;
        }
        if wire.setup.commit_complete {
            // Ordered original-server sync confirms prior setup handlers. The
            // protocol preserves its listener across creator disconnect. The
            // retained pipe writer, not this transport, owns future admission.
            self.phase = Phase::Ready;
            self.wire = None;
            self.listener = None;
            self.close_read = None;
        }
        Ok(self.phase)
    }
}
