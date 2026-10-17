#![forbid(unsafe_code)]

use crate::layout::Rgba16;
use serde_json::{Map, Value};

pub const ROLES: [&str; 9] = [
    "background",
    "foreground",
    "surface",
    "border",
    "accent",
    "muted",
    "urgent",
    "positive",
    "ink",
];
pub const FONTS: [&str; 8] = [
    "caption",
    "bodySmall",
    "body",
    "subtitle",
    "title",
    "heading",
    "display",
    "displayLarge",
];
pub const SPACING: [&str; 6] = [
    "controlGap",
    "panelPadding",
    "panelGap",
    "rowGap",
    "controlPaddingX",
    "controlPaddingY",
];
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub palette: [Rgba16; 9],
    pub base_size: u32,
    pub font_tokens: [u32; 8],
    pub spacing_tokens: [u32; 6],
}
pub struct Response {
    pub owner: [u8; 16],
    pub revision: u32,
    pub until: u64,
    pub snapshot: Option<Snapshot>,
}
fn exact<'a>(value: &'a Value, names: &[&str]) -> Option<&'a Map<String, Value>> {
    let object = value.as_object()?;
    (object.len() == names.len() && names.iter().all(|name| object.contains_key(*name)))
        .then_some(object)
}
fn integer(value: &Value, minimum: u32, maximum: u32) -> Option<u32> {
    let number = value.as_f64()?;
    (number.is_finite()
        && number >= f64::from(minimum)
        && number <= f64::from(maximum)
        && number.fract() == 0.0)
        .then_some(number as u32)
}
fn owner(value: &Value) -> Option<[u8; 16]> {
    let text = value.as_str()?.as_bytes();
    if text.len() != 32 {
        return None;
    }
    let mut bytes = [0; 16];
    fn hex(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    }
    for (out, pair) in bytes.iter_mut().zip(text.as_chunks::<2>().0) {
        *out = hex(pair[0])? * 16 + hex(pair[1])?;
    }
    Some(bytes)
}
fn decimal(value: &Value) -> Option<u64> {
    let text = value.as_str()?;
    if text.is_empty()
        || text.len() > 20
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    text.parse().ok()
}
fn tokens<const N: usize>(value: &Value, names: &[&str; N], minimum: u32) -> Option<[u32; N]> {
    let object = exact(value, names)?;
    let mut result = [0; N];
    for (out, name) in result.iter_mut().zip(names) {
        *out = integer(&object[*name], minimum, i32::MAX as u32)?;
    }
    Some(result)
}
pub fn decode(bytes: &[u8]) -> Option<Response> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return None;
    }
    // Value preserves the existing last-value duplicate-key behavior; integers
    // deliberately follow finite f64-integral admission, including 8.0 and 8e0.
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let root = exact(
        &value,
        &[
            "schema",
            "owner",
            "revision",
            "until_ns",
            "available",
            "appearance",
        ],
    )?;
    if root["schema"].as_f64()? != 3.0 {
        return None;
    }
    let owner = owner(&root["owner"])?;
    let revision = integer(&root["revision"], 0, u32::MAX)?;
    let until = decimal(&root["until_ns"])?;
    let published = root["available"].as_bool()?;
    let snapshot = if published {
        let appearance = exact(&root["appearance"], &["palette", "style"])?;
        let palette = exact(&appearance["palette"], &ROLES)?;
        let style = exact(
            &appearance["style"],
            &["base_size", "font_tokens", "spacing_tokens"],
        )?;
        let mut colors = [Rgba16::default(); 9];
        for (out, name) in colors.iter_mut().zip(ROLES) {
            let color = exact(&palette[name], &["r", "g", "b", "a"])?;
            *out = Rgba16 {
                r: integer(&color["r"], 0, 65535)? as u16,
                g: integer(&color["g"], 0, 65535)? as u16,
                b: integer(&color["b"], 0, 65535)? as u16,
                a: integer(&color["a"], 0, 65535)? as u16,
            };
        }
        Some(Snapshot {
            palette: colors,
            base_size: integer(&style["base_size"], 1, i32::MAX as u32)?,
            font_tokens: tokens(&style["font_tokens"], &FONTS, 1)?,
            spacing_tokens: tokens(&style["spacing_tokens"], &SPACING, 0)?,
        })
    } else {
        if !root["appearance"].is_null() {
            return None;
        }
        None
    };
    Some(Response {
        owner,
        revision,
        until,
        snapshot,
    })
}
