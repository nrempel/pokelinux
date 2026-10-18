//! Root-private delegation only. App identities, grants and backend targeting are absent.
use serde::{Deserialize, Serialize};
pub const LIMIT: usize = 1024;
pub const SECOND: u64 = 1_000_000_000;
pub const LEASE: u64 = 250_000_000;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Connect,
    Disconnect,
}
impl Choice {
    pub fn connected(self) -> bool {
        self == Self::Connect
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Inspect {
        id: u64,
        until_ns: String,
    },
    Apply {
        id: u64,
        choice: Choice,
        until_ns: String,
    },
}
impl Request {
    pub fn id(&self) -> u64 {
        match self {
            Self::Inspect { id, .. } | Self::Apply { id, .. } => *id,
        }
    }
    pub fn choice(&self) -> Option<Choice> {
        match self {
            Self::Inspect { .. } => None,
            Self::Apply { choice, .. } => Some(*choice),
        }
    }
    pub fn until(&self, now: u64) -> Option<u64> {
        let text = match self {
            Self::Inspect { until_ns, .. } | Self::Apply { until_ns, .. } => until_ns,
        };
        let until = decimal(text)?;
        (self.id() > 0 && until > now && until - now <= SECOND).then_some(until)
    }
}
pub fn decimal(text: &str) -> Option<u64> {
    if text.is_empty()
        || text.len() > 20
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    text.parse().ok()
}
fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub available: bool,
    #[serde(deserialize_with = "required_option")]
    pub configured: Option<bool>,
    pub observed_at_ns: String,
    pub until_ns: String,
    pub pending: bool,
    pub terminal_uncertain: bool,
    pub worker: String,
}
impl State {
    pub fn valid(&self, now: u64) -> bool {
        let Some(at) = decimal(&self.observed_at_ns) else {
            return false;
        };
        let Some(until) = decimal(&self.until_ns) else {
            return false;
        };
        let identity = self.worker.len() == 32
            && self
                .worker
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        if !self.available {
            return self.configured.is_none()
                && at == 0
                && until == 0
                && (identity || self.worker.is_empty());
        }
        !self.terminal_uncertain
            && identity
            && self.configured.is_some()
            && at > 0
            && at <= now
            && now < until
            && at.checked_add(LEASE) == Some(until)
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Observed,
    Completed,
    Refused,
    Unavailable,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub schema: u8,
    pub id: u64,
    pub status: Status,
    pub state: State,
}
impl Reply {
    pub fn valid_for(&self, request: &Request, now: u64) -> bool {
        self.schema == 1
            && self.id == request.id()
            && request.until(now).is_some()
            && self.state.valid(now)
            && match self.status {
                Status::Observed => request.choice().is_none() && self.state.available,
                Status::Completed => {
                    request
                        .choice()
                        .is_some_and(|c| self.state.configured == Some(c.connected()))
                        && self.state.available
                        && !self.state.pending
                }
                Status::Refused => request.choice().is_some(),
                Status::Unavailable => !self.state.available,
            }
    }
}
