//! One persistent selected backend epoch. Trusted provisioning only: no app IPC.
//! A failed or stopped epoch has no replacement or implicit network cleanup.
use crate::{
    kernel::VirtualLink,
    owner::{self, Action, Authority, Finish, Id, Principal, Reject, Reply},
    service_fact::{Fact, LinkIdentity, decode_record, record},
    startup::{Namespace, Profile},
    wireless_backend::WirelessBackend,
};
use pokelinux_network_state::{NamespaceIdentity, boot_time_ns};
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WorkerCommand, WorkerState},
    deadline::Deadline,
    supervisor::{ProcessIdentity, Supervisor},
};
use rustix::thread::{self, CapabilitySet, CapabilitySets};
use std::{
    fs::File,
    io,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

const LEASE: u64 = 250_000_000;
const ENVELOPE: u64 = 50_000_000;
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn cutoff(ns: u64) -> Deadline {
    Deadline::at(Duration::from_nanos(ns))
}
fn metadata(file: &File) -> io::Result<NamespaceIdentity> {
    let m = file.metadata()?;
    Ok(NamespaceIdentity {
        dev: m.dev(),
        ino: m.ino(),
    })
}

fn open_namespace(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags, fstatfs, open};
    let file = File::from(open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )?);
    // NSFS_MAGIC: reject regular configuration files and other filesystem objects.
    // The virtual backend's kernel operation additionally validates a netns peer.
    if fstatfs(&file)?.f_type as u64 != 0x6e736673 {
        return Err(invalid());
    }
    Ok(file)
}

#[derive(Clone, Copy)]
enum Selected {
    VirtualPair,
    WirelessOpen { ifindex: u32 },
}
impl Selected {
    fn accepts(self, fact: Fact) -> bool {
        match (self, fact.identity) {
            (Self::VirtualPair, LinkIdentity::VirtualPair { .. }) => true,
            (Self::WirelessOpen { ifindex }, LinkIdentity::WirelessOpen { .. }) => {
                fact.ifindex == ifindex
            }
            _ => false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerStart {
    schema: u8,
    profile: Profile,
    namespace: Namespace,
    peer_identity: Option<Namespace>,
    original_start_until_ns: u64,
}

enum Link {
    Virtual(VirtualLink),
    Wireless(WirelessBackend),
}
#[derive(Debug)]
struct BackendFailure<E> {
    operation: &'static str,
    source: E,
}
impl<E: std::fmt::Debug> std::fmt::Display for BackendFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.operation, self.source)
    }
}
impl<E: std::fmt::Debug> std::error::Error for BackendFailure<E> {}
fn backend_result<T, E: std::fmt::Debug + Send + Sync + 'static>(
    operation: &'static str,
    result: Result<T, E>,
) -> io::Result<T> {
    result.map_err(|source| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            BackendFailure { operation, source },
        )
    })
}
impl Link {
    fn virtual_fact(value: crate::kernel::Fact, at: u64) -> (Fact, u64) {
        (
            Fact {
                namespace: value.namespace,
                ifindex: value.ifindex,
                identity: LinkIdentity::VirtualPair {
                    lifetime: value.lifetime,
                },
                connected: value.connected,
            },
            at,
        )
    }
    fn wireless_fact(value: crate::wireless_backend::WirelessFact) -> (Fact, u64) {
        (
            Fact {
                namespace: value.namespace,
                ifindex: value.ifindex,
                identity: LinkIdentity::WirelessOpen {
                    wiphy: value.wiphy,
                    wdev: value.wdev,
                    observed_lifetime: value.observed_lifetime,
                },
                connected: value.connected,
            },
            value.observed_at_ns,
        )
    }
    fn refresh(&mut self, until: u64) -> io::Result<(Fact, u64)> {
        let at = boot_time_ns();
        match self {
            Self::Virtual(link) => backend_result(
                "refresh_virtual",
                link.refresh(until).map(|v| Self::virtual_fact(v, at)),
            ),
            Self::Wireless(link) => backend_result(
                "refresh_wireless",
                link.refresh(until).map(Self::wireless_fact),
            ),
        }
    }
    fn set_connected(&mut self, connected: bool, until: u64) -> io::Result<(Fact, u64)> {
        let at = boot_time_ns();
        match self {
            Self::Virtual(link) => backend_result(
                "apply_virtual",
                link.set_connected(connected, until)
                    .map(|v| Self::virtual_fact(v, at)),
            ),
            Self::Wireless(link) => backend_result(
                "apply_wireless",
                link.set_connected(connected, until)
                    .map(Self::wireless_fact),
            ),
        }
    }
}

