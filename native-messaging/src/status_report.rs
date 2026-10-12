//! Bounded GSM SMS-STATUS-REPORT metadata. Parsing establishes no delivery identity.
use crate::Encoding;
use sms_pdu::gsm_encoding::gsm_decode_string;

pub const MAX_TPDU_BYTES: usize = 164;
pub const MAX_PDU_BYTES: usize = 1 + 12 + MAX_TPDU_BYTES;
pub const MAX_PARAMETER_INDICATORS: usize = 4;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Malformed,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportQualifier {
    Submit,
    Command,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserData {
    pub encoding: Encoding,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalParameters {
    pub protocol_identifier: Option<u8>,
    pub data_coding_scheme: Option<u8>,
    pub user_data: Option<UserData>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    pub service_center: Option<String>,
    pub message_reference: u8,
    pub recipient: String,
    pub service_center_timestamp_millis: i64,
    pub discharge_timestamp_millis: i64,
    pub status: u8,
    pub qualifier: ReportQualifier,
    pub more_messages: bool,
    pub loop_prevention: bool,
    pub user_data_header: bool,
    pub optional: Option<OptionalParameters>,
}
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(DecodeError::Malformed)?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(DecodeError::Malformed)?;
        self.position = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn complete(&self) -> bool {
        self.position == self.bytes.len()
    }
}
fn number(kind: u8, bytes: &[u8], digits: usize) -> Result<String, DecodeError> {
    if ![0x81, 0x91, 0xa1].contains(&kind) {
        return Err(DecodeError::Unsupported);
    }
    if digits == 0 || bytes.len() != digits.div_ceil(2) {
        return Err(DecodeError::Malformed);
    }
    let mut result = if kind == 0x91 {
        String::from("+")
    } else {
        String::new()
    };
    for index in 0..digits {
        let digit = (bytes[index / 2] >> (index % 2 * 4)) & 15;
        if digit > 9 {
            return Err(DecodeError::Malformed);
        }
        result.push(char::from(b'0' + digit));
    }
    if !digits.is_multiple_of(2) && bytes[bytes.len() - 1] >> 4 != 15 {
        return Err(DecodeError::Malformed);
    }
    Ok(result)
}
fn service_center(cursor: &mut Cursor<'_>) -> Result<Option<String>, DecodeError> {
    let length = usize::from(cursor.byte()?);
    if length == 0 {
        return Ok(None);
    }
    if !(2..=12).contains(&length) {
        return Err(DecodeError::Malformed);
    }
    let bytes = cursor.take(length)?;
    let address = &bytes[1..];
    let digits = address.len() * 2 - usize::from(address[address.len() - 1] >> 4 == 15);
    number(bytes[0], address, digits).map(Some)
}
fn recipient(cursor: &mut Cursor<'_>) -> Result<String, DecodeError> {
    let digits = usize::from(cursor.byte()?);
    if !(1..=20).contains(&digits) {
        return Err(DecodeError::Malformed);
    }
    let kind = cursor.byte()?;
    number(kind, cursor.take(digits.div_ceil(2))?, digits)
}
fn decimal(byte: u8) -> Result<u8, DecodeError> {
    if byte & 15 > 9 || byte >> 4 > 9 {
        return Err(DecodeError::Malformed);
    }
    Ok((byte & 15) * 10 + (byte >> 4))
}
fn timestamp(bytes: &[u8]) -> Result<i64, DecodeError> {
    let year = decimal(bytes[0])?;
    let year = i32::from(year) + if year >= 90 { 1900 } else { 2000 };
    let month = time::Month::try_from(decimal(bytes[1])?).map_err(|_| DecodeError::Malformed)?;
    let date = time::Date::from_calendar_date(year, month, decimal(bytes[2])?)
        .map_err(|_| DecodeError::Malformed)?;
    let clock = time::Time::from_hms(decimal(bytes[3])?, decimal(bytes[4])?, decimal(bytes[5])?)
        .map_err(|_| DecodeError::Malformed)?;
    let quarters = i64::from(decimal(bytes[6] & !8)?);
    let direction = if bytes[6] & 8 == 0 { 1 } else { -1 };
    Ok((time::PrimitiveDateTime::new(date, clock)
        .assume_utc()
        .unix_timestamp()
        - quarters * 900 * direction)
        * 1000)
}
fn gsm_text(bytes: &[u8], units: usize) -> Result<String, DecodeError> {
    let mut septets = Vec::with_capacity(units);
    for index in 0..units {
        let bit = index * 7;
        let shift = bit % 8;
        let position = bit / 8;
        let mut value = u16::from(*bytes.get(position).ok_or(DecodeError::Malformed)?) >> shift;
        if shift > 1 {
            value |=
                u16::from(*bytes.get(position + 1).ok_or(DecodeError::Malformed)?) << (8 - shift);
        }
        septets.push((value & 127) as u8);
    }
    let mut text = String::new();
    let mut index = 0;
    while index < septets.len() {
        let begin = index;
        index += 1;
        if septets[begin] == 0x1b {
            let extension = *septets.get(index).ok_or(DecodeError::Malformed)?;
            index += 1;
            if extension == 0x0a {
                text.push('\u{000c}');
                continue;
            }
        }
        let value = gsm_decode_string(&septets[begin..index]);
        if value.is_empty() {
            return Err(DecodeError::Unsupported);
        }
        text.push_str(&value);
    }
    Ok(text)
}
fn user_data(cursor: &mut Cursor<'_>, coding: u8, header: bool) -> Result<UserData, DecodeError> {
    // Application headers, compression, 8-bit payload and other alphabets need
    // independent original-parser comparisons before this path accepts them.
    if header {
        return Err(DecodeError::Unsupported);
    }
    let encoding = match coding {
        0 | 0x10..=0x13 => Encoding::Gsm7,
        8 | 0x18..=0x1b => Encoding::Utf16,
        _ => return Err(DecodeError::Unsupported),
    };
    let units = usize::from(cursor.byte()?);
    let length = match encoding {
        Encoding::Gsm7 if units <= 160 => (units * 7).div_ceil(8),
        Encoding::Utf16 if units <= 140 && units.is_multiple_of(2) => units,
        _ => return Err(DecodeError::Malformed),
    };
    let bytes = cursor.take(length)?;
    let text = match encoding {
        Encoding::Gsm7 => gsm_text(bytes, units)?,
        Encoding::Utf16 => {
            let words: Vec<_> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_be_bytes(*b))
                .collect();
            String::from_utf16(&words).map_err(|_| DecodeError::Malformed)?
        }
    };
    Ok(UserData { encoding, text })
}
fn parameters(
    cursor: &mut Cursor<'_>,
    header: bool,
) -> Result<Option<OptionalParameters>, DecodeError> {
    if cursor.complete() {
        return Ok(None);
    }
    let first = cursor.byte()?;
    let mut last = first;
    let mut count = 1;
    while last & 0x80 != 0 {
        if count >= MAX_PARAMETER_INDICATORS {
            return Err(DecodeError::Malformed);
        }
        last = cursor.byte()?;
        count += 1;
        if last & 0x7f != 0 {
            return Err(DecodeError::Unsupported);
        }
    }
    if first & 0x78 != 0 {
        return Err(DecodeError::Unsupported);
    }
    let protocol_identifier = if first & 1 != 0 {
        Some(cursor.byte()?)
    } else {
        None
    };
    let data_coding_scheme = if first & 2 != 0 {
        Some(cursor.byte()?)
    } else {
        None
    };
    let user_data = if first & 4 != 0 {
        Some(user_data(cursor, data_coding_scheme.unwrap_or(0), header)?)
    } else {
        None
    };
    Ok(Some(OptionalParameters {
        protocol_identifier,
        data_coding_scheme,
        user_data,
    }))
}
/// Decode a complete bounded GSM PDU including its numeric SMSC prefix. Status,
/// addresses and timestamps are untrusted metadata, never unique message keys.
/// Optional text supports default GSM and UTF-16 without a user-data header.
pub fn decode_status_report(pdu: &[u8]) -> Result<StatusReport, DecodeError> {
    if pdu.len() > MAX_PDU_BYTES {
        return Err(DecodeError::Malformed);
    }
    let mut cursor = Cursor {
        bytes: pdu,
        position: 0,
    };
    let service_center = service_center(&mut cursor)?;
    if pdu.len() - cursor.position > MAX_TPDU_BYTES {
        return Err(DecodeError::Malformed);
    }
    let flags = cursor.byte()?;
    if flags & 3 != 2 {
        return Err(DecodeError::Unsupported);
    }
    let message_reference = cursor.byte()?;
    let recipient = recipient(&mut cursor)?;
    let service_center_timestamp_millis = timestamp(cursor.take(7)?)?;
    let discharge_timestamp_millis = timestamp(cursor.take(7)?)?;
    let status = cursor.byte()?;
    let user_data_header = flags & 0x40 != 0;
    let optional = parameters(&mut cursor, user_data_header)?;
    if !cursor.complete() {
        return Err(DecodeError::Malformed);
    }
    Ok(StatusReport {
        service_center,
        message_reference,
        recipient,
        service_center_timestamp_millis,
        discharge_timestamp_millis,
        status,
        qualifier: if flags & 0x20 == 0 {
            ReportQualifier::Submit
        } else {
            ReportQualifier::Command
        },
        more_messages: flags & 4 == 0,
        loop_prevention: flags & 8 != 0,
        user_data_header,
        optional,
    })
}
/// The transport hex boundary accepts complete ASCII octets with no trimming.
pub fn decode_status_report_hex(pdu: &str) -> Result<StatusReport, DecodeError> {
    if pdu.len() > MAX_PDU_BYTES * 2 || !pdu.len().is_multiple_of(2) {
        return Err(DecodeError::Malformed);
    }
    fn nibble(value: u8) -> Result<u8, DecodeError> {
        match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            b'A'..=b'F' => Ok(value - b'A' + 10),
            _ => Err(DecodeError::Malformed),
        }
    }
    let bytes: Result<Vec<_>, _> = pdu
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect();
    decode_status_report(&bytes?)
}
