use crate::{
    Bss, CommandTransaction, Completed, Error, Failure, RadioFact, RadioObservationPhase,
    RadioRefreshDiagnostic, RadioTarget, Request,
};
use netlink_sys::{Socket, protocols};
use rustix::fd::OwnedFd;
use rustix::{event, io::Errno, net, time};
use std::{fs::File, io::IoSliceMut, os::unix::fs::MetadataExt};

const REQUEST: u16 = 1;
const MULTI: u16 = 2;
const ACK: u16 = 4;
const DUMP: u16 = 0x300;
const DUMP_INTR: u16 = 0x10;
const ERROR: u16 = 2;
const DONE: u16 = 3;
const OVERRUN: u16 = 4;
const CTRL: u16 = 0x10;
// Linux UAPI nl80211.h; public source references are recorded in the report.
const WIPHY: u16 = 1;
const IFINDEX: u16 = 3;
const IFTYPE: u16 = 5;
const FREQUENCIES: u16 = 44;
const SSIDS: u16 = 45;
const GENERATION: u16 = 46;
const BSS: u16 = 47;
const WDEV: u16 = 153;
const SCAN_FLAGS: u16 = 158;
const GET_INTERFACE: u8 = 5;
const NEW_INTERFACE: u8 = 7;
const GET_SCAN: u8 = 32;
const TRIGGER_SCAN: u8 = 33;
const NEW_SCAN_RESULTS: u8 = 34;
const SCAN_ABORTED: u8 = 35;
const MAX_BSS: usize = 64;
type Result<T> = std::result::Result<T, Failure>;

pub(crate) fn boottime_ns() -> u64 {
    let v = time::clock_gettime(time::ClockId::Boottime);
    u64::try_from(v.tv_sec).expect("nonnegative BOOTTIME") * 1_000_000_000
        + u64::try_from(v.tv_nsec).expect("nonnegative BOOTTIME nanoseconds")
}
fn system(e: std::io::Error) -> Failure {
    Failure::Kernel(e.raw_os_error().unwrap_or(0))
}
fn errno(e: Errno) -> Failure {
    Failure::Kernel(e.raw_os_error())
}
fn align(n: usize) -> usize {
    (n + 3) & !3
}
fn u16n(b: &[u8]) -> Result<u16> {
    Ok(u16::from_ne_bytes(
        b.try_into().map_err(|_| Failure::Malformed)?,
    ))
}
fn u32n(b: &[u8]) -> Result<u32> {
    Ok(u32::from_ne_bytes(
        b.try_into().map_err(|_| Failure::Malformed)?,
    ))
}
fn u64n(b: &[u8]) -> Result<u64> {
    Ok(u64::from_ne_bytes(
        b.try_into().map_err(|_| Failure::Malformed)?,
    ))
}

#[derive(Debug)]
struct Frame<'a> {
    kind: u16,
    flags: u16,
    seq: u32,
    port: u32,
    body: &'a [u8],
}
fn frames(mut b: &[u8]) -> Result<Vec<Frame<'_>>> {
    let mut out = Vec::new();
    while !b.is_empty() {
        if b.len() < 16 || out.len() == 256 {
            return Err(Failure::Malformed);
        }
        let n = u32n(&b[..4])? as usize;
        if n < 16 || n > b.len() {
            return Err(Failure::Malformed);
        }
        let flags = u16n(&b[6..8])?;
        let kind = u16n(&b[4..6])?;
        if flags & DUMP_INTR != 0 || kind == OVERRUN {
            return Err(Failure::Loss);
        }
        out.push(Frame {
            kind,
            flags,
            seq: u32n(&b[8..12])?,
            port: u32n(&b[12..16])?,
            body: &b[16..n],
        });
        let next = align(n);
        if next > b.len() {
            if n != b.len() {
                return Err(Failure::Malformed);
            }
            b = &[];
        } else {
            b = &b[next..];
        }
    }
    Ok(out)
}
#[derive(Clone, Copy, Debug)]
struct Attribute<'a> {
    kind: u16,
    flags: u16,
    body: &'a [u8],
}
fn attributes(mut b: &[u8]) -> Result<Vec<Attribute<'_>>> {
    let mut out = Vec::new();
    while !b.is_empty() {
        if b.len() < 4 || out.len() == 1024 {
            return Err(Failure::Malformed);
        }
        let n = usize::from(u16n(&b[..2])?);
        if n < 4 || n > b.len() {
            return Err(Failure::Malformed);
        }
        let raw = u16n(&b[2..4])?;
        out.push(Attribute {
            kind: raw & 0x3fff,
            flags: raw & 0xc000,
            body: &b[4..n],
        });
        let next = align(n);
        if next > b.len() {
            if n != b.len() {
                return Err(Failure::Malformed);
            }
            b = &[];
        } else {
            b = &b[next..];
        }
    }
    Ok(out)
}
fn optional<'a>(attrs: &[Attribute<'a>], kind: u16) -> Result<Option<Attribute<'a>>> {
    let mut found = attrs.iter().filter(|a| a.kind == kind);
    let value = found.next().copied();
    if found.next().is_some() {
        return Err(Failure::Malformed);
    }
    Ok(value)
}
fn required<'a>(attrs: &[Attribute<'a>], kind: u16) -> Result<Attribute<'a>> {
    optional(attrs, kind)?.ok_or(Failure::MissingAttribute(kind))
}
fn raw(a: Attribute<'_>) -> Result<&[u8]> {
    if a.flags != 0 {
        return Err(Failure::Malformed);
    }
    Ok(a.body)
}
fn nested(a: Attribute<'_>) -> Result<Vec<Attribute<'_>>> {
    if a.flags & 0x4000 != 0 {
        return Err(Failure::Malformed);
    }
    attributes(a.body)
}
fn number32(a: Attribute<'_>) -> Result<u32> {
    u32n(raw(a)?)
}
fn number64(a: Attribute<'_>) -> Result<u64> {
    u64n(raw(a)?)
}
fn put(out: &mut Vec<u8>, kind: u16, body: &[u8]) {
    out.extend_from_slice(
        &u16::try_from(body.len() + 4)
            .expect("bounded authored attr")
            .to_ne_bytes(),
    );
    out.extend_from_slice(&kind.to_ne_bytes());
    out.extend_from_slice(body);
    out.resize(align(out.len()), 0);
}
fn put32(out: &mut Vec<u8>, kind: u16, n: u32) {
    put(out, kind, &n.to_ne_bytes());
}
fn genl<'a>(frame: &Frame<'a>, kind: u16, version: u8, cmd: u8) -> Result<Vec<Attribute<'a>>> {
    if frame.kind != kind
        || frame.body.len() < 4
        || frame.body[0] != cmd
        || frame.body[1] != version
        || frame.body[2..4] != [0, 0]
    {
        return Err(Failure::Malformed);
    }
    attributes(&frame.body[4..])
}

