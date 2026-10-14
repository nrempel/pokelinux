//! Bounded private pipe protocol. Responses are never formatted or serialized.
#![forbid(unsafe_code)]
use std::io::{self, Read, Write};

pub const MAX_RESPONSE: usize = 512;
pub const MAX_PROMPTS: u16 = 32;
pub const MAX_BUDGET_NS: u64 = 30_000_000_000;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Outcome {
    Authenticated = 0,
    Denied = 1,
    Unavailable = 2,
}
pub fn now() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (t.tv_sec as u64) * 1_000_000_000 + t.tv_nsec as u64
}
pub fn current(until: u64) -> io::Result<()> {
    if now() >= until {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
pub fn start(input: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 12];
    input.read_exact(&mut bytes)?;
    let until = u64::from_le_bytes(bytes[4..].try_into().unwrap());
    let sampled = now();
    if bytes[..4] != *b"PKPA" || until <= sampled || until - sampled > MAX_BUDGET_NS {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(until)
}
pub fn ready(output: &mut impl Write, until: u64) -> io::Result<()> {
    current(until)?;
    output.write_all(&[0x10])?;
    output.flush()?;
    current(until)
}
pub fn response(
    input: &mut impl Read,
    output: &mut impl Write,
    sequence: u16,
    until: u64,
) -> io::Result<zeroize::Zeroizing<Vec<u8>>> {
    if sequence == 0 || sequence > MAX_PROMPTS {
        return Err(io::ErrorKind::InvalidData.into());
    }
    current(until)?;
    let n = sequence.to_le_bytes();
    output.write_all(&[0x11, n[0], n[1]])?;
    output.flush()?;
    let mut header = [0; 5];
    input.read_exact(&mut header)?;
    let length = u16::from_le_bytes([header[3], header[4]]) as usize;
    if header[0] != 0x20 || header[1..3] != n || length == 0 || length > MAX_RESPONSE {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut response = zeroize::Zeroizing::new(vec![0; length]);
    input.read_exact(&mut response)?;
    current(until)?;
    if response.contains(&0) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(response)
}
pub fn finish(
    input: &mut impl Read,
    output: &mut impl Write,
    outcome: Outcome,
    until: u64,
) -> io::Result<()> {
    current(until)?;
    output.write_all(&[0x30, outcome as u8])?;
    output.flush()?;
    let mut ack = [0];
    input.read_exact(&mut ack)?;
    current(until)?;
    if ack != [0x40] {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}
