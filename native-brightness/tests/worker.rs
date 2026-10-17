//! Actual FIFO/process/PIPE boundaries, authored temp inputs only. No host sysfs.
#![forbid(unsafe_code)]
use pokelinux_brightness::{
    model::{Action, Observation, Owner, Ticket},
    worker::{Worker, child_check, child_setup, encode_observation},
};
use rustix::{
    fs::{CWD, Mode, OFlags, mkfifoat},
    process::{self, Pid, Signal, WaitOptions},
    time::{ClockId, clock_gettime},
};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

const VALUE: Observation = Observation {
    max: 400,
    requested: 100,
    actual: Some(90),
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn within(until: u64, mut test: impl FnMut() -> bool) {
    while !test() {
        assert!(now() < until, "original BOOTTIME test cutoff");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn ticket(until: u64) -> Ticket {
    Ticket {
        id: 1,
        action: Action::Observe,
        until_ns: until,
        publication_until_ns: until + 1_000_000_000,
        baseline: None,
    }
}
fn children() -> Vec<u32> {
    fs::read_to_string(format!("/proc/self/task/{}/children", std::process::id()))
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect()
}
fn zombie(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|s| {
        s.rsplit_once(") ").is_some_and(|(_, tail)| {
            let f: Vec<_> = tail.split_whitespace().collect();
            f.first() == Some(&"Z") && f.get(49) == Some(&"0")
        })
    })
}
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "brightness-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        mkfifoat(CWD, p.join("held"), Mode::RUSR | Mode::WUSR).unwrap();
        Self(p)
    }
    fn args(&self, mode: &str) -> Vec<String> {
        vec![
            mode.into(),
            self.0.join("held").to_str().unwrap().into(),
            self.0.join("ready").to_str().unwrap().into(),
        ]
    }
    fn ready(&self, pid: u32, until: u64) {
        within(until, || self.0.join("ready").is_file());
        let data = fs::read_to_string(self.0.join("ready")).unwrap();
        assert_eq!(data, format!("{pid} {}\n", std::process::id()));
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Retained {
    worker: Worker,
    close: u64,
    retired: bool,
}
impl Retained {
    fn start(args: &[String], ticket: Ticket) -> Self {
        let close = now() + 3_000_000_000;
        let worker =
            Worker::start(&std::env::current_exe().unwrap(), args, ticket).expect("worker start");
        Self {
            worker,
            close,
            retired: false,
        }
    }
    fn retire(&mut self) {
        within(self.close, || {
            self.retired = self.worker.poll().unwrap();
            self.retired
        });
        assert!(matches!(
            process::waitpid(Pid::from_raw(self.worker.pid() as i32), WaitOptions::NOHANG),
            Err(rustix::io::Errno::CHILD)
        ));
        assert!(!Path::new(&format!("/proc/{}", self.worker.pid())).exists());
        println!(
            "WORKER_ACTUAL_RETIRE pid={} actual_wait=true stdout_EOF=true same_parent_ECHILD=true original_close_ns={} observed_ns={}",
            self.worker.pid(),
            self.close,
            now()
        );
    }
}
impl Drop for Retained {
    fn drop(&mut self) {
        if !self.retired {
            let _ = self.worker.fence();
            while now() < self.close {
                if self.worker.poll().unwrap_or(false) {
                    self.retired = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
fn emit(bytes: &str, exit: i32, until: u64) -> Retained {
    Retained::start(
        &["emit".into(), bytes.into(), exit.to_string()],
        ticket(until),
    )
}

fn real_fifo_fence_retains_one_slot_until_actual_wait_and_eof() {
    let tree = Tree::new();
    let start = now();
    let mut model = Owner::new();
    let selected = model.observe(start).unwrap();
    let mut held = Retained::start(&tree.args("fifo"), selected);
    let pid = held.worker.pid();
    assert!(pid > 0, "worker must spawn exactly one real child");
    assert_eq!(held.worker.ticket(), selected);
    tree.ready(pid, held.close);
    assert_eq!(children(), vec![pid]);
    let pipe = fs::read_link(format!("/proc/{pid}/fd/1")).unwrap();
    let mut matching = Vec::new();
    for entry in fs::read_dir("/proc/self/fd").unwrap() {
        let entry = entry.unwrap();
        if fs::read_link(entry.path()).is_ok_and(|v| v == pipe) {
            matching.push(entry.file_name());
        }
    }
    assert_eq!(matching.len(), 1, "owner retains only its read side");
    let fdinfo = fs::read_to_string(Path::new("/proc/self/fdinfo").join(&matching[0])).unwrap();
    let flags = u64::from_str_radix(
        fdinfo
            .lines()
            .find_map(|l| l.strip_prefix("flags:\t"))
            .unwrap(),
        8,
    )
    .unwrap();
    assert_ne!(flags & (OFlags::NONBLOCK.bits() as u64), 0);
    assert_ne!(flags & (OFlags::CLOEXEC.bits() as u64), 0);
    for _ in 0..3 {
        let wall = Instant::now();
        assert!(!held.worker.poll().unwrap());
        assert!(
            wall.elapsed() < Duration::from_millis(50),
            "poll blocked on the FIFO child"
        );
        assert_eq!(held.worker.result(), None);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(model.cancel(now()), Some(selected));
    assert!(model.observe(now()).is_none());
    let wall = Instant::now();
    held.worker.fence().unwrap();
    assert!(
        wall.elapsed() < Duration::from_millis(50),
        "fence blocked waiting for child"
    );
    assert!(model.view(now()).pending);
    assert_eq!(held.worker.result(), None);
    held.retire();
    assert_eq!(held.worker.result(), None);
    assert!(!model.retired(selected, None, now()));
    assert!(model.observe(now()).is_some());
    assert!(children().is_empty());
}
fn wait_and_eof_are_independently_required() {
    let mut completed = emit("BR1 400 100 90\n", 0, now() + 2_000_000_000);
    let pid = completed.worker.pid();
    assert!(pid > 0);
    within(completed.close, || zombie(pid));
    assert_eq!(completed.worker.result(), None);
    assert!(
        !completed.worker.poll().unwrap(),
        "wait plus data is not PIPE EOF"
    );
    assert!(matches!(
        process::waitpid(Pid::from_raw(pid as i32), WaitOptions::NOHANG),
        Err(rustix::io::Errno::CHILD)
    ));
    assert_eq!(completed.worker.result(), None);
    completed.retire();
    assert_eq!(completed.worker.result(), Some(VALUE));
    let tree = Tree::new();
    let mut held = Retained::start(&tree.args("eof"), ticket(now() + 2_000_000_000));
    let pid = held.worker.pid();
    assert!(pid > 0);
    tree.ready(pid, held.close);
    assert!(
        !held.worker.poll().unwrap(),
        "EOF alone must not retire an actual blocked child"
    );
    assert_eq!(held.worker.result(), None);
    assert_eq!(children(), vec![pid]);
    held.worker.fence().unwrap();
    held.retire();
    assert_eq!(held.worker.result(), None);
}
fn strict_bounded_payloads_and_nonzero_exit_are_not_results() {
    for bytes in [
        "",
        "BR1 400 100 90",
        "BR1 0400 100 90\n",
        "BR1 400 +100 90\n",
        "BR1 400 -1 90\n",
        "BR1 0 0 -\n",
        "BR1 400 401 -\n",
        "BR1 400 100 401\n",
        "BR1 400 100 - extra\n",
        "BR1 400 100 -\nBR1 400 100 -\n",
        "BR1 4294967296 100 -\n",
        "BR1 400  100 -\n",
    ] {
        let mut worker = emit(bytes, 0, now() + 2_000_000_000);
        assert!(worker.worker.pid() > 0);
        worker.retire();
        assert_eq!(
            worker.worker.result(),
            None,
            "accepted malformed payload {bytes:?}"
        );
    }
    let mut large = emit(&"x".repeat(129), 0, now() + 2_000_000_000);
    assert!(large.worker.pid() > 0);
    large.retire();
    assert_eq!(large.worker.result(), None);
    let mut failed = emit("BR1 400 100 90\n", 1, now() + 2_000_000_000);
    assert!(failed.worker.pid() > 0);
    failed.retire();
    assert_eq!(failed.worker.result(), None);
    let mut optional = emit("BR1 400 100 -\n", 0, now() + 2_000_000_000);
    assert!(optional.worker.pid() > 0);
    optional.retire();
    assert_eq!(
        optional.worker.result(),
        Some(Observation {
            actual: None,
            ..VALUE
        })
    );
}
fn fresh_result_getter_expires_without_another_poll() {
    let selected = ticket(now() + 150_000_000);
    let mut worker = emit("BR1 400 100 90\n", 0, selected.until_ns);
    assert!(worker.worker.pid() > 0);
    worker.retire();
    assert_eq!(worker.worker.result(), Some(VALUE));
    within(worker.close, || now() >= selected.until_ns);
    assert_eq!(
        worker.worker.result(),
        None,
        "late getter retained an expired successful reading"
    );
}
fn real_successful_zombie_is_withheld_and_model_stays_busy_until_retirement() {
    let mut model = Owner::new();
    let initial = model.observe(now()).unwrap();
    let mut seed = Retained::start(
        &["emit".into(), "BR1 400 100 90\n".into(), "0".into()],
        initial,
    );
    assert!(seed.worker.pid() > 0);
    seed.retire();
    assert!(model.retired(initial, seed.worker.result(), now()));
    let revision = model.view(now()).revision;
    let until = now() + 150_000_000;
    let selected = model.set(75, revision, until, now()).unwrap();
    let mut worker = Retained::start(
        &["emit".into(), "BR1 400 300 290\n".into(), "0".into()],
        selected,
    );
    let pid = worker.worker.pid();
    assert!(pid > 0);
    within(worker.close, || zombie(pid));
    within(worker.close, || now() >= until);
    assert!(model.view(now()).pending && model.view(now()).last_set_uncertain);
    assert!(model.observe(now()).is_none());
    assert_eq!(worker.worker.result(), None);
    assert!(!worker.worker.poll().unwrap());
    assert!(
        model.observe(now()).is_none(),
        "actual wait alone released the model slot"
    );
    worker.retire();
    assert_eq!(worker.worker.result(), None);
    assert!(!model.retired(selected, worker.worker.result(), now()));
    let view = model.view(now());
    assert!(!view.pending && view.last_set_uncertain && view.observation.is_none());
    assert!(model.observe(now()).is_some());
}
fn expired_start_and_failed_spawn_create_no_child() {
    let args = vec!["emit".into(), "BR1 400 100 90\n".into(), "0".into()];
    assert!(
        Worker::start(&std::env::current_exe().unwrap(), &args, ticket(now() - 1)).is_err(),
        "expired ticket spawned a new child"
    );
    assert!(
        Worker::start(
            Path::new("/nonexistent-brightness-worker"),
            &args,
            ticket(now() + 1_000_000_000)
        )
        .is_err()
    );
    assert!(children().is_empty());
}
fn child(args: &[String]) -> io::Result<()> {
    let parent = args
        .first()
        .and_then(|v| v.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let until = args
        .get(1)
        .and_then(|v| v.parse().ok())
        .ok_or(io::ErrorKind::InvalidInput)?;
    child_setup(parent, until)?;
    assert_eq!(process::parent_process_death_signal()?, Some(Signal::KILL));
    assert_eq!(process::getppid(), Pid::from_raw(parent as i32));
    assert!(rustix::fs::fcntl_getfl(std::io::stdout())?.contains(OFlags::NONBLOCK));
    match args.get(2).map(String::as_str) {
        Some("fifo" | "eof") => {
            if args[2] == "eof" {
                rustix::stdio::dup2_stdout(fs::File::open("/dev/null")?)?;
            }
            fs::write(&args[4], format!("{} {parent}\n", std::process::id()))?;
            let mut byte = [0];
            fs::File::open(&args[3])?.read_exact(&mut byte)?;
            child_check(parent, until)?;
            std::io::stdout().write_all(encode_observation(VALUE)?.as_bytes())?;
        }
        Some("emit") => {
            child_check(parent, until)?;
            std::io::stdout().write_all(args[3].as_bytes())?;
            std::process::exit(args[4].parse().map_err(|_| io::ErrorKind::InvalidInput)?);
        }
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    }
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--brightness-worker") {
        if child(&args[2..]).is_err() {
            std::process::exit(79);
        }
        return;
    }
    let until = now() + 15_000_000_000;
    let tests: [(&str, fn()); 6] = [
        (
            "real_fifo_fence_retains_one_slot_until_actual_wait_and_eof",
            real_fifo_fence_retains_one_slot_until_actual_wait_and_eof,
        ),
        (
            "wait_and_eof_are_independently_required",
            wait_and_eof_are_independently_required,
        ),
        (
            "strict_bounded_payloads_and_nonzero_exit_are_not_results",
            strict_bounded_payloads_and_nonzero_exit_are_not_results,
        ),
        (
            "fresh_result_getter_expires_without_another_poll",
            fresh_result_getter_expires_without_another_poll,
        ),
        (
            "real_successful_zombie_is_withheld_and_model_stays_busy_until_retirement",
            real_successful_zombie_is_withheld_and_model_stays_busy_until_retirement,
        ),
        (
            "expired_start_and_failed_spawn_create_no_child",
            expired_start_and_failed_spawn_create_no_child,
        ),
    ];
    let mut failed = 0;
    for (name, test) in tests {
        if std::panic::catch_unwind(test).is_ok() {
            println!("WORKER_PASS {name}");
        } else {
            failed += 1;
            println!("WORKER_FAIL {name}");
        }
        assert!(children().is_empty(), "test left an actual child");
    }
    println!(
        "WORKER_SUMMARY groups=6 failed={failed} original_until_ns={until} completed_ns={}",
        now()
    );
    assert!(now() < until);
    if failed != 0 {
        std::process::exit(1);
    }
}
