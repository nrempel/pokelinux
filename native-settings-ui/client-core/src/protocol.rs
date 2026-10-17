//! Only the actor's public typed brightness operations. No backend paths or grants.
use serde::{Deserialize, Serialize};
pub const LIMIT: usize = 4096;
pub const SECOND: u64 = 1_000_000_000;
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub connected: bool,
    pub available: bool,
    pub revision: u64,
    pub pending: bool,
    #[serde(deserialize_with = "required_option")]
    pub requested_percent: Option<u8>,
    #[serde(deserialize_with = "required_option")]
    pub observed_percent: Option<u8>,
    #[serde(deserialize_with = "required_option")]
    pub actual_percent: Option<u8>,
    pub until_ns: String,
}
fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
impl State {
    pub fn valid(&self, now: u64) -> bool {
        let Ok(until) = self.until_ns.parse::<u64>() else {
            return false;
        };
        if until.to_string() != self.until_ns {
            return false;
        }
        if !self.available {
            return until == 0
                && self.requested_percent.is_none()
                && self.observed_percent.is_none()
                && self.actual_percent.is_none();
        }
        self.connected
            && self.revision > 0
            && now < until
            && until <= now.saturating_add(2 * SECOND)
            && self.observed_percent.is_some_and(|p| p <= 100)
            && self.actual_percent.is_none_or(|p| p <= 100)
            && self.requested_percent.is_none_or(|p| p <= 100)
    }
    pub fn until(&self) -> u64 {
        self.until_ns.parse().unwrap_or(0)
    }
}
#[derive(Debug, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    BrightnessState { state: State, can_set: bool },
    BrightnessCompleted { state: State },
    Denied,
    Unavailable,
}
#[derive(Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    BrightnessStatus {},
    BrightnessSet { request: Set },
}
#[derive(Serialize)]
pub struct Set {
    pub percent: u8,
    pub expected_revision: u64,
    pub ttl_ms: u64,
}
