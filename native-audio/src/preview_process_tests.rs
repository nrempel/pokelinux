//! Frozen process-custody requirements; include as preview_process::tests.
//! The future module is absent today: a missing-module compile is NOT a RED.
//! Genuine actor->PCM feature RED is independently owned.
use super::Process;
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{Pid, PidfdFlags, pidfd_open};
use std::fs::{self, File, OpenOptions};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(1);

fn now() -> BootTime {
    BootClock.now()
}
fn terminal(fd: &OwnedFd) -> bool {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .unwrap()
        == 1
}
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |h, byte| {
        (h ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
fn payload() -> Vec<u8> {
    (0..16_384).map(|i| (i % 251) as u8).collect()
}

struct Case {
    process: Process,
    pidfd: OwnedFd,
    root: PathBuf,
    work_until: BootTime,
    close_until: BootTime,
    held_stderr_writer: Option<File>,
    finished: Option<bool>,
}
impl Case {
    fn start(mode: &str, pcm: Vec<u8>, work_ms: u64) -> Self {
        let fixture = std::env::var_os("POKELINUX_AUDIO_PREVIEW_PROCESS_FIXTURE")
            .expect("bind the separately compiled, pinned test-only Rust child");
        let root = std::env::temp_dir().join(format!(
            "audio-preview-process-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let work_until = now().checked_after(Duration::from_millis(work_ms)).unwrap();
        let close_until = work_until.checked_after(Duration::from_secs(2)).unwrap();
        let mut command = Command::new(fixture);
        command
            .arg(mode)
            .arg(&root)
            .arg(pcm.len().to_string())
            .arg(hash(&pcm).to_string());
        // Private constructor shared with production's fixed-command start.
        // No caller command/path is added to the production IPC or CLI.
        let process = Process::spawn_owned(command, pcm, work_until).unwrap();
        let pid = Pid::from_raw(process.pid() as i32).unwrap();
        let pidfd = pidfd_open(pid, PidfdFlags::NONBLOCK).unwrap();
        Self {
            process,
            pidfd,
            root,
            work_until,
            close_until,
            held_stderr_writer: None,
            finished: None,
        }
    }
    fn tick(&mut self) {
        if let Some(done) = self.process.poll() {
            assert!(self.finished.is_none(), "one completion event only");
            assert!(
                self.process.settled(),
                "completion requires actual wait, EOF and close"
            );
            assert!(
                terminal(&self.pidfd),
                "the retained actual child is terminal"
            );
            println!(
                "PREVIEW_ACTUAL_RETIRE pid={} status={:?} local_successful={} original_work_ns={} original_close_ns={} observed_ns={}",
                self.process.pid(),
                done.status,
                done.successful,
                self.work_until.elapsed().as_nanos(),
                self.close_until.elapsed().as_nanos(),
                now().elapsed().as_nanos()
            );
            self.finished = Some(done.successful);
        }
    }
    fn marker(&mut self, name: &str, drive: bool) {
        while !self.root.join(name).exists() {
            assert!(now() < self.work_until, "original fixture readiness cutoff");
            if drive {
                self.tick();
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
    fn finish(&mut self) -> bool {
        while self.finished.is_none() {
            assert!(now() < self.close_until, "original owned close cutoff");
            self.tick();
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !PathBuf::from(format!("/proc/{}", self.process.pid())).exists(),
            "actual parent reap precedes completion"
        );
        self.finished.unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        // Close our deliberate EOF hold before the same actual owner drains.
        self.held_stderr_writer.take();
        self.process.retire();
        while !self.process.settled() && now() < self.close_until {
            let _ = self.process.poll();
            thread::sleep(Duration::from_millis(1));
        }
        if !self.process.settled() {
            // Returning would release a live owner. The outer owned test scope
            // must report this failure; never manufacture a cleanup receipt.
            std::process::abort();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn finite_input_reaches_real_child_and_normal_wait_and_stderr_EOF() {
    let mut case = Case::start("finite", payload(), 2000);
    let successful = case.finish();
    let exact_input = fs::read(case.root.join("consumed")).unwrap() == b"exact\n";
    drop(case);
    assert!(
        successful && exact_input,
        "finite exact bytes, stdin EOF and normal completion"
    );
}

#[test]
fn stop_starting_or_active_retains_custody_through_actual_wait_and_stderr_eof() {
    let mut reports = Vec::new();
    for mode in ["hold-before-input", "hold-after-input"] {
        let mut case = Case::start(mode, payload(), 2000);
        case.marker("ready", false);
        if mode == "hold-after-input" {
            case.marker("consumed", true);
        }
        assert!(!terminal(&case.pidfd) && !case.process.settled());
        // A real test-owned writer, no descendant or orphan, delays stderr EOF
        // independently of the actual child's death/reap.
        case.held_stderr_writer = Some(
            OpenOptions::new()
                .write(true)
                .open(format!("/proc/{}/fd/2", case.process.pid()))
                .unwrap(),
        );
        case.process.retire();
        while !terminal(&case.pidfd)
            || PathBuf::from(format!("/proc/{}", case.process.pid())).exists()
        {
            assert!(now() < case.work_until, "original Stop observation cutoff");
            case.tick();
            thread::sleep(Duration::from_millis(1));
        }
        let occupied_after_actual_reap = !case.process.settled() && case.finished.is_none();
        case.held_stderr_writer.take();
        let no_success_after_Stop = !case.finish();
        drop(case);
        reports.push(occupied_after_actual_reap && no_success_after_Stop);
    }
    assert_eq!(
        reports,
        [true, true],
        "Stop never frees custody at wait alone or publishes success"
    );
}

#[test]
fn original_work_expiry_fences_buffered_exit_and_actual_stderr_overflow() {
    let mut late = Case::start("exit-ready", Vec::new(), 500);
    late.marker("ready", false);
    while !terminal(&late.pidfd) {
        assert!(
            now() < late.work_until,
            "actual child exits before original expiry"
        );
        thread::sleep(Duration::from_millis(1));
    }
    // Do not poll the owner until its original work deadline has really expired.
    while now() < late.work_until {
        thread::sleep(Duration::from_millis(1));
    }
    let late_refused = !late.finish();
    drop(late);

    let mut overflow = Case::start("stderr-overflow", Vec::new(), 2000);
    overflow.marker("ready", false);
    let overflow_refused = !overflow.finish();
    let overflow_not_just_work_expiry = now() < overflow.work_until;
    drop(overflow);
    assert!(
        late_refused && overflow_refused && overflow_not_just_work_expiry,
        "late local acceptance and >4096 real stderr bytes fail without releasing unfinished ownership"
    );
}

#[test]
fn delayed_poll_after_internal_close_reaps_already_dead_child_without_abort() {
    let mut case = Case::start("exit-ready", Vec::new(), 500);
    // Both bounds are fixed at construction, before readiness or any delay.
    // The test observer remains alive beyond Process's original work+2s close.
    let process_close = case
        .work_until
        .checked_after(Duration::from_secs(2))
        .unwrap();
    case.close_until = case
        .work_until
        .checked_after(Duration::from_secs(4))
        .unwrap();
    case.marker("ready", false);
    while !terminal(&case.pidfd) {
        assert!(
            now() < case.work_until,
            "real child must become terminal before its original work expiry"
        );
        thread::sleep(Duration::from_millis(1));
    }
    let terminal_at = now();
    assert!(terminal_at < case.work_until);
    assert!(!case.process.alive());
    // Delayed owner polling only: no suspend or injected clock is claimed.
    while now() <= process_close {
        assert!(now() < case.close_until, "original delayed observer cutoff");
        thread::sleep(Duration::from_millis(1));
    }
    println!(
        "PREVIEW_DELAYED_POLL_CONTROL pid={} terminal_before_work=true terminal_ns={} original_work_ns={} original_process_close_ns={} original_observer_close_ns={} before_poll_ns={}",
        case.process.pid(),
        terminal_at.elapsed().as_nanos(),
        case.work_until.elapsed().as_nanos(),
        process_close.elapsed().as_nanos(),
        case.close_until.elapsed().as_nanos(),
        now().elapsed().as_nanos()
    );
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let finished = loop {
        assert!(now() < case.close_until, "original delayed observer cutoff");
        if let Some(finished) = case.process.poll() {
            break finished;
        }
        thread::sleep(Duration::from_millis(1));
    };
    let actual_wait_and_stderr_eof = case.process.settled();
    let exact_child_terminal = terminal(&case.pidfd);
    let actual_reaped = !PathBuf::from(format!("/proc/{}", case.process.pid())).exists();
    println!(
        "PREVIEW_ACTUAL_RETIRE pid={} status={:?} local_successful={} original_work_ns={} original_close_ns={} observed_ns={}",
        case.process.pid(),
        finished.status,
        finished.successful,
        case.work_until.elapsed().as_nanos(),
        case.close_until.elapsed().as_nanos(),
        now().elapsed().as_nanos()
    );
    case.finished = Some(finished.successful);
    drop(case);
    assert!(
        !finished.successful
            && finished.status.code() == Some(0)
            && actual_wait_and_stderr_eof
            && exact_child_terminal
            && actual_reaped,
        "late polling retires local success but still reaps/drains an already dead child without abort"
    );
}
