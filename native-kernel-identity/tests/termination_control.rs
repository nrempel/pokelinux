//! Real Linux signal/lifetime controls. Authored component only; no owner/GUI claim.
use pokelinux_kernel_identity::{TerminationGuard, termination_requested};
use rustix::process::{Pid, PidfdFlags, Signal};
use std::{
    io::{Read, Write},
    os::unix::process::ExitStatusExt,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
static PRIOR_HIT: AtomicBool = AtomicBool::new(false);
extern "C" fn prior(_: libc::c_int) {
    PRIOR_HIT.store(true, Ordering::Relaxed);
}
fn now() -> u64 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) },
        0
    );
    value.tv_sec as u64 * 1_000_000_000 + value.tv_nsec as u64
}
fn action() -> libc::sigaction {
    let mut value = std::mem::MaybeUninit::uninit();
    assert_eq!(
        unsafe { libc::sigaction(libc::SIGTERM, std::ptr::null(), value.as_mut_ptr()) },
        0
    );
    unsafe { value.assume_init() }
}
fn mask() -> libc::sigset_t {
    let mut value = std::mem::MaybeUninit::uninit();
    assert_eq!(
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), value.as_mut_ptr()) },
        0
    );
    unsafe { value.assume_init() }
}
fn same_action(a: &libc::sigaction, b: &libc::sigaction) {
    assert_eq!(a.sa_sigaction, b.sa_sigaction);
    assert_eq!(a.sa_flags, b.sa_flags);
    for signal in 1..=libc::SIGRTMAX() {
        assert_eq!(unsafe { libc::sigismember(&a.sa_mask, signal) }, unsafe {
            libc::sigismember(&b.sa_mask, signal)
        });
    }
}
fn refused(kind: std::io::ErrorKind) {
    let result = TerminationGuard::install();
    assert!(matches!(result, Err(ref error) if error.kind() == kind));
}
struct Prior(libc::sigaction);
impl Drop for Prior {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { libc::sigaction(libc::SIGTERM, &self.0, std::ptr::null_mut()) },
            0
        );
    }
}
struct Mask(libc::sigset_t);
impl Drop for Mask {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &self.0, std::ptr::null_mut()) },
            0
        );
    }
}
struct ExactChild(Child);
impl Drop for ExactChild {
    fn drop(&mut self) {
        if self.0.try_wait().expect("owned child poll").is_none() {
            self.0.kill().expect("exact owned child failure kill");
            let exit = self.0.wait().expect("actual failure parent wait");
            eprintln!(
                "TERMINATION_CHILD_FAILURE_WAIT pid={} signal={:?}",
                self.0.id(),
                exit.signal()
            );
        }
    }
}
fn wait_child(child: &mut ExactChild, until: u64) -> std::process::ExitStatus {
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status;
        }
        assert!(now() < until, "original actual child wait cutoff");
        thread::sleep(Duration::from_millis(1));
    }
}
fn child_probe() {
    assert_eq!(action().sa_sigaction, libc::SIG_DFL);
    assert_eq!(unsafe { libc::sigismember(&mask(), libc::SIGTERM) }, 0);
    assert!(!termination_requested());
    println!("TERMINATION_CHILD_DEFAULT_READY pid={}", std::process::id());
    std::io::stdout().flush().unwrap();
    loop {
        unsafe {
            libc::pause();
        }
    }
}
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--exec-child") {
        child_probe();
        return;
    }
    let start = now();
    let work = start + 5_000_000_000;
    let close = start + 8_000_000_000;
    let original = Prior(action());
    let mut custom: libc::sigaction = unsafe { std::mem::zeroed() };
    custom.sa_sigaction = prior as *const () as libc::sighandler_t;
    custom.sa_flags = libc::SA_RESTART;
    assert_eq!(unsafe { libc::sigemptyset(&mut custom.sa_mask) }, 0);
    assert_eq!(
        unsafe { libc::sigaddset(&mut custom.sa_mask, libc::SIGUSR1) },
        0
    );
    assert_eq!(
        unsafe { libc::sigaction(libc::SIGTERM, &custom, std::ptr::null_mut()) },
        0
    );
    let installed_prior = action();
    let original_mask = mask();
    assert_eq!(
        unsafe { libc::sigismember(&original_mask, libc::SIGTERM) },
        0
    );
    assert!(!termination_requested());
    {
        let guard = TerminationGuard::install().unwrap();
        refused(std::io::ErrorKind::AlreadyExists);
        assert!(!termination_requested());
        for _ in 0..3 {
            assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);
        }
        assert!(termination_requested());
        assert!(!PRIOR_HIT.load(Ordering::Relaxed));
        assert_eq!(unsafe { libc::sigismember(&mask(), libc::SIGTERM) }, 0);
        drop(guard);
    }
    assert!(!termination_requested());
    same_action(&action(), &installed_prior);
    assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);
    assert!(PRIOR_HIT.swap(false, Ordering::Relaxed));
    println!("TERMINATION_PASS latch_duplicate_and_prior_restoration");
    {
        let guard = TerminationGuard::install().unwrap();
        assert!(!termination_requested(), "new lifetime inherited old stop");
        assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);
        assert!(termination_requested());
        drop(guard);
        assert!(!termination_requested());
    }
    println!("TERMINATION_PASS fresh_lifetime");
    {
        let restore = Mask(mask());
        let mut blocked: libc::sigset_t = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::sigemptyset(&mut blocked) }, 0);
        assert_eq!(unsafe { libc::sigaddset(&mut blocked, libc::SIGTERM) }, 0);
        assert_eq!(
            unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, std::ptr::null_mut()) },
            0
        );
        refused(std::io::ErrorKind::InvalidInput);
        same_action(&action(), &installed_prior);
        assert_eq!(unsafe { libc::sigismember(&mask(), libc::SIGTERM) }, 1);
        drop(restore);
    }
    println!("TERMINATION_PASS inherited_blocked_refusal");
    {
        // One joined test-only thread proves actual process/thread refusal.
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let child = thread::spawn(move || {
            refused(std::io::ErrorKind::Unsupported);
            ready_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        });
        ready_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        let refused_main = TerminationGuard::install();
        release_tx.send(()).unwrap();
        child.join().unwrap();
        assert!(
            matches!(refused_main, Err(ref error) if error.kind() == std::io::ErrorKind::Unsupported)
        );
        same_action(&action(), &installed_prior);
    }
    println!("TERMINATION_PASS actual_multithread_and_foreign_thread_refusal");
    {
        let guard = TerminationGuard::install().unwrap();
        let spawned = Command::new(std::env::current_exe().unwrap())
            .arg("--exec-child")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut child = ExactChild(spawned);
        let pidfd = rustix::process::pidfd_open(
            Pid::from_raw(child.0.id() as i32).unwrap(),
            PidfdFlags::NONBLOCK,
        )
        .unwrap();
        let stdout = child.0.stdout.as_mut().unwrap();
        let flags = rustix::fs::fcntl_getfl(&*stdout).unwrap();
        rustix::fs::fcntl_setfl(&*stdout, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        let until = work.min(now() + 2_000_000_000);
        let mut body = Vec::new();
        while !body.ends_with(b"\n") {
            let mut data = [0; 128];
            match stdout.read(&mut data) {
                Ok(0) => panic!("child EOF before exec-state proof"),
                Ok(n) => {
                    body.extend_from_slice(&data[..n]);
                    assert!(body.len() <= 1024);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("{error}"),
            }
            assert!(now() < until);
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            String::from_utf8(body.clone()).unwrap(),
            format!("TERMINATION_CHILD_DEFAULT_READY pid={}\n", child.0.id())
        );
        rustix::process::pidfd_send_signal(&pidfd, Signal::TERM).unwrap();
        let status = wait_child(&mut child, until);
        assert_eq!(status.signal(), Some(15));
        let mut tail = Vec::new();
        child
            .0
            .stdout
            .as_mut()
            .unwrap()
            .read_to_end(&mut tail)
            .unwrap();
        assert!(tail.is_empty());
        let mut stderr = Vec::new();
        child
            .0
            .stderr
            .as_mut()
            .unwrap()
            .read_to_end(&mut stderr)
            .unwrap();
        assert!(stderr.is_empty());
        println!(
            "TERMINATION_ACTUAL_CHILD_WAIT pid={} signal=15 stdout_EOF=true stderr_EOF=true original_until_ns={} observed_boottime_ns={}",
            child.0.id(),
            until,
            now()
        );
        assert!(
            !termination_requested(),
            "child signal reached parent latch"
        );
        drop(guard);
    }
    println!("TERMINATION_PASS actual_exec_defaults_and_owned_signal_wait");
    same_action(&action(), &installed_prior);
    drop(original);
    assert!(now() < work && now() < close);
    println!(
        "TERMINATION_ALL_PASS groups=5 original_work_until_ns={work} original_close_until_ns={close} completed_boottime_ns={}",
        now()
    );
}
