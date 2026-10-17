//! Restricted service-account diagnostics; never cellular state or app authority.
//! Caller provisions an existing service-owned0700 runtime directory. No path
//! traversal/containment claim, stale-path removal, client loop or reaper lives here.
#![forbid(unsafe_code)]
use crate::{
    observation::{self, Frame, Kind},
    operator::{Admit, AdmitReply, RequestHistory},
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
use serde::{Deserialize, Deserializer};
use std::{
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::unix::fs::PermissionsExt,
    path::Path,
};

const HEADER: usize = 12;
pub const MAX_PACKET: usize = 16384;
pub const MAX_ADMIT: usize = 512;
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
    pub request_history: Option<RequestHistory>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Status,
    Stop,
    Admit(Admit),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reply {
    Status(Status),
    StopAck(Status),
    Admit(AdmitReply),
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
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
        if let Some(history) = &self.request_history {
            history.validate()?;
            if self.identity.map(ProcessIdentity::hex).as_deref()
                != Some(history.incarnation.as_str())
            {
                return Err(invalid());
            }
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
        let history = serde_json::to_string(&self.request_history).map_err(|_| invalid())?;
        let json = format!(
            "{{\"server_pid\":{},\"phase\":\"{}\",\"failure\":{},\"incarnation\":{},\"revision\":{},\"last_observation\":{},\"request_history\":{}}}\n",
            server_pid,
            self.phase.name(),
            failure,
            incarnation,
            self.revision,
            observed,
            history
        );
        if json.len() > MAX_PACKET - HEADER {
            return Err(invalid());
        }
        Ok(json)
    }
}

fn required_option<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(de)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireStatus {
    server_pid: u32,
    phase: String,
    #[serde(deserialize_with = "required_option")]
    failure: Option<String>,
    #[serde(deserialize_with = "required_option")]
    incarnation: Option<String>,
    revision: u64,
    #[serde(deserialize_with = "required_option")]
    last_observation: Option<WireObservation>,
    #[serde(deserialize_with = "required_option")]
    request_history: Option<RequestHistory>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum WireObservation {
    Raw {
        revision: u64,
        attempt: u64,
        slot_id: i32,
        serial: i32,
        rilc_msg_id: i32,
        declared_length: i32,
        error: i32,
        payload_hex: String,
    },
    RegistrationWire {
        revision: u64,
        attempt: u64,
        slot_id: i32,
        message_id: i32,
        wire: WireRegistration,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRegistration {
    registration_state_wire: u8,
    feature_wire: u8,
    feature_mask_wire: i32,
    status_code_wire: u16,
    uri_hex: String,
    reason_hex: String,
    #[serde(deserialize_with = "required_option")]
    feature_mask_ext_wire: Option<i32>,
    #[serde(deserialize_with = "required_option")]
    sip_delegate_mask_wire: Option<i64>,
    #[serde(deserialize_with = "required_option")]
    feature_ext_wire: Option<u16>,
}
fn unhex(text: &str, max: usize) -> io::Result<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || text.len() > max * 2
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|part| {
            let text = std::str::from_utf8(part).map_err(|_| invalid())?;
            u8::from_str_radix(text, 16).map_err(|_| invalid())
        })
        .collect()
}
fn status_json(bytes: &[u8], expected_pid: Option<u32>) -> io::Result<Status> {
    let value: WireStatus = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if value.server_pid == 0 || expected_pid.is_some_and(|pid| pid != value.server_pid) {
        return Err(invalid());
    }
    let identity = value
        .incarnation
        .as_deref()
        .map(|s| ProcessIdentity::from_hex(s).ok_or_else(invalid))
        .transpose()?;
    let phase = match value.phase.as_str() {
        "initializing" => Phase::Initializing,
        "observed-active" => Phase::ObservedActive,
        "unavailable" => Phase::Unavailable,
        "stopping" => Phase::Stopping,
        _ => return Err(invalid()),
    };
    let failure = value
        .failure
        .as_deref()
        .map(|text| match text {
            "malformed-callback" => Ok(ReceiverFailure::MalformedCallback),
            "queue-full" => Ok(ReceiverFailure::QueueFull),
            "consumer-gone" => Ok(ReceiverFailure::ConsumerGone),
            "internal-failure" => Ok(ReceiverFailure::InternalFailure),
            "publisher-gone" => Ok(ReceiverFailure::PublisherGone),
            "cancelled" => Ok(ReceiverFailure::Cancelled),
            "service-died" => Ok(ReceiverFailure::ServiceDied),
            "monitoring-failed" => Ok(ReceiverFailure::MonitoringFailed),
            "expired" => Ok(ReceiverFailure::Expired),
            _ => Err(invalid()),
        })
        .transpose()?;
    let last_observation = value
        .last_observation
        .map(|observed| -> io::Result<Frame> {
            let identity = identity.ok_or_else(invalid)?;
            let (revision, attempt, kind) = match observed {
                WireObservation::Raw {
                    revision,
                    attempt,
                    slot_id,
                    serial,
                    rilc_msg_id,
                    declared_length,
                    error,
                    payload_hex,
                } => (
                    revision,
                    attempt,
                    Kind::Raw {
                        slot_id,
                        serial,
                        rilc_msg_id,
                        declared_length,
                        error,
                        payload: unhex(&payload_hex, observation::MAX_PAYLOAD)?,
                    },
                ),
                WireObservation::RegistrationWire {
                    revision,
                    attempt,
                    slot_id,
                    message_id,
                    wire,
                } => {
                    let uri = unhex(&wire.uri_hex, crate::registration::MAX_URI_BYTES)?;
                    let reason = unhex(&wire.reason_hex, crate::registration::MAX_REASON_BYTES)?;
                    let wire = crate::registration::RegistrationWire::new(
                        omarchy_ims::registration::RegistrationWire {
                            registration_state_wire: wire.registration_state_wire,
                            feature_wire: wire.feature_wire,
                            feature_mask_wire: wire.feature_mask_wire,
                            status_code_wire: wire.status_code_wire,
                            uri_bytes: &uri,
                            reason_bytes: &reason,
                            feature_mask_ext_wire: wire.feature_mask_ext_wire,
                            sip_delegate_mask_wire: wire.sip_delegate_mask_wire,
                            feature_ext_wire: wire.feature_ext_wire,
                        },
                    )
                    .map_err(|_| invalid())?;
                    (
                        revision,
                        attempt,
                        Kind::RegistrationWire {
                            slot_id,
                            message_id,
                            wire,
                        },
                    )
                }
            };
            Ok(Frame {
                identity,
                revision,
                attempt,
                kind,
            })
        })
        .transpose()?;
    let status = Status {
        phase,
        identity,
        revision: value.revision,
        failure,
        last_observation,
        request_history: value.request_history,
    };
    status.validate()?;
    Ok(status)
}
impl Reply {
    /// Current RADS v2/type1 or type2, reserved00, total-u32LE, whole status JSON.
    /// Admit replies are one bounded typed JSON record, as the root operator wire requires.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        match self {
            Self::Admit(reply) => {
                reply.validate()?;
                let bytes = serde_json::to_vec(reply).map_err(|_| invalid())?;
                if bytes.len() > 1024 {
                    return Err(invalid());
                }
                Ok(bytes)
            }
            Self::Status(status) | Self::StopAck(status) => {
                let body = status.json(std::process::id())?;
                let mut bytes = Vec::with_capacity(HEADER + body.len());
                bytes.extend_from_slice(MAGIC);
                bytes.extend_from_slice(&[
                    2,
                    if matches!(self, Self::Status(_)) {
                        1
                    } else {
                        2
                    },
                    0,
                    0,
                ]);
                bytes.extend_from_slice(&((HEADER + body.len()) as u32).to_le_bytes());
                bytes.extend_from_slice(body.as_bytes());
                if bytes.len() > MAX_PACKET {
                    return Err(invalid());
                }
                Ok(bytes)
            }
        }
    }
}
fn reply(bytes: &[u8], expected_pid: Option<u32>) -> io::Result<Reply> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET {
        return Err(invalid());
    }
    if !bytes.starts_with(MAGIC) {
        if bytes.len() > 1024 {
            return Err(invalid());
        }
        let reply: AdmitReply = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        reply.validate()?;
        return Ok(Reply::Admit(reply));
    }
    if bytes.len() < HEADER
        || bytes[4] != 2
        || bytes[6..8] != [0, 0]
        || u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid())?) as usize
            != bytes.len()
    {
        return Err(invalid());
    }
    let status = status_json(&bytes[HEADER..], expected_pid)?;
    match bytes[5] {
        1 => Ok(Reply::Status(status)),
        2 => Ok(Reply::StopAck(status)),
        _ => Err(invalid()),
    }
}
/// Exactly one current-v2 record; rejects earlier layouts and unknown fields.
pub fn decode_reply(bytes: &[u8]) -> io::Result<Reply> {
    reply(bytes, None)
}
fn command(bytes: &[u8]) -> io::Result<Command> {
    match bytes {
        [1] => Ok(Command::Status),
        [2] => Ok(Command::Stop),
        [3, body @ ..] if bytes.len() <= MAX_ADMIT => {
            let value: Admit = serde_json::from_slice(body).map_err(|_| invalid())?;
            value.validate()?;
            Ok(Command::Admit(value))
        }
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
        if !self.principal.accepts(peer) && !(peer.uid.as_raw() == 0 && peer.gid.as_raw() == 0) {
            return Err(denied());
        }
        let origin = socket_peer_pidfd(&fd)?;
        if !origin_live(&origin)? {
            return Err(denied());
        }
        Ok(Some(Connection {
            fd,
            peer,
            origin,
            service: self.principal,
        }))
    }
}
pub struct Connection {
    fd: OwnedFd,
    peer: UCred,
    origin: OwnedFd,
    service: Principal,
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
        if self.service != Principal::current() || !self.is_live()? {
            return Err(denied());
        }
        let received = receive(&self.fd, self.peer);
        if !self.is_live()? {
            return Err(denied());
        }
        let command = received?.map(|bytes| command(&bytes)).transpose()?;
        if let Some(command) = &command {
            let authorized = match command {
                Command::Status | Command::Stop => {
                    self.peer.uid.as_raw() != 0 && self.service.accepts(self.peer)
                }
                Command::Admit(_) => self.peer.uid.as_raw() == 0 && self.peer.gid.as_raw() == 0,
            };
            if !authorized || !self.is_live()? {
                return Err(denied());
            }
        }
        Ok(command)
    }
    /// Exact connected origin, retained from SO_PEERPIDFD; no numeric PID reopening.
    pub fn is_live(&self) -> io::Result<bool> {
        origin_live(&self.origin)
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
fn origin_live(origin: impl AsFd) -> io::Result<bool> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let mut fds = [PollFd::new(&origin, PollFlags::IN)];
    let count = poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    if fds[0]
        .revents()
        .intersects(PollFlags::ERR | PollFlags::NVAL)
    {
        return Err(invalid());
    }
    Ok(count == 0 && fds[0].revents().is_empty())
}
/// One-shot service diagnostics; authenticated queued history survives server exit.
/// Exit observation is not native process wait or cleanup authority.
pub struct Client {
    connection: Connection,
}
impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.connection.as_fd()
    }
}
impl Client {
    pub fn connect(path: &Path) -> io::Result<Self> {
        let fd = socket()?;
        net::connect(&fd, &SocketAddrUnix::new(path)?)?;
        let peer = net::sockopt::socket_peercred(&fd)?;
        let service = Principal::current();
        if !service.accepts(peer) {
            return Err(denied());
        }
        let origin = socket_peer_pidfd(&fd)?;
        if !origin_live(&origin)? {
            return Err(denied());
        }
        Ok(Self {
            connection: Connection {
                fd,
                peer,
                origin,
                service,
            },
        })
    }
    pub fn server_pid(&self) -> u32 {
        self.connection.peer.pid.as_raw_pid() as u32
    }
    pub fn send(&self, command: Command) -> io::Result<()> {
        if !self.connection.is_live()? {
            return Err(denied());
        }
        let bytes = match command {
            Command::Status => vec![1],
            Command::Stop => vec![2],
            Command::Admit(value) => {
                value.validate()?;
                let mut bytes = vec![3];
                bytes.extend(serde_json::to_vec(&value).map_err(|_| invalid())?);
                if bytes.len() > MAX_ADMIT {
                    return Err(invalid());
                }
                bytes
            }
        };
        send(&self.connection.fd, &bytes)
    }
    pub fn receive(&self) -> io::Result<Option<Reply>> {
        // Per-packet credentials still bind the retained origin. Do not reject a
        // queued StopAck solely because its exact server has already exited.
        receive(&self.connection.fd, self.connection.peer)?
            .map(|bytes| reply(&bytes, Some(self.server_pid())))
            .transpose()
    }
    pub fn server_exited(&self) -> io::Result<bool> {
        Ok(!self.connection.is_live()?)
    }
    pub fn server_pidfd(&self) -> BorrowedFd<'_> {
        self.connection.origin.as_fd()
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
            request_history: None,
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
        // Current v2 carries JSON, so packet length varies with the retained data.
        assert!(wire.len() > HEADER && wire.len() <= MAX_PACKET);
        assert_eq!(decode_reply(&wire).unwrap(), Reply::Status(status.clone()));
        let json = status.json(42).unwrap();
        assert!(json.contains("\"phase\":\"unavailable\""));
        assert!(json.len() <= MAX_PACKET - HEADER);
    }
    #[test]
    fn packet_boundaries_unknown_tags_reserved_and_lengths_fail_closed() {
        let mut stopping = status();
        stopping.phase = Phase::Stopping;
        let stop = Reply::StopAck(stopping);
        assert_eq!(decode_reply(&stop.encode().unwrap()).unwrap(), stop);
        for reply in [Reply::Status(status()), stop] {
            let wire = reply.encode().unwrap();
            for end in 0..wire.len() {
                assert!(decode_reply(&wire[..end]).is_err(), "accepted prefix {end}");
            }
            let mut extra = wire.clone();
            extra.push(0);
            assert!(decode_reply(&extra).is_err());
            // A matching outer length must not make trailing JSON bytes valid.
            let length = extra.len() as u32;
            extra[8..12].copy_from_slice(&length.to_le_bytes());
            assert!(decode_reply(&extra).is_err());
            for (at, value) in [(0, b'X'), (4, 1), (4, 3), (5, 0), (5, 3), (6, 1), (7, 1)] {
                let mut bad = wire.clone();
                bad[at] = value;
                assert!(decode_reply(&bad).is_err());
            }
            for length in [0, wire.len() as u32 - 1, wire.len() as u32 + 1] {
                let mut bad = wire.clone();
                bad[8..12].copy_from_slice(&length.to_le_bytes());
                assert!(decode_reply(&bad).is_err());
            }
        }
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
