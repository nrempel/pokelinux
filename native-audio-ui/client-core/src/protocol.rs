#![forbid(unsafe_code)]
use crate::wire::Route;
use serde_json::{Map, Value};
#[derive(Clone)]
pub struct Row {
    pub id: Route,
    pub label: String,
}
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Preview {
    Idle,
    Starting,
    Playing,
    Retiring,
    Completed,
    Failed,
}
impl Preview {
    pub fn active(self) -> bool {
        matches!(self, Self::Starting | Self::Playing | Self::Retiring)
    }
    fn parse(value: &Value) -> Option<Self> {
        Some(match value.as_str()? {
            "idle" => Self::Idle,
            "starting" => Self::Starting,
            "playing" => Self::Playing,
            "retiring" => Self::Retiring,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}
pub struct State {
    pub connected: bool,
    pub available: bool,
    pub pending: bool,
    pub requested: Option<Route>,
    pub applied: Option<Route>,
    pub policy_settled: bool,
    pub preview: Preview,
}
pub struct Page {
    pub state: State,
    pub rows: Vec<Row>,
    pub next: Option<usize>,
    pub choose: bool,
    pub reconnect: bool,
}
pub enum Reply {
    Denied,
    Unavailable,
    Ok,
    Selected(State),
    Preview(State),
    Status(Page),
}
fn object(v: &Value, size: usize) -> Option<&Map<String, Value>> {
    let v = v.as_object()?;
    (v.len() == size).then_some(v)
}
pub fn route(s: &str) -> Option<Route> {
    let mut r = Route::EMPTY;
    r.length = s.len().try_into().ok()?;
    r.bytes.get_mut(..s.len())?.copy_from_slice(s.as_bytes());
    r.id()?;
    Some(r)
}
fn nullable(v: &Value) -> Option<Option<Route>> {
    if v.is_null() {
        Some(None)
    } else {
        Some(Some(route(v.as_str()?)?))
    }
}
fn state(v: &Value) -> Option<State> {
    let s = object(v, 7)?;
    Some(State {
        connected: s.get("connected")?.as_bool()?,
        available: s.get("available")?.as_bool()?,
        pending: s.get("pending")?.as_bool()?,
        requested: nullable(s.get("requested")?)?,
        applied: nullable(s.get("applied")?)?,
        policy_settled: s.get("policy_settled")?.as_bool()?,
        preview: Preview::parse(s.get("preview")?)?,
    })
}
fn whitespace(c: char) -> bool {
    matches!(c,'\u{9}'..='\u{d}'|'\u{20}'|'\u{85}'|'\u{a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}')
}
fn label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.chars().any(|c| !whitespace(c))
        && !s
            .chars()
            .any(|c| matches!(c,'\u{0}'..='\u{1f}'|'\u{7f}'..='\u{9f}'))
}
pub fn parse(bytes: &[u8]) -> Option<Reply> {
    // Value preserves the current Qt last-value duplicate-key semantics. Explicit
    // get checks retain required-nullable presence rather than serde Option defaults.
    let v: Value = serde_json::from_slice(bytes).ok()?;
    let s = v.as_object()?;
    match s.get("result")?.as_str()? {
        "denied" if s.len() == 1 => Some(Reply::Denied),
        "unavailable" if s.len() == 1 => Some(Reply::Unavailable),
        "ok" if s.len() == 1 => Some(Reply::Ok),
        "media_selected" if s.len() == 2 => Some(Reply::Selected(state(s.get("state")?)?)),
        "media_preview" if s.len() == 2 => Some(Reply::Preview(state(s.get("state")?)?)),
        "media_status" if s.len() == 6 => {
            let a = s.get("routes")?.as_array()?;
            if a.len() > 8 {
                return None;
            }
            let mut rows = Vec::with_capacity(a.len());
            for entry in a {
                let r = object(entry, 2)?;
                let text = r.get("label")?.as_str()?;
                if !label(text) {
                    return None;
                }
                rows.push(Row {
                    id: route(r.get("id")?.as_str()?)?,
                    label: text.to_owned(),
                });
            }
            let next = s.get("next_offset")?;
            let next = if next.is_null() {
                None
            } else {
                let n = next.as_f64()?;
                if !n.is_finite() || n < 0.0 || n > 32.0 || n.fract() != 0.0 {
                    return None;
                }
                Some(n as usize)
            };
            Some(Reply::Status(Page {
                state: state(s.get("state")?)?,
                rows,
                next,
                choose: s.get("can_choose")?.as_bool()?,
                reconnect: s.get("can_reconnect")?.as_bool()?,
            }))
        }
        _ => None,
    }
}
pub fn status(offset: usize) -> String {
    format!("{{\"offset\":{offset},\"op\":\"media_status\"}}")
}
pub fn selection(route: &Route, ttl: u32) -> String {
    let id = std::str::from_utf8(route.id().expect("validated selector")).expect("ASCII selector");
    format!("{{\"op\":\"media_route\",\"request\":{{\"route\":\"{id}\",\"ttl_ms\":{ttl}}}}}")
}
pub fn reconnect() -> String {
    "{\"op\":\"media_reconnect\"}".to_owned()
}

pub fn preview_play(route: &Route, ttl: u32, until: u64, percent: u32, muted: bool) -> String {
    let id = std::str::from_utf8(route.id().expect("validated selector")).expect("ASCII selector");
    format!(
        "{{\"op\":\"media_preview_play\",\"request\":{{\"route\":\"{id}\",\"ttl_ms\":{ttl},\"until_boottime_ns\":{until},\"level_percent\":{percent},\"muted\":{muted}}}}}"
    )
}
pub fn preview_stop() -> String {
    "{\"op\":\"media_preview_stop\"}".to_owned()
}
