//! One local descriptor generation beneath the existing SIT attempt gate.
//! Descriptor ownership and local I/O grant no channel, provider or modem authority.
#![forbid(unsafe_code)]

#[cfg(not(target_os = "linux"))]
compile_error!("pokelinux-sit-transport requires Linux");

use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
use pokelinux_sit_codec::{
    INITIAL_LTE_SETUP_V3_BYTES,
    received_chunk::{CopyError, ReceivedChunk},
};
use rustix::io::Errno;
use std::os::fd::OwnedFd;

// Native receive-buffer policy, not a protocol maximum or complete-record size.
const RECEIVE_BYTES: usize = 65_568;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    NotSent,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Deadline,
    Write,
    Read,
    ZeroRead,
    Copy,
}

/// Recorded local state only; this value grants no current liveness or authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub outcome: Outcome,
    pub terminal: Option<StopReason>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Deadline,
    Terminal(StopReason),
    AlreadyDispatched,
    NotDispatched,
    Write(Errno),
    IncompleteWrite { written: usize },
    Read(Errno),
    ZeroRead,
    Copy(CopyError),
}

/// Consumes one handle without exposing, replacing or reopening its descriptor.
/// Methods may block in the kernel; a separate Supervisor must own responsiveness.
pub struct WorkerTransport {
    fd: Option<OwnedFd>,
    deadline: Deadline,
    outcome: Outcome,
    terminal: Option<StopReason>,
    // Reused only after each positive read has been copied into owned storage.
    receive: [u8; RECEIVE_BYTES],
}

impl WorkerTransport {
    pub fn new(fd: OwnedFd, deadline: Deadline) -> Self {
        Self {
            fd: Some(fd),
            deadline,
            outcome: Outcome::NotSent,
            terminal: None,
            receive: [0; RECEIVE_BYTES],
        }
    }

    /// Local state sampled here can be historical by the time it is observed.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            outcome: self.outcome,
            terminal: self.terminal,
        }
    }

    pub fn dispatch(&mut self, request: &[u8; INITIAL_LTE_SETUP_V3_BYTES]) -> Result<(), Error> {
        self.preflight()?;
        if self.outcome == Outcome::OutcomeUnknown {
            return Err(Error::AlreadyDispatched);
        }
        // Crossing this boundary may have a remote effect even when write fails.
        self.outcome = Outcome::OutcomeUnknown;
        let result = rustix::io::write(self.fd.as_ref().expect("live descriptor"), request);
        // Preserve the original suspend-aware deadline after the actual syscall.
        self.preflight()?;
        match result {
            Ok(written) if written == request.len() => Ok(()),
            Ok(written) => {
                self.stop(StopReason::Write);
                Err(Error::IncompleteWrite { written })
            }
            Err(error) => {
                self.stop(StopReason::Write);
                Err(Error::Write(error))
            }
        }
    }

    pub fn read_chunk(&mut self) -> Result<ReceivedChunk, Error> {
        self.preflight()?;
        if self.outcome == Outcome::NotSent {
            return Err(Error::NotDispatched);
        }
        let result = rustix::io::read(
            self.fd.as_ref().expect("live descriptor"),
            &mut self.receive[..],
        );
        self.preflight()?;
        let count = match result {
            Ok(0) => {
                self.stop(StopReason::ZeroRead);
                return Err(Error::ZeroRead);
            }
            Ok(count) => count,
            Err(error) => {
                self.stop(StopReason::Read);
                return Err(Error::Read(error));
            }
        };
        let chunk = ReceivedChunk::copy_from(&self.receive[..count]);
        // Allocation/copy also spends the same deadline; no late bytes escape.
        self.preflight()?;
        match chunk {
            Ok(chunk) => Ok(chunk),
            Err(error) => {
                self.stop(StopReason::Copy);
                Err(Error::Copy(error))
            }
        }
    }

    fn stop(&mut self, reason: StopReason) {
        if self.terminal.is_none() {
            self.terminal = Some(reason);
            drop(self.fd.take());
        }
    }

    fn preflight(&mut self) -> Result<(), Error> {
        if let Some(reason) = self.terminal {
            return Err(Error::Terminal(reason));
        }
        if self.deadline.elapsed(BootClock.now()) {
            self.stop(StopReason::Deadline);
            return Err(Error::Deadline);
        }
        Ok(())
    }
}
