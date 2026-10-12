//! Request fragmentation recovered from the original radio caller.
//! This deliberately corrects its exact-multiple truncation. Vendor endpoint
//! acceptance and modem transport are not established by this module.

use std::{iter::FusedIterator, num::TryFromIntError};

const SEGMENT_BYTES: usize = 2048;

/// Arguments for raw or segmented vendor calls, excluding shared routing IDs.
#[derive(Debug, PartialEq, Eq)]
pub enum RequestPart<'a> {
    Raw(&'a [u8]),
    Segment {
        index: i32,
        total_length: i32,
        payload: &'a [u8],
    },
}

/// Borrows the complete payload and yields each request part once, in order.
///
/// At most 2,048 bytes use one raw call (including an empty payload). Larger
/// payloads use indexed segments, each carrying the original full length.
/// Null payloads are not represented. This iterator does not send or retry calls.
#[derive(Debug)]
pub struct RequestParts<'a> {
    remaining: &'a [u8],
    total_length: i32,
    index: i32,
    raw_pending: bool,
}

fn wire_length(length: usize) -> Result<i32, TryFromIntError> {
    i32::try_from(length)
}

impl<'a> RequestParts<'a> {
    /// Rejects payload lengths that cannot fit the vendor's signed length field.
    pub fn new(payload: &'a [u8]) -> Result<Self, TryFromIntError> {
        Ok(Self {
            remaining: payload,
            total_length: wire_length(payload.len())?,
            index: 0,
            raw_pending: payload.len() <= SEGMENT_BYTES,
        })
    }
}

impl<'a> Iterator for RequestParts<'a> {
    type Item = RequestPart<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raw_pending {
            self.raw_pending = false;
            let payload = self.remaining;
            self.remaining = &[];
            return Some(RequestPart::Raw(payload));
        }
        if self.remaining.is_empty() {
            return None;
        }
        let length = self.remaining.len().min(SEGMENT_BYTES);
        let (payload, remaining) = self.remaining.split_at(length);
        let part = RequestPart::Segment {
            index: self.index,
            total_length: self.total_length,
            payload,
        };
        self.remaining = remaining;
        // The validated i32 full length bounds the segment count far below i32::MAX.
        self.index += 1;
        Some(part)
    }
}

impl FusedIterator for RequestParts<'_> {}

#[cfg(test)]
mod tests {
    use super::wire_length;

    #[test]
    fn signed_wire_length_is_checked_without_allocating_gigabytes() {
        assert_eq!(wire_length(0).unwrap(), 0);
        assert_eq!(wire_length(i32::MAX as usize).unwrap(), i32::MAX);
        assert!(wire_length(i32::MAX as usize + 1).is_err());
        assert!(wire_length(usize::MAX).is_err());
    }
}
