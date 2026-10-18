//! Final single-threaded root Main; only a protected trusted-root endpoint.
//! Transferred mutations outlive requester loss; no implicit Disconnect or retry.
use crate::{
    owner::{Action, Authority, Finish, Id, Principal, Reject},
    private_channel,
    protocol::{Choice, LEASE, Reply, Request, State, Status},
    service::{Service, Snapshot},
};
use pokelinux_kernel_identity::{TerminationGuard, termination_requested};
use pokelinux_network_state::boot_time_ns;
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType, UCred},
};
use std::{
    fs,
    io::{self, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::ExitStatusExt,
    },
    path::Path,
    time::Duration,
};
fn listener(path: &Path) -> io::Result<OwnedFd> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let m = fs::symlink_metadata(parent)?;
    if !path.is_absolute()
        || !m.is_dir()
        || m.uid() != 0
        || m.gid() != 0
        || m.mode() & 0o777 != 0o700
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    net::bind(&fd, &SocketAddrUnix::new(path)?)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    net::listen(&fd, 8)?;
    Ok(fd)
}
fn state(s: Snapshot) -> State {
    let visible = s.ready && !s.terminal_uncertain && !s.stopping;
    State {
        available: visible,
        configured: if visible {
            s.fact.map(|f| f.connected)
        } else {
            None
        },
        observed_at_ns: if visible {
            s.observed_at_ns.unwrap_or(0)
        } else {
            0
        }
        .to_string(),
        until_ns: if visible {
            s.observed_at_ns.unwrap_or(0).saturating_add(LEASE)
        } else {
            0
        }
        .to_string(),
        pending: s.owner.pending.is_some(),
        terminal_uncertain: s.terminal_uncertain,
        worker: s.worker.map(|id| id.hex()).unwrap_or_default(),
    }
}
struct Flight {
    operation: Id,
    request: u64,
    until: u64,
    choice: Choice,
}
pub(crate) struct Peer {
    fd: OwnedFd,
    cred: UCred,
    serial: u64,
    last_id: u64,
    flight: Option<Flight>,
}
impl Peer {
    pub(crate) fn new(fd: OwnedFd, serial: u64) -> io::Result<Self> {
        let cred = private_channel::peer(&fd)?;
        Ok(Self {
            fd,
            cred,
            serial,
            last_id: 0,
            flight: None,
        })
    }
    fn authority(&self) -> Authority {
        // This local epoch is a trusted-root transport lease, never an app grant.
        Authority {
            principal: Principal {
                instance: self.serial,
                generation: 1,
            },
            permission_epoch: 1,
            allowed: true,
        }
    }
}
fn reply(peer: &Peer, id: u64, status: Status, s: &mut Service) -> io::Result<()> {
    let state = state(s.snapshot());
    let status = if !state.available && matches!(status, Status::Observed | Status::Completed) {
        Status::Unavailable
    } else {
        status
    };
    private_channel::send(
        &peer.fd,
        &serde_json::to_vec(&Reply {
            schema: 1,
            id,
            status,
            state,
        })?,
    )
}
/// One bounded private-client turn, shared by the normal Main and socket-boundary tests.
pub(crate) fn clients_turn(
    socket: &OwnedFd,
    service: &mut Service,
    peers: &mut Vec<Peer>,
    serial: &mut u64,
    fatal: &mut Option<io::Error>,
    stopping: &mut bool,
) {
    for _ in 0..8 {
        match net::accept_with(socket, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
            Ok(fd) => {
                if peers.len() >= 8 {
                    continue;
                }
                if let Some(next) = serial.checked_add(1)
                    && let Ok(peer) = Peer::new(fd, next)
                {
                    *serial = next;
                    peers.push(peer);
                }
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => break,
            Err(e) => {
                fatal.get_or_insert(e.into());
                *stopping = true;
                break;
            }
        }
    }
    let mut n = 0;
    while !*stopping && n < peers.len() {
        if termination_requested() {
            *stopping = true;
            break;
        }
        // Do not overwrite an uncollected result with another effect.
        let any_flight = peers.iter().any(|p| p.flight.is_some());
        let mut remove = match private_channel::receive(&peers[n].fd, &peers[n].cred) {
            Ok(None) => false,
            Err(_) => true,
            Ok(Some(bytes)) => {
                let request = serde_json::from_slice::<Request>(&bytes)
                    .ok()
                    .filter(|r| r.id() > peers[n].last_id && r.until(boot_time_ns()).is_some());
                if let Some(request) = request {
                    if peers[n].flight.is_some() {
                        true
                    } else {
                        peers[n].last_id = request.id();
                        match request.choice() {
                            None => {
                                reply(&peers[n], request.id(), Status::Observed, service).is_err()
                            }
                            Some(_) if any_flight => {
                                reply(&peers[n], request.id(), Status::Refused, service).is_err()
                            }
                            Some(choice) => {
                                let until = request.until(boot_time_ns());
                                if let Some(until) = until {
                                    let action = if choice.connected() {
                                        Action::Connect
                                    } else {
                                        Action::Disconnect
                                    };
                                    match service.submit(peers[n].authority(), action, until) {
                                        Ok(operation) => {
                                            peers[n].flight = Some(Flight {
                                                operation,
                                                request: request.id(),
                                                until,
                                                choice,
                                            });
                                            false
                                        }
                                        Err(reject) => {
                                            let status = if reject == Reject::Uncertain {
                                                Status::Unavailable
                                            } else {
                                                Status::Refused
                                            };
                                            reply(&peers[n], request.id(), status, service).is_err()
                                        }
                                    }
                                } else {
                                    true
                                }
                            }
                        }
                    }
                } else {
                    true
                }
            }
        };
        // Request-half EOF/extra input above fences publication first.
        // After a full Apply transfer, removing this peer withdraws its
        // requester only; Service retains completion responsibility.
        if !remove && let Some(flight) = peers[n].flight.as_ref() {
            if boot_time_ns() >= flight.until {
                remove = true;
            } else if let Some(done) = service.take_reply(flight.operation, peers[n].authority()) {
                let status = match done.outcome {
                    Finish::Observed(connected) if connected == flight.choice.connected() => {
                        Status::Completed
                    }
                    Finish::DefinitelyUnsent => Status::Refused,
                    _ => Status::Unavailable,
                };
                remove = reply(&peers[n], flight.request, status, service).is_err();
                peers[n].flight = None;
            }
        }
        if remove {
            let p = peers.remove(n);
            service.withdraw(crate::owner::Principal {
                instance: p.serial,
                generation: 1,
            });
        } else {
            n += 1;
        }
    }
}

/// Trusted root startup only. No app identity, grant, interface or backend selector is accepted over the socket.
pub fn run(
    path: &Path,
    peer_namespace: &Path,
    journal: &Path,
    worker: &Path,
    original_start_until: u64,
) -> io::Result<()> {
    if [
        rustix::process::getuid().as_raw(),
        rustix::process::geteuid().as_raw(),
        rustix::process::getgid().as_raw(),
        rustix::process::getegid().as_raw(),
    ] != [0; 4]
        || !peer_namespace.is_absolute()
        || !journal.is_absolute()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let _termination = TerminationGuard::install()?;
    let socket = listener(path)?;
    let mut service = Service::start(
        worker,
        fs::File::open(peer_namespace)?,
        journal.into(),
        original_start_until,
    )?;
    let mut peers = Vec::<Peer>::new();
    let mut serial = 0u64;
    let mut fatal = None;
    let mut stopping = !service.snapshot().main_caps_dropped;
    loop {
        if termination_requested() || fatal.is_some() {
            stopping = true;
        }
        if stopping {
            for p in peers.drain(..) {
                service.withdraw(crate::owner::Principal {
                    instance: p.serial,
                    generation: 1,
                });
            }
            if let Err(e) = service.stop() {
                fatal.get_or_insert(e);
            }
        }
        if let Err(e) = service.poll() {
            fatal.get_or_insert(e);
            stopping = true;
        }
        if stopping && !service.occupied() {
            break;
        }
        if !stopping {
            clients_turn(
                &socket,
                &mut service,
                &mut peers,
                &mut serial,
                &mut fatal,
                &mut stopping,
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let s = service.snapshot();
    let closed = s.reaped && s.output_eof && !s.occupied;
    let receipt = serde_json::json!({"event":"connectivity_main_closure","closed":closed,"reaped":s.reaped,"output_eof":s.output_eof,"occupied":s.occupied,"namespace_handles_open":s.namespace_handles_open,"worker_exit_code":s.exit_status.and_then(|s|s.code()),"worker_exit_signal":s.exit_status.and_then(|s|s.signal()),"completed_at_ns":boot_time_ns()});
    let emitted = writeln!(io::stdout().lock(), "{receipt}");
    let _ = fs::remove_file(path);
    emitted?;
    if let Some(e) = fatal {
        Err(e)
    } else if closed {
        Ok(())
    } else {
        Err(io::ErrorKind::Other.into())
    }
}
