//! Private root-actor protocol. Device selection is absent from every packet.
use crate::model::{Observation, View};
use serde::{Deserialize, Serialize};
pub const LIMIT: usize = 512;
pub const SECOND: u64 = 1_000_000_000;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Inspect {
        id: u64,
        until_ns: String,
    },
    Set {
        id: u64,
        percent: u8,
        expected_revision: u64,
        until_ns: String,
    },
}
impl Request {
    pub fn id(&self) -> u64 {
        match self {
            Self::Inspect { id, .. } | Self::Set { id, .. } => *id,
        }
    }
    pub fn until(&self, now: u64) -> Option<u64> {
        let text = match self {
            Self::Inspect { until_ns, .. } | Self::Set { until_ns, .. } => until_ns,
        };
        let until = decimal(text)?;
        if self.id() == 0 || until <= now || until - now > SECOND {
            return None;
        }
        if let Self::Set {
            percent,
            expected_revision,
            ..
        } = self
            && (*percent > 100 || *expected_revision == 0)
        {
            return None;
        }
        Some(until)
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
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    pub max: u32,
    pub requested: u32,
    #[serde(deserialize_with = "required_option")]
    pub actual: Option<u32>,
}
impl From<Observation> for Attributes {
    fn from(v: Observation) -> Self {
        Self {
            max: v.max,
            requested: v.requested,
            actual: v.actual,
        }
    }
}
impl From<Attributes> for Observation {
    fn from(v: Attributes) -> Self {
        Self {
            max: v.max,
            requested: v.requested,
            actual: v.actual,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub schema: u8,
    pub id: u64,
    pub ok: bool,
    pub revision: u64,
    pub pending: bool,
    #[serde(deserialize_with = "required_option")]
    pub observation: Option<Attributes>,
    pub until_ns: String,
    #[serde(deserialize_with = "required_option")]
    pub requested_percent: Option<u8>,
    pub last_set_uncertain: bool,
}
impl Reply {
    pub fn from_view(id: u64, ok: bool, view: View, completed_percent: Option<u8>) -> Self {
        Self {
            schema: 1,
            id,
            ok,
            revision: view.revision,
            pending: view.pending,
            observation: view.observation.map(Into::into),
            until_ns: view.until_ns.to_string(),
            requested_percent: if ok { completed_percent } else { None },
            last_set_uncertain: view.last_set_uncertain,
        }
    }
    pub fn valid(&self, now: u64) -> bool {
        if self.schema != 1 || self.id == 0 || self.requested_percent.is_some_and(|p| p > 100) {
            return false;
        }
        let Some(until) = decimal(&self.until_ns) else {
            return false;
        };
        match self.observation {
            None => until == 0 && !self.ok && self.requested_percent.is_none(),
            Some(v) => {
                let value: Observation = v.into();
                value.valid()
                    && self.revision > 0
                    && now < until
                    && until - now <= 2 * SECOND
                    && (!self.ok || !self.pending)
                    && (self.requested_percent.is_none() || self.ok && !self.last_set_uncertain)
            }
        }
    }
}
