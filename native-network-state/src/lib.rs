//! Bounded kernel network observation in the constructing thread's retained namespace.
//! Observations are data; they grant no connectivity or modem authority.
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamespaceIdentity {
    pub dev: u64,
    pub ino: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressSnapshot {
    pub local: IpAddr,
    pub peer: Option<IpAddr>,
    pub prefix_len: u8,
    pub scope: u8,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkSnapshot {
    pub ifindex: u32,
    pub name: String,
    /// Local lifetime established by a continuously observed create notification.
    /// An interface present before subscription has no such lifetime witness.
    pub observed_lifetime: Option<u64>,
    pub flags: u32,
    pub mtu: u32,
    pub operstate: Option<u8>,
    pub addresses: Vec<AddressSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelSnapshot {
    pub namespace: NamespaceIdentity,
    pub revision: u64,
    pub link_dump_sequence: u32,
    pub address_dump_sequence: u32,
    pub observed_at_boottime_ns: u64,
    pub links: Vec<LinkSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserveError {
    Unsupported,
    Deadline,
    Io(i32),
    InvalidData,
    BoundExceeded,
    EvidenceLost,
    ConcurrentChange,
}

// Linux NETLINK_ROUTE UAPI. All scalar fields use native byte order.
// Namespace retention and socket construction happen together on this thread;
// callers cannot associate an arbitrary socket with an unrelated namespace FD.
use rustix::fd::OwnedFd;
use rustix::net::{self, netlink::SocketAddrNetlink};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::IoSliceMut;
use std::os::unix::fs::MetadataExt;
use std::thread;
use std::time::Duration;

const MAX_LINKS: usize = 64;
const MAX_ADDRESSES: usize = 16;
const MAX_MESSAGES: usize = 256;
const MAX_DATAGRAM: usize = 65_536;
const MAX_BYTES: usize = 1_048_576;
const NEW_LINK: u16 = 16;
const DEL_LINK: u16 = 17;
const GET_LINK: u16 = 18;
const NEW_ADDR: u16 = 20;
const DEL_ADDR: u16 = 21;
const GET_ADDR: u16 = 22;

type Result<T> = std::result::Result<T, ObserveError>;

#[derive(Default)]
struct Budget {
    messages: usize,
    datagrams: usize,
    bytes: usize,
}

struct Message {
    kind: u16,
    flags: u16,
    sequence: u32,
    multicast_group: u32,
    body: Vec<u8>,
}

struct Lifetime {
    name: String,
    epoch: Option<u64>,
}

pub struct KernelObserver {
    _retained_namespace: File,
    namespace: NamespaceIdentity,
    socket: OwnedFd,
    port: u32,
    sequence: u32,
    revision: u64,
    next_epoch: u64,
    synchronized: bool,
    lifetimes: BTreeMap<u32, Lifetime>,
    terminal: Option<ObserveError>,
}

impl KernelObserver {
    /// Retain the constructing thread's namespace and subscribe before collecting
    /// its initial inventory. Pre-existing links have no witnessed create epoch.
    /// No worker or link-creation descriptor is acquired.
    pub fn subscribe_current_namespace(until_boottime_ns: u64) -> Result<Self> {
        before(until_boottime_ns)?;
        let retained = File::open("/proc/thread-self/ns/net").map_err(io_error)?;
        let metadata = retained.metadata().map_err(io_error)?;
        let namespace = NamespaceIdentity {
            dev: metadata.dev(),
            ino: metadata.ino(),
        };
        let socket = net::socket_with(
            net::AddressFamily::NETLINK,
            net::SocketType::RAW,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
            None,
        )
        .map_err(errno)?;
        // RTMGRP_LINK | RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR.
        net::bind(&socket, &SocketAddrNetlink::new(0, 0x111)).map_err(errno)?;
        let address = SocketAddrNetlink::try_from(net::getsockname(&socket).map_err(errno)?)
            .map_err(|_| ObserveError::InvalidData)?;
        let current = File::open("/proc/thread-self/ns/net")
            .map_err(io_error)?
            .metadata()
            .map_err(io_error)?;
        if namespace
            != (NamespaceIdentity {
                dev: current.dev(),
                ino: current.ino(),
            })
        {
            return Err(ObserveError::EvidenceLost);
        }
        before(until_boottime_ns)?;
        let mut observer = Self {
            _retained_namespace: retained,
            namespace,
            socket,
            port: address.pid(),
            sequence: 0,
            revision: 0,
            next_epoch: 0,
            synchronized: false,
            lifetimes: BTreeMap::new(),
            terminal: None,
        };
        let initial = observer.refresh(until_boottime_ns)?;
        // NEWLINK can mean an update. Initial synchronization never invents a
        // creation epoch from absence in an incompletely initialized map.
        observer.lifetimes = initial
            .links
            .into_iter()
            .map(|link| {
                (
                    link.ifindex,
                    Lifetime {
                        name: link.name,
                        epoch: None,
                    },
                )
            })
            .collect();
        observer.synchronized = true;
        Ok(observer)
    }

    /// Collect two complete dumps and process all currently queued notifications.
    /// Concurrent notifications invalidate this candidate, while preserving the
    /// continuous lifetime bookkeeping. A successful read is observational, not
    /// an atomic assertion that the link cannot disappear immediately afterward.
    pub fn refresh(&mut self, until_boottime_ns: u64) -> Result<KernelSnapshot> {
        if let Some(error) = self.terminal {
            return Err(error);
        }
        let result = self.collect(until_boottime_ns);
        if let Err(error) = result
            && error != ObserveError::ConcurrentChange
        {
            self.terminal = Some(error);
        }
        result
    }

    fn collect(&mut self, until: u64) -> Result<KernelSnapshot> {
        before(until)?;
        let mut budget = Budget::default();
        self.drain(until, &mut budget)?;
        let revision = self.revision;
        let (link_sequence, messages) = self.dump(GET_LINK, until, &mut budget)?;
        let mut links = BTreeMap::new();
        for message in messages {
            let link = parse_link(&message.body)?;
            if links.insert(link.ifindex, link).is_some() || links.len() > MAX_LINKS {
                return Err(ObserveError::BoundExceeded);
            }
        }
        let (address_sequence, messages) = self.dump(GET_ADDR, until, &mut budget)?;
        for message in messages {
            let (ifindex, address) = parse_address(&message.body)?;
            let link = links
                .get_mut(&ifindex)
                .ok_or(ObserveError::ConcurrentChange)?;
            if link.addresses.len() >= MAX_ADDRESSES || link.addresses.contains(&address) {
                return Err(ObserveError::BoundExceeded);
            }
            link.addresses.push(address);
        }
        self.drain(until, &mut budget)?;
        if self.revision != revision {
            return Err(ObserveError::ConcurrentChange);
        }
        if self.synchronized {
            if links.len() != self.lifetimes.len()
                || links.iter().any(|(ifindex, link)| {
                    self.lifetimes
                        .get(ifindex)
                        .is_none_or(|lifetime| lifetime.name != link.name)
                })
            {
                return Err(ObserveError::EvidenceLost);
            }
            for (ifindex, link) in &mut links {
                link.observed_lifetime = self.lifetimes[ifindex].epoch;
            }
        }
        let observed_at = boot_time_ns();
        if observed_at >= until {
            return Err(ObserveError::Deadline);
        }
        Ok(KernelSnapshot {
            namespace: self.namespace,
            revision: self.revision,
            link_dump_sequence: link_sequence,
            address_dump_sequence: address_sequence,
            observed_at_boottime_ns: observed_at,
            links: links.into_values().collect(),
        })
    }

    fn notification(&mut self, message: &Message) -> Result<()> {
        // Kernel multicast notifications can echo the originating request's
        // sequence/header port. Route them using recvmsg's group metadata;
        // neither a zero sequence nor a zero header port proves notification.
        let expected_group = match message.kind {
            NEW_LINK | DEL_LINK => 1,
            NEW_ADDR | DEL_ADDR => match message.body.first() {
                Some(2) => 0x10,
                Some(10) => 0x100,
                _ => return Err(ObserveError::InvalidData),
            },
            1 => return Ok(()),
            4 => return Err(ObserveError::EvidenceLost),
            _ => return Err(ObserveError::InvalidData),
        };
        if message.multicast_group != expected_group {
            return Err(ObserveError::InvalidData);
        }
        match message.kind {
            NEW_LINK => {
                let link = parse_link(&message.body)?;
                if let Some(lifetime) = self.lifetimes.get_mut(&link.ifindex) {
                    lifetime.name = link.name;
                } else {
                    if self.lifetimes.len() >= MAX_LINKS {
                        return Err(ObserveError::BoundExceeded);
                    }
                    let epoch = if self.synchronized {
                        self.next_epoch = self
                            .next_epoch
                            .checked_add(1)
                            .ok_or(ObserveError::BoundExceeded)?;
                        Some(self.next_epoch)
                    } else {
                        None
                    };
                    self.lifetimes.insert(
                        link.ifindex,
                        Lifetime {
                            name: link.name,
                            epoch,
                        },
                    );
                }
            }
            DEL_LINK => {
                let link = parse_link(&message.body)?;
                if self.lifetimes.remove(&link.ifindex).is_none() && self.synchronized {
                    return Err(ObserveError::EvidenceLost);
                }
            }
            NEW_ADDR | DEL_ADDR => {
                parse_address(&message.body)?;
            }
            1 => return Ok(()), // NLMSG_NOOP has no state effect.
            4 => return Err(ObserveError::EvidenceLost), // NLMSG_OVERRUN.
            _ => return Err(ObserveError::InvalidData),
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ObserveError::BoundExceeded)?;
        Ok(())
    }

    fn drain(&mut self, until: u64, budget: &mut Budget) -> Result<()> {
        // Success requires actual EAGAIN; exhausting a budget never means quiet.
        while let Some(messages) = self.receive(until, budget)? {
            for message in messages {
                self.notification(&message)?;
            }
        }
        Ok(())
    }

    fn dump(&mut self, kind: u16, until: u64, budget: &mut Budget) -> Result<(u32, Vec<Message>)> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(ObserveError::BoundExceeded)?;
        let sequence = self.sequence;
        let body_size = if kind == GET_LINK { 16 } else { 8 };
        let mut request = vec![0_u8; 16 + body_size];
        let request_length = request.len() as u32;
        request[..4].copy_from_slice(&request_length.to_ne_bytes());
        request[4..6].copy_from_slice(&kind.to_ne_bytes());
        request[6..8].copy_from_slice(&0x301_u16.to_ne_bytes()); // REQUEST | DUMP.
        request[8..12].copy_from_slice(&sequence.to_ne_bytes());
        before(until)?;
        let sent = net::sendto(
            &self.socket,
            &request,
            net::SendFlags::DONTWAIT,
            &SocketAddrNetlink::new(0, 0),
        )
        .map_err(errno)?;
        if sent != request.len() {
            return Err(ObserveError::EvidenceLost);
        }
        before(until)?;
        let mut output = Vec::new();
        loop {
            let Some(messages) = self.receive(until, budget)? else {
                thread::sleep(Duration::from_nanos(
                    (until - boot_time_ns().min(until)).min(2_000_000),
                ));
                continue;
            };
            let mut done = false;
            for message in messages {
                if message.multicast_group != 0 {
                    self.notification(&message)?;
                    continue;
                }
                if message.sequence != sequence || done {
                    return Err(ObserveError::InvalidData);
                }
                match message.kind {
                    3 => {
                        // A complete successful matching multipart dump.
                        if message.flags & 2 == 0
                            || message.body.len() != 4
                            || i32_at(&message.body, 0)? != 0
                        {
                            return Err(ObserveError::EvidenceLost);
                        }
                        done = true;
                    }
                    2 => {
                        // NLMSG_ERROR includes the original request header.
                        if message.body.len() < 20
                            || u32_at(&message.body, 12)? != sequence
                            || u16_at(&message.body, 8)? != kind
                        {
                            return Err(ObserveError::InvalidData);
                        }
                        let error = i32_at(&message.body, 0)?;
                        if error != 0 {
                            return Err(ObserveError::Io(error.saturating_abs()));
                        }
                    }
                    expected if expected == kind - 2 && message.flags & 2 != 0 => {
                        output.push(message)
                    }
                    4 => return Err(ObserveError::EvidenceLost),
                    _ => return Err(ObserveError::InvalidData),
                }
            }
            if done {
                return Ok((sequence, output));
            }
        }
    }

    fn receive(&self, until: u64, budget: &mut Budget) -> Result<Option<Vec<Message>>> {
        before(until)?;
        let mut buffer = vec![0_u8; MAX_DATAGRAM];
        let mut ancillary_space = [];
        let mut ancillary = net::RecvAncillaryBuffer::new(&mut ancillary_space);
        let received = match net::recvmsg(
            &self.socket,
            &mut [IoSliceMut::new(&mut buffer)],
            &mut ancillary,
            net::RecvFlags::DONTWAIT,
        ) {
            Ok(value) => value,
            Err(rustix::io::Errno::AGAIN) => {
                before(until)?;
                return Ok(None);
            }
            Err(rustix::io::Errno::NOBUFS) => return Err(ObserveError::EvidenceLost),
            Err(error) => return Err(errno(error)),
        };
        before(until)?;
        if received
            .flags
            .intersects(net::ReturnFlags::TRUNC | net::ReturnFlags::CTRUNC)
            || received.bytes > buffer.len()
            || received.bytes == 0
        {
            return Err(ObserveError::EvidenceLost);
        }
        let sender =
            SocketAddrNetlink::try_from(received.address.ok_or(ObserveError::InvalidData)?)
                .map_err(|_| ObserveError::InvalidData)?;
        if sender.pid() != 0 {
            return Err(ObserveError::EvidenceLost);
        }
        let multicast_group = sender.groups();
        if multicast_group & !0x111 != 0 {
            return Err(ObserveError::InvalidData);
        }
        budget.datagrams += 1;
        budget.bytes += received.bytes;
        if budget.datagrams > MAX_MESSAGES || budget.bytes > MAX_BYTES {
            return Err(ObserveError::BoundExceeded);
        }
        let bytes = &buffer[..received.bytes];
        let mut at = 0;
        let mut messages = Vec::new();
        while at < bytes.len() {
            let rest = &bytes[at..];
            if rest.len() < 16 {
                return Err(ObserveError::InvalidData);
            }
            let length = u32_at(rest, 0)? as usize;
            if length < 16 || length > rest.len() {
                return Err(ObserveError::InvalidData);
            }
            let flags = u16_at(rest, 6)?;
            if flags & 0x10 != 0 {
                return Err(ObserveError::EvidenceLost);
            } // DUMP_INTR.
            let port = u32_at(rest, 12)?;
            if multicast_group == 0 && port != 0 && port != self.port {
                return Err(ObserveError::InvalidData);
            }
            budget.messages += 1;
            if budget.messages > MAX_MESSAGES {
                return Err(ObserveError::BoundExceeded);
            }
            #[cfg(test)]
            eprintln!(
                "NATIVE-NETLINK-METADATA kernel_sender_pid=0 group={} kind={} sequence={} header_port={}",
                multicast_group,
                u16_at(rest, 4)?,
                u32_at(rest, 8)?,
                port
            );
            messages.push(Message {
                kind: u16_at(rest, 4)?,
                flags,
                sequence: u32_at(rest, 8)?,
                multicast_group,
                body: rest[16..length].to_vec(),
            });
            let aligned = length.checked_add(3).ok_or(ObserveError::InvalidData)? & !3;
            if aligned > rest.len() {
                return Err(ObserveError::InvalidData);
            }
            at += aligned;
        }
        before(until)?;
        Ok(Some(messages))
    }
}

fn before(until: u64) -> Result<()> {
    if boot_time_ns() < until {
        Ok(())
    } else {
        Err(ObserveError::Deadline)
    }
}
fn errno(error: rustix::io::Errno) -> ObserveError {
    ObserveError::Io(error.raw_os_error())
}
fn io_error(error: std::io::Error) -> ObserveError {
    ObserveError::Io(error.raw_os_error().unwrap_or(5))
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_ne_bytes(
        bytes
            .get(at..at + 2)
            .ok_or(ObserveError::InvalidData)?
            .try_into()
            .map_err(|_| ObserveError::InvalidData)?,
    ))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_ne_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(ObserveError::InvalidData)?
            .try_into()
            .map_err(|_| ObserveError::InvalidData)?,
    ))
}
fn i32_at(bytes: &[u8], at: usize) -> Result<i32> {
    Ok(u32_at(bytes, at)? as i32)
}

