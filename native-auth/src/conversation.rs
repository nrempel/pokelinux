//! Private conversation transport. No process/cgroup custody or result authority.
use super::{Barrier, Candidate, Retired, boottime_ns, nonblocking};
use rustix::fd::AsFd;
use std::io;

const MAX_PROMPTS: u16 = 32;
const MAX_RESPONSE: usize = 512;

pub(super) fn parse_worker_protocol(
    protocol: &mut Vec<u8>,
    barrier: &mut Barrier,
    pending: &mut Vec<u8>,
) -> io::Result<()> {
    if barrier.retired.is_some() {
        return Ok(());
    }
    if protocol.is_empty() {
        return Ok(());
    }
    if !barrier.start_released {
        return Err(io::ErrorKind::InvalidData.into());
    }
    if !barrier.worker_ready && protocol.first() == Some(&0x10) {
        protocol.remove(0);
        barrier.worker_ready = true;
    }
    if protocol.is_empty() {
        return Ok(());
    }
    if !barrier.worker_ready || barrier.candidate.is_some() || barrier.hidden_prompt.is_some() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    match protocol[0] {
        0x11 => {
            if protocol.len() < 3 {
                return Ok(());
            }
            let sequence = u16::from_le_bytes([protocol[1], protocol[2]]);
            if protocol.len() != 3
                || sequence == 0
                || sequence > MAX_PROMPTS
                || sequence != barrier.prompts + 1
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            barrier.prompts = sequence;
            barrier.hidden_prompt = Some(sequence);
            protocol.clear();
        }
        0x30 => {
            if protocol.len() == 1 {
                return Ok(());
            }
            if protocol.len() != 2 {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let candidate = match protocol[1] {
                0 => Candidate::Authenticated,
                1 => Candidate::Denied,
                2 => Candidate::Unavailable,
                _ => return Err(io::ErrorKind::InvalidData.into()),
            };
            protocol.clear();
            barrier.candidate = Some(candidate);
            pending.push(0x40);
        }
        _ => return Err(io::ErrorKind::InvalidData.into()),
    }
    Ok(())
}

// No formatting/Debug/Clone. Keep the complete frame until release so prefix
// writes never leave forgotten response bytes in a drained Vec's spare memory.
pub(crate) struct ResponseFrame {
    pub(super) bytes: zeroize::Zeroizing<Vec<u8>>,
    pub(super) offset: usize,
}
impl ResponseFrame {
    pub(crate) fn new(sequence: u16, value: zeroize::Zeroizing<Vec<u8>>) -> io::Result<Self> {
        if !(1..=MAX_PROMPTS).contains(&sequence)
            || value.is_empty()
            || value.len() > MAX_RESPONSE
            || value.contains(&0)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity(5 + value.len()));
        bytes.push(0x20);
        bytes.extend_from_slice(&sequence.to_le_bytes());
        bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&value);
        Ok(Self { bytes, offset: 0 })
    }
    pub(crate) fn write_once(&mut self, fd: &impl AsFd) -> io::Result<bool> {
        if self.offset == self.bytes.len() {
            return Ok(true);
        }
        match rustix::io::write(fd, &self.bytes[self.offset..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => self.offset += count,
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(self.offset == self.bytes.len())
    }
    pub(crate) fn has_started(&self) -> bool {
        self.offset != 0
    }
}

pub(super) struct Conversation {
    until: u64,
    // Neither retained pipe can be used until both became nonblocking.
    initialized: bool,
    pending: Vec<u8>,
    response: Option<ResponseFrame>,
    protocol: Vec<u8>,
}
impl Conversation {
    pub(super) fn new(until: u64) -> Self {
        Self {
            until,
            initialized: false,
            pending: Vec::new(),
            response: None,
            protocol: Vec::new(),
        }
    }
    fn require_initialized(&mut self, barrier: &mut Barrier) -> io::Result<()> {
        if !self.initialized {
            self.retire(barrier, Retired::Setup);
            return Err(io::ErrorKind::NotConnected.into());
        }
        Ok(())
    }
    fn expire(&mut self, barrier: &mut Barrier) -> io::Result<()> {
        if barrier.retired.is_none() && boottime_ns() >= self.until {
            self.retire(barrier, Retired::Deadline);
            return Err(io::ErrorKind::TimedOut.into());
        }
        Ok(())
    }
    pub(super) fn deadline(&self) -> u64 {
        self.until
    }
    pub(super) fn initialize(
        &mut self,
        input: Option<&impl AsFd>,
        output: Option<&impl AsFd>,
    ) -> io::Result<()> {
        self.initialized = false;
        nonblocking(input.ok_or(io::ErrorKind::Other)?)?;
        nonblocking(output.ok_or(io::ErrorKind::Other)?)?;
        self.initialized = true;
        Ok(())
    }
    pub(super) fn release_start(
        &mut self,
        input: Option<&impl AsFd>,
        barrier: &mut Barrier,
    ) -> io::Result<()> {
        self.require_initialized(barrier)?;
        self.expire(barrier)?;
        self.pending.extend_from_slice(b"PKPA");
        self.pending.extend_from_slice(&self.until.to_le_bytes());
        self.write_pending(input, barrier)
    }
    pub(super) fn retire(&mut self, barrier: &mut Barrier, reason: Retired) {
        barrier.retire(reason);
        self.response = None;
        self.pending.clear();
        self.protocol.clear();
    }
    /// Observation only. The original deadline is never replaced or renewed.
    pub(super) fn pending_hidden_prompt(&self, barrier: &Barrier) -> Option<u16> {
        (self.initialized && barrier.retired.is_none() && boottime_ns() < self.until)
            .then_some(barrier.hidden_prompt)
            .flatten()
    }

    /// Queue one matching response; all pipe writes remain nonblocking.
    /// Called only in the dedicated supervisor. This isolated API is not wired
    /// into a broker or app ingress; its outside parent retains responsiveness.
    pub(super) fn respond_hidden_prompt(
        &mut self,
        barrier: &mut Barrier,
        sequence: u16,
        value: zeroize::Zeroizing<Vec<u8>>,
    ) -> io::Result<()> {
        if !self.initialized {
            drop(value);
            self.retire(barrier, Retired::Setup);
            return Err(io::ErrorKind::NotConnected.into());
        }
        if boottime_ns() >= self.until {
            drop(value);
            self.retire(barrier, Retired::Deadline);
            return Err(io::ErrorKind::TimedOut.into());
        }
        if barrier.retired.is_some()
            || barrier.hidden_prompt != Some(sequence)
            || self.response.is_some()
            || barrier.candidate.is_some()
            || !self.pending.is_empty()
        {
            drop(value);
            self.retire(barrier, Retired::Protocol);
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let frame = match ResponseFrame::new(sequence, value) {
            Ok(frame) => frame,
            Err(error) => {
                self.retire(barrier, Retired::Protocol);
                return Err(error);
            }
        };
        if boottime_ns() >= self.until {
            drop(frame);
            self.retire(barrier, Retired::Deadline);
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.response = Some(frame);
        Ok(())
    }

    pub(super) fn write_pending(
        &mut self,
        input: Option<&impl AsFd>,
        barrier: &mut Barrier,
    ) -> io::Result<()> {
        self.require_initialized(barrier)?;
        self.expire(barrier)?;
        self.write_once(input, barrier)?;
        self.expire(barrier)
    }

    fn write_once(&mut self, input: Option<&impl AsFd>, barrier: &mut Barrier) -> io::Result<()> {
        if self.pending.is_empty() && self.response.is_none() {
            return Ok(());
        }
        let input = input.ok_or(io::ErrorKind::BrokenPipe)?;
        if !self.pending.is_empty() {
            match rustix::io::write(input, &self.pending) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => {
                    self.pending.drain(..count);
                    if self.pending.is_empty() && !barrier.start_released {
                        barrier.start_released = true;
                    }
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
            return Ok(());
        }
        if let Some(frame) = &mut self.response
            && frame.write_once(input)?
        {
            self.response = None;
            barrier.hidden_prompt = None;
        }
        Ok(())
    }

    pub(super) fn read_protocol(
        &mut self,
        output: Option<&impl AsFd>,
        barrier: &mut Barrier,
    ) -> io::Result<()> {
        self.require_initialized(barrier)?;
        self.expire(barrier)?;
        let output = output.ok_or(io::ErrorKind::BrokenPipe)?;
        let mut bytes = [0; 16];
        match rustix::io::read(output, &mut bytes) {
            Ok(0) => {
                barrier.worker_stdout_eof = true;
                if barrier.retired.is_none() && barrier.candidate.is_none() {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
            }
            Ok(count) => {
                if barrier.retired.is_none() {
                    self.protocol.extend_from_slice(&bytes[..count]);
                }
            }
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        // A resume may occur after the pre-read clock sample. Recheck before
        // any new frame can enter local candidate state.
        self.expire(barrier)?;
        parse_worker_protocol(&mut self.protocol, barrier, &mut self.pending)
    }
}
