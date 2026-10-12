//! Owned observation worker. The caller's event loop drives independent expiry.
use crate::{
    lease_timer::LeaseTimer,
    observer::{Lease, Observer},
    tls::{TlsClient, WatchCancel},
};
use rustix::{
    event::{EventfdFlags, eventfd},
    fd::{AsFd, BorrowedFd, OwnedFd},
};
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

fn now() -> Duration {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(value.tv_sec as u64, value.tv_nsec as u32)
}
fn poisoned() -> io::Error {
    io::Error::other("observer state failed")
}
struct Shared {
    observer: Mutex<Observer>,
    timer: LeaseTimer,
    changed: OwnedFd,
    stop: Mutex<bool>,
    wake: Condvar,
    cancel: WatchCancel,
}
impl Shared {
    fn signal(&self) -> io::Result<()> {
        loop {
            match rustix::io::write(&self.changed, &1u64.to_ne_bytes()) {
                Ok(8) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                // Saturation still means readable: updates are coalesced.
                Err(rustix::io::Errno::AGAIN) => return Ok(()),
                Err(error) => return Err(error.into()),
                Ok(_) => return Err(io::Error::other("invalid observer signal write")),
            }
        }
    }
    fn stopped(&self) -> io::Result<bool> {
        Ok(*self.stop.lock().map_err(|_| poisoned())?)
    }
    fn wait(&self, delay: Duration) -> io::Result<()> {
        let stopped = self.stop.lock().map_err(|_| poisoned())?;
        let _ = self
            .wake
            .wait_timeout_while(stopped, delay, |stopped| !*stopped)
            .map_err(|_| poisoned())?;
        Ok(())
    }
    fn stop(&self) {
        {
            let mut stopped = self.stop.lock().unwrap_or_else(|error| error.into_inner());
            *stopped = true;
            self.wake.notify_all();
        }
        self.cancel.cancel();
        self.observer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .invalidate();
        let _ = self.timer.arm(None);
        let _ = self.signal();
    }
}
struct StopOnExit(Arc<Shared>);
impl Drop for StopOnExit {
    fn drop(&mut self) {
        self.0.stop();
    }
}

pub struct BackgroundObserver {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<io::Result<()>>>,
    client: TlsClient,
    address: SocketAddr,
}
impl BackgroundObserver {
    pub fn start(client: TlsClient, address: SocketAddr) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            observer: Mutex::new(Observer::default()),
            timer: LeaseTimer::new()?,
            changed: eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)?,
            stop: Mutex::new(false),
            wake: Condvar::new(),
            cancel: WatchCancel::default(),
        });
        let worker_shared = shared.clone();
        let worker_client = client.clone();
        let worker = thread::Builder::new()
            .name("phone-observe".into())
            .spawn(move || {
                let _stop = StopOnExit(worker_shared.clone());
                run(&worker_shared, worker_client, address)
            })?;
        Ok(Self {
            shared,
            worker: Some(worker),
            client,
            address,
        })
    }
    /// Poll this coalesced update signal and timer() independently of TLS work.
    pub fn changes(&self) -> BorrowedFd<'_> {
        self.shared.changed.as_fd()
    }
    pub fn timer(&self) -> &LeaseTimer {
        &self.shared.timer
    }
    pub fn drain_changes(&self) -> io::Result<bool> {
        let mut count = [0; 8];
        loop {
            match rustix::io::read(&self.shared.changed, &mut count) {
                Ok(8) => return Ok(true),
                Err(rustix::io::Errno::INTR) => continue,
                Err(rustix::io::Errno::AGAIN) => return Ok(false),
                Err(error) => return Err(error.into()),
                Ok(_) => return Err(io::Error::other("invalid observer signal read")),
            }
        }
    }
    /// Safe to inspect after either event. Never returns an expired lease.
    pub fn lease(&self) -> io::Result<Option<Lease>> {
        if self.shared.stopped()? {
            return Ok(None);
        }
        Ok(self
            .shared
            .observer
            .lock()
            .map_err(|_| poisoned())?
            .lease(now())
            .cloned())
    }
    pub fn expire(&self) -> io::Result<bool> {
        if !self.shared.timer.consume()? {
            return Ok(false);
        }
        let mut observer = self.shared.observer.lock().map_err(|_| poisoned())?;
        let expired = observer
            .expiry()
            .is_some_and(|(id, _)| observer.expire(&id, now()));
        self.shared
            .timer
            .arm(observer.expiry().map(|(_, deadline)| deadline))?;
        drop(observer);
        if expired {
            self.shared.signal()?;
        }
        Ok(expired)
    }
    /// Start a fresh observation lifetime after stopping and joining the old
    /// worker. The event-loop owner must rebuild its poll set: descriptors are
    /// replaced. A failed restart leaves the previous lifetime stopped.
    /// Stop before suspend; restart only after lifecycle authority is restored.
    pub fn restart(&mut self) -> io::Result<()> {
        self.shutdown()?;
        let replacement = Self::start(self.client.clone(), self.address)?;
        *self = replacement;
        Ok(())
    }
    pub fn shutdown(&mut self) -> io::Result<()> {
        self.shared.stop();
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| io::Error::other("observer worker panicked"))??;
        }
        Ok(())
    }
}
impl Drop for BackgroundObserver {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
fn run(shared: &Shared, client: TlsClient, address: SocketAddr) -> io::Result<()> {
    let mut retry_seconds = 1;
    while !shared.stopped()? {
        let started = now();
        let pending = shared
            .observer
            .lock()
            .map_err(|_| poisoned())?
            .begin(started);
        // No observer/state lock is held across a network operation.
        let result = client.watch(address, pending.query(), &shared.cancel);
        let mut observer = shared.observer.lock().map_err(|_| poisoned())?;
        let valid = if shared.stopped()? {
            observer.invalidate();
            false
        } else {
            match result {
                Ok(snapshot) => observer.complete(pending, snapshot, now()).unwrap_or(false),
                Err(_) => {
                    observer.failed(pending);
                    false
                }
            }
        };
        shared
            .timer
            .arm(observer.expiry().map(|(_, deadline)| deadline))?;
        drop(observer);
        shared.signal()?;
        if valid {
            retry_seconds = 1;
            // Prevent a peer returning unchanged snapshots immediately from
            // turning the observer into an unbounded request loop.
            shared
                .wait(Duration::from_millis(250).saturating_sub(now().saturating_sub(started)))?;
        } else {
            shared.wait(Duration::from_secs(retry_seconds))?;
            retry_seconds = (retry_seconds * 2).min(8);
        }
    }
    Ok(())
}
