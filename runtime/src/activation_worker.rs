//! One in-flight app activation, with pollable completion and no request queue.
use crate::notification_bus::activate_phone;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{io, os::unix::net::UnixStream, thread};

struct Pending {
    completion: UnixStream,
    thread: thread::JoinHandle<zbus::Result<()>>,
}
#[derive(Default)]
pub struct ActivationSlot {
    pending: Option<Pending>,
}
impl ActivationSlot {
    /// The service checks current activation policy before admission. Reject an
    /// overlap even if the old result is ready: it must be consumed explicitly.
    pub fn start(&mut self, address: String) -> io::Result<()> {
        if self.pending.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Phone activation pending",
            ));
        }
        let (completion, wake) = UnixStream::pair()?;
        completion.set_nonblocking(true)?;
        let thread = thread::Builder::new()
            .name("phone-activation".into())
            .spawn(move || {
                // EOF wakes poll on normal completion or unwind. No wake-byte write
                // can fail or block; this worker owns the sole peer descriptor.
                let _wake = wake;
                futures_lite::future::block_on(activate_phone(&address))
            })?;
        self.pending = Some(Pending { completion, thread });
        Ok(())
    }
    pub fn completion(&self) -> Option<&UnixStream> {
        self.pending.as_ref().map(|pending| &pending.completion)
    }
    /// Does not wait for D-Bus. Once the worker closes its peer, join and return
    /// its result exactly once. An I/O/panic error is distinct from app refusal.
    pub fn finish(&mut self) -> io::Result<Option<zbus::Result<()>>> {
        let Some(pending) = &self.pending else {
            return Ok(None);
        };
        let mut fds = [PollFd::new(&pending.completion, PollFlags::IN)];
        match poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        ) {
            Ok(0) | Err(rustix::io::Errno::INTR) => return Ok(None),
            Err(error) => return Err(error.into()),
            Ok(_) => (),
        }
        let pending = self.pending.take().expect("pending activation");
        pending
            .thread
            .join()
            .map(Some)
            .map_err(|_| io::Error::other("Phone activation worker failed"))
    }
}
impl Drop for ActivationSlot {
    fn drop(&mut self) {
        if let Some(pending) = self.pending.take() {
            // activate_phone bounds setup and exchange; do not detach work on
            // service shutdown. The normal event loop uses nonblocking finish.
            let _ = pending.thread.join();
        }
    }
}
