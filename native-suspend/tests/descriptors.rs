use pokelinux_suspend::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

struct Fixture {
    root: PathBuf,
    coordinator: Arc<Coordinator>,
    count: UnixStream,
    state: UnixStream,
}
impl Fixture {
    fn new(readonly: &str) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "pokelinux-suspend-fd-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let file = |name: &str| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(root.join(name))
                .unwrap()
        };
        let (count_fd, count) = UnixStream::pair().unwrap();
        let (state_fd, state) = UnixStream::pair().unwrap();
        count
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        state
            .set_read_timeout(Some(Duration::from_millis(80)))
            .unwrap();
        let abort = file("abort");
        let unlock = file("unlock");
        let state_fd = if readonly == "state" {
            drop(file("state"));
            File::open(root.join("state")).unwrap().into()
        } else {
            state_fd.into()
        };
        let abort = if readonly == "abort" {
            drop(abort);
            File::open(root.join("abort")).unwrap()
        } else {
            abort
        };
        let unlock = if readonly == "unlock" {
            drop(unlock);
            File::open(root.join("unlock")).unwrap()
        } else {
            unlock
        };
        let backend =
            DescriptorBackend::new(count_fd.into(), state_fd, abort.into(), unlock.into()).unwrap();
        let coordinator = Arc::new(Coordinator::new(backend, file("writer").into()).unwrap());
        Self {
            root,
            coordinator,
            count,
            state,
        }
    }
    fn no_mem(&mut self) {
        let mut b = [0; 3];
        assert!(self.state.read(&mut b).is_err());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.coordinator.stop();
        fs::remove_dir_all(&self.root).unwrap();
    }
}
struct Worker<T> {
    handle: Option<thread::JoinHandle<T>>,
    done: std::sync::mpsc::Receiver<()>,
    coordinator: Arc<Coordinator>,
}
impl<T> Worker<T> {
    fn join(mut self) -> T {
        self.wait();
        self.handle.take().unwrap().join().unwrap()
    }
    fn wait(&self) {
        if self.done.recv_timeout(Duration::from_secs(3)).is_err() {
            eprintln!("owned FD worker failed to reap");
            std::process::abort();
        }
    }
}
impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            self.coordinator.stop();
            self.wait();
            let _ = h.join();
        }
    }
}
fn start(c: Arc<Coordinator>) -> Worker<Attempt> {
    let (tx, done) = std::sync::mpsc::channel();
    let coordinator = c.clone();
    let handle = Some(thread::spawn(move || {
        let result = c.attempt();
        let _ = tx.send(());
        result
    }));
    Worker {
        handle,
        done,
        coordinator,
    }
}
fn phase(c: &Coordinator, desired: Phase) {
    let end = Instant::now() + Duration::from_secs(2);
    while c.snapshot().phase != desired {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn actual_duplex_descriptors_commit_count_before_mem() {
    let mut f = Fixture::new("");
    let _authority = f.coordinator.arm().unwrap();
    f.count.write_all(b"17\n").unwrap();
    assert_eq!(f.coordinator.attempt(), Attempt::Returned);
    let mut count = [0; 2];
    f.count.read_exact(&mut count).unwrap();
    assert_eq!(&count, b"17");
    let mut mem = [0; 3];
    f.state.read_exact(&mut mem).unwrap();
    assert_eq!(&mem, b"mem");
    assert_eq!(f.coordinator.snapshot().phase, Phase::Idle);
}
#[test]
fn fragmented_count_is_not_committed_as_a_partial_number() {
    let mut f = Fixture::new("");
    let _authority = f.coordinator.arm().unwrap();
    let worker = start(f.coordinator.clone());
    f.count.write_all(b"1").unwrap();
    thread::sleep(Duration::from_millis(25));
    f.count.write_all(b"7\n").unwrap();
    assert_eq!(worker.join(), Attempt::Returned);
    let mut count = [0; 2];
    f.count.read_exact(&mut count).unwrap();
    assert_eq!(&count, b"17");
}
#[test]
fn malformed_and_oversized_counts_latch_without_mem() {
    for input in [b"x\n".as_slice(), b"\n", b"123456789012345678901\n"] {
        let mut f = Fixture::new("");
        let _authority = f.coordinator.arm().unwrap();
        f.count.write_all(input).unwrap();
        assert_eq!(f.coordinator.attempt(), Attempt::Uncertain);
        assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Degraded);
        assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
        f.no_mem();
    }
}
#[test]
fn actual_readonly_mem_failure_does_not_retry() {
    let mut f = Fixture::new("state");
    let _authority = f.coordinator.arm().unwrap();
    f.count.write_all(b"7\n").unwrap();
    assert_eq!(f.coordinator.attempt(), Attempt::Uncertain);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Degraded);
    assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
    assert!(fs::read(f.root.join("state")).unwrap().is_empty());
}
#[test]
fn actual_count_commit_failure_is_uncertain_and_never_mem() {
    let f = Fixture::new("");
    fs::write(f.root.join("count-readonly"), b"7\n").unwrap();
    let count = File::open(f.root.join("count-readonly")).unwrap();
    let backend = DescriptorBackend::new(
        count.into(),
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(f.root.join("other-state"))
            .unwrap()
            .into(),
        OpenOptions::new()
            .write(true)
            .open(f.root.join("abort"))
            .unwrap()
            .into(),
        OpenOptions::new()
            .write(true)
            .open(f.root.join("unlock"))
            .unwrap()
            .into(),
    )
    .unwrap();
    let c = Coordinator::new(
        backend,
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(f.root.join("other-writer"))
            .unwrap()
            .into(),
    )
    .unwrap();
    let _authority = c.arm().unwrap();
    assert_eq!(c.attempt(), Attempt::Uncertain);
    assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
    assert!(fs::read(f.root.join("other-state")).unwrap().is_empty());
}
#[test]
fn actual_abort_write_failure_never_grants_an_object() {
    let f = Fixture::new("abort");
    assert!(matches!(f.coordinator.acquire(), Err(Error::AbortFailed)));
    assert_eq!(f.coordinator.snapshot().wake_locks, 0);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Degraded);
}
#[test]
fn stop_interrupts_actual_idle_socket_poll_and_reaps_worker() {
    let f = Fixture::new("");
    let _authority = f.coordinator.arm().unwrap();
    let worker = start(f.coordinator.clone());
    phase(&f.coordinator, Phase::ReadingCount);
    let before = Instant::now();
    f.coordinator.stop();
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::Stopped);
    assert_eq!(worker.join(), Attempt::Cancelled);
    assert!(before.elapsed() < Duration::from_millis(300));
}
#[test]
fn controller_loss_cancels_actual_count_read() {
    let f = Fixture::new("");
    let authority = f.coordinator.arm().unwrap();
    let worker = start(f.coordinator.clone());
    phase(&f.coordinator, Phase::ReadingCount);
    drop(authority);
    assert_eq!(worker.join(), Attempt::Cancelled);
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::AuthorityLost);
}
struct CountUnlock {
    backend: DescriptorBackend,
    calls: Arc<AtomicUsize>,
}
impl Backend for CountUnlock {
    fn read_count(&self, c: &Cancellation) -> Result<Vec<u8>, BackendError> {
        self.backend.read_count(c)
    }
    fn commit_count(&self, b: &[u8], c: &Cancellation) -> Result<(), BackendError> {
        self.backend.commit_count(b, c)
    }
    fn enter_mem(&self, c: &Cancellation) -> Result<(), BackendError> {
        self.backend.enter_mem(c)
    }
    fn begin_wake(&self) -> Result<(), BackendError> {
        self.backend.begin_wake()
    }
    fn finish_wake(&self) -> Result<(), BackendError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.backend.finish_wake()
    }
}
#[test]
fn uncertain_abort_unlock_is_not_retried_implicitly() {
    let f = Fixture::new("");
    let calls = Arc::new(AtomicUsize::new(0));
    let fd = |name: &str| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(f.root.join(name))
            .unwrap()
    };
    let unlock = fd("broken-unlock");
    drop(unlock);
    let backend = DescriptorBackend::new(
        UnixStream::pair().unwrap().0.into(),
        fd("new-mem").into(),
        fd("new-abort").into(),
        File::open(f.root.join("broken-unlock")).unwrap().into(),
    )
    .unwrap();
    let c = Coordinator::new(
        CountUnlock {
            backend,
            calls: calls.clone(),
        },
        fd("new-guard").into(),
    )
    .unwrap();
    assert!(matches!(c.acquire(), Err(Error::AbortFailed)));
    assert_eq!(c.snapshot().wake_locks, 0);
    assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

