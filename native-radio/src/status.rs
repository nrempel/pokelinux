//! Restricted service-account diagnostics; never cellular state or app authority.
//! Caller provisions an existing service-owned0700 runtime directory. No path
//! traversal/containment claim, stale-path removal, client loop or reaper lives here.
#![forbid(unsafe_code)]
use crate::{
    observation::{self, Frame, Kind},
    receiver::ReceiverFailure,
};
use pokelinux_kernel_identity::socket_peer_pidfd;
use pokelinux_process_owner::supervisor::ProcessIdentity;
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType, UCred,
    },
    process,
};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::unix::fs::PermissionsExt,
    path::Path,
};

const HEADER: usize = 12;
const FIXED: usize = 44;
pub const MAX_PACKET: usize = FIXED + observation::MAX_FRAME_LEN;
const MAGIC: &[u8; 4] = b"RADS";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Initializing,
    ObservedActive,
    Unavailable,
    Stopping,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub phase: Phase,
    pub identity: Option<ProcessIdentity>,
    pub revision: u64,
    pub failure: Option<ReceiverFailure>,
    /// One historical transport observation, retained independently of local phase.
    pub last_observation: Option<Frame>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Status,
    Stop,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Status(Status),
    StopAck,
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}
fn failure(code: u8) -> io::Result<Option<ReceiverFailure>> {
    Ok(match code {
        0 => None,
        1 => Some(ReceiverFailure::MalformedCallback),
        2 => Some(ReceiverFailure::QueueFull),
        3 => Some(ReceiverFailure::ConsumerGone),
        4 => Some(ReceiverFailure::InternalFailure),
        5 => Some(ReceiverFailure::PublisherGone),
        6 => Some(ReceiverFailure::Cancelled),
        7 => Some(ReceiverFailure::ServiceDied),
        8 => Some(ReceiverFailure::MonitoringFailed),
        9 => Some(ReceiverFailure::Expired),
        _ => return Err(invalid()),
    })
}
pub fn failure_name(reason: ReceiverFailure) -> &'static str {
    match reason {
        ReceiverFailure::MalformedCallback => "malformed-callback",
        ReceiverFailure::QueueFull => "queue-full",
        ReceiverFailure::ConsumerGone => "consumer-gone",
        ReceiverFailure::InternalFailure => "internal-failure",
        ReceiverFailure::PublisherGone => "publisher-gone",
        ReceiverFailure::Cancelled => "cancelled",
        ReceiverFailure::ServiceDied => "service-died",
        ReceiverFailure::MonitoringFailed => "monitoring-failed",
        ReceiverFailure::Expired => "expired",
    }
}
impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Initializing => "initializing",
            Self::ObservedActive => "observed-active",
            Self::Unavailable => "unavailable",
            Self::Stopping => "stopping",
        }
    }
    fn code(self) -> u8 {
        match self {
            Self::Initializing => 1,
            Self::ObservedActive => 2,
            Self::Unavailable => 3,
            Self::Stopping => 4,
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 15) as usize] as char);
    }
    text
}
fn optional<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".into(), |value| value.to_string())
}
impl Status {
    fn validate(&self) -> io::Result<()> {
        if (self.identity.is_none() && (self.revision != 0 || self.last_observation.is_some()))
            || (self.phase == Phase::ObservedActive
                && (self.identity.is_none() || self.revision == 0))
            || (self.failure.is_some()
                && matches!(self.phase, Phase::Initializing | Phase::ObservedActive))
        {
            return Err(invalid());
        }
        if let Some(observed) = &self.last_observation {
            if !matches!(
                &observed.kind,
                Kind::Raw { .. } | Kind::RegistrationWire { .. }
            ) || Some(observed.identity) != self.identity
                || observed.attempt != 1
                || observed.revision == 0
                || observed.revision > self.revision
            {
                return Err(invalid());
            }
            // The same bounded canonical representation governs binary and JSON.
            let _ = observed.encode()?;
        }
        Ok(())
    }
    /// Decimal integers, fixed enum strings and hex only; no unescaped text input.
    pub fn json(&self, server_pid: u32) -> io::Result<String> {
        self.validate()?;
        let incarnation = self
            .identity
            .map_or_else(|| "null".into(), |id| format!("\"{}\"", id.hex()));
        let failure = self.failure.map_or_else(
            || "null".into(),
            |reason| format!("\"{}\"", failure_name(reason)),
        );
        let observed = match &self.last_observation {
            None => "null".into(),
            Some(frame) => match &frame.kind {
                Kind::Raw {
                    slot_id,
                    serial,
                    rilc_msg_id,
                    declared_length,
                    error,
                    payload,
                } => format!(
                    "{{\"kind\":\"raw\",\"revision\":{},\"attempt\":{},\"slot_id\":{},\"serial\":{},\"rilc_msg_id\":{},\"declared_length\":{},\"error\":{},\"payload_hex\":\"{}\"}}",
                    frame.revision,
                    frame.attempt,
                    slot_id,
                    serial,
                    rilc_msg_id,
                    declared_length,
                    error,
                    hex(payload)
                ),
                Kind::RegistrationWire {
                    slot_id,
                    message_id,
                    wire,
                } => format!(
                    concat!(
                        "{{\"kind\":\"registration-wire\",\"revision\":{},\"attempt\":{},\"slot_id\":{},\"message_id\":{},\"wire\":{{",
                        "\"registration_state_wire\":{},\"feature_wire\":{},\"feature_mask_wire\":{},\"status_code_wire\":{},",
                        "\"uri_hex\":\"{}\",\"reason_hex\":\"{}\",\"feature_mask_ext_wire\":{},\"sip_delegate_mask_wire\":{},\"feature_ext_wire\":{}}}}}"
                    ),
                    frame.revision,
                    frame.attempt,
                    slot_id,
                    message_id,
                    wire.registration_state_wire,
                    wire.feature_wire,
                    wire.feature_mask_wire,
                    wire.status_code_wire,
                    hex(wire.uri_bytes()),
                    hex(wire.reason_bytes()),
                    optional(wire.feature_mask_ext_wire),
                    optional(wire.sip_delegate_mask_wire),
                    optional(wire.feature_ext_wire)
                ),
                _ => return Err(invalid()),
            },
        };
        let json = format!(
            "{{\"server_pid\":{},\"phase\":\"{}\",\"failure\":{},\"incarnation\":{},\"revision\":{},\"last_observation\":{}}}\n",
            server_pid,
            self.phase.name(),
            failure,
            incarnation,
            self.revision,
            observed
        );
        if json.len() > 5120 {
            return Err(invalid());
        }
        Ok(json)
    }
}

