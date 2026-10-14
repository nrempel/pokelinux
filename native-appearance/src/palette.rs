//! Selected opaque public Omarchy Color contract, not arbitrary TOML or Qt colors.
//! See docs/native-appearance-service.md for original-code qualification gates.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub background: String,
    pub foreground: String,
    pub surface: String,
    pub border: String,
    pub accent: String,
    pub muted: String,
    pub urgent: String,
    pub positive: String,
    pub ink: String,
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn rgb(s: &str) -> Option<String> {
    (s.len() == 7 && s.starts_with('#') && s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
        .then(|| s.to_ascii_lowercase())
}
impl Palette {
    pub fn valid(&self) -> bool {
        [
            &self.background,
            &self.foreground,
            &self.surface,
            &self.border,
            &self.accent,
            &self.muted,
            &self.urgent,
            &self.positive,
            &self.ink,
        ]
        .iter()
        .all(|s| rgb(s).is_some())
    }
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
    roles: &BTreeMap<String, String>,
    shell: &BTreeMap<String, String>,
    depth: usize,
) -> io::Result<String> {
    if depth >= 16 {
        return Err(invalid());
    }
    let token = first_color(value).ok_or_else(invalid)?;
    let lower = token.to_ascii_lowercase();
    if let Some(next) = shell.get(&lower) {
        return flat(next, roles, shell, depth + 1);
    }
    if let Some(color) = roles.get(&lower) {
        return Ok(color.clone());
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
            return Ok(format!("#{}", &hex[..6]));
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
        let color = v[..7].to_ascii_lowercase();
        if matches!(k, "red" | "color1") {
            urgent = Some(color.clone());
        }
        base.insert(k.to_string(), color);
    }
    let choose = |name, alias| {
        base.get(name)
            .or_else(|| base.get(alias))
            .cloned()
            .ok_or_else(invalid)
    };
    let background = choose("background", "color0")?;
    let foreground = choose("foreground", "color7")?;
    let accent = choose("accent", "color4")?;
    let muted = choose("muted", "color8").unwrap_or_else(|_| foreground.clone());
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
        roles.insert(k.into(), v.clone());
    }
    // Unsupported translucency is refused until actual original alpha/pixel tests.
    for k in ["popups.background-alpha", "popups.border-alpha"] {
        if shell
            .get(k)
            .is_some_and(|v| v.parse::<f64>().ok() != Some(1.0))
        {
            return Err(invalid());
        }
    }
    let surface = flat(
        shell
            .get("popups.background")
            .map(String::as_str)
            .unwrap_or(&background),
        &roles,
        shell,
        0,
    )?;
    let border = flat(
        shell
            .get("popups.border")
            .map(String::as_str)
            .unwrap_or(&accent),
        &roles,
        shell,
        0,
    )?;
    // Original popup text is a direct color pick, not surface-alpha composition.
    let popup_text = shell
        .get("popups.text")
        .map(String::as_str)
        .unwrap_or(&foreground);
    let foreground = rgb(popup_text).ok_or_else(invalid)?;
    Ok(Palette {
        background: background.clone(),
        foreground,
        surface,
        border,
        accent: accent.clone(),
        muted,
        urgent,
        positive: choose("green", "color2").unwrap_or(accent),
        ink: background,
    })
}
