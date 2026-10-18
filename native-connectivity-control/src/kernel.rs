//! Linux-only virtual backend. Exclusive supervisor custody is a deployment
//! precondition, not something a namespace inventory or ifindex can prove.
//! Kernel dispatch can block despite DONTWAIT; isolate this component from a
//! responsive service Main. Deadline checks fence accepted completion only.
use pokelinux_network_state::{
    KernelObserver, KernelSnapshot, NamespaceIdentity, ObserveError, boot_time_ns,
};
use rustix::{
    fd::OwnedFd,
    net::{self, netlink::SocketAddrNetlink},
};
use std::{
    fs::File,
    io::IoSliceMut,
    net::{IpAddr, Ipv4Addr},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    thread,
    time::Duration,
};

const SECOND: u64 = 1_000_000_000;
const LOCAL_NAME: &str = "pklown0";
const PEER_NAME: &str = "pklpeer0";
const LOCAL_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Deadline,
    Io(i32),
    InvalidData,
    Conflict,
    Uncertain,
    Observation(ObserveError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fact {
    pub namespace: NamespaceIdentity,
    pub ifindex: u32,
    pub lifetime: u64,
    pub connected: bool,
}

/// One private pair in two fresh namespaces. The trusted supervisor must retain
/// both namespaces and exclude all competing administrators for this epoch.
/// Dropping this object closes descriptors; it is not vendor or link cleanup.
/// No destructive rollback is attempted on an uncertain create or later failure.
pub struct VirtualLink {
    _current: File,
    _peer: File,
    observer: KernelObserver,
    mutator: Mutator,
    last: Fact,
    terminal: bool,
}
impl VirtualLink {
    pub fn create(peer: File, until: u64) -> Result<Self, Error> {
        command_deadline(until)?;
        let current = File::open("/proc/thread-self/ns/net").map_err(io_error)?;
        let namespace = identity(&current)?;
        if namespace == identity(&peer)? {
            return Err(Error::Conflict);
        }
        // Subscribe before pair creation. This establishes a witnessed lifetime,
        // not an atomic generation token usable by a later rtnetlink write.
        let mut observer =
            KernelObserver::subscribe_current_namespace(until).map_err(Error::Observation)?;
        let initial = collect(&mut observer, until)?;
        if initial.namespace != namespace
            || initial.links.len() != 1
            || initial.links[0].name != "lo"
        {
            return Err(Error::Conflict);
        }
        let mut mutator = Mutator::new()?;
        let mut peer_body = vec![0; 16]; // ifinfomsg, AF_UNSPEC.
        attribute(&mut peer_body, 3, nul_name(PEER_NAME).as_slice()); // IFLA_IFNAME.
        attribute(&mut peer_body, 28, &peer.as_raw_fd().to_ne_bytes()); // IFLA_NET_NS_FD.
        let mut data = Vec::new();
        attribute(&mut data, 0x8001, &peer_body); // VETH_INFO_PEER | NESTED.
        let mut info = Vec::new();
        attribute(&mut info, 1, b"veth\0"); // IFLA_INFO_KIND.
        attribute(&mut info, 0x8002, &data); // IFLA_INFO_DATA | NESTED.
        let mut create = vec![0; 16];
        attribute(&mut create, 3, nul_name(LOCAL_NAME).as_slice());
        attribute(&mut create, 0x8012, &info); // IFLA_LINKINFO | NESTED.
        mutator.exchange(16, 0x605, &create, until)?; // NEWLINK REQUEST|ACK|CREATE|EXCL.
        let created = collect(&mut observer, until)?;
        let link = created
            .links
            .iter()
            .find(|link| link.name == LOCAL_NAME)
            .ok_or(Error::Conflict)?;
        let lifetime = link.observed_lifetime.ok_or(Error::Conflict)?;
        let index = link.ifindex;
        if created.namespace != namespace || created.links.len() != 2 || index <= 1 || lifetime == 0
        {
            return Err(Error::Conflict);
        }
        let mut up = vec![0; 16];
        up[4..8].copy_from_slice(&index.to_ne_bytes());
        up[8..12].copy_from_slice(&1_u32.to_ne_bytes()); // IFF_UP.
        up[12..16].copy_from_slice(&1_u32.to_ne_bytes());
        mutator.exchange(16, 5, &up, until)?;
        let mut output = Self {
            _current: current,
            _peer: peer,
            observer,
            mutator,
            last: Fact {
                namespace,
                ifindex: index,
                lifetime,
                connected: false,
            },
            terminal: false,
        };
        output.verify_creation(until)?;
        Ok(output)
    }

    /// Last completed observation only; this does not refresh kernel state.
    pub fn fact(&self) -> Result<Fact, Error> {
        if self.terminal {
            Err(Error::Uncertain)
        } else {
            Ok(self.last)
        }
    }

    fn verify_creation(&mut self, until: u64) -> Result<(), Error> {
        let snapshot = collect(&mut self.observer, until)?;
        let local = snapshot
            .links
            .iter()
            .find(|link| link.ifindex == self.last.ifindex)
            .ok_or(Error::Conflict)?;
        if snapshot.namespace != self.last.namespace
            || snapshot.links.len() != 2
            || local.name != LOCAL_NAME
            || local.observed_lifetime != Some(self.last.lifetime)
            || local.flags & 1 == 0
            || local
                .addresses
                .iter()
                .any(|address| address.local.is_ipv4())
        {
            return Err(Error::Conflict);
        }
        before(until)
    }

    /// One explicit fixed-profile choice. No address or interface selector comes
    /// from an application. A possibly dispatched failure retires this epoch;
    /// no rollback, compensating delete or implicit retry is attempted.
    pub fn set_connected(&mut self, connected: bool, until: u64) -> Result<Fact, Error> {
        if self.terminal {
            return Err(Error::Uncertain);
        }
        command_deadline(until)?;
        let observed = match self.read_profile(until) {
            Ok(value) => value,
            Err(error) => {
                self.terminal = true;
                return Err(error);
            }
        };
        if observed.connected != self.last.connected {
            self.terminal = true;
            return Err(Error::Conflict);
        }
        if observed.connected == connected {
            before(until)?;
            self.last = observed;
            return Ok(observed);
        }
        let mut body = vec![2, 30, 0, 0]; // ifaddrmsg: AF_INET, /30, global scope.
        body.extend_from_slice(&self.last.ifindex.to_ne_bytes());
        attribute(&mut body, 2, &LOCAL_ADDRESS.octets()); // IFA_LOCAL.
        attribute(&mut body, 1, &LOCAL_ADDRESS.octets()); // IFA_ADDRESS, same local.
        let (kind, flags) = if connected { (20, 0x605) } else { (21, 5) };
        match self.mutator.exchange(kind, flags, &body, until) {
            Ok(()) => {}
            // exchange's Deadline is its last pre-send check. All later expiry
            // is Uncertain. This path has not entered the mutating syscall.
            Err(Error::Deadline) => return Err(Error::Deadline),
            Err(_) => {
                self.terminal = true;
                return Err(Error::Uncertain);
            }
        }
        // A kernel ACK alone is not our connection fact. Observe the actual
        // address under the original deadline, on the continuously witnessed
        // endpoint; the exclusive lifecycle contract remains necessary.
        let result = self.read_profile(until);
        match result {
            Ok(value) if value.connected == connected && before(until).is_ok() => {
                self.last = value;
                Ok(value)
            }
            _ => {
                self.terminal = true;
                Err(Error::Uncertain)
            }
        }
    }

    fn read_profile(&mut self, until: u64) -> Result<Fact, Error> {
        let snapshot = collect(&mut self.observer, until)?;
        let local = snapshot
            .links
            .iter()
            .find(|link| link.ifindex == self.last.ifindex)
            .ok_or(Error::Conflict)?;
        if snapshot.namespace != self.last.namespace
            || snapshot.links.len() != 2
            || local.name != LOCAL_NAME
            || local.observed_lifetime != Some(self.last.lifetime)
            || local.flags & 1 == 0
        {
            return Err(Error::Conflict);
        }
        let addresses: Vec<_> = local
            .addresses
            .iter()
            .filter(|address| address.local.is_ipv4())
            .collect();
        let connected = match addresses.as_slice() {
            [] => false,
            [address]
                if address.local == IpAddr::V4(LOCAL_ADDRESS)
                    && address.peer.is_none()
                    && address.prefix_len == 30
                    && address.scope == 0 =>
            {
                true
            }
            _ => return Err(Error::Conflict),
        };
        before(until)?;
        Ok(Fact {
            connected,
            ..self.last
        })
    }
}

fn identity(file: &File) -> Result<NamespaceIdentity, Error> {
    let m = file.metadata().map_err(io_error)?;
    Ok(NamespaceIdentity {
        dev: m.dev(),
        ino: m.ino(),
    })
}
fn before(until: u64) -> Result<(), Error> {
    if boot_time_ns() < until {
        Ok(())
    } else {
        Err(Error::Deadline)
    }
}
fn command_deadline(until: u64) -> Result<(), Error> {
    let now = boot_time_ns();
    if until > now && until - now <= SECOND {
        Ok(())
    } else {
        Err(Error::Deadline)
    }
}
fn collect(observer: &mut KernelObserver, until: u64) -> Result<KernelSnapshot, Error> {
    loop {
        before(until)?;
        match observer.refresh(until) {
            // This repeats only read-only collection, never a mutating syscall.
            Err(ObserveError::ConcurrentChange) => pause(until)?,
            Err(error) => return Err(Error::Observation(error)),
            Ok(value) => return Ok(value),
        }
    }
}
fn pause(until: u64) -> Result<(), Error> {
    before(until)?;
    thread::sleep(Duration::from_nanos(
        (until - boot_time_ns().min(until)).min(1_000_000),
    ));
    before(until)
}
fn io_error(error: std::io::Error) -> Error {
    Error::Io(error.raw_os_error().unwrap_or(5))
}
fn errno(error: rustix::io::Errno) -> Error {
    Error::Io(error.raw_os_error())
}
fn nul_name(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    bytes
}
fn attribute(out: &mut Vec<u8>, kind: u16, value: &[u8]) {
    let length = u16::try_from(value.len() + 4).expect("fixed bounded UAPI attribute");
    out.extend_from_slice(&length.to_ne_bytes());
    out.extend_from_slice(&kind.to_ne_bytes());
    out.extend_from_slice(value);
    out.resize((out.len() + 3) & !3, 0);
}

struct Mutator {
    socket: OwnedFd,
    port: u32,
    sequence: u32,
}
impl Mutator {
    fn new() -> Result<Self, Error> {
        let socket = net::socket_with(
            net::AddressFamily::NETLINK,
            net::SocketType::RAW,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
            None,
        )
        .map_err(errno)?;
        net::bind(&socket, &SocketAddrNetlink::new(0, 0)).map_err(errno)?;
        let addr = SocketAddrNetlink::try_from(net::getsockname(&socket).map_err(errno)?)
            .map_err(|_| Error::InvalidData)?;
        Ok(Self {
            socket,
            port: addr.pid(),
            sequence: 0,
        })
    }
    fn exchange(&mut self, kind: u16, flags: u16, body: &[u8], until: u64) -> Result<(), Error> {
        self.sequence = self.sequence.checked_add(1).ok_or(Error::InvalidData)?;
        let mut request = Vec::with_capacity(16 + body.len());
        request.extend_from_slice(
            &u32::try_from(16 + body.len())
                .map_err(|_| Error::InvalidData)?
                .to_ne_bytes(),
        );
        request.extend_from_slice(&kind.to_ne_bytes());
        request.extend_from_slice(&flags.to_ne_bytes());
        request.extend_from_slice(&self.sequence.to_ne_bytes());
        request.extend_from_slice(&0_u32.to_ne_bytes());
        request.extend_from_slice(body);
        before(until)?;
        let sent = net::sendto(
            &self.socket,
            &request,
            net::SendFlags::DONTWAIT,
            &SocketAddrNetlink::new(0, 0),
        )
        .map_err(errno)?;
        if sent != request.len() {
            return Err(Error::Uncertain);
        }
        // After send, expiry/errors must never be called definitely unsent.
        loop {
            if before(until).is_err() {
                return Err(Error::Uncertain);
            }
            let mut buffer = [0_u8; 4096];
            let mut space = [];
            let mut ancillary = net::RecvAncillaryBuffer::new(&mut space);
            let value = match net::recvmsg(
                &self.socket,
                &mut [IoSliceMut::new(&mut buffer)],
                &mut ancillary,
                net::RecvFlags::DONTWAIT,
            ) {
                Ok(v) => v,
                Err(rustix::io::Errno::AGAIN) => {
                    if pause(until).is_err() {
                        return Err(Error::Uncertain);
                    }
                    continue;
                }
                Err(_) => return Err(Error::Uncertain),
            };
            if before(until).is_err()
                || value
                    .flags
                    .intersects(net::ReturnFlags::TRUNC | net::ReturnFlags::CTRUNC)
                || value.bytes < 36
                || value.bytes > buffer.len()
            {
                return Err(Error::Uncertain);
            }
            let sender = SocketAddrNetlink::try_from(value.address.ok_or(Error::Uncertain)?)
                .map_err(|_| Error::Uncertain)?;
            let bytes = &buffer[..value.bytes];
            let n = u32_at(bytes, 0)? as usize;
            let port = u32_at(bytes, 12)?;
            if sender.pid() != 0
                || sender.groups() != 0
                || n < 36
                || n > bytes.len()
                || ((n + 3) & !3) != bytes.len()
                || u16_at(bytes, 4)? != 2
                || u32_at(bytes, 8)? != self.sequence
                || (port != 0 && port != self.port)
                || bytes[20..36] != request[..16]
            {
                return Err(Error::Uncertain);
            }
            let result =
                i32::from_ne_bytes(bytes[16..20].try_into().map_err(|_| Error::Uncertain)?);
            if result == 0 {
                return Ok(());
            }
            if (-4095..0).contains(&result) {
                return Err(Error::Io(-result));
            }
            return Err(Error::Uncertain);
        }
    }
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_ne_bytes(
        bytes
            .get(at..at + 2)
            .ok_or(Error::Uncertain)?
            .try_into()
            .map_err(|_| Error::Uncertain)?,
    ))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_ne_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(Error::Uncertain)?
            .try_into()
            .map_err(|_| Error::Uncertain)?,
    ))
}