fn attributes(bytes: &[u8]) -> Result<Vec<(u16, &[u8])>> {
    let mut at = 0;
    let mut output = Vec::new();
    while at < bytes.len() {
        let rest = &bytes[at..];
        let length = usize::from(u16_at(rest, 0)?);
        let kind = u16_at(rest, 2)?;
        if length < 4 || length > rest.len() || output.len() >= 256 {
            return Err(ObserveError::InvalidData);
        }
        output.push((kind, &rest[4..length]));
        let aligned = length.checked_add(3).ok_or(ObserveError::InvalidData)? & !3;
        if aligned > rest.len() {
            return Err(ObserveError::InvalidData);
        }
        at += aligned;
    }
    Ok(output)
}

fn parse_link(body: &[u8]) -> Result<LinkSnapshot> {
    if body.len() < 16 {
        return Err(ObserveError::InvalidData);
    }
    let ifindex = i32_at(body, 4)?;
    if ifindex <= 0 {
        return Err(ObserveError::InvalidData);
    }
    let mut name = None;
    let mut mtu = None;
    let mut operstate = None;
    for (kind, value) in attributes(&body[16..])? {
        match kind {
            3 => {
                if name.is_some()
                    || !(2..=16).contains(&value.len())
                    || value.last() != Some(&0)
                    || value[..value.len() - 1]
                        .iter()
                        .any(|byte| !byte.is_ascii() || *byte == 0)
                {
                    return Err(ObserveError::InvalidData);
                }
                name = Some(
                    std::str::from_utf8(&value[..value.len() - 1])
                        .map_err(|_| ObserveError::InvalidData)?
                        .to_owned(),
                );
            }
            4 => {
                if mtu.is_some() || value.len() != 4 {
                    return Err(ObserveError::InvalidData);
                }
                mtu = Some(u32_at(value, 0)?);
            }
            16 => {
                if operstate.is_some() || value.len() != 1 {
                    return Err(ObserveError::InvalidData);
                }
                operstate = Some(value[0]);
            }
            _ => {}
        }
    }
    Ok(LinkSnapshot {
        ifindex: ifindex as u32,
        name: name.ok_or(ObserveError::InvalidData)?,
        observed_lifetime: None,
        flags: u32_at(body, 8)?,
        mtu: mtu.ok_or(ObserveError::InvalidData)?,
        operstate,
        addresses: Vec::new(),
    })
}