struct Budget {
    until: u64,
    timer: OwnedFd,
    reads: usize,
    bytes: usize,
}
impl Budget {
    fn new(until: u64) -> Result<Self> {
        let timer = time::timerfd_create(
            time::TimerfdClockId::Boottime,
            time::TimerfdFlags::CLOEXEC | time::TimerfdFlags::NONBLOCK,
        )
        .map_err(errno)?;
        time::timerfd_settime(
            &timer,
            time::TimerfdTimerFlags::ABSTIME,
            &time::Itimerspec {
                it_interval: time::Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: time::Timespec {
                    tv_sec: (until / 1_000_000_000)
                        .try_into()
                        .map_err(|_| Failure::Malformed)?,
                    tv_nsec: (until % 1_000_000_000)
                        .try_into()
                        .map_err(|_| Failure::Malformed)?,
                },
            },
        )
        .map_err(errno)?;
        Ok(Self {
            until,
            timer,
            reads: 0,
            bytes: 0,
        })
    }
    fn fence(&self) -> Result<()> {
        if boottime_ns() >= self.until {
            Err(Failure::Expired)
        } else {
            Ok(())
        }
    }
    fn attempt(&mut self) -> Result<()> {
        self.fence()?;
        self.reads += 1;
        if self.reads > 4096 {
            return Err(Failure::Limit);
        }
        Ok(())
    }
}
// Unique socket: never derive Clone or expose netlink-sys's shallow RawFd Clone.
struct Connection {
    socket: Socket,
    port: u32,
    sequence: u32,
    transaction: std::cell::Cell<CommandTransaction>,
    request_flags: Option<u16>,
}
impl Connection {
    fn new() -> Result<Self> {
        let mut socket = Socket::new(protocols::NETLINK_GENERIC).map_err(|e| {
            if matches!(e.raw_os_error(), Some(93 | 95 | 97)) {
                Failure::Unsupported
            } else {
                system(e)
            }
        })?;
        let port = socket.bind_auto().map_err(system)?.port_number();
        socket.set_non_blocking(true).map_err(system)?;
        Ok(Self {
            socket,
            port,
            sequence: 0,
            transaction: std::cell::Cell::new(CommandTransaction::NotSent),
            request_flags: Some(0),
        })
    }
    fn request(
        &mut self,
        kind: u16,
        version: u8,
        cmd: u8,
        flags: u16,
        attrs: &[u8],
    ) -> Result<Vec<u8>> {
        self.sequence = self.sequence.checked_add(1).ok_or(Failure::Limit)?;
        if self.transaction.get() == CommandTransaction::Pending {
            // A new request cannot erase an older unvalidated expectation.
            self.request_flags = None;
        } else {
            self.request_flags = Some(flags);
            self.transaction.set(CommandTransaction::NotSent);
        }
        let mut out = Vec::new();
        out.extend_from_slice(
            &u32::try_from(20 + attrs.len())
                .map_err(|_| Failure::Limit)?
                .to_ne_bytes(),
        );
        out.extend_from_slice(&kind.to_ne_bytes());
        out.extend_from_slice(&(REQUEST | flags).to_ne_bytes());
        out.extend_from_slice(&self.sequence.to_ne_bytes());
        out.extend_from_slice(&self.port.to_ne_bytes());
        out.extend_from_slice(&[cmd, version, 0, 0]);
        out.extend_from_slice(attrs);
        Ok(out)
    }
    fn send(&self, request: &[u8], budget: &Budget) -> Result<()> {
        budget.fence()?;
        // Conservative even if sendto itself fails or its post-fence expires.
        self.transaction.set(CommandTransaction::Pending);
        let n = net::sendto(
            &self.socket,
            request,
            net::SendFlags::DONTWAIT,
            &net::netlink::SocketAddrNetlink::new(0, 0),
        )
        .map_err(errno)?;
        budget.fence()?;
        if n != request.len() {
            return Err(Failure::Loss);
        }
        Ok(())
    }
    fn ready(&self, budget: &mut Budget) -> Result<Option<Vec<u8>>> {
        budget.attempt()?;
        let mut buf = vec![0; 65536];
        let mut space = [];
        let mut ancillary = net::RecvAncillaryBuffer::new(&mut space);
        let msg = match net::recvmsg(
            &self.socket,
            &mut [IoSliceMut::new(&mut buf)],
            &mut ancillary,
            net::RecvFlags::DONTWAIT,
        ) {
            Ok(v) => v,
            Err(Errno::AGAIN) => {
                budget.fence()?;
                return Ok(None);
            }
            Err(Errno::NOBUFS) => return Err(Failure::Loss),
            Err(e) => return Err(errno(e)),
        };
        budget.fence()?;
        if msg
            .flags
            .intersects(net::ReturnFlags::TRUNC | net::ReturnFlags::CTRUNC)
            || msg.bytes == 0
            || msg.bytes > buf.len()
        {
            return Err(Failure::Loss);
        }
        let sender =
            net::netlink::SocketAddrNetlink::try_from(msg.address.ok_or(Failure::Malformed)?)
                .map_err(|_| Failure::Malformed)?;
        if sender.pid() != 0 {
            return Err(Failure::Malformed);
        }
        budget.bytes = budget.bytes.checked_add(msg.bytes).ok_or(Failure::Limit)?;
        if budget.bytes > 1024 * 1024 {
            return Err(Failure::Limit);
        }
        buf.truncate(msg.bytes);
        Ok(Some(buf))
    }
    fn receive(&self, budget: &mut Budget) -> Result<Vec<u8>> {
        loop {
            if let Some(data) = self.ready(budget)? {
                return Ok(data);
            }
            budget.fence()?;
            let mut fds = [
                event::PollFd::new(&self.socket, event::PollFlags::IN),
                event::PollFd::new(&budget.timer, event::PollFlags::IN),
            ];
            match event::poll(&mut fds, None) {
                Ok(_) => {}
                Err(Errno::INTR) => continue,
                Err(e) => return Err(errno(e)),
            }
            budget.fence()?;
            if !fds[1].revents().is_empty() {
                return Err(Failure::Expired);
            }
            if fds[0]
                .revents()
                .intersects(event::PollFlags::ERR | event::PollFlags::HUP | event::PollFlags::NVAL)
            {
                return Err(Failure::Loss);
            }
        }
    }
    fn single(
        &mut self,
        kind: u16,
        version: u8,
        cmd: u8,
        reply_cmd: u8,
        attrs: &[u8],
        budget: &mut Budget,
    ) -> Result<Vec<u8>> {
        let request = self.request(kind, version, cmd, 0, attrs)?;
        self.send(&request, budget)?;
        let data = self.receive(budget)?;
        let messages = frames(&data)?;
        if messages.len() != 1 {
            return Err(Failure::Malformed);
        }
        let frame = &messages[0];
        self.correlated(frame)?;
        if frame.kind == ERROR {
            let failure = error_reply(frame, &request)?.ok_or(Failure::Malformed)?;
            self.complete(0);
            return Err(failure);
        }
        if frame.flags & MULTI != 0 {
            return Err(Failure::Malformed);
        }
        genl(frame, kind, version, reply_cmd)?;
        self.complete(0);
        Ok(frame.body[4..].to_vec())
    }
    fn complete(&self, expected_flags: u16) {
        // Current requests use exactly no flags, ACK, or DUMP. A combined or
        // unknown expectation remains Pending, including DUMP with an ACK.
        if matches!(expected_flags, 0 | ACK | DUMP)
            && self.request_flags == Some(expected_flags)
            && self.transaction.get() == CommandTransaction::Pending
        {
            self.transaction.set(CommandTransaction::Completed);
        }
    }
    fn correlated(&self, frame: &Frame<'_>) -> Result<()> {
        if frame.seq != self.sequence || (frame.port != 0 && frame.port != self.port) {
            return Err(Failure::Malformed);
        }
        Ok(())
    }
    fn acknowledge(&self, request: &[u8], budget: &mut Budget) -> Result<()> {
        let data = self.receive(budget)?;
        let messages = frames(&data)?;
        if messages.len() != 1 {
            return Err(Failure::Malformed);
        }
        let frame = &messages[0];
        self.correlated(frame)?;
        let error = error_reply(frame, request)?;
        self.complete(ACK);
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }
}
fn error_reply(frame: &Frame<'_>, request: &[u8]) -> Result<Option<Failure>> {
    if frame.kind != ERROR || frame.body.len() < 20 || frame.body[4..20] != request[..16] {
        return Err(Failure::Malformed);
    }
    let code = i32::from_ne_bytes(frame.body[..4].try_into().map_err(|_| Failure::Malformed)?);
    match code {
        0 => Ok(None),
        -4095..=-1 => Ok(Some(Failure::Kernel(-code))),
        _ => Err(Failure::Malformed),
    }
}
struct Family {
    id: u16,
    version: u8,
    group: u32,
}
fn family(connection: &mut Connection, budget: &mut Budget) -> Result<Family> {
    family_group(connection, budget, b"scan\0")
}
fn family_group(
    connection: &mut Connection,
    budget: &mut Budget,
    group_name: &[u8],
) -> Result<Family> {
    let mut query = Vec::new();
    put(&mut query, 2, b"nl80211\0");
    let data = connection
        .single(CTRL, 2, 3, 1, &query, budget)
        .map_err(|e| {
            if e == Failure::Kernel(2) {
                Failure::Unsupported
            } else {
                e
            }
        })?;
    let attrs = attributes(&data)?;
    if raw(required(&attrs, 2)?)? != b"nl80211\0" {
        return Err(Failure::Malformed);
    }
    let id = u16n(raw(required(&attrs, 1)?)?)?;
    let version = number32(required(&attrs, 3)?)?;
    if id <= CTRL || version != 1 {
        return Err(Failure::Malformed);
    }
    let groups = nested(required(&attrs, 7)?)?;
    let mut scan_group = None;
    for group in groups {
        let fields = nested(group)?;
        let name = raw(required(&fields, 1)?)?;
        let value = number32(required(&fields, 2)?)?;
        if name == group_name {
            if scan_group.is_some() || value == 0 {
                return Err(Failure::Malformed);
            }
            scan_group = Some(value);
        }
    }
    Ok(Family {
        id,
        version: version as u8,
        group: scan_group.ok_or(Failure::Malformed)?,
    })
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Identity {
    ifindex: u32,
    wiphy: u32,
    wdev: u64,
}
fn interface(
    connection: &mut Connection,
    family: &Family,
    ifindex: u32,
    budget: &mut Budget,
) -> Result<Identity> {
    let mut query = Vec::new();
    put32(&mut query, IFINDEX, ifindex);
    let data = connection
        .single(
            family.id,
            family.version,
            GET_INTERFACE,
            NEW_INTERFACE,
            &query,
            budget,
        )
        .map_err(|e| {
            if matches!(e, Failure::Kernel(2 | 19 | 95)) {
                Failure::Unsupported
            } else {
                e
            }
        })?;
    let attrs = attributes(&data)?;
    let identity = identity(&attrs)?;
    if identity.ifindex != ifindex
        || number32(required(&attrs, IFTYPE)?)? != 2
        || identity.wdev == 0
    {
        return Err(Failure::Identity);
    }
    Ok(identity)
}
fn identity(attrs: &[Attribute<'_>]) -> Result<Identity> {
    Ok(Identity {
        ifindex: number32(required(attrs, IFINDEX)?)?,
        wiphy: number32(required(attrs, WIPHY)?)?,
        wdev: number64(required(attrs, WDEV)?)?,
    })
}
fn event_command(frame: &Frame<'_>, family: &Family, selected: Identity) -> Result<Option<u8>> {
    if frame.kind != family.id || frame.seq != 0 || frame.port != 0 || frame.body.len() < 4 {
        return Err(Failure::Malformed);
    }
    let command = frame.body[0];
    let attrs = genl(frame, family.id, family.version, command)?;
    Ok((identity(&attrs)? == selected).then_some(command))
}
fn scan_event(frame: &Frame<'_>, family: &Family, selected: Identity) -> Result<bool> {
    match event_command(frame, family, selected)? {
        None | Some(TRIGGER_SCAN) => Ok(false),
        Some(NEW_SCAN_RESULTS) => Ok(true),
        Some(SCAN_ABORTED) => Err(Failure::Aborted),
        Some(_) => Err(Failure::Contended),
    }
}
fn reject_selected_event(frame: &Frame<'_>, family: &Family, selected: Identity) -> Result<()> {
    if event_command(frame, family, selected)?.is_some() {
        return Err(Failure::Contended);
    }
    Ok(())
}
fn require_quiet_events(
    events: &Connection,
    family: &Family,
    selected: Identity,
    budget: &mut Budget,
) -> Result<()> {
    for _ in 0..64 {
        match events.ready(budget)? {
            None => return Ok(()),
            Some(data) => {
                for frame in frames(&data)? {
                    reject_selected_event(&frame, family, selected)?;
                }
            }
        }
    }
    Err(Failure::Limit)
}
fn ssid(ies: &[u8]) -> Result<Vec<u8>> {
    let mut pos = 0;
    let mut found = None;
    while pos < ies.len() {
        if ies.len() - pos < 2 {
            return Err(Failure::Malformed);
        }
        let id = ies[pos];
        let size = usize::from(ies[pos + 1]);
        pos += 2;
        let value = ies.get(pos..pos + size).ok_or(Failure::Malformed)?;
        if id == 0 {
            if found.is_some() || size > 32 {
                return Err(Failure::Malformed);
            }
            found = Some(value.to_vec());
        }
        pos += size;
    }
    found.ok_or(Failure::Malformed)
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[usize::from(b >> 4)] as char);
        out.push(DIGITS[usize::from(b & 15)] as char);
    }
    out
}
fn bss(attrs: &[Attribute<'_>], frequency: u32, started: u64, observed: u64) -> Result<Bss> {
    let a = nested(required(attrs, BSS)?)?;
    let bssid = raw(required(&a, 1)?)?;
    if bssid.len() != 6 || bssid[0] & 1 != 0 || bssid == [0; 6] {
        return Err(Failure::Malformed);
    }
    let freq = number32(required(&a, 2)?)?;
    if freq != frequency || optional(&a, 20)?.map(number32).transpose()?.unwrap_or(0) != 0 {
        return Err(Failure::Identity);
    }
    let stamp = number64(required(&a, 15)?)?;
    if stamp == 0 || stamp < started || stamp > observed {
        return Err(Failure::Malformed);
    }
    let probe = optional(&a, 6)?
        .map(raw)
        .transpose()?
        .map(ssid)
        .transpose()?;
    let beacon = optional(&a, 11)?
        .map(raw)
        .transpose()?
        .map(ssid)
        .transpose()?;
    if let (Some(p), Some(b)) = (&probe, &beacon)
        && !b.is_empty()
        && p != b
    {
        return Err(Failure::Malformed);
    }
    let name = probe.or(beacon).ok_or(Failure::Malformed)?;
    Ok(Bss {
        bssid_hex: hex(bssid),
        ssid_hex: hex(&name),
        frequency_mhz: freq,
        last_seen_boottime_ns: stamp,
    })
}
fn dump(
    connection: &mut Connection,
    family: &Family,
    selected: Identity,
    request: Request,
    started: u64,
    budget: &mut Budget,
) -> Result<Vec<Bss>> {
    let mut query = Vec::new();
    put32(&mut query, IFINDEX, selected.ifindex);
    let packet = connection.request(family.id, family.version, GET_SCAN, DUMP, &query)?;
    connection.send(&packet, budget)?;
    let mut out: Vec<Bss> = Vec::new();
    let mut generation = None;
    for _ in 0..128 {
        let data = connection.receive(budget)?;
        let messages = frames(&data)?;
        for (i, frame) in messages.iter().enumerate() {
            connection.correlated(frame)?;
            if frame.kind == ERROR {
                return Err(error_reply(frame, &packet)?.ok_or(Failure::Malformed)?);
            }
            if frame.flags & MULTI == 0 {
                return Err(Failure::Malformed);
            }
            if frame.kind == DONE {
                if frame.body.len() != 4 || u32n(frame.body)? != 0 || i + 1 != messages.len() {
                    return Err(Failure::Loss);
                }
                connection.complete(DUMP);
                budget.fence()?;
                return Ok(out);
            }
            let attrs = genl(frame, family.id, family.version, NEW_SCAN_RESULTS)?;
            if number32(required(&attrs, IFINDEX)?)? != selected.ifindex
                || number64(required(&attrs, WDEV)?)? != selected.wdev
            {
                return Err(Failure::Identity);
            }
            let next = number32(required(&attrs, GENERATION)?)?;
            if generation
                .replace(next)
                .is_some_and(|previous| previous != next)
            {
                return Err(Failure::Loss);
            }
            let value = bss(&attrs, request.frequency_mhz, started, boottime_ns())?;
            if out.len() == MAX_BSS
                || out
                    .iter()
                    .any(|previous| previous.bssid_hex == value.bssid_hex)
            {
                return Err(Failure::Limit);
            }
            out.push(value);
        }
    }
    Err(Failure::Limit)
}
struct Namespace {
    file: File,
    dev: u64,
    ino: u64,
}
impl Namespace {
    fn retain() -> Result<Self> {
        let file = File::open("/proc/thread-self/ns/net").map_err(system)?;
        let m = file.metadata().map_err(system)?;
        Ok(Self {
            file,
            dev: m.dev(),
            ino: m.ino(),
        })
    }
    fn check(&self) -> Result<()> {
        let current = std::fs::metadata("/proc/thread-self/ns/net").map_err(system)?;
        let retained = self.file.metadata().map_err(system)?;
        if (current.dev(), current.ino()) != (self.dev, self.ino)
            || (retained.dev(), retained.ino()) != (self.dev, self.ino)
        {
            return Err(Failure::Identity);
        }
        Ok(())
    }
}
pub(crate) fn scan(request: Request, started: u64, dispatched: &mut bool) -> Result<Completed> {
    let mut budget = Budget::new(request.original_until_ns)?;
    let namespace = Namespace::retain()?;
    let mut command = Connection::new()?;
    let family = family(&mut command, &mut budget)?;
    let selected = interface(&mut command, &family, request.ifindex, &mut budget)?;
    let events = Connection::new()?;
    events.socket.add_membership(family.group).map_err(system)?;
    // No old notification can authorize this new scan. Draining is bounded;
    // exclusive fresh-radio custody still comes from the caller's fixture.
    require_quiet_events(&events, &family, selected, &mut budget)?;
    namespace.check()?;
    let mut attrs = Vec::new();
    put32(&mut attrs, IFINDEX, selected.ifindex);
    let mut frequency = Vec::new();
    put32(&mut frequency, 1, request.frequency_mhz);
    put(&mut attrs, FREQUENCIES | 0x8000, &frequency);
    let mut wildcard = Vec::new();
    put(&mut wildcard, 1, &[]);
    put(&mut attrs, SSIDS | 0x8000, &wildcard);
    put32(&mut attrs, SCAN_FLAGS, 2); // NL80211_SCAN_FLAG_FLUSH
    let packet = command.request(family.id, family.version, TRIGGER_SCAN, ACK, &attrs)?;
    budget.fence()?;
    // Conservatively latch before the sole syscall, including a blocked send.
    // Any later failure is uncertain; nothing here resends or aborts it.
    *dispatched = true;
    command.send(&packet, &budget)?;
    command.acknowledge(&packet, &mut budget)?;
    let mut completed = false;
    for _ in 0..128 {
        let data = events.receive(&mut budget)?;
        for frame in frames(&data)? {
            if scan_event(&frame, &family, selected)? {
                if completed {
                    return Err(Failure::Contended);
                }
                completed = true;
            }
        }
        if completed {
            break;
        }
    }
    if !completed {
        return Err(Failure::Limit);
    }
    let bss = dump(
        &mut command,
        &family,
        selected,
        request,
        started,
        &mut budget,
    )?;
    if interface(&mut command, &family, request.ifindex, &mut budget)? != selected {
        return Err(Failure::Identity);
    }
    namespace.check()?;
    // Refuse queued contradictions/loss after the dump as well as before it.
    // This checks observed events; it is not an ongoing ownership watch.
    require_quiet_events(&events, &family, selected, &mut budget)?;
    budget.fence()?;
    let completed_boottime_ns = boottime_ns();
    if completed_boottime_ns >= request.original_until_ns {
        return Err(Failure::Expired);
    }
    Ok(Completed {
        request,
        wiphy: selected.wiphy,
        wdev: selected.wdev,
        started_boottime_ns: started,
        completed_boottime_ns,
        bss,
    })
}

// Linux nl80211 open-system CONNECT and explicit DISCONNECT. MLME has no
// per-request cookie: callers must retain the fresh exclusive radio epoch.
const CONNECT: u8 = 46;
const DISCONNECT: u8 = 48;
const MAC: u16 = 6;
const SSID: u16 = 52;
const AUTH_TYPE: u16 = 53;
const REASON_CODE: u16 = 54;
const TIMED_OUT: u16 = 65;
const DISCONNECTED_BY_AP: u16 = 71;
const STATUS_CODE: u16 = 72;
const WIPHY_FREQ: u16 = 38;

#[derive(Clone, Copy)]
enum MlmePhase {
    Before,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
}

fn mlme_event(
    frame: &Frame<'_>,
    family: &Family,
    selected: Identity,
    phase: MlmePhase,
    peer: &[u8; 6],
) -> Result<bool> {
    if frame.kind != family.id
        || frame.seq != 0
        || frame.port != 0
        || frame.flags & MULTI != 0
        || frame.body.len() < 4
    {
        return Err(Failure::Malformed);
    }
    let command = frame.body[0];
    let attrs = genl(frame, family.id, family.version, command)?;
    let index = number32(required(&attrs, IFINDEX)?)?;
    // The shared MLME group includes other interfaces' notifications, whose
    // command-specific payload need not carry our selected identity fields.
    if index != selected.ifindex {
        return Ok(false);
    }
    let phy = optional(&attrs, WIPHY)?.map(number32).transpose()?;
    if phy.is_some_and(|value| value != selected.wiphy)
        || optional(&attrs, WDEV)?
            .map(number64)
            .transpose()?
            .is_some_and(|v| v != selected.wdev)
    {
        return Err(Failure::Identity);
    }
    if optional(&attrs, TIMED_OUT)?.is_some() {
        return Err(Failure::Aborted);
    }
    // Nonterminal MLME notifications have command-specific payloads. They can
    // neither complete this action nor require the terminal CONNECT schema.
    if command == 47 {
        // ROAM contradicts our exact restricted peer selection.
        return Err(Failure::Contended);
    }
    if (37..=40).contains(&command)
        && !matches!(
            (phase, command),
            (MlmePhase::Connecting, 37 | 38) | (MlmePhase::Disconnecting, 39 | 40)
        )
    {
        // Authentication/association or deauthentication/disassociation is
        // expected progress only during its explicitly requested transition.
        return Err(Failure::Contended);
    }
    if command != CONNECT && command != DISCONNECT {
        return Ok(false);
    }
    phy.ok_or(Failure::MissingAttribute(WIPHY))?;
    match (phase, command) {
        (MlmePhase::Connecting, CONNECT) => {
            if u16n(raw(required(&attrs, STATUS_CODE)?)?)? != 0 {
                return Err(Failure::Aborted);
            }
            if raw(required(&attrs, MAC)?)? != peer {
                return Err(Failure::Identity);
            }
            Ok(true)
        }
        (MlmePhase::Disconnecting, DISCONNECT) => {
            if optional(&attrs, MAC)?
                .map(raw)
                .transpose()?
                .is_some_and(|actual| actual != peer)
            {
                return Err(Failure::Identity);
            }
            if optional(&attrs, DISCONNECTED_BY_AP)?.is_some()
                || u16n(raw(required(&attrs, REASON_CODE)?)?)? != 3
            {
                return Err(Failure::Contended);
            }
            Ok(true)
        }
        _ => Err(Failure::Contended),
    }
}

fn quiet_mlme(
    events: &Connection,
    family: &Family,
    selected: Identity,
    phase: MlmePhase,
    peer: &[u8; 6],
    budget: &mut Budget,
) -> Result<()> {
    for _ in 0..64 {
        match events.ready(budget)? {
            None => return Ok(()),
            Some(data) => {
                for frame in frames(&data)? {
                    if mlme_event(&frame, family, selected, phase, peer)
                        .map_err(|f| mlme_context(f, frame.body.first().copied().unwrap_or(0)))?
                    {
                        return Err(Failure::Contended);
                    }
                }
            }
        }
    }
    Err(Failure::Limit)
}

fn await_mlme(
    events: &Connection,
    family: &Family,
    selected: Identity,
    connecting: bool,
    peer: &[u8; 6],
    budget: &mut Budget,
) -> Result<()> {
    let mut completed = false;
    for _ in 0..128 {
        let data = events.receive(budget)?;
        for frame in frames(&data)? {
            let phase = match (connecting, completed) {
                (true, false) => MlmePhase::Connecting,
                (true, true) => MlmePhase::Connected,
                (false, false) => MlmePhase::Disconnecting,
                (false, true) => MlmePhase::Disconnected,
            };
            if mlme_event(&frame, family, selected, phase, peer)
                .map_err(|f| mlme_context(f, frame.body.first().copied().unwrap_or(0)))?
            {
                completed = true;
            }
        }
        if completed {
            return Ok(());
        }
    }
    Err(Failure::Limit)
}

#[derive(Debug, Eq, PartialEq)]
struct AssociatedBss {
    mac: [u8; 6],
    ssid: Vec<u8>,
    frequency: u32,
}

fn associated_bss(attrs: &[Attribute<'_>]) -> Result<Option<AssociatedBss>> {
    let fields = nested(required(attrs, BSS)?)?;
    let status = optional(&fields, 9)?.map(number32).transpose()?;
    let Some(status) = status else {
        return Ok(None);
    };
    if status != 1 {
        // NL80211_BSS_STATUS_ASSOCIATED, not merely authenticated.
        return Err(Failure::Contended);
    }
    let mac: [u8; 6] = raw(required(&fields, 1)?)?
        .try_into()
        .map_err(|_| Failure::Malformed)?;
    if mac == [0; 6] || mac[0] & 1 != 0 {
        return Err(Failure::Malformed);
    }
    let frequency = number32(required(&fields, 2)?)?;
    if frequency == 0
        || optional(&fields, 20)?
            .map(number32)
            .transpose()?
            .unwrap_or(0)
            != 0
    {
        return Err(Failure::Identity);
    }
    if u16n(raw(required(&fields, 5)?)?)? & 0x10 != 0 {
        return Err(Failure::Identity); // This primitive admits no protected BSS.
    }
    let probe = optional(&fields, 6)?
        .map(raw)
        .transpose()?
        .map(ssid)
        .transpose()?;
    let beacon = optional(&fields, 11)?
        .map(raw)
        .transpose()?
        .map(ssid)
        .transpose()?;
    if let (Some(p), Some(b)) = (&probe, &beacon)
        && !b.is_empty()
        && p != b
    {
        return Err(Failure::Malformed);
    }
    Ok(Some(AssociatedBss {
        mac,
        ssid: probe.or(beacon).ok_or(Failure::Malformed)?,
        frequency,
    }))
}

fn association_readback(
    connection: &mut Connection,
    family: &Family,
    selected: Identity,
    budget: &mut Budget,
) -> Result<Option<AssociatedBss>> {
    let mut query = Vec::new();
    put32(&mut query, IFINDEX, selected.ifindex);
    let packet = connection.request(family.id, family.version, GET_SCAN, DUMP, &query)?;
    connection.send(&packet, budget)?;
    let mut associated = None;
    let mut generation = None;
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for _ in 0..128 {
        let data = connection.receive(budget)?;
        let messages = frames(&data)?;
        for (i, frame) in messages.iter().enumerate() {
            connection.correlated(frame)?;
            if frame.kind == ERROR {
                return Err(error_reply(frame, &packet)?.ok_or(Failure::Malformed)?);
            }
            if frame.flags & MULTI == 0 {
                return Err(Failure::Malformed);
            }
            if frame.kind == DONE {
                if frame.body.len() != 4 || u32n(frame.body)? != 0 || i + 1 != messages.len() {
                    return Err(Failure::Loss);
                }
                connection.complete(DUMP);
                budget.fence()?;
                return Ok(associated);
            }
            let attrs = genl(frame, family.id, family.version, NEW_SCAN_RESULTS)?;
            if number32(required(&attrs, IFINDEX)?)? != selected.ifindex
                || number64(required(&attrs, WDEV)?)? != selected.wdev
            {
                return Err(Failure::Identity);
            }
            let next = number32(required(&attrs, GENERATION)?)?;
            if generation
                .replace(next)
                .is_some_and(|previous| previous != next)
            {
                return Err(Failure::Loss);
            }
            let fields = nested(required(&attrs, BSS)?)?;
            let mac = raw(required(&fields, 1)?)?;
            if mac.len() != 6 || seen.len() == MAX_BSS || seen.iter().any(|v| v == mac) {
                return Err(Failure::Limit);
            }
            seen.push(mac.to_vec());
            if let Some(value) = associated_bss(&attrs)?
                && associated.replace(value).is_some()
            {
                return Err(Failure::Contended);
            }
        }
    }
    Err(Failure::Limit)
}

// Fixed contexts:1 ACK,2 MLME,3 associated dump,4 interface readback.
// Command0 means a non-MLME stage. No raw payload or refreshed deadline.
fn association_context(failure: Failure, stage: u8) -> Failure {
    match failure {
        Failure::Malformed => Failure::MalformedAssociation(stage),
        Failure::MissingAttribute(attribute) => Failure::MissingAssociationAttribute {
            stage,
            command: 0,
            attribute,
        },
        other => other,
    }
}
fn mlme_context(failure: Failure, command: u8) -> Failure {
    match failure {
        Failure::MissingAttribute(attribute) => Failure::MissingAssociationAttribute {
            stage: 2,
            command,
            attribute,
        },
        other => association_context(other, 2),
    }
}

fn valid_radio_cutoff(started: u64, until: u64) -> bool {
    until > started && until - started <= 1_000_000_000
}

fn check_radio_association(
    target: &RadioTarget,
    expected: bool,
    actual: Option<&AssociatedBss>,
) -> Result<()> {
    match (expected, actual) {
        (false, None) => Ok(()),
        (true, Some(peer))
            if peer.mac == target.bssid
                && peer.ssid == target.ssid
                && peer.frequency == target.frequency_mhz =>
        {
            Ok(())
        }
        _ => Err(Failure::Identity),
    }
}

// Retirement is permanent. Keeping the original error also prevents a caller
// from mistaking a later pre-dispatch refusal for resolved mutation uncertainty.
struct RadioState {
    thread: std::thread::ThreadId,
    associated: bool,
    retired: Option<Error>,
    refresh_failure_context: Option<RadioRefreshDiagnostic>,
}
impl RadioState {
    fn new() -> Self {
        Self {
            thread: std::thread::current().id(),
            associated: false,
            retired: None,
            refresh_failure_context: None,
        }
    }
    fn check_thread(&mut self) -> std::result::Result<(), Error> {
        if let Some(error) = self.retired {
            return Err(error);
        }
        if self.thread != std::thread::current().id() {
            return Err(self.fail(Failure::Identity, false));
        }
        Ok(())
    }
    fn fail(&mut self, failure: Failure, dispatched: bool) -> Error {
        *self
            .retired
            .get_or_insert_with(|| crate::classify_failure(failure, dispatched))
    }
    fn fail_refresh(&mut self, failure: Failure, context: RadioRefreshDiagnostic) -> Error {
        if self.retired.is_none() {
            self.refresh_failure_context = Some(context);
        }
        self.fail(failure, false)
    }
}

/// Persistent nl80211 custody for one exclusively administered open radio.
///
/// The caller supplies fresh-radio exclusivity; kernel identity alone cannot
/// establish it. The namespace handle, constructing thread, command socket,
/// subscribed MLME socket and exact interface/peer remain pinned until drop.
/// Runtime loss, contradictory events, identity changes and uncertainty retire
/// this value permanently. Drop only closes handles; it sends no Disconnect.
pub struct RadioEpoch {
    namespace: Namespace,
    command: Connection,
    events: Connection,
    family: Family,
    selected: Identity,
    target: RadioTarget,
    state: RadioState,
    phase: RadioObservationPhase,
}
impl RadioEpoch {
    /// Attach without changing the radio, requiring an initially disconnected
    /// baseline. The original CLOCK_BOOTTIME cutoff must be at most one second.
    pub fn attach_disconnected(
        target: RadioTarget,
        until: u64,
    ) -> std::result::Result<Self, Error> {
        let started = boottime_ns();
        if !valid_radio_cutoff(started, until)
            || target.namespace_dev == 0
            || target.namespace_ino == 0
            || target.ifindex == 0
            || target.frequency_mhz == 0
            || target.frequency_mhz > 100_000
            || target.bssid == [0; 6]
            || target.bssid[0] & 1 != 0
            || target.ssid.is_empty()
            || target.ssid.len() > 32
        {
            return Err(Error::InvalidRequest);
        }
        Self::attach_inner(target, until).map_err(|failure| crate::classify_failure(failure, false))
    }
    fn attach_inner(target: RadioTarget, until: u64) -> Result<Self> {
        let state = RadioState::new();
        let namespace = Namespace::retain()?;
        if (namespace.dev, namespace.ino) != (target.namespace_dev, target.namespace_ino) {
            return Err(Failure::Identity);
        }
        namespace.check()?;
        // Expected namespace identity is established before either socket opens.
        let mut budget = Budget::new(until)?;
        let mut command = Connection::new()?;
        let family = family_group(&mut command, &mut budget, b"mlme\0")?;
        let events = Connection::new()?;
        events.socket.add_membership(family.group).map_err(system)?;
        // Subscribe before even the initial interface and association baseline.
        let selected = interface(&mut command, &family, target.ifindex, &mut budget)
            .map_err(|f| association_context(f, 4))?;
        let mut epoch = Self {
            namespace,
            command,
            events,
            family,
            selected,
            target,
            state,
            phase: RadioObservationPhase::NamespaceBefore,
        };
        epoch.observe(false, &mut budget)?;
        Ok(epoch)
    }
    fn budget(&mut self, until: u64) -> std::result::Result<Budget, Error> {
        self.state.check_thread()?;
        if !valid_radio_cutoff(boottime_ns(), until) {
            return Err(Error::InvalidRequest);
        }
        Budget::new(until).map_err(|failure| self.state.fail(failure, false))
    }
    fn phase(associated: bool) -> MlmePhase {
        if associated {
            MlmePhase::Connected
        } else {
            MlmePhase::Disconnected
        }
    }
    fn quiet(&self, associated: bool, budget: &mut Budget) -> Result<()> {
        quiet_mlme(
            &self.events,
            &self.family,
            self.selected,
            Self::phase(associated),
            &self.target.bssid,
            budget,
        )
    }
    fn observe(&mut self, expected: bool, budget: &mut Budget) -> Result<RadioFact> {
        self.phase = RadioObservationPhase::NamespaceBefore;
        self.namespace.check()?;
        self.phase = RadioObservationPhase::QuietBefore;
        self.quiet(expected, budget)?;
        // A dump is not an atomic kernel snapshot. Conservatively date the fact
        // before the first applicable readback, and fence both ends with events.
        let observed_at_boottime_ns = boottime_ns();
        self.phase = RadioObservationPhase::InterfaceBefore;
        self.check_interface(budget)?;
        self.phase = RadioObservationPhase::AssociationDump;
        let actual = association_readback(&mut self.command, &self.family, self.selected, budget)
            .map_err(|f| association_context(f, 3))?;
        check_radio_association(&self.target, expected, actual.as_ref())?;
        self.phase = RadioObservationPhase::InterfaceAfter;
        self.check_interface(budget)?;
        self.phase = RadioObservationPhase::QuietAfter;
        self.quiet(expected, budget)?;
        self.phase = RadioObservationPhase::NamespaceAfter;
        self.namespace.check()?;
        self.phase = RadioObservationPhase::FinalFence;
        budget.fence()?;
        Ok(RadioFact {
            associated: expected,
            observed_at_boottime_ns,
            wiphy: self.selected.wiphy,
            wdev: self.selected.wdev,
        })
    }
    fn check_interface(&mut self, budget: &mut Budget) -> Result<()> {
        if interface(&mut self.command, &self.family, self.target.ifindex, budget)
            .map_err(|f| association_context(f, 4))?
            != self.selected
        {
            return Err(Failure::Identity);
        }
        Ok(())
    }
    /// Refresh actual radio state without replacing retained identity or sockets.
    pub fn refresh(&mut self, until: u64) -> std::result::Result<RadioFact, Error> {
        let mut budget = self.budget(until)?;
        self.observe(self.state.associated, &mut budget)
            .map_err(|failure| {
                self.state.fail_refresh(
                    failure,
                    RadioRefreshDiagnostic {
                        phase: self.phase,
                        transaction: self.command.transaction.get(),
                    },
                )
            })
    }
    /// Snapshot of the first failed refresh that reached observation. Admission
    /// failures may have no context. This grants no freshness or retry authority.
    pub fn refresh_failure_context(&self) -> Option<RadioRefreshDiagnostic> {
        self.state.refresh_failure_context
    }
    /// Make at most one explicitly requested CONNECT or DISCONNECT. A repeated
    /// request for the confirmed state only refreshes readback. No retry, rollback,
    /// automatic disconnect or socket replacement occurs after any failure.
    pub fn set_associated(
        &mut self,
        associated: bool,
        until: u64,
    ) -> std::result::Result<RadioFact, Error> {
        let mut budget = self.budget(until)?;
        let mut dispatched = false;
        let result = self.set_inner(associated, &mut budget, &mut dispatched);
        match result {
            Ok(fact) => {
                self.state.associated = associated;
                Ok(fact)
            }
            Err(failure) => Err(self.state.fail(failure, dispatched)),
        }
    }
    fn set_inner(
        &mut self,
        associated: bool,
        budget: &mut Budget,
        dispatched: &mut bool,
    ) -> Result<RadioFact> {
        let before = self.observe(self.state.associated, budget)?;
        if associated == self.state.associated {
            return Ok(before);
        }
        let mut attrs = Vec::new();
        put32(&mut attrs, IFINDEX, self.selected.ifindex);
        put(&mut attrs, WDEV, &self.selected.wdev.to_ne_bytes());
        if associated {
            put(&mut attrs, MAC, &self.target.bssid);
            put32(&mut attrs, WIPHY_FREQ, self.target.frequency_mhz);
            put(&mut attrs, SSID, &self.target.ssid);
            put32(&mut attrs, AUTH_TYPE, 0);
            put32(&mut attrs, 152, 0); // No background scan or SOCKET_OWNER.
        } else {
            put(&mut attrs, REASON_CODE, &3u16.to_ne_bytes());
        }
        let packet = self.command.request(
            self.family.id,
            self.family.version,
            if associated { CONNECT } else { DISCONNECT },
            ACK,
            &attrs,
        )?;
        self.namespace.check()?;
        self.quiet(self.state.associated, budget)?;
        budget.fence()?;
        // Set before the sole potentially admitted mutation syscall. All errors
        // after this line remain uncertain, even send/ACK/kernel errors.
        *dispatched = true;
        self.command.send(&packet, budget)?;
        self.command
            .acknowledge(&packet, budget)
            .map_err(|f| association_context(f, 1))?;
        await_mlme(
            &self.events,
            &self.family,
            self.selected,
            associated,
            &self.target.bssid,
            budget,
        )
        .map_err(|f| association_context(f, 2))?;
        self.observe(associated, budget)
    }
}

pub(crate) fn associate(
    request: crate::AssociationRequest,
    started: u64,
    dispatched: &mut bool,
) -> Result<crate::Associated> {
    let mut budget = Budget::new(request.original_until_ns)?;
    let namespace = Namespace::retain()?;
    let mut command = Connection::new()?;
    let family = family_group(&mut command, &mut budget, b"mlme\0")?;
    let selected = interface(&mut command, &family, request.ifindex, &mut budget)
        .map_err(|f| association_context(f, 4))?;
    let events = Connection::new()?;
    events.socket.add_membership(family.group).map_err(system)?;
    quiet_mlme(
        &events,
        &family,
        selected,
        MlmePhase::Before,
        &request.bssid,
        &mut budget,
    )?;
    if association_readback(&mut command, &family, selected, &mut budget)
        .map_err(|f| association_context(f, 3))?
        .is_some()
    {
        return Err(Failure::Contended);
    }
    namespace.check()?;
    let mut attrs = Vec::new();
    put32(&mut attrs, IFINDEX, selected.ifindex);
    put(&mut attrs, WDEV, &selected.wdev.to_ne_bytes());
    put(&mut attrs, MAC, &request.bssid);
    put32(&mut attrs, WIPHY_FREQ, request.frequency_mhz);
    put(&mut attrs, SSID, &request.ssid);
    put32(&mut attrs, AUTH_TYPE, 0); // NL80211_AUTHTYPE_OPEN_SYSTEM.
    put32(&mut attrs, 152, 0); // NL80211_ATTR_BG_SCAN_PERIOD: no background scan.
    // No SOCKET_OWNER: worker exit must preserve a confirmed association.
    let packet = command.request(family.id, family.version, CONNECT, ACK, &attrs)?;
    budget.fence()?;
    *dispatched = true; // Conservative latch before the sole potentially blocked send.
    command.send(&packet, &budget)?;
    command
        .acknowledge(&packet, &mut budget)
        .map_err(|f| association_context(f, 1))?;
    await_mlme(
        &events,
        &family,
        selected,
        true,
        &request.bssid,
        &mut budget,
    )
    .map_err(|f| association_context(f, 2))?;
    let actual = association_readback(&mut command, &family, selected, &mut budget)
        .map_err(|f| association_context(f, 3))?;
    if actual
        != Some(AssociatedBss {
            mac: request.bssid,
            ssid: request.ssid.clone(),
            frequency: request.frequency_mhz,
        })
    {
        return Err(Failure::Identity);
    }
    if interface(&mut command, &family, request.ifindex, &mut budget)
        .map_err(|f| association_context(f, 4))?
        != selected
    {
        return Err(Failure::Identity);
    }
    namespace.check()?;
    quiet_mlme(
        &events,
        &family,
        selected,
        MlmePhase::Connected,
        &request.bssid,
        &mut budget,
    )?;
    budget.fence()?;
    let completed_boottime_ns = boottime_ns();
    if completed_boottime_ns >= request.original_until_ns {
        return Err(Failure::Expired);
    }
    Ok(crate::Associated {
        request,
        wiphy: selected.wiphy,
        wdev: selected.wdev,
        started_boottime_ns: started,
        completed_boottime_ns,
    })
}

pub(crate) fn disconnect(
    request: crate::DisconnectRequest,
    started: u64,
    dispatched: &mut bool,
) -> Result<crate::Disconnected> {
    let mut budget = Budget::new(request.original_until_ns)?;
    let namespace = Namespace::retain()?;
    let mut command = Connection::new()?;
    let family = family_group(&mut command, &mut budget, b"mlme\0")?;
    let selected = interface(&mut command, &family, request.ifindex, &mut budget)
        .map_err(|f| association_context(f, 4))?;
    let events = Connection::new()?;
    events.socket.add_membership(family.group).map_err(system)?;
    let peer = association_readback(&mut command, &family, selected, &mut budget)
        .map_err(|f| association_context(f, 3))?
        .ok_or(Failure::Identity)?
        .mac;
    quiet_mlme(
        &events,
        &family,
        selected,
        MlmePhase::Before,
        &peer,
        &mut budget,
    )?;
    namespace.check()?;
    let mut attrs = Vec::new();
    put32(&mut attrs, IFINDEX, selected.ifindex);
    put(&mut attrs, WDEV, &selected.wdev.to_ne_bytes());
    put(&mut attrs, REASON_CODE, &3u16.to_ne_bytes());
    let packet = command.request(family.id, family.version, DISCONNECT, ACK, &attrs)?;
    budget.fence()?;
    *dispatched = true;
    command.send(&packet, &budget)?;
    command
        .acknowledge(&packet, &mut budget)
        .map_err(|f| association_context(f, 1))?;
    await_mlme(&events, &family, selected, false, &peer, &mut budget)?;
    if association_readback(&mut command, &family, selected, &mut budget)
        .map_err(|f| association_context(f, 3))?
        .is_some()
        || interface(&mut command, &family, request.ifindex, &mut budget)
            .map_err(|f| association_context(f, 4))?
            != selected
    {
        return Err(Failure::Identity);
    }
    namespace.check()?;
    quiet_mlme(
        &events,
        &family,
        selected,
        MlmePhase::Disconnected,
        &peer,
        &mut budget,
    )?;
    budget.fence()?;
    let completed_boottime_ns = boottime_ns();
    if completed_boottime_ns >= request.original_until_ns {
        return Err(Failure::Expired);
    }
    Ok(crate::Disconnected {
        request,
        wiphy: selected.wiphy,
        wdev: selected.wdev,
        started_boottime_ns: started,
        completed_boottime_ns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nlctrl_query() -> Vec<u8> {
        let mut query = Vec::new();
        // Query the built-in family by ID: no module-name lookup or autoload.
        put(&mut query, 1, &CTRL.to_ne_bytes());
        query
    }

    fn original_timer_expired(budget: &Budget) {
        let mut fds = [event::PollFd::new(&budget.timer, event::PollFlags::IN)];
        event::poll(
            &mut fds,
            Some(&event::Timespec {
                tv_sec: 1,
                tv_nsec: 0,
            }),
        )
        .unwrap();
        assert!(fds[0].revents().contains(event::PollFlags::IN));
        assert_eq!(budget.fence(), Err(Failure::Expired));
    }

    #[test]
    fn radio_diagnostic_real_nlctrl_expired_before_send_stays_not_sent() {
        let mut command = Connection::new().unwrap();
        let packet = command.request(CTRL, 2, 3, 0, &nlctrl_query()).unwrap();
        let budget = Budget::new(boottime_ns().saturating_sub(1)).unwrap();
        assert_eq!(command.send(&packet, &budget), Err(Failure::Expired));
        assert_eq!(command.transaction.get(), CommandTransaction::NotSent);
    }

    #[test]
    fn radio_diagnostic_real_nlctrl_expired_pending_reply_stays_pending() {
        let mut command = Connection::new().unwrap();
        let packet = command.request(CTRL, 2, 3, 0, &nlctrl_query()).unwrap();
        let mut budget = Budget::new(boottime_ns() + 100_000_000).unwrap();
        command.send(&packet, &budget).unwrap();
        original_timer_expired(&budget);
        // Pending describes the unvalidated transaction, not whether the
        // kernel has already queued its reply. Expiry never drains that reply.
        assert_eq!(command.receive(&mut budget), Err(Failure::Expired));
        assert_eq!(
            command.transaction.get(),
            CommandTransaction::Pending,
            "dispatched read-only query without terminal validation must be Pending"
        );
    }

    #[test]
    fn radio_diagnostic_real_nlctrl_completed_then_final_fence_stays_completed() {
        let mut command = Connection::new().unwrap();
        let mut budget = Budget::new(boottime_ns() + 100_000_000).unwrap();
        let data = command
            .single(CTRL, 2, 3, 1, &nlctrl_query(), &mut budget)
            .unwrap();
        let attrs = attributes(&data).unwrap();
        assert_eq!(
            u16n(raw(required(&attrs, 1).unwrap()).unwrap()).unwrap(),
            CTRL
        );
        assert_eq!(raw(required(&attrs, 2).unwrap()).unwrap(), b"nlctrl\0");
        original_timer_expired(&budget);
        assert_eq!(
            command.transaction.get(),
            CommandTransaction::Completed,
            "validated single reply without expected ACK or DONE must be Completed"
        );
    }

    fn radio_target() -> crate::RadioTarget {
        let namespace = Namespace::retain().unwrap();
        crate::RadioTarget {
            namespace_dev: namespace.dev,
            namespace_ino: namespace.ino,
            ifindex: 3,
            frequency_mhz: 2412,
            bssid: [2, 0, 0, 0, 0, 1],
            ssid: vec![0xff, 0, 0x80],
        }
    }
    #[test]
    fn radio_diagnostic_real_nlctrl_receiving_bytes_does_not_complete() {
        let mut command = Connection::new().unwrap();
        let packet = command.request(CTRL, 2, 3, 0, &nlctrl_query()).unwrap();
        let mut budget = Budget::new(boottime_ns() + 100_000_000).unwrap();
        command.send(&packet, &budget).unwrap();
        let data = command.receive(&mut budget).unwrap();
        let replies = frames(&data).unwrap();
        assert_eq!(replies.len(), 1);
        command.correlated(&replies[0]).unwrap();
        genl(&replies[0], CTRL, 2, 1).unwrap();
        assert_eq!(command.transaction.get(), CommandTransaction::Pending);
    }
    #[test]
    fn radio_diagnostic_completion_requires_matching_unambiguous_flags() {
        let mut command = Connection::new().unwrap();
        command.complete(0);
        assert_eq!(command.transaction.get(), CommandTransaction::NotSent);
        for flags in [ACK | DUMP, 0x8000] {
            command.request_flags = Some(flags);
            command.transaction.set(CommandTransaction::Pending);
            for terminal in [0, ACK, DUMP, flags] {
                command.complete(terminal);
                assert_eq!(command.transaction.get(), CommandTransaction::Pending);
            }
        }
        for flags in [0, ACK, DUMP] {
            command.request_flags = Some(flags);
            command.transaction.set(CommandTransaction::Pending);
            command.complete(flags);
            assert_eq!(command.transaction.get(), CommandTransaction::Completed);
        }
        command.transaction.set(CommandTransaction::Pending);
        command.request(CTRL, 2, 3, 0, &nlctrl_query()).unwrap();
        command.complete(0);
        assert_eq!(command.transaction.get(), CommandTransaction::Pending);
    }
    #[test]
    fn radio_diagnostic_first_refresh_context_preserves_retirement() {
        let first = RadioRefreshDiagnostic {
            phase: RadioObservationPhase::AssociationDump,
            transaction: CommandTransaction::Pending,
        };
        let later = RadioRefreshDiagnostic {
            phase: RadioObservationPhase::FinalFence,
            transaction: CommandTransaction::Completed,
        };
        let mut state = RadioState::new();
        assert_eq!(
            state.fail_refresh(Failure::Expired, first),
            Error::Failed(Failure::Expired)
        );
        assert_eq!(state.refresh_failure_context, Some(first));
        assert_eq!(
            state.fail_refresh(Failure::Loss, later),
            Error::Failed(Failure::Expired)
        );
        assert_eq!(state.refresh_failure_context, Some(first));
        assert_eq!(state.check_thread(), Err(Error::Failed(Failure::Expired)));
    }
    #[test]
    fn retained_radio_validates_original_one_second_cutoff_and_target_before_sockets() {
        assert!(valid_radio_cutoff(100, 1_000_000_100));
        for until in [0, 99, 100, 1_000_000_101] {
            assert!(!valid_radio_cutoff(100, until));
        }
        let mut target = radio_target();
        target.namespace_ino ^= 1;
        assert!(matches!(
            crate::RadioEpoch::attach_disconnected(target, boottime_ns() + 500_000_000),
            Err(crate::Error::Failed(Failure::Identity))
        ));
        for target in [
            crate::RadioTarget {
                ifindex: 0,
                ..radio_target()
            },
            crate::RadioTarget {
                namespace_ino: 0,
                ..radio_target()
            },
            crate::RadioTarget {
                frequency_mhz: 0,
                ..radio_target()
            },
            crate::RadioTarget {
                bssid: [1; 6],
                ..radio_target()
            },
            crate::RadioTarget {
                ssid: vec![],
                ..radio_target()
            },
        ] {
            assert!(matches!(
                crate::RadioEpoch::attach_disconnected(target, boottime_ns() + 500_000_000),
                Err(crate::Error::InvalidRequest)
            ));
        }
    }
    #[test]
    fn retained_radio_refuses_same_ifindex_with_changed_identity() {
        let mut body = mlme_bytes(CONNECT, [2; 6], 0);
        put(&mut body, WDEV, &5u64.to_ne_bytes());
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Connecting,
                &[2; 6]
            ),
            Err(Failure::Identity)
        );
    }
    #[test]
    fn retained_radio_observation_requires_the_exact_target_peer() {
        let target = radio_target();
        assert_eq!(check_radio_association(&target, false, None), Ok(()));
        assert_eq!(
            check_radio_association(&target, true, None),
            Err(Failure::Identity)
        );
        let peer = AssociatedBss {
            mac: target.bssid,
            ssid: target.ssid.clone(),
            frequency: 2412,
        };
        assert_eq!(check_radio_association(&target, true, Some(&peer)), Ok(()));
        assert_eq!(
            check_radio_association(&target, false, Some(&peer)),
            Err(Failure::Identity)
        );
        for peer in [
            AssociatedBss {
                mac: [2; 6],
                ssid: target.ssid.clone(),
                frequency: 2412,
            },
            AssociatedBss {
                mac: target.bssid,
                ssid: b"other".to_vec(),
                frequency: 2412,
            },
            AssociatedBss {
                mac: target.bssid,
                ssid: target.ssid.clone(),
                frequency: 2417,
            },
        ] {
            assert_eq!(
                check_radio_association(&target, true, Some(&peer)),
                Err(Failure::Identity)
            );
        }
    }
    #[test]
    fn retained_radio_failure_permanently_blocks_later_operations() {
        let mut state = RadioState::new();
        assert_eq!(state.check_thread(), Ok(()));
        assert_eq!(
            state.fail(Failure::Loss, true),
            crate::Error::Uncertain(Failure::Loss)
        );
        assert_eq!(
            state.check_thread(),
            Err(crate::Error::Uncertain(Failure::Loss))
        );
        // A later apparent success or a different failure cannot clear retirement.
        assert_eq!(
            state.fail(Failure::Expired, false),
            crate::Error::Uncertain(Failure::Loss)
        );
        assert_eq!(
            state.check_thread(),
            Err(crate::Error::Uncertain(Failure::Loss))
        );
    }
    #[test]
    fn retained_radio_constructing_thread_is_part_of_epoch_identity() {
        let mut state = RadioState::new();
        let state = std::thread::spawn(move || {
            assert_eq!(
                state.check_thread(),
                Err(crate::Error::Failed(Failure::Identity))
            );
            state
        })
        .join()
        .unwrap();
        let mut state = state;
        assert_eq!(
            state.check_thread(),
            Err(crate::Error::Failed(Failure::Identity))
        );
    }
    #[test]
    fn retained_radio_quiet_observation_refuses_roam_or_unrequested_disconnect() {
        for command in [47, DISCONNECT] {
            let body = mlme_bytes(command, [2; 6], 0);
            let frame = Frame {
                kind: 42,
                flags: 0,
                seq: 0,
                port: 0,
                body: &body,
            };
            assert_eq!(
                mlme_event(
                    &frame,
                    &family_id(),
                    selected(),
                    MlmePhase::Connected,
                    &[2; 6]
                ),
                Err(Failure::Contended)
            );
        }
    }
    #[test]
    fn retained_radio_rejects_unsolicited_authentication_and_link_loss_events() {
        for command in [37, 38, 39, 40] {
            for phase in [MlmePhase::Connected, MlmePhase::Disconnected] {
                let body = mlme_bytes(command, [2; 6], 0);
                let frame = Frame {
                    kind: 42,
                    flags: 0,
                    seq: 0,
                    port: 0,
                    body: &body,
                };
                assert_eq!(
                    mlme_event(&frame, &family_id(), selected(), phase, &[2; 6]),
                    Err(Failure::Contended)
                );
            }
        }
    }
    #[test]
    fn disconnect_notification_cannot_name_a_different_peer() {
        let mut body = mlme_bytes(DISCONNECT, [2; 6], 0);
        put(&mut body, REASON_CODE, &3u16.to_ne_bytes());
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Disconnecting,
                &[4; 6]
            ),
            Err(Failure::Identity)
        );
    }
    fn selected() -> Identity {
        Identity {
            ifindex: 3,
            wiphy: 1,
            wdev: 0x100000001,
        }
    }
    fn family_id() -> Family {
        Family {
            id: 42,
            version: 1,
            group: 9,
        }
    }
    fn event_bytes(command: u8) -> Vec<u8> {
        let mut out = vec![command, 1, 0, 0];
        put32(&mut out, IFINDEX, 3);
        put32(&mut out, WIPHY, 1);
        put(&mut out, WDEV, &0x100000001u64.to_ne_bytes());
        out
    }
    fn bss_bytes() -> Vec<u8> {
        let mut fields = Vec::new();
        put(&mut fields, 1, &[2, 0, 0, 0, 0, 0]);
        put32(&mut fields, 2, 2412);
        put(&mut fields, 15, &150u64.to_ne_bytes());
        put(&mut fields, 6, &[0, 3, 0xff, 0, 0x80]);
        let mut out = Vec::new();
        put(&mut out, BSS | 0x8000, &fields);
        out
    }
    #[test]
    fn duplicate_selected_attributes_refuse_while_repeated_padding_is_valid() {
        let mut bytes = Vec::new();
        put32(&mut bytes, IFINDEX, 3);
        put(&mut bytes, 229, &[]);
        put(&mut bytes, 229, &[]);
        assert_eq!(
            number32(required(&attributes(&bytes).unwrap(), IFINDEX).unwrap()),
            Ok(3)
        );
        put32(&mut bytes, IFINDEX, 3);
        assert!(required(&attributes(&bytes).unwrap(), IFINDEX).is_err());
    }
    #[test]
    fn attribute_lengths_and_network_order_flags_refuse() {
        assert!(attributes(&[3, 0, 1, 0]).is_err());
        assert!(attributes(&[8, 0, 1, 0, 7]).is_err());
        let a = Attribute {
            kind: IFINDEX,
            flags: 0x4000,
            body: &3u32.to_ne_bytes(),
        };
        assert!(number32(a).is_err());
    }
    #[test]
    fn truncated_and_interrupted_datagrams_refuse() {
        let mut p = vec![0; 16];
        p[..4].copy_from_slice(&20u32.to_ne_bytes());
        assert!(frames(&p).is_err());
        p[..4].copy_from_slice(&16u32.to_ne_bytes());
        p[6..8].copy_from_slice(&DUMP_INTR.to_ne_bytes());
        assert!(matches!(frames(&p), Err(Failure::Loss)));
        p[6..8].fill(0);
        p[4..6].copy_from_slice(&OVERRUN.to_ne_bytes());
        assert!(matches!(frames(&p), Err(Failure::Loss)));
    }
    #[test]
    fn ack_requires_exact_echo_and_never_turns_kernel_error_into_success() {
        let request = vec![7; 20];
        let mut body = vec![0; 4];
        body.extend_from_slice(&request[..16]);
        let f = Frame {
            kind: ERROR,
            flags: 0,
            seq: 1,
            port: 0,
            body: &body,
        };
        assert_eq!(error_reply(&f, &request), Ok(None));
        body[0..4].copy_from_slice(&(-95i32).to_ne_bytes());
        let f = Frame {
            kind: ERROR,
            flags: 0,
            seq: 1,
            port: 0,
            body: &body,
        };
        assert_eq!(error_reply(&f, &request), Ok(Some(Failure::Kernel(95))));
        body[4] ^= 1;
        let f = Frame {
            kind: ERROR,
            flags: 0,
            seq: 1,
            port: 0,
            body: &body,
        };
        assert_eq!(error_reply(&f, &request), Err(Failure::Malformed));
    }
    #[test]
    fn start_is_not_completion_and_abort_is_failure() {
        for (cmd, want) in [
            (TRIGGER_SCAN, Ok(false)),
            (NEW_SCAN_RESULTS, Ok(true)),
            (SCAN_ABORTED, Err(Failure::Aborted)),
        ] {
            let bytes = event_bytes(cmd);
            let f = Frame {
                kind: 42,
                flags: 0,
                seq: 0,
                port: 0,
                body: &bytes,
            };
            assert_eq!(scan_event(&f, &family_id(), selected()), want);
        }
    }
    #[test]
    fn preflight_and_terminal_checks_refuse_any_selected_scan_event() {
        for cmd in [TRIGGER_SCAN, NEW_SCAN_RESULTS, SCAN_ABORTED] {
            let bytes = event_bytes(cmd);
            let frame = Frame {
                kind: 42,
                flags: 0,
                seq: 0,
                port: 0,
                body: &bytes,
            };
            assert_eq!(
                reject_selected_event(&frame, &family_id(), selected()),
                Err(Failure::Contended)
            );
            assert_eq!(
                reject_selected_event(
                    &frame,
                    &family_id(),
                    Identity {
                        wdev: 5,
                        ..selected()
                    }
                ),
                Ok(())
            );
        }
    }
    #[test]
    fn multicast_is_not_correlated_by_our_request_sequence() {
        let body = event_bytes(NEW_SCAN_RESULTS);
        let f = Frame {
            kind: 42,
            flags: 0,
            seq: 9,
            port: 0,
            body: &body,
        };
        assert_eq!(
            scan_event(&f, &family_id(), selected()),
            Err(Failure::Malformed)
        );
        let other = Identity {
            wdev: 5,
            ..selected()
        };
        let f = Frame { seq: 0, ..f };
        assert_eq!(scan_event(&f, &family_id(), other), Ok(false));
    }
    #[test]
    fn binary_ssid_is_preserved_and_duplicate_or_truncated_ie_refuses() {
        let bytes = bss_bytes();
        let result = bss(&attributes(&bytes).unwrap(), 2412, 100, 200).unwrap();
        assert_eq!(result.ssid_hex, "ff0080");
        assert_eq!(result.bssid_hex, "020000000000");
        assert!(ssid(&[0, 1, 3, 0, 1, 4]).is_err());
        assert!(ssid(&[0, 3, 1]).is_err());
        assert!(ssid(&[1, 1, 7]).is_err());
    }
    #[test]
    fn bss_timestamp_cannot_be_reconstructed_or_widened() {
        let bytes = bss_bytes();
        let a = attributes(&bytes).unwrap();
        assert!(bss(&a, 2412, 151, 200).is_err());
        assert!(bss(&a, 2412, 100, 149).is_err());
        let mut fields = Vec::new();
        put(&mut fields, 1, &[2, 0, 0, 0, 0, 0]);
        put32(&mut fields, 2, 2412);
        put32(&mut fields, 10, 0);
        put(&mut fields, 6, &[0, 1, 7]);
        let mut outer = Vec::new();
        put(&mut outer, BSS | 0x8000, &fields);
        assert!(bss(&attributes(&outer).unwrap(), 2412, 100, 200).is_err());
    }
    #[test]
    fn fractional_frequency_is_not_truncated() {
        let bytes = bss_bytes();
        let mut fields = attributes(&bytes).unwrap()[0].body.to_vec();
        put32(&mut fields, 20, 1);
        let mut out = Vec::new();
        put(&mut out, BSS | 0x8000, &fields);
        assert_eq!(
            bss(&attributes(&out).unwrap(), 2412, 100, 200),
            Err(Failure::Identity)
        );
    }
    #[test]
    fn original_boottime_timer_expires_without_deadline_renewal() {
        let until = boottime_ns() + 20_000_000;
        let budget = Budget::new(until).unwrap();
        let mut fds = [event::PollFd::new(&budget.timer, event::PollFlags::IN)];
        event::poll(
            &mut fds,
            Some(&event::Timespec {
                tv_sec: 1,
                tv_nsec: 0,
            }),
        )
        .unwrap();
        assert!(fds[0].revents().contains(event::PollFlags::IN));
        assert_eq!(budget.fence(), Err(Failure::Expired));
        assert_eq!(budget.until, until);
    }
    #[test]
    fn invalid_request_refuses_before_netlink_effects() {
        assert_eq!(
            crate::scan_once(Request {
                ifindex: 0,
                frequency_mhz: 2412,
                original_until_ns: boottime_ns() + 1_000_000
            }),
            Err(crate::Error::InvalidRequest)
        );
        assert_eq!(
            crate::scan_once(Request {
                ifindex: 1,
                frequency_mhz: 2412,
                original_until_ns: boottime_ns().saturating_sub(1)
            }),
            Err(crate::Error::InvalidRequest)
        );
    }
    fn mlme_bytes(command: u8, peer: [u8; 6], status: u16) -> Vec<u8> {
        let mut out = vec![command, 1, 0, 0];
        put32(&mut out, IFINDEX, 3);
        put32(&mut out, WIPHY, 1);
        put(&mut out, MAC, &peer);
        put(&mut out, STATUS_CODE, &status.to_ne_bytes());
        out
    }
    #[test]
    fn connect_event_requires_success_exact_peer_and_no_timeout() {
        let peer = [2, 0, 0, 0, 0, 1];
        for (status, expected) in [(0, Ok(true)), (1, Err(Failure::Aborted))] {
            let body = mlme_bytes(CONNECT, peer, status);
            let frame = Frame {
                kind: 42,
                flags: 0,
                seq: 0,
                port: 0,
                body: &body,
            };
            assert_eq!(
                mlme_event(
                    &frame,
                    &family_id(),
                    selected(),
                    MlmePhase::Connecting,
                    &peer
                ),
                expected
            );
            if status == 0 {
                assert_eq!(
                    mlme_event(&frame, &family_id(), selected(), MlmePhase::Before, &peer),
                    Err(Failure::Contended)
                );
                assert_eq!(
                    mlme_event(
                        &frame,
                        &family_id(),
                        selected(),
                        MlmePhase::Connected,
                        &peer
                    ),
                    Err(Failure::Contended)
                );
                assert_eq!(
                    mlme_event(
                        &frame,
                        &family_id(),
                        selected(),
                        MlmePhase::Connecting,
                        &[2; 6]
                    ),
                    Err(Failure::Identity)
                );
            }
        }
        let mut body = mlme_bytes(CONNECT, peer, 0);
        put(&mut body, TIMED_OUT, &[]);
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Connecting,
                &peer
            ),
            Err(Failure::Aborted)
        );
        let body = mlme_bytes(37, peer, 0);
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Connecting,
                &peer
            ),
            Ok(false)
        );
    }
    #[test]
    fn disconnect_requires_local_reason_three_and_refuses_remote_drop() {
        let peer = [2; 6];
        let mut body = mlme_bytes(DISCONNECT, peer, 0);
        put(&mut body, REASON_CODE, &3u16.to_ne_bytes());
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Disconnecting,
                &peer
            ),
            Ok(true)
        );
        put(&mut body, DISCONNECTED_BY_AP, &[]);
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Disconnecting,
                &peer
            ),
            Err(Failure::Contended)
        );
    }
    #[test]
    fn associated_readback_needs_actual_status_and_open_exact_ies() {
        let mut fields = Vec::new();
        put(&mut fields, 1, &[2; 6]);
        put32(&mut fields, 2, 2412);
        put(&mut fields, 5, &0u16.to_ne_bytes());
        put(&mut fields, 6, &[0, 3, 0xff, 0, 0x80]);
        let mut outer = Vec::new();
        put(&mut outer, BSS | 0x8000, &fields);
        assert_eq!(associated_bss(&attributes(&outer).unwrap()), Ok(None));
        put32(&mut fields, 9, 1);
        outer.clear();
        put(&mut outer, BSS | 0x8000, &fields);
        assert_eq!(
            associated_bss(&attributes(&outer).unwrap()),
            Ok(Some(AssociatedBss {
                mac: [2; 6],
                ssid: vec![0xff, 0, 0x80],
                frequency: 2412
            }))
        );
        put32(&mut fields, 9, 1); // Duplicate status cannot authorize completion.
        outer.clear();
        put(&mut outer, BSS | 0x8000, &fields);
        assert_eq!(
            associated_bss(&attributes(&outer).unwrap()),
            Err(Failure::Malformed)
        );
    }
    #[test]
    fn unselected_mlme_interface_does_not_require_selected_wiphy() {
        let mut body = vec![19, 1, 0, 0]; // AP NEW_STATION may carry only IFINDEX.
        put32(&mut body, IFINDEX, 2);
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Connecting,
                &[2; 6]
            ),
            Ok(false)
        );
        body.clear();
        body.extend_from_slice(&[CONNECT, 1, 0, 0]);
        put32(&mut body, IFINDEX, 3);
        let frame = Frame {
            kind: 42,
            flags: 0,
            seq: 0,
            port: 0,
            body: &body,
        };
        assert_eq!(
            mlme_event(
                &frame,
                &family_id(),
                selected(),
                MlmePhase::Connecting,
                &[2; 6]
            ),
            Err(Failure::MissingAttribute(WIPHY))
        );
    }
    #[test]
    fn selected_nonterminal_progress_neither_requires_terminal_wiphy_nor_completes() {
        for (command, phase) in [
            (37, MlmePhase::Connecting),
            (38, MlmePhase::Connecting),
            (39, MlmePhase::Disconnecting),
            (40, MlmePhase::Disconnecting),
            (125, MlmePhase::Connected),
        ] {
            let mut body = vec![command, 1, 0, 0];
            put32(&mut body, IFINDEX, 3);
            let frame = Frame {
                kind: 42,
                flags: 0,
                seq: 0,
                port: 0,
                body: &body,
            };
            assert_eq!(
                mlme_event(&frame, &family_id(), selected(), phase, &[2; 6]),
                Ok(false)
            );
        }
    }
}
