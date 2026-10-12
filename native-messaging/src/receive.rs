//! Bounded text SMS-DELIVER parsing. Decoded sender/timestamp are untrusted
//! network content; parsing supplies no vendor receipt or delivery authority.
use crate::Encoding;
use sms_pdu::gsm_encoding::gsm_decode_string;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveError {
    Malformed,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceWidth {
    Eight,
    Sixteen,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomingConcatenation {
    pub reference: u16,
    pub total: u8,
    pub sequence: u8,
    pub width: ReferenceWidth,
}
pub struct Received {
    pub sender: String,
    pub text: String,
    pub encoding: Encoding,
    pub timestamp_millis: i64,
    pub concatenation: Option<IncomingConcatenation>,
    pub reply_path: bool,
    pub message_class: Option<u8>,
}
struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], ReceiveError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(ReceiveError::Malformed)?;
        let data = self
            .data
            .get(self.position..end)
            .ok_or(ReceiveError::Malformed)?;
        self.position = end;
        Ok(data)
    }
    fn byte(&mut self) -> Result<u8, ReceiveError> {
        Ok(self.take(1)?[0])
    }
}
fn decimal(value: u8) -> Result<u8, ReceiveError> {
    if value & 15 > 9 || value >> 4 > 9 {
        return Err(ReceiveError::Malformed);
    }
    Ok((value & 15) * 10 + (value >> 4))
}
fn timestamp(bytes: &[u8]) -> Result<i64, ReceiveError> {
    let year = decimal(bytes[0])?;
    let year = if year >= 90 {
        1900 + i32::from(year)
    } else {
        2000 + i32::from(year)
    };
    let month = time::Month::try_from(decimal(bytes[1])?).map_err(|_| ReceiveError::Malformed)?;
    let date = time::Date::from_calendar_date(year, month, decimal(bytes[2])?)
        .map_err(|_| ReceiveError::Malformed)?;
    let clock = time::Time::from_hms(decimal(bytes[3])?, decimal(bytes[4])?, decimal(bytes[5])?)
        .map_err(|_| ReceiveError::Malformed)?;
    let quarters = i32::from(decimal(bytes[6] & !8)?);
    let offset = time::UtcOffset::from_whole_seconds(
        quarters * 900 * if bytes[6] & 8 != 0 { -1 } else { 1 },
    )
    .map_err(|_| ReceiveError::Malformed)?;
    Ok(time::PrimitiveDateTime::new(date, clock)
        .assume_offset(offset)
        .unix_timestamp()
        * 1000)
}
fn septets(bytes: &[u8], start: usize, count: usize) -> Result<String, ReceiveError> {
    let mut unpacked = Vec::with_capacity(count);
    for index in 0..count {
        let bit = start + index * 7;
        let shift = bit % 8;
        let position = bit / 8;
        let mut value = u16::from(*bytes.get(position).ok_or(ReceiveError::Malformed)?) >> shift;
        if shift > 1 {
            value |=
                u16::from(*bytes.get(position + 1).ok_or(ReceiveError::Malformed)?) << (8 - shift);
        }
        unpacked.push((value & 127) as u8);
    }
    let mut text = String::new();
    let mut index = 0;
    while index < unpacked.len() {
        let begin = index;
        index += 1;
        if unpacked[begin] == 0x1b {
            let extension = *unpacked.get(index).ok_or(ReceiveError::Malformed)?;
            index += 1;
            if extension == 0x0a {
                text.push('\u{000c}');
                continue;
            }
        }
        let decoded = gsm_decode_string(&unpacked[begin..index]);
        if decoded.is_empty() {
            return Err(ReceiveError::Unsupported);
        }
        text.push_str(&decoded);
    }
    Ok(text)
}
fn address(cursor: &mut Cursor<'_>) -> Result<String, ReceiveError> {
    let length = usize::from(cursor.byte()?);
    let kind = cursor.byte()?;
    if length == 0 || length > 20 {
        return Err(ReceiveError::Malformed);
    }
    let bytes = cursor.take(length.div_ceil(2))?;
    if kind == 0xd0 {
        return septets(bytes, 0, length * 4 / 7);
    }
    if ![0x81, 0x91, 0xa1].contains(&kind) {
        return Err(ReceiveError::Unsupported);
    }
    let mut result = if kind == 0x91 {
        String::from("+")
    } else {
        String::new()
    };
    for index in 0..length {
        let digit = (bytes[index / 2] >> (index % 2 * 4)) & 15;
        if digit > 9 {
            return Err(ReceiveError::Malformed);
        }
        result.push(char::from(b'0' + digit));
    }
    if length % 2 != 0 && bytes[bytes.len() - 1] >> 4 != 15 {
        return Err(ReceiveError::Malformed);
    }
    Ok(result)
}
fn header(bytes: &[u8]) -> Result<(usize, Option<IncomingConcatenation>), ReceiveError> {
    let length = usize::from(*bytes.first().ok_or(ReceiveError::Malformed)?) + 1;
    let data = bytes.get(1..length).ok_or(ReceiveError::Malformed)?;
    let mut cursor = Cursor { data, position: 0 };
    let mut concat = None;
    while cursor.position < data.len() {
        let id = cursor.byte()?;
        let size = usize::from(cursor.byte()?);
        let value = cursor.take(size)?;
        let parsed = match (id, size) {
            (0, 3) => IncomingConcatenation {
                reference: u16::from(value[0]),
                total: value[1],
                sequence: value[2],
                width: ReferenceWidth::Eight,
            },
            (8, 4) => IncomingConcatenation {
                reference: u16::from_be_bytes([value[0], value[1]]),
                total: value[2],
                sequence: value[3],
                width: ReferenceWidth::Sixteen,
            },
            // Port addressing, language shifts, security and other headers need
            // a qualified receiving path; they must not silently become text.
            _ => return Err(ReceiveError::Unsupported),
        };
        if concat.is_some()
            || parsed.total == 0
            || parsed.sequence == 0
            || parsed.sequence > parsed.total
        {
            return Err(ReceiveError::Malformed);
        }
        concat = Some(parsed);
    }
    Ok((length, concat))
}
/// Parse a complete GSM PDU including its SMSC-length prefix. Current text
/// support is default GSM/UTF-16 and optional 8/16-bit concatenation. Unsupported
/// binary, message-waiting, PID and application headers remain explicit errors.
pub fn decode_delivery(pdu: &[u8]) -> Result<Received, ReceiveError> {
    if pdu.len() > 176 {
        return Err(ReceiveError::Malformed);
    }
    let mut cursor = Cursor {
        data: pdu,
        position: 0,
    };
    let center = usize::from(cursor.byte()?);
    if center > 12 {
        return Err(ReceiveError::Malformed);
    }
    cursor.take(center)?;
    let flags = cursor.byte()?;
    if flags & 3 != 0 {
        return Err(ReceiveError::Unsupported);
    }
    let sender = address(&mut cursor)?;
    if cursor.byte()? != 0 {
        return Err(ReceiveError::Unsupported);
    }
    let coding = cursor.byte()?;
    let encoding = match coding {
        0 | 0x10..=0x13 => Encoding::Gsm7,
        8 | 0x18..=0x1b => Encoding::Utf16,
        _ => return Err(ReceiveError::Unsupported),
    };
    let message_class = if coding & 0x10 != 0 {
        Some(coding & 3)
    } else {
        None
    };
    let timestamp_millis = timestamp(cursor.take(7)?)?;
    let units = usize::from(cursor.byte()?);
    let length = match encoding {
        Encoding::Gsm7 if units <= 160 => (units * 7).div_ceil(8),
        Encoding::Utf16 if units <= 140 => units,
        _ => return Err(ReceiveError::Malformed),
    };
    let bytes = cursor.take(length)?;
    if cursor.position != pdu.len() {
        return Err(ReceiveError::Malformed);
    }
    let (header_bytes, concatenation) = if flags & 0x40 != 0 {
        header(bytes)?
    } else {
        (0, None)
    };
    let text = match encoding {
        Encoding::Gsm7 => {
            let occupied = (header_bytes * 8).div_ceil(7);
            let count = units.checked_sub(occupied).ok_or(ReceiveError::Malformed)?;
            septets(bytes, occupied * 7, count)?
        }
        Encoding::Utf16 => {
            let data = &bytes[header_bytes..];
            if data.len() % 2 != 0 {
                return Err(ReceiveError::Malformed);
            }
            let words: Vec<_> = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16(&words).map_err(|_| ReceiveError::Malformed)?
        }
    };
    Ok(Received {
        sender,
        text,
        encoding,
        timestamp_millis,
        concatenation,
        reply_path: flags & 0x80 != 0,
        message_class,
    })
}