impl Reply {
    /// Header magic/version1/type/reserved00/total-u32LE. Status body contains
    /// phase/failure/identity-present/reserved0, incarnation16, revision-u64LE,
    /// observation-frame-length-u32LE, and at most one exact Raw/Registration Frame.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(MAX_PACKET);
        bytes.extend(MAGIC);
        bytes.extend([1, 0, 0, 0]);
        bytes.extend([0; 4]);
        match self {
            Self::StopAck => bytes[5] = 2,
            Self::Status(status) => {
                status.validate()?;
                bytes[5] = 1;
                let raw = status
                    .last_observation
                    .as_ref()
                    .map(Frame::encode)
                    .transpose()?;
                bytes.extend([
                    status.phase.code(),
                    status.failure.map_or(0, |r| r as u8),
                    u8::from(status.identity.is_some()),
                    0,
                ]);
                bytes.extend(status.identity.map_or([0; 16], |id| *id.as_bytes()));
                bytes.extend(status.revision.to_le_bytes());
                bytes.extend((raw.as_ref().map_or(0, Vec::len) as u32).to_le_bytes());
                if let Some(raw) = raw {
                    bytes.extend(raw);
                }
            }
        }
        let length = bytes.len();
        if length > MAX_PACKET {
            return Err(invalid());
        }
        bytes[8..12].copy_from_slice(&(length as u32).to_le_bytes());
        Ok(bytes)
    }
}
/// One SEQPACKET packet must contain exactly one reply; no partial acceptance.
pub fn decode_reply(bytes: &[u8]) -> io::Result<Reply> {
    if bytes.len() < HEADER
        || bytes.len() > MAX_PACKET
        || &bytes[..4] != MAGIC
        || bytes[4] != 1
        || bytes[6..8] != [0, 0]
        || u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid())?) as usize
            != bytes.len()
    {
        return Err(invalid());
    }
    match bytes[5] {
        2 if bytes.len() == HEADER => Ok(Reply::StopAck),
        1 if bytes.len() >= FIXED => {
            let phase = match bytes[12] {
                1 => Phase::Initializing,
                2 => Phase::ObservedActive,
                3 => Phase::Unavailable,
                4 => Phase::Stopping,
                _ => return Err(invalid()),
            };
            let identity = match bytes[14] {
                0 if bytes[16..32] == [0; 16] => None,
                1 => Some(ProcessIdentity::from_bytes(
                    bytes[16..32].try_into().map_err(|_| invalid())?,
                )),
                _ => return Err(invalid()),
            };
            if bytes[15] != 0 {
                return Err(invalid());
            }
            let raw_len =
                u32::from_le_bytes(bytes[40..44].try_into().map_err(|_| invalid())?) as usize;
            if raw_len != bytes.len() - FIXED {
                return Err(invalid());
            }
            let status = Status {
                phase,
                identity,
                revision: u64::from_le_bytes(bytes[32..40].try_into().map_err(|_| invalid())?),
                failure: failure(bytes[13])?,
                last_observation: if raw_len == 0 {
                    None
                } else {
                    Some(observation::decode_exact(&bytes[FIXED..])?)
                },
            };
            status.validate()?;
            Ok(Reply::Status(status))
        }
        _ => Err(invalid()),
    }
}
fn command(bytes: &[u8]) -> io::Result<Command> {
    match bytes {
        [1] => Ok(Command::Status),
        [2] => Ok(Command::Stop),
        _ => Err(invalid()),
    }
}
fn socket() -> io::Result<OwnedFd> {
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    Ok(fd)
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Principal {
    uid: u32,
    gid: u32,
}
impl Principal {
    fn current() -> Self {
        Self {
            uid: process::geteuid().as_raw(),
            gid: process::getegid().as_raw(),
        }
    }
    fn accepts(self, peer: UCred) -> bool {
        peer.uid.as_raw() == self.uid && peer.gid.as_raw() == self.gid
    }
}
pub struct Listener {
    fd: OwnedFd,
    principal: Principal,
}
impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
impl Listener {
    /// Refuses any pre-existing socket path. Does not unlink on errors or Drop.
    pub fn bind(path: &Path) -> io::Result<Self> {
        let principal = Principal::current();
        let fd = socket()?;
        net::bind(&fd, &SocketAddrUnix::new(path)?)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        net::listen(&fd, 8)?;
        Ok(Self { fd, principal })
    }
    pub fn accept(&self) -> io::Result<Option<Connection>> {
        if self.principal != Principal::current() {
            return Err(denied());
        }
        let fd = match net::accept_with(&self.fd, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
            Err(rustix::io::Errno::AGAIN) => return Ok(None),
            value => value?,
        };
        net::sockopt::set_socket_passcred(&fd, true)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        if !self.principal.accepts(peer) {
            return Err(denied());
        }
        Ok(Some(Connection { fd, peer }))
    }
}
pub struct Connection {
    fd: OwnedFd,
    peer: UCred,
}
impl AsFd for Connection {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
impl Connection {
    pub fn peer(&self) -> UCred {
        self.peer
    }
    pub fn receive(&self) -> io::Result<Option<Command>> {
        receive(&self.fd, self.peer)?
            .map(|bytes| command(&bytes))
            .transpose()
    }
    /// Recompute a Status from current local state on each caller retry.
    /// Never queue an encoded historical response for later admission.
    pub fn send(&self, reply: &Reply) -> io::Result<()> {
        send(&self.fd, &reply.encode()?)
    }
}
fn receive(fd: impl AsFd, expected: UCred) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = [0; MAX_PACKET];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1), ScmRights(1))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let message = match net::recvmsg(
        fd,
        &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary,
        RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
    ) {
        Err(rustix::io::Errno::AGAIN) => return Ok(None),
        value => value?,
    };
    let mut invalid_message = message
        .flags
        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
    let mut cred = None;
    for entry in ancillary.drain() {
        match entry {
            RecvAncillaryMessage::ScmCredentials(value) if cred.is_none() => cred = Some(value),
            RecvAncillaryMessage::ScmRights(handles) => {
                for handle in handles {
                    drop(handle);
                }
                invalid_message = true;
            }
            _ => invalid_message = true,
        }
    }
    if invalid_message {
        return Err(invalid());
    }
    if cred != Some(expected) {
        return Err(denied());
    }
    if message.bytes == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(Some(bytes[..message.bytes].to_vec()))
}
fn send(fd: impl AsFd, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET {
        return Err(invalid());
    }
    let count = net::send(fd, bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL)?;
    if count != bytes.len() {
        return Err(io::ErrorKind::WriteZero.into());
    }
    Ok(())
}
/// One-shot diagnostics client; retains the exact server socket-origin pidfd.
pub struct Client {
    connection: Connection,
    server: OwnedFd,
}
impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.connection.as_fd()
    }
}
impl Client {
    /// Nonblocking attempt. Caller may retry EAGAIN/INTR under its original budget.
    pub fn connect(path: &Path) -> io::Result<Self> {
        let fd = socket()?;
        net::connect(&fd, &SocketAddrUnix::new(path)?)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        if !Principal::current().accepts(peer) {
            return Err(denied());
        }
        let server = socket_peer_pidfd(&fd)?;
        Ok(Self {
            connection: Connection { fd, peer },
            server,
        })
    }
    pub fn server_pid(&self) -> u32 {
        self.connection.peer.pid.as_raw_pid() as u32
    }
    pub fn send(&self, command: Command) -> io::Result<()> {
        send(
            &self.connection.fd,
            &[match command {
                Command::Status => 1,
                Command::Stop => 2,
            }],
        )
    }
    pub fn receive(&self) -> io::Result<Option<Reply>> {
        receive(&self.connection.fd, self.connection.peer)?
            .map(|bytes| decode_reply(&bytes))
            .transpose()
    }
    /// Local process exit only: this client is neither the parent nor a reaper.
    pub fn server_exited(&self) -> io::Result<bool> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        let mut fds = [PollFd::new(&self.server, PollFlags::IN)];
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )?;
        let flags = fds[0].revents();
        if flags.intersects(PollFlags::ERR | PollFlags::NVAL) {
            return Err(invalid());
        }
        Ok(flags.contains(PollFlags::IN))
    }
    pub fn server_pidfd(&self) -> BorrowedFd<'_> {
        self.server.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn status() -> Status {
        let identity = ProcessIdentity::from_hex("0123456789abcdef0123456789abcdef").unwrap();
        Status {
            phase: Phase::Unavailable,
            identity: Some(identity),
            revision: 9,
            failure: Some(ReceiverFailure::ServiceDied),
            last_observation: Some(Frame {
                identity,
                attempt: 1,
                revision: 7,
                kind: Kind::Raw {
                    slot_id: -1,
                    serial: 7,
                    rilc_msg_id: 137,
                    declared_length: 100,
                    error: -19,
                    payload: vec![7; observation::MAX_PAYLOAD],
                },
            }),
        }
    }
    #[test]
    fn terminal_status_preserves_bounded_history_without_claiming_availability() {
        let status = status();
        let wire = Reply::Status(status.clone()).encode().unwrap();
        assert_eq!(wire.len(), MAX_PACKET);
        assert_eq!(decode_reply(&wire).unwrap(), Reply::Status(status.clone()));
        assert!(
            status
                .json(42)
                .unwrap()
                .contains("\"phase\":\"unavailable\"")
        );
        assert!(status.json(42).unwrap().len() <= 5120);
    }
    #[test]
    fn packet_boundaries_unknown_tags_reserved_and_lengths_fail_closed() {
        let wire = Reply::Status(status()).encode().unwrap();
        for end in 0..wire.len() {
            assert!(decode_reply(&wire[..end]).is_err());
        }
        let mut extra = wire.clone();
        extra.push(0);
        assert!(decode_reply(&extra).is_err());
        for (at, value) in [
            (4, 2),
            (5, 3),
            (6, 1),
            (12, 0),
            (13, 10),
            (14, 2),
            (15, 1),
            (40, 0),
        ] {
            let mut bad = wire.clone();
            bad[at] = value;
            assert!(decode_reply(&bad).is_err());
        }
        assert_eq!(
            decode_reply(&Reply::StopAck.encode().unwrap()).unwrap(),
            Reply::StopAck
        );
        assert!(command(&[]).is_err());
        assert!(command(&[1, 2]).is_err());
        assert!(command(&[3]).is_err());
    }
    #[test]
    fn actual_seqpacket_refuses_truncation_and_rights_without_releasing_data() {
        use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage};
        let (left, right) = net::socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&right, true).unwrap();
        let peer = net::sockopt::socket_peercred(&right).unwrap();
        let oversized = [1; MAX_PACKET + 1];
        net::send(&left, &oversized, SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap();
        assert_eq!(
            receive(&right, peer).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let file = std::fs::File::open("/dev/null").unwrap();
        let handles = [file.as_fd()];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut ancillary = SendAncillaryBuffer::new(&mut space);
        assert!(ancillary.push(SendAncillaryMessage::ScmRights(&handles)));
        net::sendmsg(
            &left,
            &[std::io::IoSlice::new(&[1])],
            &mut ancillary,
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        )
        .unwrap();
        assert_eq!(
            receive(&right, peer).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        net::send(&left, &[1], SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap();
        assert_eq!(receive(&right, peer).unwrap(), Some(vec![1]));
    }
    #[test]
    fn mismatched_history_or_live_terminal_contradiction_is_rejected() {
        let mut value = status();
        value.last_observation.as_mut().unwrap().revision = 10;
        assert!(Reply::Status(value).encode().is_err());
        let mut value = status();
        value.last_observation.as_mut().unwrap().kind = Kind::Active { client_id: 42 };
        assert!(Reply::Status(value).encode().is_err());
        let mut value = status();
        value.identity = None;
        assert!(Reply::Status(value).encode().is_err());
        let mut value = status();
        value.phase = Phase::ObservedActive;
        assert!(Reply::Status(value).encode().is_err());
        let mut value = status();
        value.last_observation.as_mut().unwrap().attempt = 2;
        assert!(Reply::Status(value).encode().is_err());
    }
}

#[cfg(test)]
#[path = "registration_status_tests.rs"]
mod registration_status_tests;
