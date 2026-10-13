//! Bounded receive-only worker ingress. Callbacks perform no SQLite or modem command.
use crate::{
    projection::Store,
    worker::{Binding, Response},
};
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_messaging::receipt::{IncomingKind, Source};
use rustix::fd::OwnedFd;
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};
pub const MAX_QUEUED: usize = 8;
pub const ARRIVAL_TTL_NS: u64 = 1_000_000_000;
pub trait Backend {
    fn initialize(&mut self, store: &mut Store, control: &Control, until: u64) -> io::Result<()>;
    fn after_raw(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_complete(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) struct Arrival {
    pub source: Source,
    pub kind: IncomingKind,
    pub raw: Vec<u8>,
    pub occurrence: u64,
    pub until: u64,
}
struct Sequence {
    next: u64,
    outstanding: usize,
}
struct Shared {
    fd: Arc<OwnedFd>,
    worker: String,
    generation: String,
    sequence: Mutex<Sequence>,
    failed: AtomicBool,
    start_full: AtomicBool,
    queue: SyncSender<Arrival>,
}
#[derive(Clone)]
pub struct Control(Arc<Shared>);
#[derive(Clone)]
pub struct Callback {
    control: Control,
    source: Source,
}
pub(crate) struct Incoming {
    pub control: Control,
    pub queue: Receiver<Arrival>,
}
impl Incoming {
    pub(crate) fn new(fd: Arc<OwnedFd>, worker: &str, generation: &str) -> Self {
        let (queue, receiver) = mpsc::sync_channel(MAX_QUEUED);
        Self {
            control: Control(Arc::new(Shared {
                fd,
                worker: worker.into(),
                generation: generation.into(),
                sequence: Mutex::new(Sequence {
                    next: 0,
                    outstanding: 0,
                }),
                failed: AtomicBool::new(false),
                start_full: AtomicBool::new(false),
                queue,
            })),
            queue: receiver,
        }
    }
}
impl Control {
    #[cfg(feature = "isolated-fixture")]
    pub fn fixture_send_buffer(&self, bytes: usize) -> io::Result<()> {
        Ok(rustix::net::sockopt::set_socket_send_buffer_size(
            &*self.0.fd,
            bytes,
        )?)
    }
    #[cfg(feature = "isolated-fixture")]
    pub fn fixture_start_eagain(&self) -> bool {
        self.0.start_full.load(Ordering::SeqCst)
    }
    pub fn failed(&self) -> bool {
        self.0.failed.load(Ordering::SeqCst)
    }
    fn send(&self, response: &Response) -> io::Result<()> {
        let result = ipc::send(&*self.0.fd, &serde_json::to_vec(response)?);
        if matches!(response, Response::ArrivalStarted { .. })
            && result.as_ref().is_err_and(|e| e.raw_os_error() == Some(11))
        {
            self.0.start_full.store(true, Ordering::SeqCst);
        }
        result
    }
    pub fn fail(&self, reason: &str) {
        if !self.0.failed.swap(true, Ordering::SeqCst) {
            let _ = self.send(&Response::SourceFailed {
                source: self.0.worker.clone(),
                reason: reason.into(),
            });
        }
        // Failure notification cannot depend on space in the metadata/arrival queue.
        let _ = rustix::net::shutdown(&*self.0.fd, rustix::net::Shutdown::Both);
    }
    pub fn bind(&self, source: Source) -> io::Result<Callback> {
        let binding = Binding {
            slot: source.slot().0,
            incarnation: source.incarnation().simple().to_string(),
            generation: self.0.generation.clone(),
        };
        if self.failed()
            || self
                .send(&Response::SourceBound {
                    source: self.0.worker.clone(),
                    binding,
                })
                .is_err()
        {
            self.fail("source_binding_ipc_failed");
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(Callback {
            control: self.clone(),
            source,
        })
    }
    pub(crate) fn raw_committed(&self, arrival: &Arrival, receipt: String) -> io::Result<()> {
        if self.failed() || boottime_ns() >= arrival.until {
            self.fail("arrival_deadline");
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.send(&Response::RawCommitted {
            source: self.0.worker.clone(),
            occurrence: arrival.occurrence,
            binding: Binding {
                slot: arrival.source.slot().0,
                incarnation: arrival.source.incarnation().simple().to_string(),
                generation: self.0.generation.clone(),
            },
            until: arrival.until,
            receipt,
        })
    }
    pub(crate) fn complete(&self, arrival: &Arrival, receipt: String) -> io::Result<()> {
        if self.failed() || boottime_ns() >= arrival.until {
            self.fail("arrival_deadline");
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.send(&Response::ArrivalComplete {
            source: self.0.worker.clone(),
            occurrence: arrival.occurrence,
            binding: Binding {
                slot: arrival.source.slot().0,
                incarnation: arrival.source.incarnation().simple().to_string(),
                generation: self.0.generation.clone(),
            },
            until: arrival.until,
            receipt,
        })?;
        let mut sequence = self.0.sequence.lock().map_err(|_| io::ErrorKind::Other)?;
        sequence.outstanding = sequence
            .outstanding
            .checked_sub(1)
            .ok_or(io::ErrorKind::InvalidData)?;
        Ok(())
    }
}
impl Callback {
    pub fn fail(&self, reason: &str) {
        self.control.fail(reason);
    }
    pub fn admit(&self, kind: IncomingKind, raw: &[u8]) -> io::Result<()> {
        // Capture ingress before any lock/scheduling delay; a late start never renews it.
        let Some(until) = boottime_ns().checked_add(ARRIVAL_TTL_NS) else {
            self.fail("deadline_exhausted");
            return Err(io::ErrorKind::Other.into());
        };
        if raw.len() > 512 || self.control.failed() {
            self.fail("raw_bound_or_failed_source");
            return Err(io::ErrorKind::InvalidData.into());
        }
        // This mutex spans only bounded memory/copy/nonblocking IPC, never SQLite/Binder commands.
        let mut sequence = self.control.0.sequence.lock().map_err(|_| {
            self.fail("callback_sequence_unavailable");
            io::ErrorKind::Other
        })?;
        if sequence.outstanding > MAX_QUEUED {
            self.fail("arrival_queue_overflow");
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let Some(occurrence) = sequence.next.checked_add(1) else {
            self.fail("occurrence_exhausted");
            return Err(io::ErrorKind::Other.into());
        };
        if boottime_ns() >= until {
            self.fail("arrival_deadline");
            return Err(io::ErrorKind::TimedOut.into());
        }
        let binding = Binding {
            slot: self.source.slot().0,
            incarnation: self.source.incarnation().simple().to_string(),
            generation: self.control.0.generation.clone(),
        };
        if self
            .control
            .send(&Response::ArrivalStarted {
                source: self.control.0.worker.clone(),
                occurrence,
                binding,
                until,
            })
            .is_err()
        {
            self.fail("arrival_start_ipc_failed");
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        sequence.next = occurrence;
        sequence.outstanding += 1;
        if self
            .control
            .0
            .queue
            .try_send(Arrival {
                source: self.source,
                kind,
                raw: raw.to_vec(),
                occurrence,
                until,
            })
            .is_err()
        {
            self.fail("arrival_queue_overflow");
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(())
    }
}
