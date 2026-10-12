use pokelinux_suspend::*;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "pokelinux-suspend-time-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, name: &str) -> File {
        OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(self.0.join(name))
            .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Jump {
    samples: Mutex<VecDeque<Duration>>,
}
impl Clock for Jump {
    fn now(&self) -> Duration {
        self.samples
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Duration::from_secs(30))
    }
}
#[test]
fn late_successful_count_read_cannot_become_a_current_commit() {
    let temp = Temp::new();
    let mut count = temp.file("count");
    count.write_all(b"7\n").unwrap();
    let mut observed = count.try_clone().unwrap();
    let clock = Arc::new(Jump {
        samples: Mutex::new([0, 0, 0, 30].map(Duration::from_secs).into()),
    });
    let backend = DescriptorBackend::with_clock(
        count.into(),
        temp.file("state").into(),
        temp.file("abort").into(),
        temp.file("unlock").into(),
        clock,
    )
    .unwrap();
    let c = Coordinator::new(backend, temp.file("writer").into()).unwrap();
    let _authority = c.arm().unwrap();
    assert_eq!(c.attempt(), Attempt::Uncertain);
    assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
    assert_eq!(
        observed.stream_position().unwrap(),
        2,
        "successful read must have happened before injected clock jump"
    );
    assert!(fs::read(temp.0.join("state")).unwrap().is_empty());
    assert_eq!(c.attempt(), Attempt::Disabled);
}
#[test]
fn late_successful_writes_are_uncertain_even_when_bytes_reached_the_fd() {
    for phase in ["commit", "mem", "abort", "unlock"] {
        let temp = Temp::new();
        let (count, mut peer) = UnixStream::pair().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let preceding_samples = match phase {
            "commit" => 4,
            "mem" => 8,
            "abort" => 0,
            _ => 4,
        };
        let mut samples = VecDeque::from(vec![Duration::ZERO; preceding_samples + 3]);
        samples.push_back(Duration::from_secs(30));
        let backend = DescriptorBackend::with_clock(
            count.into(),
            temp.file("state").into(),
            temp.file("abort").into(),
            temp.file("unlock").into(),
            Arc::new(Jump {
                samples: Mutex::new(samples),
            }),
        )
        .unwrap();
        let c = Coordinator::new(backend, temp.file("writer").into()).unwrap();
        let _authority = c.arm().unwrap();
        if phase == "commit" || phase == "mem" {
            peer.write_all(b"7\n").unwrap();
            assert_eq!(c.attempt(), Attempt::Uncertain);
            let mut byte = [0];
            peer.read_exact(&mut byte).unwrap();
            assert_eq!(&byte, b"7", "count write really returned success");
        } else {
            assert!(matches!(c.acquire(), Err(Error::AbortFailed)));
            assert_eq!(c.snapshot().wake_locks, 0);
        }
        assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
        assert_eq!(c.attempt(), Attempt::Disabled);
        let expected_mem: &[u8] = if phase == "mem" { b"mem" } else { b"" };
        assert_eq!(fs::read(temp.0.join("state")).unwrap(), expected_mem);
        if phase == "abort" || phase == "unlock" {
            assert_eq!(fs::read(temp.0.join(phase)).unwrap(), b"userspace-abort");
        }
    }
}
struct MovingClock {
    seconds: AtomicU64,
    samples: AtomicUsize,
}
impl Clock for MovingClock {
    fn now(&self) -> Duration {
        self.samples.fetch_add(1, Ordering::AcqRel);
        Duration::from_secs(self.seconds.load(Ordering::Acquire))
    }
}
struct Worker {
    handle: Option<thread::JoinHandle<Attempt>>,
    done: mpsc::Receiver<()>,
    c: Arc<Coordinator>,
}
impl Worker {
    fn join(mut self) -> Attempt {
        self.wait();
        self.handle.take().unwrap().join().unwrap()
    }
    fn wait(&self) {
        if self.done.recv_timeout(Duration::from_secs(3)).is_err() {
            eprintln!("owned deadline worker did not stop");
            std::process::abort();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            self.c.stop();
            self.wait();
            let _ = h.join();
        }
    }
}
#[test]
fn suspend_time_jump_expires_a_pending_real_fd_poll_without_renewal() {
    let temp = Temp::new();
    let (count, _peer) = UnixStream::pair().unwrap();
    let clock = Arc::new(MovingClock {
        seconds: AtomicU64::new(0),
        samples: AtomicUsize::new(0),
    });
    let backend = DescriptorBackend::with_clock(
        count.into(),
        temp.file("state").into(),
        temp.file("abort").into(),
        temp.file("unlock").into(),
        clock.clone(),
    )
    .unwrap();
    let c = Arc::new(Coordinator::new(backend, temp.file("writer").into()).unwrap());
    let _authority = c.arm().unwrap();
    let (tx, done) = mpsc::channel();
    let worker_c = c.clone();
    let handle = Some(thread::spawn(move || {
        let result = worker_c.attempt();
        let _ = tx.send(());
        result
    }));
    let worker = Worker {
        handle,
        done,
        c: c.clone(),
    };
    let end = Instant::now() + Duration::from_secs(1);
    while clock.samples.load(Ordering::Acquire) < 5 {
        assert!(
            Instant::now() < end,
            "injected deadline clock was not consulted"
        );
        thread::sleep(Duration::from_millis(1));
    }
    let start = Instant::now();
    clock.seconds.store(30, Ordering::Release);
    assert_eq!(worker.join(), Attempt::Uncertain);
    assert!(start.elapsed() < Duration::from_millis(300));
    assert_eq!(c.snapshot().lifecycle, Lifecycle::Degraded);
    assert!(fs::read(temp.0.join("state")).unwrap().is_empty());
}
