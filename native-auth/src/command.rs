//! Bounded decoding for the parent/supervisor private command pipe.
//!
//! Parsing does not grant response admission or lifecycle authority. The
//! callback must apply commands to its current target and reject commands after
//! target retirement. Callers own and zeroize read buffers, including any tail
//! left unconsumed after an error.
use std::io;
use zeroize::Zeroizing;

const MAX_SEQUENCE: u16 = 32;
const MAX_RESPONSE: usize = 512;

// No Debug, Clone or formatting: Response owns authentication input.
pub enum Command {
    Cancel,
    Stop,
    Snapshot,
    Response {
        sequence: u16,
        value: Zeroizing<Vec<u8>>,
    },
}

/// Local decoded intent, applied only after the caller's read buffer is gone.
/// A retiring batch never retains a response. It carries no result authority.
#[derive(Default)]
pub struct Batch {
    pub retired: Option<crate::owner::Retired>,
    pub snapshot: bool,
    pub response: Option<(u16, Zeroizing<Vec<u8>>)>,
}
impl Batch {
    fn retire(&mut self, reason: crate::owner::Retired) {
        self.response = None;
        self.retired.get_or_insert(reason);
    }
    fn accept(&mut self, command: Command) {
        match command {
            Command::Cancel => self.retire(crate::owner::Retired::Cancel),
            Command::Stop => self.retire(crate::owner::Retired::Stop),
            Command::Snapshot => self.snapshot = true,
            Command::Response { sequence, value } => {
                if self.retired.is_some() {
                    drop(value);
                } else if self.response.is_some() {
                    drop(value);
                    self.retire(crate::owner::Retired::Protocol);
                } else {
                    self.response = Some((sequence, value));
                }
            }
        }
    }
    pub(crate) fn retired_snapshot() -> Self {
        Self {
            snapshot: true,
            ..Self::default()
        }
    }
}

enum State {
    Opcode,
    Header {
        bytes: [u8; 4],
        used: usize,
    },
    Body {
        sequence: u16,
        expected: usize,
        value: Zeroizing<Vec<u8>>,
    },
    Unusable(io::ErrorKind),
}

// Only a fixed public header and one bounded zeroizing body are retained.
// Complete commands move directly to the callback; no secret Vec is drained.
pub struct Decoder {
    state: State,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            state: State::Opcode,
        }
    }

    /// Drop an incomplete command. This does not revive an unusable decoder.
    /// The caller must also discard its unread input before reusing the stream.
    pub fn discard(&mut self) {
        if !matches!(self.state, State::Unusable(_)) {
            self.state = State::Opcode;
        }
    }

    /// Drop buffered input and permanently refuse further commands.
    pub fn retire(&mut self) {
        if !matches!(self.state, State::Unusable(_)) {
            self.state = State::Unusable(io::ErrorKind::BrokenPipe);
        }
    }

    fn fail(&mut self, kind: io::ErrorKind) {
        self.state = State::Unusable(kind);
    }

    fn dispatch(
        &mut self,
        command: Command,
        callback: &mut impl FnMut(Command) -> io::Result<()>,
    ) -> io::Result<()> {
        if let Err(error) = callback(command) {
            self.fail(error.kind());
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn batch(&mut self, input: &[u8]) -> Batch {
        let mut batch = Batch::default();
        if self
            .feed(input, |command| {
                batch.accept(command);
                Ok(())
            })
            .is_err()
        {
            batch.retire(crate::owner::Retired::Protocol);
        }
        if batch.retired.is_some() {
            self.retire();
        }
        batch
    }

    /// Decode arbitrary fragments and dispatch every complete command in order.
    /// The first protocol error returns InvalidData; callback errors are returned
    /// unchanged. Either error latches its kind and drops all decoder-owned
    /// input. Later calls, including empty feeds, fail without calling back.
    pub fn feed(
        &mut self,
        mut input: &[u8],
        mut callback: impl FnMut(Command) -> io::Result<()>,
    ) -> io::Result<()> {
        if let State::Unusable(kind) = self.state {
            return Err(kind.into());
        }
        while !input.is_empty() {
            match std::mem::replace(&mut self.state, State::Opcode) {
                State::Opcode => {
                    let opcode = input[0];
                    input = &input[1..];
                    let command = match opcode {
                        b'C' => Command::Cancel,
                        b'S' => Command::Stop,
                        b'?' => Command::Snapshot,
                        0x20 => {
                            self.state = State::Header {
                                bytes: [0; 4],
                                used: 0,
                            };
                            continue;
                        }
                        _ => {
                            self.fail(io::ErrorKind::InvalidData);
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                    };
                    self.dispatch(command, &mut callback)?;
                }
                State::Header {
                    mut bytes,
                    mut used,
                } => {
                    let count = (bytes.len() - used).min(input.len());
                    bytes[used..used + count].copy_from_slice(&input[..count]);
                    used += count;
                    input = &input[count..];
                    if used < bytes.len() {
                        self.state = State::Header { bytes, used };
                        continue;
                    }
                    let sequence = u16::from_le_bytes([bytes[0], bytes[1]]);
                    let expected = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
                    if !(1..=MAX_SEQUENCE).contains(&sequence)
                        || !(1..=MAX_RESPONSE).contains(&expected)
                    {
                        self.fail(io::ErrorKind::InvalidData);
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    self.state = State::Body {
                        sequence,
                        expected,
                        value: Zeroizing::new(Vec::with_capacity(expected)),
                    };
                }
                State::Body {
                    sequence,
                    expected,
                    mut value,
                } => {
                    let count = (expected - value.len()).min(input.len());
                    let chunk = &input[..count];
                    if chunk.contains(&0) {
                        self.fail(io::ErrorKind::InvalidData);
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    value.extend_from_slice(chunk);
                    input = &input[count..];
                    if value.len() < expected {
                        self.state = State::Body {
                            sequence,
                            expected,
                            value,
                        };
                        continue;
                    }
                    self.dispatch(Command::Response { sequence, value }, &mut callback)?;
                }
                State::Unusable(_) => unreachable!("unusable decoder checked before parsing"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
