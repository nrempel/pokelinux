//! Private root lifecycle actor. Backend IO is owned by one retained child.
//! Trusted single-threaded exec requires normal SIGCHLD/wait semantics; the
//! termination guard separately refuses unusable inherited SIGTERM state.
#![forbid(unsafe_code)]
use pokelinux_brightness::{
    Backend, BackendKind,
    model::{Action, Owner},
    protocol::{LIMIT, Reply, Request, decimal},
    worker::{Worker, child_check, child_setup, encode_observation},
};
use pokelinux_kernel_identity::{TerminationGuard, termination_requested};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
    time::{ClockId, clock_gettime},
};
use std::{
    fs,
    io::{self, IoSliceMut, Write},
    mem::MaybeUninit,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
fn now() -> u64 {
    let value = clock_gettime(ClockId::Boottime);
    (value.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(value.tv_nsec as u64)
}
fn backend_kind(kind: &str) -> io::Result<BackendKind> {
    match kind {
        "linux-backlight" => Ok(BackendKind::LinuxBacklight),
        "synthetic-tree" => Ok(BackendKind::SyntheticTree),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
fn worker_entry(
    parent: &str,
    until: &str,
    kind: &str,
    root: &str,
    device: &str,
    action: &str,
    percent: Option<&str>,
) -> io::Result<()> {
    let parent = parent
        .parse::<u32>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let until = decimal(until).ok_or(io::ErrorKind::InvalidInput)?;
    child_setup(parent, until)?;
    let backend = Backend::new(PathBuf::from(root), device.to_owned(), backend_kind(kind)?)?;
    child_check(parent, until)?;
    let observation = match (action, percent) {
        ("observe", None) => backend.read()?,
        ("set", Some(text)) => {
            let percent = text
                .parse::<u8>()
                .ok()
                .filter(|v| *v <= 100)
                .ok_or(io::ErrorKind::InvalidInput)?;
            backend.set_percent_before(percent, until)?.observed
        }
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    };
    let bytes = encode_observation(observation)?;
    child_check(parent, until)?;
    std::io::stdout().lock().write_all(bytes.as_bytes())
}
fn listener(path: &Path) -> io::Result<OwnedFd> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let metadata = fs::symlink_metadata(parent)?;
    if !path.is_absolute()
        || !metadata.is_dir()
        || metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.permissions().mode() & 0o777 != 0o700
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
    if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .and_then(|()| net::listen(&fd, 16).map_err(Into::into))
    {
        drop(fd);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(fd)
}
struct Peer {
    fd: OwnedFd,
    serial: u64,
    credentials: UCred,
}
struct Flight {
    worker: Worker,
    peer: Option<u64>,
    request: u64,
}
fn receive(peer: &Peer) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = [0; LIMIT];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        &peer.fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(None),
        result => result?,
    };
    let mut credential = None;
    let mut invalid = message.bytes == 0
        || message
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if credential.is_none() => {
                credential = Some(value)
            }
            _ => invalid = true,
        }
    }
    if invalid {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let credential = credential.ok_or(io::ErrorKind::PermissionDenied)?;
    if credential.pid != peer.credentials.pid
        || credential.uid != peer.credentials.uid
        || credential.gid != peer.credentials.gid
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(Some(bytes[..message.bytes].to_vec()))
}
fn send(peer: &Peer, reply: &Reply) -> io::Result<()> {
    let bytes = serde_json::to_vec(reply)?;
    if bytes.is_empty() || bytes.len() > LIMIT {
        return Err(io::ErrorKind::InvalidData.into());
    }
    if net::send(&peer.fd, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)? != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
fn snapshot(owner: &mut Owner, id: u64, ok: bool, percent: Option<u8>) -> Reply {
    let mut view = owner.view(now());
    // A busy slot is unavailable, even if an older readonly value still exists
    // internally. It cannot authorize a second Set or imply completion.
    if view.pending {
        view.observation = None;
        view.until_ns = 0;
    }
    Reply::from_view(
        id,
        ok && view.observation.is_some() && !view.pending,
        view,
        percent,
    )
}
fn fence(owner: &mut Owner, flight: &mut Option<Flight>, fatal: &mut Option<io::Error>) {
    owner.cancel(now());
    if let Some(flight) = flight
        && let Err(error) = flight.worker.fence()
    {
        fatal.get_or_insert(error);
    }
}
fn drop_peer(
    peers: &mut Vec<Peer>,
    index: usize,
    owner: &mut Owner,
    flight: &mut Option<Flight>,
    fatal: &mut Option<io::Error>,
) {
    let serial = peers.remove(index).serial;
    if flight.as_ref().is_some_and(|v| v.peer == Some(serial)) {
        if let Some(flight) = flight.as_mut() {
            flight.peer = None;
        }
        fence(owner, flight, fatal);
    }
}
fn launch(
    owner: &mut Owner,
    request: &Request,
    backend: &[String],
    peer: u64,
    executable: &Path,
) -> io::Result<Option<Flight>> {
    let sample = now();
    let until = request.until(sample).ok_or(io::ErrorKind::InvalidInput)?;
    let ticket = match request {
        Request::Inspect { .. } => owner.observe_until(sample, until),
        Request::Set {
            percent,
            expected_revision,
            ..
        } => owner.set(*percent, *expected_revision, until, sample).ok(),
    };
    let Some(ticket) = ticket else {
        return Ok(None);
    };
    let mut args = backend.to_vec();
    match ticket.action {
        Action::Observe => args.push("observe".into()),
        Action::Set { percent } => {
            args.push("set".into());
            args.push(percent.to_string());
        }
    }
    match Worker::start(executable, &args, ticket) {
        Ok(worker) => Ok(Some(Flight {
            worker,
            peer: Some(peer),
            request: request.id(),
        })),
        Err(_) => {
            owner.retired(ticket, None, now());
            Ok(None)
        }
    }
}
fn serve(path: &Path, backend: &[String]) -> io::Result<()> {
    if [
        rustix::process::getuid().as_raw(),
        rustix::process::geteuid().as_raw(),
        rustix::process::getgid().as_raw(),
        rustix::process::getegid().as_raw(),
    ] != [0; 4]
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    backend_kind(&backend[0])?;
    let _termination = TerminationGuard::install()?;
    let executable = std::env::current_exe()?;
    let listener = listener(path)?;
    let mut peers = Vec::<Peer>::new();
    let mut serial = 0_u64;
    let mut owner = Owner::new();
    let mut flight: Option<Flight> = None;
    let mut stopping = false;
    let mut fatal = None;
    loop {
        if termination_requested() || fatal.is_some() {
            stopping = true;
        }
        if stopping {
            fence(&mut owner, &mut flight, &mut fatal);
            peers.clear();
        } else {
            for _ in 0..16 {
                match net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                    Ok(fd) => {
                        if peers.len() == 16 {
                            continue;
                        }
                        if let Ok(credentials) = net::sockopt::socket_peercred(&fd)
                            && credentials.uid.as_raw() == 0
                            && credentials.gid.as_raw() == 0
                            && let Some(next) = serial.checked_add(1)
                        {
                            serial = next;
                            peers.push(Peer {
                                fd,
                                serial,
                                credentials,
                            });
                        }
                    }
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => break,
                    Err(error) => {
                        fatal.get_or_insert(error.into());
                        stopping = true;
                        break;
                    }
                }
            }
            let mut index = 0;
            while index < peers.len() {
                if termination_requested() {
                    stopping = true;
                    fence(&mut owner, &mut flight, &mut fatal);
                    break;
                }
                let packet = receive(&peers[index]);
                match packet {
                    Ok(None) => {
                        index += 1;
                        continue;
                    }
                    Err(_) => {
                        drop_peer(&mut peers, index, &mut owner, &mut flight, &mut fatal);
                        continue;
                    }
                    Ok(Some(bytes)) => {
                        let busy = flight
                            .as_ref()
                            .is_some_and(|v| v.peer == Some(peers[index].serial));
                        let request = serde_json::from_slice::<Request>(&bytes)
                            .ok()
                            .filter(|v| v.until(now()).is_some());
                        if busy || request.is_none() {
                            drop_peer(&mut peers, index, &mut owner, &mut flight, &mut fatal);
                            continue;
                        }
                        let request = request.unwrap();
                        if flight.is_none() {
                            match launch(
                                &mut owner,
                                &request,
                                backend,
                                peers[index].serial,
                                &executable,
                            ) {
                                Ok(Some(started)) => {
                                    flight = Some(started);
                                    index += 1;
                                    continue;
                                }
                                Ok(None) => {}
                                Err(_) => {
                                    drop_peer(
                                        &mut peers,
                                        index,
                                        &mut owner,
                                        &mut flight,
                                        &mut fatal,
                                    );
                                    continue;
                                }
                            }
                        }
                        let reply = snapshot(&mut owner, request.id(), false, None);
                        if send(&peers[index], &reply).is_err() {
                            drop_peer(&mut peers, index, &mut owner, &mut flight, &mut fatal);
                        } else {
                            index += 1;
                        }
                    }
                }
            }
        }
        // Always fence the original deadline and owner loss before polling a
        // possibly queued positive child result. No poll renews the deadline.
        owner.view(now());
        if flight
            .as_ref()
            .is_some_and(|v| now() >= v.worker.ticket().until_ns)
        {
            fence(&mut owner, &mut flight, &mut fatal);
        }
        let retired = match flight.as_mut().map(|v| v.worker.poll()) {
            Some(Ok(ready)) => ready,
            Some(Err(error)) => {
                fatal.get_or_insert(error);
                fence(&mut owner, &mut flight, &mut fatal);
                false
            }
            None => false,
        };
        if retired {
            // A fresh final bounded receive catches peer close/pipelining that
            // arrived while the child's wait/EOF was being observed.
            if let Some(index) = flight
                .as_ref()
                .and_then(|v| v.peer)
                .and_then(|serial| peers.iter().position(|p| p.serial == serial))
                && !matches!(receive(&peers[index]), Ok(None))
            {
                drop_peer(&mut peers, index, &mut owner, &mut flight, &mut fatal);
            }
            if termination_requested() || stopping || fatal.is_some() {
                stopping = true;
                fence(&mut owner, &mut flight, &mut fatal);
            }
            let finished = flight.take().expect("actual retained retired worker");
            let ticket = finished.worker.ticket();
            let confirmed = owner.retired(ticket, finished.worker.result(), now());
            if let Some(index) = finished
                .peer
                .and_then(|serial| peers.iter().position(|p| p.serial == serial))
            {
                let percent = match ticket.action {
                    Action::Set { percent } if confirmed => Some(percent),
                    _ => None,
                };
                let reply = snapshot(&mut owner, finished.request, confirmed, percent);
                if send(&peers[index], &reply).is_err() {
                    drop_peer(&mut peers, index, &mut owner, &mut flight, &mut fatal);
                }
            }
            // Worker::poll true was actual wait plus PIPE EOF; only now may its
            // occupied slot and process custody be released.
            drop(finished);
        }
        if stopping && flight.is_none() {
            break;
        }
        let mut descriptors = vec![PollFd::new(&listener, PollFlags::IN)];
        for peer in &peers {
            descriptors.push(PollFd::new(
                &peer.fd,
                PollFlags::IN | PollFlags::HUP | PollFlags::ERR,
            ));
        }
        if let Err(error) = poll(
            &mut descriptors,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 20_000_000,
            }),
        ) && error != rustix::io::Errno::INTR
        {
            fatal.get_or_insert(error.into());
            stopping = true;
        }
    }
    drop(peers);
    drop(listener);
    fs::remove_file(path)?;
    fatal.map_or(Ok(()), Err)
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, socket, kind, root, device] if mode == "--serve" => serve(
            Path::new(socket),
            &[kind.clone(), root.clone(), device.clone()],
        ),
        [_, mode, parent, until, kind, root, device, action] if mode == "--brightness-worker" => {
            worker_entry(parent, until, kind, root, device, action, None)
        }
        [_, mode, parent, until, kind, root, device, action, percent]
            if mode == "--brightness-worker" =>
        {
            worker_entry(parent, until, kind, root, device, action, Some(percent))
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
