//! Bounded parent-side queue; no process, scope-closure or result authority.
use crate::{boottime_ns, owner::ResponseFrame};
use rustix::fd::AsFd;
use std::io;

#[derive(Default)]
pub(super) struct PromptQueue {
    sequence: u16,
    pending: Option<u16>,
    response: Option<ResponseFrame>,
}

impl PromptQueue {
    pub(super) fn clear(&mut self) {
        self.response = None;
        self.pending = None;
    }

    pub(super) fn observe(
        &mut self,
        scope: Option<crate::owner::Snapshot>,
        worker_pid: u32,
        sequence: u16,
        retired: bool,
    ) -> io::Result<()> {
        if retired {
            self.clear();
            return Ok(());
        }
        let valid_scope =
            scope.is_some_and(|scope| scope.worker_pid == worker_pid && live_scope(scope));
        if !valid_scope
            || !(1..=32).contains(&sequence)
            || sequence != self.sequence + 1
            || self.pending.is_some()
            || self.response.is_some()
        {
            self.clear();
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.sequence = sequence;
        self.pending = Some(sequence);
        Ok(())
    }

    pub(super) fn pending(&self, until: u64) -> Option<u16> {
        (boottime_ns() < until && self.response.is_none())
            .then_some(self.pending)
            .flatten()
    }

    pub(super) fn respond(
        &mut self,
        until: u64,
        sequence: u16,
        value: zeroize::Zeroizing<Vec<u8>>,
    ) -> io::Result<()> {
        if boottime_ns() >= until {
            drop(value);
            self.clear();
            return Err(io::ErrorKind::TimedOut.into());
        }
        if self.pending != Some(sequence) || self.response.is_some() {
            drop(value);
            self.clear();
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let frame = match ResponseFrame::new(sequence, value) {
            Ok(frame) => frame,
            Err(error) => {
                self.clear();
                return Err(error);
            }
        };
        if boottime_ns() >= until {
            drop(frame);
            self.clear();
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.response = Some(frame);
        Ok(())
    }

    pub(super) fn flush(&mut self, until: u64, input: &impl AsFd) -> io::Result<()> {
        if boottime_ns() >= until {
            self.clear();
            return Err(io::ErrorKind::TimedOut.into());
        }
        if let Some(frame) = &mut self.response {
            match frame.write_once(input) {
                Ok(true) => self.clear(),
                Ok(false) if !frame.has_started() => {}
                // A <=517-byte nonblocking Linux pipe write must be atomic.
                // Never append a control byte after an unexpected partial frame.
                Ok(false) => {
                    self.clear();
                    return Err(io::ErrorKind::InvalidData.into());
                }
                Err(error) => {
                    self.clear();
                    return Err(error);
                }
            }
        }
        if boottime_ns() >= until {
            self.clear();
            return Err(io::ErrorKind::TimedOut.into());
        }
        Ok(())
    }
}

pub(super) fn live_scope(scope: crate::owner::Snapshot) -> bool {
    scope.occupied
        && scope.retired.is_none()
        && scope.worker_ready
        && scope.start_released
        && !scope.candidate_buffered
        && scope.worker_wait.is_none()
        && !scope.worker_stdout_eof
        && !scope.cleanup_fault
}
