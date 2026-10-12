//! Bounded reconstruction of one candidate, not proof of message-instance identity.
//! The session owner must supply lifetime, aggregate budgets and epoch fencing.
use crate::indication::{IndicationKind, RadioIndication};
use std::collections::BTreeMap;

/// Local supported limits, not inferred vendor maxima. Assembly briefly owns
/// both stored fragments and output; budget up to twice `bytes` plus map overhead.
#[derive(Clone, Copy)]
pub struct Limits {
    pub bytes: usize,
    pub fragments: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyError {
    Limit,
    Total,
    Routing,
    Index,
    Duplicate,
    Length,
    Gap,
    NotSegment,
    Allocation,
    Finished,
}

pub struct Assembly {
    message_id: i32,
    slot_id: i32,
    total: usize,
    limits: Limits,
    parts: BTreeMap<usize, Vec<u8>>,
    received: usize,
    failure: Option<AssemblyError>,
    finished: bool,
}
impl Assembly {
    pub fn new(
        message_id: i32,
        slot_id: i32,
        total: i32,
        limits: Limits,
    ) -> Result<Self, AssemblyError> {
        if limits.bytes == 0 || limits.fragments == 0 {
            return Err(AssemblyError::Limit);
        }
        let total = usize::try_from(total).map_err(|_| AssemblyError::Total)?;
        if total > limits.bytes {
            return Err(AssemblyError::Limit);
        }
        Ok(Self {
            message_id,
            slot_id,
            total,
            limits,
            parts: BTreeMap::new(),
            received: 0,
            failure: None,
            finished: false,
        })
    }
    /// Any error invalidates this candidate permanently; never reseed it with a
    /// later fragment. Successful completion also consumes its lifetime.
    pub fn push(&mut self, event: &RadioIndication) -> Result<Option<Vec<u8>>, AssemblyError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.finished {
            return Err(AssemblyError::Finished);
        }
        let result = self.accept(event);
        if let Err(error) = result {
            self.parts.clear();
            self.received = 0;
            self.failure = Some(error);
        }
        result
    }
    fn accept(&mut self, event: &RadioIndication) -> Result<Option<Vec<u8>>, AssemblyError> {
        if event.message_id != self.message_id || event.slot_id != self.slot_id {
            return Err(AssemblyError::Routing);
        }
        let IndicationKind::Segment {
            index,
            total_length,
        } = event.kind
        else {
            return Err(AssemblyError::NotSegment);
        };
        if usize::try_from(total_length).ok() != Some(self.total) {
            return Err(AssemblyError::Total);
        }
        let index = usize::try_from(index).map_err(|_| AssemblyError::Index)?;
        if index >= self.limits.fragments {
            return Err(AssemblyError::Index);
        }
        if let Some(previous) = self.parts.get(&index) {
            return if previous == &event.payload {
                Ok(None)
            } else {
                Err(AssemblyError::Duplicate)
            };
        }
        if event.payload.is_empty() && self.total != 0 {
            return Err(AssemblyError::Length);
        }
        let received = self
            .received
            .checked_add(event.payload.len())
            .ok_or(AssemblyError::Length)?;
        if received > self.total {
            return Err(AssemblyError::Length);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(event.payload.len())
            .map_err(|_| AssemblyError::Allocation)?;
        owned.extend_from_slice(&event.payload);
        self.parts.insert(index, owned);
        self.received = received;
        if received != self.total {
            return Ok(None);
        }
        if self
            .parts
            .keys()
            .copied()
            .enumerate()
            .any(|(expected, actual)| expected != actual)
        {
            return Err(AssemblyError::Gap);
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(self.total)
            .map_err(|_| AssemblyError::Allocation)?;
        for (_, part) in std::mem::take(&mut self.parts) {
            output.extend_from_slice(&part);
        }
        self.finished = true;
        Ok(Some(output))
    }
}