struct PanicBoundary {
    backend: DescriptorBackend,
    site: &'static str,
}
impl Backend for PanicBoundary {
    fn read_count(&self, c: &Cancellation) -> Result<Vec<u8>, BackendError> {
        self.backend.read_count(c)
    }
    fn commit_count(&self, b: &[u8], c: &Cancellation) -> Result<(), BackendError> {
        if self.site == "commit" {
            panic!("injected backend unwind");
        }
        self.backend.commit_count(b, c)
    }
    fn enter_mem(&self, c: &Cancellation) -> Result<(), BackendError> {
        self.backend.enter_mem(c)
    }
    fn begin_wake(&self) -> Result<(), BackendError> {
        if self.site == "begin" {
            panic!("injected backend unwind");
        }
        self.backend.begin_wake()
    }
    fn finish_wake(&self) -> Result<(), BackendError> {
        if self.site == "unlock" {
            panic!("injected backend unwind");
        }
        self.backend.finish_wake()
    }
}
#[test]
fn backend_unwind_latches_uncertainty_and_rolls_back_unpublished_object() {
    for site in ["commit", "begin", "unlock"] {
        let f = Fixture::new("");
        let fd = |name: &str| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(f.root.join(name))
                .unwrap()
        };
        let mut count = fd("panic-count");
        count.write_all(b"7\n").unwrap();
        let backend = DescriptorBackend::new(
            count.into(),
            fd("panic-state").into(),
            fd("panic-abort").into(),
            fd("panic-unlock").into(),
        )
        .unwrap();
        let c =
            Coordinator::new(PanicBoundary { backend, site }, fd("panic-writer").into()).unwrap();
        let _authority = c.arm().unwrap();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if site == "commit" {
                    c.attempt();
                } else {
                    let _ = c.acquire();
                }
            }))
            .is_err()
        );
        assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
        assert_eq!(c.snapshot().wake_locks, 0);
        assert!(c.arm().is_err());
        assert_eq!(c.attempt(), Attempt::Disabled);
        assert!(fs::read(f.root.join("panic-state")).unwrap().is_empty());
    }
}
