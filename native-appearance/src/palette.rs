//! Selected public Omarchy Color contract with exact bounded RGBA16 channels.
//! See docs/native-appearance-service.md for original-code qualification gates.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub border: Color,
    pub accent: Color,
    pub muted: Color,
    pub urgent: Color,
    pub positive: Color,
    pub ink: Color,
}
/// QColor's integer RGBA precision, independent of any consumer's hex syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}
impl Color {
    pub fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: u16::from(r) * 257,
            g: u16::from(g) * 257,
            b: u16::from(b) * 257,
            a: u16::MAX,
        }
    }
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn rgb(s: &str) -> Option<Color> {
    if s.len() != 7 || !s.starts_with('#') || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(Color::opaque(
        u8::from_str_radix(&s[1..3], 16).ok()?,
        u8::from_str_radix(&s[3..5], 16).ok()?,
        u8::from_str_radix(&s[5..7], 16).ok()?,
    ))
}
fn text_color(s: &str) -> Option<Color> {
    if let Some(color) = rgb(s) {
        return Some(color);
    }
    // Direct popup text is QColor's #AARRGGBB, unlike the surface parser's
    // trailing-alpha syntax. Its text-alpha companion is not consumed.
    if s.len() != 9 || !s.starts_with('#') || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut color = rgb(&format!("#{}", &s[3..]))?;
    color.a = u16::from(u8::from_str_radix(&s[1..3], 16).ok()?) * 257;
    Some(color)
}
fn alpha(value: Option<&String>) -> u16 {
    let value = value
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(1.0);
    // Original Util.alpha replaces intrinsic alpha through Qt.rgba's float
    // interface; it does not multiply or round the companion to an 8-bit byte.
    ((value.clamp(0.0, 1.0) as f32) * 65535.0).round() as u16
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
fn decimal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let mut parts = s.split('.');
    let integer = parts.next().unwrap_or_default();
    !integer.is_empty()
        && integer.bytes().all(|c| c.is_ascii_digit())
        && parts
            .next()
            .is_none_or(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
        && parts.next().is_none()
}
pub(crate) fn shell(raw: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let mut section = "";
    for line in raw.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(tail) = line.strip_prefix('[')
            && let Some((s, end)) = tail.split_once(']')
            && key(s)
            && (end.trim().is_empty() || end.trim().starts_with('#'))
        {
            section = s;
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if !key(k.trim()) || section.is_empty() {
            continue;
        }
        let v = v.trim();
        let value = if v.starts_with(['\'', '"']) {
            let quote = v.chars().next().unwrap();
            let Some((inside, end)) = v[1..].split_once(quote) else {
                continue;
            };
            if inside.is_empty()
                || inside.contains(['\'', '"'])
                || !(end.trim().is_empty() || end.trim().starts_with('#'))
            {
                continue;
            }
            inside
        } else {
            let v = v.split('#').next().unwrap().trim();
            let words: Vec<_> = v.split_whitespace().collect();
            let numeric = (1..=4).contains(&words.len()) && words.iter().all(|s| decimal(s));
            let identifier = key(v) && v.as_bytes()[0].is_ascii_alphabetic();
            if !numeric && !identifier {
                continue;
            }
            v
        };
        result.insert(format!("{section}.{}", k.trim()), value.into());
    }
    result
}
fn first_color(value: &str) -> Option<&str> {
    value
        .split_whitespace()
        .find(|word| !word.strip_suffix("deg").is_some_and(decimal))
}
fn flat(
    value: &str,
    roles: &BTreeMap<String, Color>,
    shell: &BTreeMap<String, String>,
    depth: usize,
) -> io::Result<Color> {
    if depth >= 16 {
        return Err(invalid());
    }
    let token = first_color(value).ok_or_else(invalid)?;
    let lower = token.to_ascii_lowercase();
    if let Some(next) = shell.get(&lower) {
        return flat(next, roles, shell, depth + 1);
    }
    if let Some(color) = roles.get(&lower) {
        return Ok(*color);
    }
    if let Some(color) = rgb(token) {
        return Ok(color);
    }
    // Intrinsic alpha is replaced by original Util.alpha for surface consumers.
    if token.starts_with('#')
        && token.len() == 9
        && token[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return rgb(&token[..7]).ok_or_else(invalid);
    }
    for prefix in ["rgba(", "rgb("] {
        if let Some(hex) = lower.strip_prefix(prefix).and_then(|s| s.strip_suffix(')'))
            && matches!(hex.len(), 6 | 8)
            && hex.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return rgb(&format!("#{}", &hex[..6])).ok_or_else(invalid);
        }
    }
    Err(invalid())
}
pub fn project(colors: &str, appearance: &str) -> io::Result<Palette> {
    if colors.len() > 64 * 1024 || appearance.len() > 64 * 1024 {
        return Err(invalid());
    }
    project_values(colors, &shell(appearance))
}
pub(crate) fn project_values(
    colors: &str,
    shell: &BTreeMap<String, String>,
) -> io::Result<Palette> {
    let mut base = BTreeMap::new();
    let mut urgent = None;
    for line in colors.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if !key(k) {
            continue;
        }
        let v = v.trim().trim_start_matches(['\'', '"']);
        if !v.starts_with('#')
            || v.len() < 7
            || !v.as_bytes()[1..7].iter().all(u8::is_ascii_hexdigit)
        {
            continue;
        }
        let color = rgb(&v[..7]).ok_or_else(invalid)?;
        if matches!(k, "red" | "color1") {
            urgent = Some(color);
        }
        base.insert(k.to_string(), color);
    }
    let choose = |name, alias| {
        base.get(name)
            .or_else(|| base.get(alias))
            .copied()
            .ok_or_else(invalid)
    };
    let background = choose("background", "color0")?;
    let foreground = choose("foreground", "color7")?;
    let accent = choose("accent", "color4")?;
    let muted = choose("muted", "color8").unwrap_or(foreground);
    let urgent = urgent.ok_or_else(invalid)?;
    let mut roles = BTreeMap::new();
    for (k, v) in [
        ("background", &background),
        ("foreground", &foreground),
        ("text", &foreground),
        ("accent", &accent),
        ("muted", &muted),
        ("urgent", &urgent),
    ] {
        roles.insert(k.into(), *v);
    }
    let mut surface = flat(
        shell
            .get("popups.background")
            .map(String::as_str)
            .unwrap_or("background"),
        &roles,
        shell,
        0,
    )?;
    surface.a = alpha(shell.get("popups.background-alpha"));
    let mut border = flat(
        shell
            .get("popups.border")
            .map(String::as_str)
            .unwrap_or("accent"),
        &roles,
        shell,
        0,
    )?;
    border.a = alpha(shell.get("popups.border-alpha"));
    // Original popup text is a direct color pick, not surface-alpha composition.
    let foreground = match shell.get("popups.text") {
        Some(value) => text_color(value).ok_or_else(invalid)?,
        None => foreground,
    };
    Ok(Palette {
        background,
        foreground,
        surface,
        border,
        accent,
        muted,
        urgent,
        positive: choose("green", "color2").unwrap_or(accent),
        ink: background,
    })
}
