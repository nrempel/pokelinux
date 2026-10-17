#![forbid(unsafe_code)]

use crate::layout::{Id, CAPACITY, DENIED, UNAVAILABLE};
use serde_json::{value::RawValue, Value};
use std::collections::BTreeMap;

type Object = BTreeMap<String, Box<RawValue>>;
pub enum Page {
    Conversations {
        raw: Vec<u8>,
        first: Option<Id>,
        last: Option<Id>,
    },
    Messages {
        raw: Vec<u8>,
        last: Option<u64>,
    },
}
fn string(object: &Object, key: &str) -> Option<String> {
    serde_json::from_str(object.get(key)?.get()).ok()
}
/// Exact decimal integrality, without rounding fractional fields through f64.
fn integral(raw: &str, min: u64, max: u64) -> Option<u64> {
    let negative = raw.starts_with('-');
    let raw = raw.strip_prefix('-').unwrap_or(raw);
    let (mantissa, exponent) = match raw.find(['e', 'E']) {
        Some(at) => {
            let exponent = &raw[at + 1..];
            (
                &raw[..at],
                exponent
                    .parse::<i64>()
                    .unwrap_or(if exponent.starts_with('-') {
                        i64::MIN
                    } else {
                        i64::MAX
                    }),
            )
        }
        None => (raw, 0),
    };
    let fractional = mantissa.find('.').map_or(0, |at| mantissa.len() - at - 1);
    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.is_empty() {
        return None;
    }
    if digits.bytes().all(|b| b == b'0') {
        return (min == 0).then_some(0);
    }
    if negative {
        return None;
    }
    let mut shift = exponent.checked_sub(i64::try_from(fractional).ok()?)?;
    while shift < 0 && digits.ends_with('0') {
        digits.pop();
        shift += 1;
    }
    if shift < 0 {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    if shift > 4 || digits.len().checked_add(shift as usize)? > 4 {
        return None;
    }
    let value = digits
        .parse::<u64>()
        .ok()?
        .checked_mul(10u64.checked_pow(shift as u32)?)?;
    (min..=max).contains(&value).then_some(value)
}
fn number(object: &Object, key: &str, min: u64, max: u64) -> Option<u64> {
    integral(object.get(key)?.get(), min, max)
}
fn decode(bytes: &[u8]) -> Option<Page> {
    // Validate the complete document's Unicode as well as required strings.
    let value: Value = serde_json::from_slice(bytes).ok()?;
    if !value.is_object() {
        return None;
    }
    let root: Object = serde_json::from_slice(bytes).ok()?;
    let kind = string(&root, "page")?;
    let items = root.get("items")?.get();
    let rows: Vec<Object> = serde_json::from_str(items).ok()?;
    if items.len() > CAPACITY {
        return None;
    }
    let limit = match kind.as_str() {
        "conversations" => 8,
        "messages" => 2,
        _ => return None,
    };
    if rows.len() > limit {
        return None;
    }
    let mut first = None;
    let mut last = None;
    let mut last_ordinal = None;
    for row in rows {
        let sender = string(&row, "sender")?;
        if sender.is_empty() || sender.len() > 64 {
            return None;
        }
        if kind == "conversations" {
            let id = Id::from_text(&string(&row, "id")?)?;
            number(&row, "slot", 0, 7)?;
            if first.is_none() {
                first = Some(id);
            }
            last = Some(id);
        } else {
            Id::from_text(&string(&row, "receipt")?)?;
            if string(&row, "text")?.len() > 640 {
                return None;
            }
            last_ordinal = Some(number(&row, "ordinal", 1, 4096)?);
        }
    }
    let raw = items.as_bytes().to_vec();
    Some(if kind == "conversations" {
        Page::Conversations { raw, first, last }
    } else {
        Page::Messages {
            raw,
            last: last_ordinal,
        }
    })
}
pub fn parse(bytes: &[u8]) -> Result<Page, u32> {
    if bytes.len() > CAPACITY {
        return Err(UNAVAILABLE);
    }
    if let Some(page) = decode(bytes) {
        return Ok(page);
    }
    let denied = serde_json::from_slice::<Object>(bytes)
        .ok()
        .and_then(|root| string(&root, "result"))
        .is_some_and(|v| v == "denied");
    Err(if denied { DENIED } else { UNAVAILABLE })
}
pub fn request(conversation_cycle: bool, selected: Id, cursor: Id, after: u64) -> Vec<u8> {
    let query = if conversation_cycle || selected.length == 0 {
        let cursor = if cursor.length == 0 {
            "null".to_owned()
        } else {
            format!("\"{}\"", cursor.text())
        };
        format!("{{\"after\":{cursor},\"limit\":8,\"query\":\"conversations\"}}")
    } else {
        format!("{{\"after_ordinal\":{after},\"conversation\":\"{}\",\"limit\":2,\"query\":\"messages\"}}", selected.text())
    };
    format!("{{\"op\":\"messages_read\",\"request\":{{\"query\":{query},\"ttl_ms\":1000}}}}")
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "11111111111111111111111111111111";
    #[test]
    fn raw_last_duplicate_array_and_finite_unknown_number() {
        let selected = format!(r#"[ {{"id":"{ID}","sender":"A","slot":0,"unknown":1e2}} ]"#);
        let wire = format!(
            r#"{{"page":"messages","items":null,"page":"conversations","items":{selected}}}"#
        );
        let Ok(Page::Conversations { raw, first, last }) = parse(wire.as_bytes()) else {
            panic!("valid last-value page");
        };
        assert_eq!(raw, selected.as_bytes());
        assert_eq!(first, Id::from_text(ID));
        assert_eq!(last, first);
        // Actual Qt rejects a numeric overflow in the raw array. Refuse the
        // page in Rust before mechanical Qt projection can abort the app.
        let overflow = wire.replace("1e2", "1e400");
        assert!(matches!(parse(overflow.as_bytes()), Err(UNAVAILABLE)));
    }
    #[test]
    fn required_integer_is_exact_not_a_rounded_float() {
        for (number, valid) in [
            ("7.0", true),
            ("7e0", true),
            ("70e-1", true),
            ("7.00000000000000000000001", false),
            ("7.1", false),
            ("true", false),
        ] {
            let wire = format!(
                r#"{{"page":"conversations","items":[{{"id":"{ID}","sender":"A","slot":{number}}}]}}"#
            );
            assert_eq!(parse(wire.as_bytes()).is_ok(), valid, "{number}");
        }
    }
    #[test]
    fn valid_page_wins_denied_and_invalid_page_clears_as_denied() {
        assert!(parse(br#"{"result":"denied","page":"messages","items":[]}"#).is_ok());
        assert!(matches!(
            parse(br#"{"result":"denied","page":"messages","items":[{}]}"#),
            Err(DENIED)
        ));
    }
}