/// Run once on the single final-UID/GID main thread, before bootstrap/admission.
/// Bounding/ambient/permitted sets are irreversibly reduced; no launcher remains.
pub(crate) fn reduce(keep: CapabilitySet) -> io::Result<()> {
    if thread::gettid() != rustix::process::getpid()
        || std::fs::read_dir("/proc/self/task")?.count() != 1
    {
        return Err(invalid());
    }
    let last: u32 = std::fs::read_to_string("/proc/sys/kernel/cap_last_cap")?
        .trim()
        .parse()
        .map_err(|_| invalid())?;
    if last >= 64 {
        return Err(invalid());
    }
    thread::set_no_new_privs(true)?;
    thread::clear_ambient_capability_set()?;
    for index in 0..=last {
        let cap = CapabilitySet::from_bits_retain(1_u64 << index);
        if !keep.contains(cap) {
            thread::remove_capability_from_bounding_set(cap)?;
        }
    }
    thread::set_capabilities(
        None,
        CapabilitySets {
            effective: keep,
            permitted: keep,
            inheritable: CapabilitySet::empty(),
        },
    )?;
    let actual = thread::capabilities(None)?;
    if actual.effective != keep
        || actual.permitted != keep
        || !actual.inheritable.is_empty()
        || !thread::no_new_privs()?
    {
        return Err(invalid());
    }
    for index in 0..=last {
        let cap = CapabilitySet::from_bits_retain(1_u64 << index);
        if thread::capability_is_in_bounding_set(cap)? != keep.contains(cap)
            || thread::capability_is_in_ambient_set(cap)?
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub worker: Option<ProcessIdentity>,
    pub ready: bool,
    pub fact: Option<Fact>,
    pub observed_at_ns: Option<u64>,
    pub owner: owner::View,
    pub terminal_uncertain: bool,
    pub stopping: bool,
    pub output_eof: bool,
    pub reaped: bool,
    pub exit_status: Option<ExitStatus>,
    pub main_caps_dropped: bool,
    pub occupied: bool,
    pub namespace_handles_open: bool,
}

pub struct Service {
    supervisor: Supervisor,
    identity: Option<ProcessIdentity>,
    _current: File,
    _peer: Option<File>,
    selected: Selected,
    namespace: NamespaceIdentity,
    owner: owner::Owner,
    observed: Option<(Fact, u64)>,
    handshake: bool,
    terminal: bool,
    stopping: bool,
    output_eof: bool,
    reaped: bool,
    exit_status: Option<ExitStatus>,
    main_caps_dropped: bool,
    start_until: u64,
}
impl Service {
    /// Trusted startup for one exclusively administered selected backend. Once
    /// spawning is attempted, even a failed startup returns an owned terminal
    /// handle: caller must Stop/poll through closure. If main_caps_dropped is
    /// false, the caller must then exit this process. No clients are installed.
    pub fn start(
        executable: &Path,
        mut profile: Profile,
        journal: PathBuf,
        original_start_until: u64,
    ) -> io::Result<Self> {
        let now = boot_time_ns();
        if original_start_until <= now || original_start_until - now > 3_000_000_000 {
            return Err(invalid());
        }
        profile.validate()?;
        let current = open_namespace(Path::new("/proc/thread-self/ns/net"))?;
        let namespace = metadata(&current)?;
        let (peer, peer_identity, selected) = match &mut profile {
            Profile::VirtualPair { peer_namespace } => {
                let file = open_namespace(peer_namespace)?;
                let other = metadata(&file)?;
                if namespace == other {
                    return Err(invalid());
                }
                *peer_namespace = PathBuf::from(format!(
                    "/proc/{}/fd/{}",
                    std::process::id(),
                    file.as_raw_fd()
                ));
                (
                    Some(file),
                    Some(Namespace {
                        dev: other.dev,
                        ino: other.ino,
                    }),
                    Selected::VirtualPair,
                )
            }
            Profile::WirelessOpen { target, .. } => {
                if (target.namespace.dev, target.namespace.ino) != (namespace.dev, namespace.ino) {
                    return Err(invalid());
                }
                (
                    None,
                    None,
                    Selected::WirelessOpen {
                        ifindex: target.ifindex,
                    },
                )
            }
        };
        let startup = WorkerStart {
            schema: 2,
            profile,
            namespace: Namespace {
                dev: namespace.dev,
                ino: namespace.ino,
            },
            peer_identity,
            original_start_until_ns: original_start_until,
        };
        let mut command = WorkerCommand::new(executable)?;
        command.arg(serde_json::to_string(&startup)?);
        let mut service = Self {
            supervisor: Supervisor::new(journal),
            identity: None,
            _current: current,
            _peer: peer,
            selected,
            namespace,
            owner: owner::Owner::default(),
            observed: None,
            handshake: false,
            terminal: false,
            stopping: false,
            output_eof: false,
            reaped: false,
            exit_status: None,
            main_caps_dropped: false,
            start_until: original_start_until,
        };
        let launched = service.supervisor.start_worker(&mut command);
        match launched {
            Ok(id) => service.identity = Some(id),
            Err(_) => {
                service.terminal = true;
            }
        }
        service.main_caps_dropped = reduce(CapabilitySet::empty()).is_ok();
        if !service.main_caps_dropped {
            service.terminal = true;
        }
        if service.terminal {
            let _ = service.stop();
        }
        Ok(service)
    }
    pub fn occupied(&self) -> bool {
        self.supervisor.occupied()
    }
    /// Diagnostic only; signaling tests must retain their own actual pidfd.
    pub fn child_pid(&self) -> Option<u32> {
        self.supervisor.child_pid()
    }
    fn expired(&mut self, now: u64) {
        if self.handshake
            && !self
                .identity
                .is_some_and(|identity| self.supervisor.admits(identity))
        {
            self.terminal = true;
            self.observed = None;
        }
        if self.owner.view(now).pending.is_some_and(|p| {
            p.phase == owner::Phase::Dispatched && now >= p.command.original_until_ns
        }) {
            self.terminal = true;
        }
    }
    /// Only bounded local model work and CLOCK_BOOTTIME; no backend I/O.
    pub fn snapshot(&mut self) -> Snapshot {
        let now = boot_time_ns();
        self.expired(now);
        let fresh = self
            .observed
            .filter(|(_, at)| now >= *at && now - *at < LEASE);
        let mut visible_owner = self.owner.view(now);
        visible_owner.observed = visible_owner.observed.filter(|observation| {
            !self.stopping
                && fresh.is_some_and(|(fact, at)| {
                    observation.observed_at_ns == at && observation.connected == fact.connected
                })
        });
        visible_owner.terminal_uncertain |= self.terminal;
        Snapshot {
            worker: self.identity,
            ready: self.handshake && fresh.is_some() && !self.terminal && !self.stopping,
            fact: if self.stopping {
                None
            } else {
                fresh.map(|(f, _)| f)
            },
            observed_at_ns: if self.stopping {
                None
            } else {
                fresh.map(|(_, at)| at)
            },
            owner: visible_owner,
            terminal_uncertain: self.terminal,
            stopping: self.stopping,
            output_eof: self.output_eof,
            reaped: self.reaped,
            exit_status: self.exit_status,
            main_caps_dropped: self.main_caps_dropped,
            occupied: self.occupied(),
            namespace_handles_open: true,
        }
    }
    pub fn submit(
        &mut self,
        authority: Authority,
        action: Action,
        original_until: u64,
    ) -> Result<Id, Reject> {
        let s = self.snapshot();
        if self.terminal {
            return Err(Reject::Uncertain);
        }
        if !s.ready {
            return Err(Reject::Unavailable);
        }
        let now = boot_time_ns();
        let id = self.owner.stage(authority, action, original_until, now)?;
        let command = self.owner.dispatch(id, authority, boot_time_ns())?;
        let mut bytes = [0_u8; 19];
        bytes[0] = 2;
        bytes[1] = 1;
        bytes[2..10].copy_from_slice(&id.0.to_le_bytes());
        bytes[10..18].copy_from_slice(&command.original_until_ns.to_le_bytes());
        bytes[18] = u8::from(action == Action::Connect);
        if self
            .supervisor
            .send_message(
                self.identity.ok_or(Reject::Unavailable)?,
                cutoff(original_until),
                &bytes,
            )
            .is_err()
        {
            self.terminal = true;
            let _ = self.stop();
            return Err(Reject::Uncertain);
        }
        Ok(id)
    }
    pub fn withdraw(&mut self, principal: Principal) {
        self.owner.withdraw(principal, boot_time_ns());
    }
    pub fn take_reply(&mut self, id: Id, authority: Authority) -> Option<Reply> {
        let now = boot_time_ns();
        self.expired(now);
        if self.terminal || self.stopping {
            None
        } else {
            self.owner.take_reply(id, authority, now)
        }
    }
    /// Revocation precedes the signal. Retains child and selected namespace handles;
    /// this call does not promise the kernel will make the worker reclaimable.
    pub fn stop(&mut self) -> io::Result<()> {
        self.stopping = true;
        self.terminal = true;
        self.observed = None;
        self.supervisor.stop()
    }
    pub fn poll(&mut self) -> io::Result<()> {
        let now = boot_time_ns();
        self.expired(now);
        if !self.stopping && !self.handshake {
            if now >= self.start_until {
                let _ = self.stop();
            } else {
                match self.supervisor.poll_worker() {
                    Ok(WorkerState::Ready) => self.handshake = true,
                    Ok(WorkerState::Initializing) => {}
                    _ => {
                        let _ = self.stop();
                    }
                }
            }
        }
        if self.handshake && !self.stopping {
            let result = self.supervisor.receive_message(
                self.identity.ok_or_else(invalid)?,
                cutoff(now.saturating_add(ENVELOPE)),
            );
            match result {
                Ok(Some(bytes)) => {
                    if self.accept(&bytes).is_err() {
                        let _ = self.stop();
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    let _ = self.stop();
                }
            }
        }
        if self.occupied() && !self.output_eof {
            let mut buffer = [0_u8; 1024];
            match self.supervisor.read_output(&mut buffer) {
                Ok(0) => self.output_eof = true,
                Ok(_) => {
                    let _ = self.stop();
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    let _ = self.stop();
                    return Err(e);
                }
            }
        }
        // The shared owner revokes at actual wait and returns Some only after
        // an actual nonempty-buffer EOF witness. A held writer keeps custody.
        if self.occupied()
            && let Some(status) = self.supervisor.poll_reap()?
        {
            self.output_eof = true;
            self.exit_status = Some(status);
            self.reaped = true;
            self.terminal = true;
            self.handshake = false;
            self.observed = None;
            if let Some(pending) = self.owner.view(boot_time_ns()).pending {
                self.owner
                    .finish(pending.command.id, boot_time_ns(), Finish::Uncertain);
            }
        }
        Ok(())
    }
    fn accept(&mut self, bytes: &[u8]) -> io::Result<()> {
        let (_, id, fact, at) = decode_record(bytes)?;
        let id = Id(id);
        let now = boot_time_ns();
        if at > now
            || now - at >= LEASE
            || fact.namespace != self.namespace
            || !self.selected.accepts(fact)
        {
            return Err(invalid());
        }
        if let Some((old, old_at)) = self.observed
            && (!old.same_target(fact) || at < old_at)
        {
            return Err(invalid());
        }
        if bytes[1] == 1 {
            if id.0 != 0 {
                return Err(invalid());
            }
            self.observed = Some((fact, at));
            self.owner.observe(fact.connected, at);
        } else {
            let pending = self.owner.view(now).pending.ok_or_else(invalid)?;
            if pending.command.id != id {
                return Err(invalid());
            }
            let outcome = match bytes[1] {
                2 => Finish::Observed(fact.connected),
                3 => Finish::DefinitelyUnsent,
                _ => Finish::Uncertain,
            };
            if bytes[1] == 2
                && (now >= pending.command.original_until_ns
                    || at >= pending.command.original_until_ns
                    || fact.connected != (pending.command.action == Action::Connect))
            {
                self.terminal = true;
            }
            if !self.owner.finish(
                id,
                at,
                if self.terminal {
                    Finish::Uncertain
                } else {
                    outcome
                },
            ) {
                return Err(invalid());
            }
            if bytes[1] == 2 {
                self.observed = Some((fact, at));
            }
            if bytes[1] == 4 {
                self.terminal = true;
                self.observed = None;
            }
        }
        Ok(())
    }
}
/// Error-only context; it grants no retry and changes no operation cutoff.
#[derive(Debug)]
enum WorkerStep {
    ReduceCapabilities,
    BindParent,
    PublishInitial,
    CheckCommandReady,
    ReceiveCommand,
    PublishRetirement,
    PublishCompletion,
    PublishObservation,
    RefreshBackend,
    ApplyBackend,
}
#[derive(Debug)]
struct WorkerFailure {
    stage: WorkerStep,
    original_until_ns: Option<u64>,
    detected_at_ns: u64,
    source: io::Error,
}
impl std::fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} at {} with original cutoff {:?}: {}",
            self.stage, self.detected_at_ns, self.original_until_ns, self.source
        )
    }
}
impl std::error::Error for WorkerFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
fn worker_step<T>(
    stage: WorkerStep,
    original_until_ns: Option<u64>,
    result: io::Result<T>,
) -> io::Result<T> {
    result.map_err(|source| {
        io::Error::new(
            source.kind(),
            WorkerFailure {
                stage,
                original_until_ns,
                detected_at_ns: boot_time_ns(),
                source,
            },
        )
    })
}
/// Fixed cooperative worker entry. Arguments originate only in trusted startup,
/// then the authenticated actual parent and exact retained namespace are checked.
pub fn worker(arguments: &[String]) -> io::Result<()> {
    if arguments.len() != 1 || arguments[0].len() > 8192 {
        return Err(invalid());
    }
    worker_step(
        WorkerStep::ReduceCapabilities,
        None,
        reduce(CapabilitySet::NET_ADMIN),
    )?;
    let mut parent = worker_step(WorkerStep::BindParent, None, BoundParent::from_stdin())?;
    let startup: WorkerStart = serde_json::from_str(&arguments[0])?;
    if startup.schema != 2 {
        return Err(invalid());
    }
    startup.profile.validate()?;
    let current = open_namespace(Path::new("/proc/thread-self/ns/net"))?;
    let actual = metadata(&current)?;
    if (actual.dev, actual.ino) != (startup.namespace.dev, startup.namespace.ino) {
        return Err(invalid());
    }
    let until = startup.original_start_until_ns;
    let now = boot_time_ns();
    if until <= now || until - now > 3_000_000_000 {
        return Err(invalid());
    }
    let backend_until = until.min(now.saturating_add(1_000_000_000));
    let mut link = match startup.profile {
        Profile::VirtualPair { peer_namespace } => {
            let prefix = format!(
                "/proc/{}/fd/",
                rustix::process::getppid().ok_or_else(invalid)?
            );
            let path = peer_namespace.to_str().ok_or_else(invalid)?;
            let tail = path.strip_prefix(&prefix).ok_or_else(invalid)?;
            if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            let peer = open_namespace(&peer_namespace)?;
            let expected = startup.peer_identity.ok_or_else(invalid)?;
            let actual = metadata(&peer)?;
            if (actual.dev, actual.ino) != (expected.dev, expected.ino) {
                return Err(invalid());
            }
            Link::Virtual(backend_result(
                "attach_virtual",
                VirtualLink::create(peer, backend_until),
            )?)
        }
        Profile::WirelessOpen {
            target,
            frequency_mhz,
            bssid,
            ssid,
        } => {
            if startup.peer_identity.is_some() || target.namespace != startup.namespace {
                return Err(invalid());
            }
            let target = crate::fixed_address::ExistingLinkIdentity {
                namespace: actual,
                ifindex: target.ifindex,
                name: target.name,
            };
            Link::Wireless(backend_result(
                "attach_wireless",
                WirelessBackend::attach(target, frequency_mhz, bssid, ssid, backend_until),
            )?)
        }
    };
    let (mut fact, mut refreshed_at) = worker_step(
        WorkerStep::RefreshBackend,
        Some(backend_until),
        link.refresh(backend_until),
    )?;
    worker_step(
        WorkerStep::PublishInitial,
        Some(until),
        parent.send_message(cutoff(until), &record(1, 0, fact, refreshed_at)),
    )?;
    let mut last_seen_id = 0;
    loop {
        if worker_step(WorkerStep::CheckCommandReady, None, parent.message_ready())? {
            let envelope = boot_time_ns().saturating_add(ENVELOPE);
            if let Some(bytes) = worker_step(
                WorkerStep::ReceiveCommand,
                Some(envelope),
                parent.receive_message(cutoff(envelope)),
            )? {
                if bytes.len() != 19 || bytes[0..2] != [2, 1] || bytes[18] > 1 {
                    return Err(invalid());
                }
                let id = u64::from_le_bytes(bytes[2..10].try_into().unwrap());
                if id == 0 || id <= last_seen_id {
                    return Err(invalid());
                }
                last_seen_id = id;
                let original_until = u64::from_le_bytes(bytes[10..18].try_into().unwrap());
                let sampled = boot_time_ns();
                if sampled >= original_until {
                    // Retirement receipt uses a separate transport envelope; it
                    // cannot renew or publish success for the expired operation.
                    let retirement_until = sampled.saturating_add(ENVELOPE);
                    worker_step(
                        WorkerStep::PublishRetirement,
                        Some(retirement_until),
                        parent
                            .send_message(cutoff(retirement_until), &record(3, id, fact, sampled)),
                    )?;
                } else {
                    if original_until - sampled > 1_000_000_000 {
                        return Err(invalid());
                    }
                    (fact, refreshed_at) = worker_step(
                        WorkerStep::ApplyBackend,
                        Some(original_until),
                        link.set_connected(bytes[18] != 0, original_until),
                    )?;
                    worker_step(
                        WorkerStep::PublishCompletion,
                        Some(original_until),
                        parent.send_message(
                            cutoff(original_until),
                            &record(2, id, fact, refreshed_at),
                        ),
                    )?;
                }
            }
        }
        let sampled = boot_time_ns();
        if sampled.saturating_sub(refreshed_at) >= 50_000_000 {
            let read_until = sampled.saturating_add(100_000_000);
            (fact, refreshed_at) = worker_step(
                WorkerStep::RefreshBackend,
                Some(read_until),
                link.refresh(read_until),
            )?;
            worker_step(
                WorkerStep::PublishObservation,
                Some(read_until),
                parent.send_message(cutoff(read_until), &record(1, 0, fact, refreshed_at)),
            )?;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "service_wire_tests.rs"]
mod wire_tests;
