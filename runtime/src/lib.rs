//! Native phone service contracts. No device access or dispatch occurs here.
#[cfg(target_os = "linux")]
pub mod activation_worker;
pub mod android_protocol;
#[cfg(target_os = "linux")]
pub mod authorization;
#[cfg(target_os = "linux")]
pub mod background;
#[cfg(target_os = "linux")]
pub mod coordinator;
#[cfg(target_os = "linux")]
pub mod dispatch;
#[cfg(target_os = "linux")]
pub mod enrollment;
#[cfg(target_os = "linux")]
pub mod ipc;
#[cfg(target_os = "linux")]
pub mod lease_timer;
#[cfg(target_os = "linux")]
pub mod ledger;
#[cfg(target_os = "linux")]
pub mod local_client;
pub mod local_protocol;
pub mod local_reply;
pub mod notification;
#[cfg(target_os = "linux")]
pub mod notification_bus;
#[cfg(target_os = "linux")]
pub mod notification_events;
#[cfg(target_os = "linux")]
pub mod notification_generation;
#[cfg(target_os = "linux")]
pub mod notification_inputs;
#[cfg(target_os = "linux")]
pub mod notification_pending;
pub mod notification_policy;
#[cfg(target_os = "linux")]
pub mod notification_supervisor;
#[cfg(target_os = "linux")]
pub mod notification_visible;
#[cfg(target_os = "linux")]
pub mod notification_worker;
pub mod observer;
#[cfg(target_os = "linux")]
pub mod presentation_owner;
#[cfg(target_os = "linux")]
pub mod rpc;
#[cfg(target_os = "linux")]
pub mod service_socket;
#[cfg(target_os = "linux")]
pub mod tls;
pub mod watch;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_FRAME: usize = 16_384;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Identity(String);

impl TryFrom<String> for Identity {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == 32
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(Self(value))
        } else {
            Err("invalid identity")
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct PhoneNumber(String);

impl std::fmt::Debug for PhoneNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PhoneNumber([redacted])")
    }
}

impl TryFrom<String> for PhoneNumber {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let digits = value.strip_prefix('+').unwrap_or(&value);
        if (3..=32).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()) {
            Ok(Self(value))
        } else {
            Err("invalid telephone number")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    Ringing,
    Dialing,
    Connecting,
    Active,
    Held,
    Disconnected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    Answer,
    Reject,
    Disconnect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Dial {
        generation: Identity,
        offer: Identity,
        number: PhoneNumber,
    },
    Control {
        generation: Identity,
        call_id: u32,
        revision: u64,
        state: CallState,
        action: ControlAction,
    },
}

impl Command {
    fn valid(&self) -> bool {
        match self {
            Self::Dial { .. } => true,
            Self::Control {
                call_id,
                revision,
                state,
                action,
                ..
            } => {
                (1..=2_147_483_647).contains(call_id)
                    && (1..=9_007_199_254_740_991).contains(revision)
                    && match action {
                        ControlAction::Answer | ControlAction::Reject => {
                            *state == CallState::Ringing
                        }
                        ControlAction::Disconnect => matches!(
                            state,
                            CallState::Dialing
                                | CallState::Connecting
                                | CallState::Active
                                | CallState::Held
                        ),
                    }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    v: u8,
    id: Identity,
    epoch: Identity,
    session: Identity,
    command: Command,
}

#[derive(Debug)]
pub struct Request(Envelope);

/// Decode exactly one complete local frame. Admission and permissions belong to
/// the service; successful decoding grants no authority to execute a command.
pub fn decode_request(frame: &[u8]) -> Result<Request, &'static str> {
    if frame.is_empty()
        || frame.len() > MAX_FRAME
        || frame.last() != Some(&b'\n')
        || frame[..frame.len() - 1].contains(&b'\n')
    {
        return Err("invalid local frame");
    }
    let envelope: Envelope =
        serde_json::from_slice(frame).map_err(|_| "invalid command request")?;
    if envelope.v != 1 || !envelope.command.valid() {
        return Err("invalid command request");
    }
    Ok(Request(envelope))
}

impl Request {
    pub fn id(&self) -> &Identity {
        &self.0.id
    }
    pub fn epoch(&self) -> &Identity {
        &self.0.epoch
    }
    pub fn session(&self) -> &Identity {
        &self.0.session
    }
    pub fn command(&self) -> &Command {
        &self.0.command
    }

    /// Target-format digest: enum tag and fields have a fixed serialization
    /// order. All values are bounded strings/integers, so serialization cannot
    /// encounter non-finite floats or invalid object keys.
    pub fn command_digest(&self) -> [u8; 32] {
        let encoded = serde_json::to_vec(&self.0.command).expect("typed command is serializable");
        Sha256::digest(encoded).into()
    }
}