fn parse_address(body: &[u8]) -> Result<(u32, AddressSnapshot)> {
    if body.len() < 8 {
        return Err(ObserveError::InvalidData);
    }
    let family = body[0];
    if !matches!(family, 2 | 10) || body[1] > if family == 2 { 32 } else { 128 } {
        return Err(ObserveError::InvalidData);
    }
    let ifindex = u32_at(body, 4)?;
    if ifindex == 0 {
        return Err(ObserveError::InvalidData);
    }
    let mut local = None;
    let mut address = None;
    let mut flags = None;
    for (kind, value) in attributes(&body[8..])? {
        match kind {
            1 | 2 => {
                let ip = if family == 2 {
                    IpAddr::V4(std::net::Ipv4Addr::from(
                        <[u8; 4]>::try_from(value).map_err(|_| ObserveError::InvalidData)?,
                    ))
                } else {
                    IpAddr::V6(std::net::Ipv6Addr::from(
                        <[u8; 16]>::try_from(value).map_err(|_| ObserveError::InvalidData)?,
                    ))
                };
                let field = if kind == 1 { &mut address } else { &mut local };
                if field.replace(ip).is_some() {
                    return Err(ObserveError::InvalidData);
                }
            }
            8 => {
                if flags.is_some() || value.len() != 4 {
                    return Err(ObserveError::InvalidData);
                }
                flags = Some(u32_at(value, 0)?);
            }
            _ => {}
        }
    }
    // IPv4 point-to-point IFA_ADDRESS may be the destination. It must never
    // stand in for a missing local address. IPv6 normally supplies IFA_ADDRESS.
    let local = if family == 2 {
        local
    } else {
        local.or(address)
    }
    .ok_or(ObserveError::InvalidData)?;
    Ok((
        ifindex,
        AddressSnapshot {
            local,
            peer: address.filter(|value| *value != local),
            prefix_len: body[1],
            scope: body[3],
            flags: flags.unwrap_or(u32::from(body[2])),
        },
    ))
}

pub fn boot_time_ns() -> u64 {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(value.tv_sec)
        .expect("nonnegative BOOTTIME")
        .checked_mul(1_000_000_000)
        .and_then(|seconds| seconds.checked_add(value.tv_nsec as u64))
        .expect("BOOTTIME fits u64 nanoseconds")
}

#[cfg(test)]
mod kernel_tests;
