//! Actual kernel process/Unix-FD tests. On ARM64, run --ignored in a guest kernel.
use pokelinux_suspend::*;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};
fn lifetime_peer() -> PathBuf {
    if let Some(path) = std::env::var_os("POKELINUX_SUSPEND_LIFETIME_PEER") {
        return PathBuf::from(path);
    }
    let executable = std::env::current_exe().expect("current test executable");
    executable
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .join("lifetime-peer")
}
struct OwnedChild(Child);
impl OwnedChild {
    fn kill_reap(&mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }
    fn reap(&mut self) -> std::process::ExitStatus {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(s) = self.0.try_wait().unwrap() {
                return s;
            }
            assert!(Instant::now() < end, "owned child did not observe EOF");
            thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Worker {
    handle: Option<thread::JoinHandle<()>>,
    done: mpsc::Receiver<()>,
    cancel: mpsc::Sender<()>,
}
impl Worker {
    fn join(mut self) {
        self.wait();
        self.handle.take().unwrap().join().unwrap();
    }
    fn wait(&self) {
        if self.done.recv_timeout(Duration::from_secs(3)).is_err() {
            eprintln!("owned IPC worker did not stop");
            std::process::abort();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            let _ = self.cancel.send(());
            self.wait();
            let _ = h.join();
        }
    }
}
fn watch(mut stream: UnixStream, lifetime: impl Send + 'static) -> Worker {
    stream
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    let (tx, done) = mpsc::channel();
    let (cancel, rx) = mpsc::channel();
    let handle = Some(thread::spawn(move || {
        let _lifetime = lifetime;
        let mut byte = [0; 1];
        loop {
            if rx.try_recv().is_ok() {
                break;
            }
            match stream.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => panic!("IPC fixture failure: {e}"),
            }
        }
        drop(_lifetime);
        let _ = tx.send(());
    }));
    Worker {
        handle,
        done,
        cancel,
    }
}
struct Fixture {
    root: PathBuf,
    coordinator: Arc<Coordinator>,
    listener: UnixListener,
}
impl Fixture {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "pokelinux-suspend-ipc-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
        fs::write(root.join("count"), b"7\n").unwrap();
        let backend = DescriptorBackend::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(root.join("count"))
                .unwrap()
                .into(),
            file("state").into(),
            file("abort").into(),
            file("unlock").into(),
        )
        .unwrap();
        let coordinator = Arc::new(Coordinator::new(backend, file("writer").into()).unwrap());
        let listener = UnixListener::bind(root.join("peer")).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            root,
            coordinator,
            listener,
        }
    }
    fn peer(&self) -> (OwnedChild, UnixStream) {
        let child = OwnedChild(
            Command::new(lifetime_peer())
                .arg(self.root.join("peer"))
                .spawn()
                .unwrap(),
        );
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            match self.listener.accept() {
                Ok((s, _)) => return (child, s),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < end);
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => panic!("accept: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.coordinator.stop();
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires ARM64 guest kernel; run --ignored there"
)]
fn killed_client_eof_destroys_one_native_object_after_actual_reap() {
    let f = Fixture::new();
    let _authority = f.coordinator.arm().unwrap();
    let (mut child, stream) = f.peer();
    let lock = f.coordinator.acquire().unwrap();
    let worker = watch(stream, lock);
    assert_eq!(f.coordinator.snapshot().wake_locks, 1);
    assert_eq!(f.coordinator.attempt(), Attempt::Blocked);
    child.kill_reap();
    worker.join();
    assert_eq!(f.coordinator.snapshot().wake_locks, 0);
    assert_eq!(f.coordinator.attempt(), Attempt::Returned);
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires ARM64 guest kernel; run --ignored there"
)]
fn killed_controller_eof_drops_authority_without_reentry() {
    let f = Fixture::new();
    let authority = f.coordinator.arm().unwrap();
    let (mut child, stream) = f.peer();
    let worker = watch(stream, authority);
    child.kill_reap();
    worker.join();
    assert_eq!(f.coordinator.snapshot().lifecycle, Lifecycle::AuthorityLost);
    assert_eq!(f.coordinator.attempt(), Attempt::Disabled);
    assert!(f.coordinator.arm().is_err());
    assert!(fs::read(f.root.join("state")).unwrap().is_empty());
}
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "requires ARM64 guest kernel; run --ignored there"
)]
fn actual_service_side_disconnect_reaches_client_as_eof() {
    let f = Fixture::new();
    let (mut child, stream) = f.peer();
    drop(stream);
    assert!(child.reap().success());
}
