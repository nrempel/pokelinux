use pokelinux_suspend::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "pokelinux-suspend-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, name: &str) -> File {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.0.join(name))
            .unwrap()
    }
    fn guard(&self) -> OwnedFd {
        self.file("writer").into()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Default)]
struct Events {
    values: Mutex<Vec<String>>,
    changed: Condvar,
}
impl Events {
    fn push(&self, value: impl Into<String>) {
        self.values.lock().unwrap().push(value.into());
        self.changed.notify_all();
    }
    fn wait(&self, value: &str) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut v = self.values.lock().unwrap();
        while !v.iter().any(|x| x == value) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "event {value} missing: {v:?}");
            v = self.changed.wait_timeout(v, remaining).unwrap().0;
        }
    }
}
struct Boundary {
    events: Arc<Events>,
    read: Mutex<UnixStream>,
    committed: Mutex<UnixStream>,
    mem: Mutex<File>,
    wake: Mutex<File>,
    unlock: Mutex<File>,
    pause: Option<Arc<(Mutex<bool>, Condvar)>>,
    changed: bool,
}
impl Backend for Boundary {
    fn read_count(&self, cancel: &Cancellation) -> Result<Vec<u8>, BackendError> {
        self.events.push("reading");
        let mut stream = self.read.lock().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        loop {
            if cancel.cancelled() {
                return Err(BackendError::Cancelled);
            }
            let mut byte = [0; 1];
            match stream.read(&mut byte) {
                Ok(1) => return Ok(byte.to_vec()),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                _ => return Err(BackendError::Uncertain),
            }
        }
    }
    fn commit_count(&self, count: &[u8], _: &Cancellation) -> Result<(), BackendError> {
        self.events.push("commit");
        if let Some(p) = &self.pause {
            let mut allow = p.0.lock().unwrap();
            while !*allow {
                allow = p.1.wait_timeout(allow, Duration::from_secs(2)).unwrap().0;
            }
        }
        if self.changed {
            return Err(BackendError::CountChanged);
        }
        self.committed
            .lock()
            .unwrap()
            .write_all(count)
            .map_err(|_| BackendError::Uncertain)
    }
    fn enter_mem(&self, _: &Cancellation) -> Result<(), BackendError> {
        self.events.push("mem");
        self.mem
            .lock()
            .unwrap()
            .write_all(b"mem")
            .map_err(|_| BackendError::Uncertain)
    }
    fn begin_wake(&self) -> Result<(), BackendError> {
        self.events.push("abort");
        self.wake
            .lock()
            .unwrap()
            .write_all(b"userspace-abort")
            .map_err(|_| BackendError::Uncertain)
    }
    fn finish_wake(&self) -> Result<(), BackendError> {
        self.events.push("unlock");
        self.unlock
            .lock()
            .unwrap()
            .write_all(b"userspace-abort")
            .map_err(|_| BackendError::Uncertain)
    }
}
struct Fixture {
    temp: Temp,
    coordinator: Arc<Coordinator>,
    send: UnixStream,
    receive: UnixStream,
    events: Arc<Events>,
}
impl Fixture {
    fn new(fail_mem: bool, changed: bool, pause: Option<Arc<(Mutex<bool>, Condvar)>>) -> Self {
        let temp = Temp::new();
        let events = Arc::new(Events::default());
        let (read, send) = UnixStream::pair().unwrap();
        let (committed, receive) = UnixStream::pair().unwrap();
        receive
            .set_read_timeout(Some(Duration::from_millis(80)))
            .unwrap();
        let mem = temp.file("mem");
        let mem = if fail_mem {
            drop(mem);
            File::open(temp.0.join("mem")).unwrap()
        } else {
            mem
        };
        let boundary = Boundary {
            events: events.clone(),
            read: Mutex::new(read),
            committed: Mutex::new(committed),
            mem: Mutex::new(mem),
            wake: Mutex::new(temp.file("wake")),
            unlock: Mutex::new(temp.file("unlock")),
            pause,
            changed,
        };
        let coordinator = Arc::new(Coordinator::new(boundary, temp.guard()).unwrap());
        Self {
            temp,
            coordinator,
            send,
            receive,
            events,
        }
    }
    fn count(&mut self) {
        self.send.write_all(b"7").unwrap();
    }
}
struct Worker<T> {
    handle: Option<thread::JoinHandle<T>>,
    done: mpsc::Receiver<()>,
    coordinator: Arc<Coordinator>,
    unpause: Option<Arc<(Mutex<bool>, Condvar)>>,
}
impl<T> Worker<T> {
    fn wait(&self) {
        if self.done.recv_timeout(Duration::from_secs(3)).is_err() {
            eprintln!("owned worker failed to stop; aborting test process instead of detaching");
            std::process::abort();
        }
    }
    fn join(mut self) -> T {
        self.wait();
        self.handle.take().unwrap().join().unwrap()
    }
}
impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            self.coordinator.stop();
            if let Some(p) = &self.unpause {
                *p.0.lock().unwrap() = true;
                p.1.notify_all();
            }
            self.wait();
            let _ = h.join();
        }
    }
}
fn attempt_worker(c: Arc<Coordinator>) -> Worker<Attempt> {
    let (tx, done) = mpsc::channel();
    let coordinator = c.clone();
    let handle = Some(thread::spawn(move || {
        let r = c.attempt();
        let _ = tx.send(());
        r
    }));
    Worker {
        handle,
        done,
        coordinator,
        unpause: None,
    }
}
#[test]
fn live_object_blocks_commit_and_release_is_idempotent() {
    let mut f = Fixture::new(false, false, None);
    let _authority = f.coordinator.arm().unwrap();
    let w = f.coordinator.acquire().unwrap();
    let clone = w.clone();
    assert_eq!(f.coordinator.snapshot().wake_locks, 1);
    f.count();
    assert_eq!(f.coordinator.attempt(), Attempt::Blocked);
    w.release();
    clone.release();
    drop(w);
    drop(clone);
    assert_eq!(f.coordinator.snapshot().wake_locks, 0);
    f.count();
    assert_eq!(f.coordinator.attempt(), Attempt::Returned);
    let mut byte = [0];
    f.receive.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"7");
    assert_eq!(fs::read(f.temp.0.join("mem")).unwrap(), b"mem");
}
#[test]
fn last_reference_destruction_releases_once() {
    let f = Fixture::new(false, false, None);
    let w = f.coordinator.acquire().unwrap();
    let clone = w.clone();
    drop(w);
    assert_eq!(f.coordinator.snapshot().wake_locks, 1);
    drop(clone);
    assert_eq!(f.coordinator.snapshot().wake_locks, 0);
}
#[test]
fn commit_rejection_never_writes_mem_and_can_be_retried_explicitly() {
    let mut f = Fixture::new(false, true, None);
    let _authority = f.coordinator.arm().unwrap();
    f.count();
    assert_eq!(f.coordinator.attempt(), Attempt::CountChanged);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Ready);
    assert!(fs::read(f.temp.0.join("mem")).unwrap().is_empty());
}
#[test]
fn actual_mem_write_failure_latches_uncertainty() {
    let mut f = Fixture::new(true, false, None);
    let authority = f.coordinator.arm().unwrap();
    f.count();
    assert_eq!(f.coordinator.attempt(), Attempt::Uncertain);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Degraded);
    assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
    drop(authority);
    assert!(f.coordinator.arm().is_err());
    assert_eq!(
        f.events
            .values
            .lock()
            .unwrap()
            .iter()
            .filter(|x| x.as_str() == "mem")
            .count(),
        1
    );
}
#[test]
fn controller_destruction_disables_without_automatic_reentry() {
    let mut f = Fixture::new(false, false, None);
    let authority = f.coordinator.arm().unwrap();
    drop(authority);
    f.count();
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::AuthorityLost);
    assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
    assert!(f.coordinator.arm().is_err());
    assert!(fs::read(f.temp.0.join("mem")).unwrap().is_empty());
}
#[test]
fn only_one_writer_even_after_coordinator_handle_is_dropped() {
    let f = Fixture::new(false, false, None);
    let lock = f.coordinator.acquire().unwrap();
    let another = || Boundary {
        events: Arc::new(Events::default()),
        read: Mutex::new(UnixStream::pair().unwrap().0),
        committed: Mutex::new(UnixStream::pair().unwrap().0),
        mem: Mutex::new(f.temp.file("other")),
        wake: Mutex::new(f.temp.file("wake2")),
        unlock: Mutex::new(f.temp.file("unlock2")),
        pause: None,
        changed: false,
    };
    assert!(matches!(
        Coordinator::new(another(), f.temp.guard()),
        Err(Error::WriterBusy)
    ));
    drop(f.coordinator);
    assert!(matches!(
        Coordinator::new(another(), f.temp.guard()),
        Err(Error::WriterBusy)
    ));
    drop(lock);
    assert!(Coordinator::new(another(), f.temp.guard()).is_ok());
}
#[test]
fn concurrent_acquire_aborts_before_gate_admission() {
    let pause = Arc::new((Mutex::new(false), Condvar::new()));
    let mut f = Fixture::new(false, false, Some(pause.clone()));
    let _authority = f.coordinator.arm().unwrap();
    f.count();
    let mut attempt = attempt_worker(f.coordinator.clone());
    attempt.unpause = Some(pause.clone());
    f.events.wait("commit");
    let c = f.coordinator.clone();
    let (tx, rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let acquire_handle = thread::spawn(move || {
        let result = c.acquire();
        let _ = tx.send(result);
        let _ = done_tx.send(());
    });
    let acquire = Worker {
        handle: Some(acquire_handle),
        done: done_rx,
        coordinator: f.coordinator.clone(),
        unpause: Some(pause.clone()),
    };
    f.events.wait("abort");
    assert!(rx.try_recv().is_err());
    assert_eq!(f.coordinator.snapshot().phase, Phase::CommittingCount);
    *pause.0.lock().unwrap() = true;
    pause.1.notify_all();
    assert_eq!(attempt.join(), Attempt::Returned);
    let lock = rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap();
    acquire.join();
    assert_eq!(f.coordinator.snapshot().wake_locks, 1);
    drop(lock);
    let events = f.events.values.lock().unwrap();
    assert!(
        events.iter().position(|x| x == "abort").unwrap()
            < events.iter().position(|x| x == "mem").unwrap()
    );
    assert!(
        events.iter().position(|x| x == "mem").unwrap()
            < events.iter().position(|x| x == "unlock").unwrap()
    );
}
#[test]
fn stop_and_snapshot_remain_responsive_during_count_read() {
    let f = Fixture::new(false, false, None);
    let _authority = f.coordinator.arm().unwrap();
    let worker = attempt_worker(f.coordinator.clone());
    f.events.wait("reading");
    let start = Instant::now();
    assert_eq!(f.coordinator.snapshot().phase, Phase::ReadingCount);
    f.coordinator.stop();
    assert!(start.elapsed() < Duration::from_millis(100));
    assert_eq!(worker.join(), Attempt::Cancelled);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Stopped);
}
#[test]
fn simultaneous_attempt_is_busy_and_never_a_second_writer() {
    let f = Fixture::new(false, false, None);
    let _authority = f.coordinator.arm().unwrap();
    let worker = attempt_worker(f.coordinator.clone());
    f.events.wait("reading");
    assert_eq!(f.coordinator.attempt(), Attempt::Busy);
    f.coordinator.stop();
    assert_eq!(worker.join(), Attempt::Cancelled);
}

#[test]
fn stop_during_commit_is_responsive_and_prevents_mem_after_return() {
    let pause = Arc::new((Mutex::new(false), Condvar::new()));
    let mut f = Fixture::new(false, false, Some(pause.clone()));
    let _authority = f.coordinator.arm().unwrap();
    f.count();
    let mut worker = attempt_worker(f.coordinator.clone());
    worker.unpause = Some(pause.clone());
    f.events.wait("commit");
    let before = Instant::now();
    f.coordinator.stop();
    assert!(before.elapsed() < Duration::from_millis(100));
    assert_eq!(f.coordinator.snapshot().phase, Phase::CommittingCount);
    *pause.0.lock().unwrap() = true;
    pause.1.notify_all();
    assert_eq!(worker.join(), Attempt::Cancelled);
    assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
    assert!(fs::read(f.temp.0.join("mem")).unwrap().is_empty());
}
