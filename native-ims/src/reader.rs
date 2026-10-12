//! Independent bounded reader for qualified scalar and opaque CP wire fields.
//! This module grants no response identity, IMS state or permission to publish.
//! A semantic decoder must validate the complete packet before exposing fields.

/// Local decoder policy, not a recovered vendor maximum.
pub const MAX_INPUT_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    InputTooLarge,
    InvalidRange,
    NegativeLength,
    Truncated,
    TrailingData,
}

/// One bounded immutable view with a cursor and a permanent first-error fence.
/// No stream, allocation, clone, seek, reset or Java runtime is retained.
pub struct DataReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    failed: Option<ReadError>,
}

impl<'a> DataReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, ReadError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(ReadError::InputTooLarge);
        }
        Self::from_range(bytes, 0, bytes.len() as i32)
    }

    /// Unlike the original stream wrapper, never clamps an invalid range.
    /// Signed wire values are checked before conversion or arithmetic.
    pub fn from_range(bytes: &'a [u8], offset: i32, length: i32) -> Result<Self, ReadError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(ReadError::InputTooLarge);
        }
        let offset = usize::try_from(offset).map_err(|_| ReadError::InvalidRange)?;
        let length = usize::try_from(length).map_err(|_| ReadError::InvalidRange)?;
        let end = offset.checked_add(length).ok_or(ReadError::InvalidRange)?;
        let view = bytes.get(offset..end).ok_or(ReadError::InvalidRange)?;
        Ok(Self {
            bytes: view,
            cursor: 0,
            failed: None,
        })
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }
    pub fn position(&self) -> usize {
        self.cursor
    }

    fn live(&self) -> Result<(), ReadError> {
        self.failed.map_or(Ok(()), Err)
    }

    fn fail<T>(&mut self, error: ReadError) -> Result<T, ReadError> {
        Err(*self.failed.get_or_insert(error))
    }

    fn take(&mut self, length: i32) -> Result<&'a [u8], ReadError> {
        self.live()?;
        let length = match usize::try_from(length) {
            Ok(value) => value,
            Err(_) => return self.fail(ReadError::NegativeLength),
        };
        let Some(end) = self
            .cursor
            .checked_add(length)
            .filter(|&end| end <= self.bytes.len())
        else {
            return self.fail(ReadError::Truncated);
        };
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    pub fn read_i8(&mut self) -> Result<i8, ReadError> {
        self.take(1).map(|v| v[0] as i8)
    }
    pub fn read_u8(&mut self) -> Result<u8, ReadError> {
        self.take(1).map(|v| v[0])
    }
    pub fn read_i16(&mut self) -> Result<i16, ReadError> {
        self.take(2).map(|v| i16::from_le_bytes([v[0], v[1]]))
    }
    pub fn read_u16(&mut self) -> Result<u16, ReadError> {
        self.take(2).map(|v| u16::from_le_bytes([v[0], v[1]]))
    }
    pub fn read_i32(&mut self) -> Result<i32, ReadError> {
        self.take(4)
            .map(|v| i32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    }
    pub fn read_i64(&mut self) -> Result<i64, ReadError> {
        self.take(8)
            .map(|v| i64::from_le_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]]))
    }
    pub fn read_bytes(&mut self, length: i32) -> Result<&'a [u8], ReadError> {
        self.take(length)
    }
    pub fn skip_bytes(&mut self, length: i32) -> Result<(), ReadError> {
        self.take(length).map(|_| ())
    }

    /// Signed length prefix, zero means absent, positive length selects exact raw
    /// bytes including NUL/non-UTF8. The original "String" getter renders a signed
    /// byte-array diagnostic; that diagnostic is not a text encoding contract.
    pub fn read_cp_byte_field(&mut self) -> Result<Option<&'a [u8]>, ReadError> {
        self.cp_field(false)
    }
    pub fn read_cp_short_field(&mut self) -> Result<Option<&'a [u8]>, ReadError> {
        self.cp_field(true)
    }
    fn cp_field(&mut self, wide: bool) -> Result<Option<&'a [u8]>, ReadError> {
        self.live()?;
        let width = if wide { 2 } else { 1 };
        if self.remaining() < width {
            return self.fail(ReadError::Truncated);
        }
        let prefix = &self.bytes[self.cursor..self.cursor + width];
        let length = if wide {
            i16::from_le_bytes([prefix[0], prefix[1]]) as i32
        } else {
            (prefix[0] as i8) as i32
        };
        let length = match usize::try_from(length) {
            Ok(value) => value,
            Err(_) => return self.fail(ReadError::NegativeLength),
        };
        let total = width + length;
        if total > self.remaining() {
            return self.fail(ReadError::Truncated);
        }
        let start = self.cursor + width;
        self.cursor += total;
        Ok(if length == 0 {
            None
        } else {
            Some(&self.bytes[start..self.cursor])
        })
    }

    /// Consumes the reader. A complete semantic decoder must check this before
    /// publishing; successful individual getters do not validate trailing bytes.
    pub fn finish(self) -> Result<(), ReadError> {
        self.live()?;
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(ReadError::TrailingData)
        }
    }
}
