//! Typed presentation from independently observed generated and user shell inputs.
//! Concrete font resolution/resources and translucent colors are separate gates.
use crate::palette::{self, Palette};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub palette: Palette,
    pub style: Style,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub base_size: u32,
    pub font_tokens: FontTokens,
    pub spacing_tokens: SpacingTokens,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FontTokens {
    pub caption: u32,
    pub body_small: u32,
    pub body: u32,
    pub subtitle: u32,
    pub title: u32,
    pub heading: u32,
    pub display: u32,
    pub display_large: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SpacingTokens {
    pub control_gap: u32,
    pub panel_padding: u32,
    pub panel_gap: u32,
    pub row_gap: u32,
    pub control_padding_x: u32,
    pub control_padding_y: u32,
}
fn supported(value: u32, floor: u32) -> bool {
    (floor..=i32::MAX as u32).contains(&value)
}
impl Appearance {
    pub fn valid(&self) -> bool {
        let f = &self.style.font_tokens;
        let s = &self.style.spacing_tokens;
        self.palette.valid()
            && supported(self.style.base_size, 1)
            && [
                f.caption,
                f.body_small,
                f.body,
                f.subtitle,
                f.title,
                f.heading,
                f.display,
                f.display_large,
            ]
            .iter()
            .all(|v| supported(*v, 1))
            && [
                s.control_gap,
                s.panel_padding,
                s.panel_gap,
                s.row_gap,
                s.control_padding_x,
                s.control_padding_y,
            ]
            .iter()
            .all(|v| supported(*v, 0))
    }
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
// Original Style uses JS parseInt(...,10)/parseFloat, including quoted prefixes.
fn integer(value: &str) -> Option<f64> {
    let value = value.trim_start();
    let start = usize::from(value.starts_with(['+', '-']));
    let count = value.as_bytes()[start..]
        .iter()
        .take_while(|v| v.is_ascii_digit())
        .count();
    if count == 0 {
        return None;
    }
    value[..start + count]
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}
fn number(value: &str) -> Option<f64> {
    let value = value.trim_start();
    let bytes = value.as_bytes();
    let mut end = usize::from(value.starts_with(['+', '-']));
    let mut digits = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
        digits += 1;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if bytes.get(end).is_some_and(|v| matches!(v, b'e' | b'E')) {
        let previous = end;
        end += 1;
        if bytes.get(end).is_some_and(|v| matches!(v, b'+' | b'-')) {
            end += 1;
        }
        let first = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == first {
            end = previous;
        }
    }
    value[..end].parse::<f64>().ok().filter(|v| v.is_finite())
}
fn pixels(value: f64, floor: u32) -> io::Result<u32> {
    let rounded = value.round().max(f64::from(floor));
    // Refuse unsupported Qt signed-int overflow; never silently clamp a large
    // user size. Original overflow coercions require independent execution.
    if !rounded.is_finite() || rounded > f64::from(i32::MAX) {
        return Err(invalid());
    }
    Ok(rounded as u32)
}
fn tokens(values: &BTreeMap<String, String>) -> io::Result<Style> {
    let raw_base = values
        .get("font.base-size")
        .and_then(|v| integer(v))
        .unwrap_or(12.0)
        .max(1.0);
    let base_size = pixels(raw_base, 1)?;
    let font = |key: &str, multiplier: f64| {
        let value = values
            .get(&format!("font.{key}"))
            .and_then(|v| integer(v))
            .filter(|v| *v > 0.0)
            .unwrap_or(f64::from(base_size) * multiplier);
        pixels(value, 1)
    };
    let scale = values
        .get("spacing.scale")
        .and_then(|v| number(v))
        .filter(|v| *v >= 0.0)
        .unwrap_or(1.0);
    let with_font = values.get("spacing.scale-with-font").is_none_or(|v| {
        !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        )
    });
    let effective = scale
        * if with_font {
            f64::from(base_size) / 12.0
        } else {
            1.0
        };
    let space = |key: &str, default: f64| match values
        .get(&format!("spacing.{key}"))
        .and_then(|v| number(v))
        .filter(|v| *v >= 0.0)
    {
        Some(value) => pixels(value, 0),
        None if default * effective <= 0.0 => Ok(0),
        None => pixels(default * effective, 1),
    };
    Ok(Style {
        base_size,
        font_tokens: FontTokens {
            caption: font("caption", 0.833)?,
            body_small: font("body-small", 0.917)?,
            body: font("body", 1.0)?,
            subtitle: font("subtitle", 1.083)?,
            title: font("title", 1.167)?,
            heading: font("heading", 1.333)?,
            display: font("display", 2.0)?,
            display_large: font("display-large", 2.333)?,
        },
        spacing_tokens: SpacingTokens {
            control_gap: space("control-gap", 8.0)?,
            panel_padding: space("panel-padding", 18.0)?,
            panel_gap: space("panel-gap", 14.0)?,
            row_gap: space("row-gap", 8.0)?,
            control_padding_x: space("control-padding-x", 10.0)?,
            control_padding_y: space("control-padding-y", 6.0)?,
        },
    })
}
pub fn project(colors: &str, generated: &str, user: &str) -> io::Result<Appearance> {
    if [colors, generated, user]
        .iter()
        .any(|v| v.len() > 64 * 1024)
    {
        return Err(invalid());
    }
    let mut values = palette::shell(generated);
    values.extend(palette::shell(user));
    Ok(Appearance {
        palette: palette::project_values(colors, &values)?,
        style: tokens(&values)?,
    })
}
