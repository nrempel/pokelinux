//! Read-only current ringing snapshots. These types carry no command authority.
use crate::Identity;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    generation: Identity,
    revision: u64,
}
impl Cursor {
    pub fn generation(&self) -> &Identity {
        &self.generation
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

pub struct WatchQuery {
    id: Identity,
    after: Option<Cursor>,
    wait_ms: u32,
}
impl WatchQuery {
    pub fn new(id: Identity, after: Option<Cursor>, wait_ms: u32) -> Result<Self, &'static str> {
        if wait_ms > 25_000 {
            return Err("watch wait exceeds bound");
        }
        Ok(Self { id, after, wait_ms })
    }
    pub fn id(&self) -> &Identity {
        &self.id
    }
    pub fn wait_ms(&self) -> u32 {
        self.wait_ms
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut value = json!({"v":1,"op":"watch","id":self.id,"wait_ms":self.wait_ms});
        if let Some(cursor) = &self.after {
            value["after_generation"] = json!(cursor.generation);
            value["after_revision"] = json!(cursor.revision);
        }
        serde_json::to_vec(&value).expect("fixed watch fields serialize")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    /// The current bound call registry was sampled; not proof of global idle.
    Observed,
    /// InCallService is detached. This alone is not a gateway failure or idle proof.
    Detached,
    /// Role, service or observation prerequisites are missing.
    Unavailable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RingingCall {
    id: u32,
    episode: Identity,
}
impl RingingCall {
    #[cfg(target_os = "linux")]
    pub(crate) fn from_stored(id: u32, episode: Identity) -> Result<Self, &'static str> {
        if !(1..=2_147_483_647).contains(&id) {
            return Err("invalid stored call identity");
        }
        Ok(Self { id, episode })
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn episode(&self) -> &Identity {
        &self.episode
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSnapshot {
    v: u8,
    id: Identity,
    generation: Identity,
    revision: u64,
    source: String,
    source_state: SourceState,
    ringing: Vec<RingingCall>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    cursor: Cursor,
    source_state: SourceState,
    ringing: Vec<RingingCall>,
}
impl Snapshot {
    pub fn cursor(&self) -> Cursor {
        self.cursor.clone()
    }
    pub fn source_state(&self) -> SourceState {
        self.source_state
    }
    pub fn ringing(&self) -> &[RingingCall] {
        &self.ringing
    }
}

pub fn decode_snapshot(bytes: &[u8], id: &Identity) -> Result<Snapshot, &'static str> {
    if bytes.is_empty() || bytes.len() > 65_536 {
        return Err("invalid watch frame length");
    }
    // Deserialize directly: a generic map would discard duplicate fields.
    let raw: RawSnapshot = serde_json::from_slice(bytes).map_err(|_| "invalid watch schema")?;
    if raw.v != 1
        || &raw.id != id
        || raw.source != "android.telecom.InCallService"
        || !(1..=9_007_199_254_740_991).contains(&raw.revision)
        || raw.ringing.len() > 64
        || (raw.source_state != SourceState::Observed && !raw.ringing.is_empty())
    {
        return Err("invalid watch snapshot");
    }
    let mut ids = HashSet::new();
    for (index, call) in raw.ringing.iter().enumerate() {
        if !(1..=2_147_483_647).contains(&call.id)
            || !ids.insert(call.id)
            || raw.ringing[..index]
                .iter()
                .any(|other| other.episode == call.episode)
        {
            return Err("invalid ringing identity");
        }
    }
    Ok(Snapshot {
        cursor: Cursor {
            generation: raw.generation,
            revision: raw.revision,
        },
        source_state: raw.source_state,
        ringing: raw.ringing,
    })
}
