//! Device-neutral 3GPP SMS payloads. Codecs do not establish submission or delivery.
//! GSM alphabet mapping comes from the pinned open-source sms-pdu crate. Planning
//! preserves complete characters; no transport, random reference or retry is used.
use sms_pdu::gsm_encoding::try_gsm_encode_char;

pub const MAX_MESSAGE_BYTES: usize = 16_384;
pub const MAX_PARTS: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Gsm7,
    Utf16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concatenation {
    pub reference: u16,
    pub total: u8,
    pub sequence: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitPart {
    pub text: String,
    pub encoding: Encoding,
    pub concatenation: Option<Concatenation>,
    /// SMS-SUBMIT TPDU, excluding the SMSC address. TP-MR is zero; the actual
    /// modem/IMS adapter must establish wire correlation and delivery semantics.
    pub tpdu: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidRecipient,
    EmptyMessage,
    InvalidConcatenation,
    Encoding,
    PartTooLong,
    MessageTooLong,
}

// The pinned upstream table omits form-feed; AOSP execution verifies its
// standard extension encoding. Keep the rest of the alphabet in upstream code.
fn gsm_character(character: char, output: &mut Vec<u8>) -> bool {
    if character == '\u{000c}' {
        output.extend([0x1b, 0x0a]);
        true
    } else {
        try_gsm_encode_char(character, output)
    }
}
fn gsm_string(text: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    for character in text.chars() {
        if !gsm_character(character, &mut output) {
            return None;
        }
    }
    Some(output)
}
fn recipient_digits(recipient: &str) -> Result<(&[u8], u8), Error> {
    let (digits, kind) = match recipient.strip_prefix('+') {
        Some(digits) => (digits, 0x91),
        None => (recipient, 0x81),
    };
    if digits.is_empty() || digits.len() > 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidRecipient);
    }
    Ok((digits.as_bytes(), kind))
}
fn header(concatenation: Option<Concatenation>) -> Result<Vec<u8>, Error> {
    let Some(value) = concatenation else {
        return Ok(Vec::new());
    };
    if value.total == 0 || value.sequence == 0 || value.sequence > value.total {
        return Err(Error::InvalidConcatenation);
    }
    let reference = value.reference.to_be_bytes();
    Ok(vec![
        6,
        8,
        4,
        reference[0],
        reference[1],
        value.total,
        value.sequence,
    ])
}
fn user_data(text: &str, encoding: Encoding, mut bytes: Vec<u8>) -> Result<(Vec<u8>, u8), Error> {
    match encoding {
        Encoding::Gsm7 => {
            let septets = gsm_string(text).ok_or(Error::Encoding)?;
            let header_septets = (bytes.len() * 8).div_ceil(7);
            let length = header_septets + septets.len();
            if length > 160 {
                return Err(Error::PartTooLong);
            }
            bytes.resize((length * 7).div_ceil(8), 0);
            for (index, value) in septets.into_iter().enumerate() {
                let bit = (header_septets + index) * 7;
                let position = bit / 8;
                let shift = bit % 8;
                bytes[position] |= value << shift;
                if shift > 1 {
                    bytes[position + 1] |= value >> (8 - shift);
                }
            }
            Ok((bytes, length as u8))
        }
        Encoding::Utf16 => {
            for unit in text.encode_utf16() {
                bytes.extend(unit.to_be_bytes());
            }
            if bytes.len() > 140 {
                return Err(Error::PartTooLong);
            }
            let length = bytes.len() as u8;
            Ok((bytes, length))
        }
    }
}
/// Encode one bounded part using the explicitly selected alphabet. Empty text is
/// a valid codec input; the user-intent planner rejects an empty message.
pub fn encode_part(
    recipient: &str,
    text: &str,
    encoding: Encoding,
    report: bool,
    concatenation: Option<Concatenation>,
) -> Result<Vec<u8>, Error> {
    let (digits, kind) = recipient_digits(recipient)?;
    let header = header(concatenation)?;
    if text.len() > 640 {
        return Err(Error::PartTooLong);
    }
    let (data, length) = user_data(text, encoding, header)?;
    let mut pdu = Vec::with_capacity(8 + digits.len().div_ceil(2) + data.len());
    let first = 1 | if report { 0x20 } else { 0 } | if concatenation.is_some() { 0x40 } else { 0 };
    pdu.extend([first, 0, digits.len() as u8, kind]);
    for pair in digits.chunks(2) {
        let low = pair[0] - b'0';
        let high = pair.get(1).map_or(15, |value| value - b'0');
        pdu.push(low | high << 4);
    }
    pdu.extend([0, if encoding == Encoding::Gsm7 { 0 } else { 8 }, length]);
    pdu.extend(data);
    Ok(pdu)
}
fn split(text: &str, encoding: Encoding, maximum: usize) -> Result<Vec<&str>, Error> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut units = 0;
    let mut scratch = Vec::with_capacity(2);
    for (index, character) in text.char_indices() {
        let cost = match encoding {
            Encoding::Utf16 => character.len_utf16(),
            Encoding::Gsm7 => {
                scratch.clear();
                if !gsm_character(character, &mut scratch) {
                    return Err(Error::Encoding);
                }
                scratch.len()
            }
        };
        if units + cost > maximum {
            if result.len() >= MAX_PARTS - 1 {
                return Err(Error::MessageTooLong);
            }
            result.push(&text[start..index]);
            start = index;
            units = 0;
        }
        units += cost;
    }
    result.push(&text[start..]);
    Ok(result)
}
/// Plan one current 3GPP implementation: default GSM alphabet or UTF-16BE, and
/// 16-bit concatenation references. The caller must reserve the supplied reference
/// durably before transmission; this pure function grants no sending permission.
pub fn plan_submit(
    recipient: &str,
    text: &str,
    report: bool,
    reference: u16,
) -> Result<Vec<SubmitPart>, Error> {
    recipient_digits(recipient)?;
    if text.is_empty() {
        return Err(Error::EmptyMessage);
    }
    if text.len() > MAX_MESSAGE_BYTES {
        return Err(Error::MessageTooLong);
    }
    let (encoding, units, single, multipart) = match gsm_string(text) {
        Some(septets) => (Encoding::Gsm7, septets.len(), 160, 152),
        None => (Encoding::Utf16, text.encode_utf16().count(), 70, 66),
    };
    let segments = if units <= single {
        vec![text]
    } else {
        split(text, encoding, multipart)?
    };
    let total = segments.len() as u8;
    segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            let concatenation = if total == 1 {
                None
            } else {
                Some(Concatenation {
                    reference,
                    total,
                    sequence: index as u8 + 1,
                })
            };
            Ok(SubmitPart {
                text: segment.into(),
                encoding,
                concatenation,
                tpdu: encode_part(recipient, segment, encoding, report, concatenation)?,
            })
        })
        .collect()
}

#[cfg(target_os = "linux")]
pub mod outbox;
