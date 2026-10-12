//! Bounded handoff from the async notification loop to the daemon poll loop.
//! Events retain order; only the eventfd wake is coalesced. No disk/network I/O.
use crate::notification_generation::Notice;
use rustix::{
    event::{EventfdFlags, eventfd},
    fd::{AsFd, BorrowedFd, OwnedFd},
};
use std::{
    io,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
};
const CAPACITY: usize = 16;

pub struct NoticeSender {
    sender: Option<SyncSender<Notice>>,
    changed: Arc<OwnedFd>,
}
pub struct NoticeReceiver {
    receiver: Receiver<Notice>,
    changed: Arc<OwnedFd>,
}
pub struct NoticeBatch {
    pub notices: Vec<Notice>,
    pub closed: bool,
}
fn signal(fd: &OwnedFd) -> io::Result<()> {
    loop {
        match rustix::io::write(fd, &1u64.to_ne_bytes()) {
            Ok(8) => return Ok(()),
            Err(rustix::io::Errno::INTR) => continue,
            Err(rustix::io::Errno::AGAIN) => return Ok(()),
            Err(error) => return Err(error.into()),
            Ok(_) => return Err(io::Error::other("invalid notification event signal")),
        }
    }
}
pub fn notice_channel() -> io::Result<(NoticeSender, NoticeReceiver)> {
    let changed = Arc::new(eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)?);
    let (sender, receiver) = mpsc::sync_channel(CAPACITY);
    Ok((
        NoticeSender {
            sender: Some(sender),
            changed: changed.clone(),
        },
        NoticeReceiver { receiver, changed },
    ))
}
impl NoticeSender {
    /// Never wait for service capacity. A full/closed queue fails the generation
    /// so it can clean up rather than blocking expiry or silently losing intent.
    pub fn send(&self, notice: Notice) -> io::Result<()> {
        self.sender
            .as_ref()
            .expect("live notice sender")
            .try_send(notice)
            .map_err(|error| match error {
                TrySendError::Full(_) => {
                    io::Error::new(io::ErrorKind::WouldBlock, "notification event queue full")
                }
                TrySendError::Disconnected(_) => io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "notification event receiver closed",
                ),
            })?;
        signal(&self.changed)
    }
}
impl Drop for NoticeSender {
    fn drop(&mut self) {
        // Disconnect before waking: otherwise the consumer can see Empty and
        // go back to sleep before the sender field's implicit destructor runs.
        drop(self.sender.take());
        let _ = signal(&self.changed);
    }
}
impl AsFd for NoticeReceiver {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.changed.as_fd()
    }
}
impl NoticeReceiver {
    /// Drain at most one bounded batch. Clear readiness before reading events;
    /// concurrent enqueue/close writes a new wake. If the batch is full, rearm
    /// explicitly so remaining events or terminal state cannot be stranded.
    pub fn drain(&self) -> io::Result<NoticeBatch> {
        let mut counter = [0; 8];
        loop {
            match rustix::io::read(&*self.changed, &mut counter) {
                Ok(8) => break,
                Err(rustix::io::Errno::INTR) => continue,
                Err(rustix::io::Errno::AGAIN) => break,
                Err(error) => return Err(error.into()),
                Ok(_) => return Err(io::Error::other("invalid notification event read")),
            }
        }
        let mut notices = Vec::with_capacity(CAPACITY);
        while notices.len() < CAPACITY {
            match self.receiver.try_recv() {
                Ok(notice) => notices.push(notice),
                Err(TryRecvError::Empty) => {
                    return Ok(NoticeBatch {
                        notices,
                        closed: false,
                    });
                }
                Err(TryRecvError::Disconnected) => {
                    return Ok(NoticeBatch {
                        notices,
                        closed: true,
                    });
                }
            }
        }
        signal(&self.changed)?;
        Ok(NoticeBatch {
            notices,
            closed: false,
        })
    }
}
