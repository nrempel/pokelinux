//! Owned raw bytes at the SIT backend's receive boundary.
//! Copying grants no framing, decoding, dispatch or modem capability.

// Native policy selected from the observed factory ManagerRun read buffer.
// This is neither an RCM protocol maximum nor a matched-kernel claim.
const MAX_RECEIVED_CHUNK_BYTES: usize = 65_568;

#[derive(Debug, PartialEq, Eq)]
pub enum CopyError {
    Empty,
    TooLarge,
    Allocation,
}

/// Opaque owned bytes with no mutable view or implicit clone.
pub struct ReceivedChunk {
    bytes: Vec<u8>,
}

impl ReceivedChunk {
    /// Copies one nonempty chunk within the native ingress policy bound.
    pub fn copy_from(bytes: &[u8]) -> Result<Self, CopyError> {
        if bytes.is_empty() {
            return Err(CopyError::Empty);
        }
        if bytes.len() > MAX_RECEIVED_CHUNK_BYTES {
            return Err(CopyError::TooLarge);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| CopyError::Allocation)?;
        owned.extend_from_slice(bytes);
        Ok(Self { bytes: owned })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
