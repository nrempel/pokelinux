//! CPIF-specific local read policy. No device acquisition or source authority.
//! A supported syntax observation can still be old queued data or a packet tail.
use super::{Error as TransportError, Outcome, Snapshot, WorkerTransport};
use pokelinux_process_owner::deadline::Deadline;
use pokelinux_sit_codec::{
    INITIAL_LTE_SETUP_V3_BYTES,
    received_chunk::ReceivedChunk,
    response::{BaseSetupResponse, DecodeError},
};
use rustix::{
    io::Errno,
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
        timerfd_settime,
    },
};
use std::{num::NonZeroU16, os::fd::OwnedFd, time::Duration};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Transport(TransportError),
    Timer(Errno),
    TimeRange,
    Poll(Errno),
    Readiness(u16),
    WorkLimit,
}

/// Opaque bytes plus the existing strict parser's local syntactic observation.
/// This carries no source generation, channel proof or attempt admission.
pub struct LocalChunk {
    bytes: ReceivedChunk,
    syntax: Result<BaseSetupResponse, DecodeError>,
}
impl LocalChunk {
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }
    pub fn syntax(&self) -> &Result<BaseSetupResponse, DecodeError> {
        &self.syntax
    }
}
pub enum Observation {
    Idle,
    Chunk(LocalChunk),
}

/// Owns the supplied descriptor and one absolute CLOCK_BOOTTIME timer.
/// Each admitted observe spends one unit before poll, including idle/I/O failure.
/// Predispatch, terminal and expired calls issue no poll and spend no unit.
/// Poll/read/close may block; this value belongs inside an owned worker process.
pub struct CpifReadPolicy {
    transport: WorkerTransport,
    timer: OwnedFd,
    remaining: u16,
}
impl CpifReadPolicy {
    pub fn new(
        fd: OwnedFd,
        original_until: Duration,
        work_limit: NonZeroU16,
    ) -> Result<Self, Error> {
        let seconds = original_until
            .as_secs()
            .try_into()
            .map_err(|_| Error::TimeRange)?;
        let timer = timerfd_create(
            TimerfdClockId::Boottime,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )
        .map_err(Error::Timer)?;
        timerfd_settime(
            &timer,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec::default(),
                it_value: Timespec {
                    tv_sec: seconds,
                    tv_nsec: original_until.subsec_nanos().into(),
                },
            },
        )
        .map_err(Error::Timer)?;
        Ok(Self {
            transport: WorkerTransport::new(fd, Deadline::at(original_until)),
            timer,
            remaining: work_limit.get(),
        })
    }
    pub fn dispatch(
        &mut self,
        request: &[u8; INITIAL_LTE_SETUP_V3_BYTES],
    ) -> Result<(), TransportError> {
        self.transport.dispatch(request)
    }
    pub fn snapshot(&self) -> Snapshot {
        self.transport.snapshot()
    }
    pub fn remaining(&self) -> u16 {
        self.remaining
    }
    pub fn observe(&mut self, expected_token: u32) -> Result<Observation, Error> {
        self.transport.preflight().map_err(Error::Transport)?;
        if self.transport.outcome == Outcome::NotSent {
            return Err(Error::Transport(TransportError::NotDispatched));
        }
        if self.remaining == 0 {
            self.transport.stop(super::StopReason::WorkLimit);
            return Err(Error::WorkLimit);
        }
        // No unbounded retry, wake or idle loop is hidden inside this operation.
        self.remaining -= 1;
        let readable = rustix::event::PollFlags::IN | rustix::event::PollFlags::RDNORM;
        let (result, events) = {
            let mut fds = [
                rustix::event::PollFd::new(
                    self.transport.fd.as_ref().expect("live descriptor"),
                    readable,
                ),
                rustix::event::PollFd::new(&self.timer, rustix::event::PollFlags::IN),
            ];
            let result = rustix::event::poll(&mut fds, None);
            (result, fds[0].revents())
        };
        // Absolute BOOTTIME timer and all rechecks share the original cutoff.
        self.transport.preflight().map_err(Error::Transport)?;
        if let Err(error) = result {
            self.transport.stop(super::StopReason::Poll);
            return Err(Error::Poll(error));
        }
        // HUP/ERR/NVAL, even alongside readable data, withdraw this generation.
        // A readable indication does not reserve a packet or prove its start.
        if !events.intersects(readable) || !(events & !readable).is_empty() {
            self.transport.stop(super::StopReason::Readiness);
            return Err(Error::Readiness(events.bits()));
        }
        let result = rustix::io::read(
            self.transport.fd.as_ref().expect("live descriptor"),
            &mut self.transport.receive[..],
        );
        self.transport.preflight().map_err(Error::Transport)?;
        let count = match result {
            Ok(0) => return Ok(Observation::Idle),
            Ok(count) => count,
            Err(error) => {
                self.transport.stop(super::StopReason::Read);
                return Err(Error::Transport(TransportError::Read(error)));
            }
        };
        let copied = ReceivedChunk::copy_from(&self.transport.receive[..count]);
        self.transport.preflight().map_err(Error::Transport)?;
        let bytes = match copied {
            Ok(bytes) => bytes,
            Err(error) => {
                self.transport.stop(super::StopReason::Copy);
                return Err(Error::Transport(TransportError::Copy(error)));
            }
        };
        let syntax = BaseSetupResponse::parse(bytes.as_bytes(), expected_token);
        self.transport.preflight().map_err(Error::Transport)?;
        Ok(Observation::Chunk(LocalChunk { bytes, syntax }))
    }
}
