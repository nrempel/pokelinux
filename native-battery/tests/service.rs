//! Real socket/process/FIFO controls, not a simulated reader implementation.
use rustix::{
    fs::{CWD, Mode, mkfifoat},
    process::{Pid, Signal, kill_process},
    time::{ClockId, clock_gettime},
};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{OpenOptionsExt, symlink},
        net::UnixStream,
    },
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn within(until: u64, mut f: impl FnMut() -> bool) {
    loop {
        if f() {
            return;
        }
        assert!(now() < until, "original BOOTTIME test deadline elapsed");
        std::thread::sleep(Duration::from_millis(5));
    }
}
struct Realm {
    root: PathBuf,
    child: Child,
}
impl Realm {
    fn start(name: &str, fifo: bool) -> Self {
        let root =
            std::env::temp_dir().join(format!("battery-service-{}-{name}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("class")).unwrap();
        fs::create_dir(root.join("device")).unwrap();
        symlink("../device", root.join("class/test_battery")).unwrap();
        if fifo {
            mkfifoat(CWD, root.join("device/uevent"), Mode::RUSR | Mode::WUSR).unwrap();
        } else {
            fs::write(root.join("device/uevent"), Self::data(73, "Charging")).unwrap();
        }
        let child = Command::new(env!("CARGO_BIN_EXE_pokelinux-battery"))
            .args([
                "--serve",
                root.join("battery.sock").to_str().unwrap(),
                root.join("class").to_str().unwrap(),
                "test_battery",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let realm = Self { root, child };
        let until = now() + 2_000_000_000;
        within(until, || realm.snapshot().is_some());
        realm
    }
    fn data(cap: u8, status: &str) -> String {
        format!(
            "POWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_PRESENT=1\nPOWER_SUPPLY_CAPACITY={cap}\nPOWER_SUPPLY_STATUS={status}\n"
        )
    }
    fn snapshot(&self) -> Option<Value> {
        let mut s = UnixStream::connect(self.root.join("battery.sock")).ok()?;
        s.set_read_timeout(Some(Duration::from_millis(100))).ok()?;
        let mut b = Vec::new();
        Read::by_ref(&mut s).take(4097).read_to_end(&mut b).ok()?;
        if b.len() > 4096 || b.last() != Some(&b'\n') {
            return None;
        }
        serde_json::from_slice(&b).ok()
    }
    fn children(&self) -> Vec<u32> {
        fs::read_to_string(format!(
            "/proc/{}/task/{}/children",
            self.child.id(),
            self.child.id()
        ))
        .unwrap_or_default()
        .split_whitespace()
        .map(|p| p.parse().unwrap())
        .collect()
    }
    fn stop(&mut self) -> ExitStatus {
        kill_process(Pid::from_raw(self.child.id() as i32).unwrap(), Signal::TERM).unwrap();
        let until = now() + 2_000_000_000;
        let mut code = None;
        within(until, || {
            code = self.child.try_wait().unwrap();
            code.is_some()
        });
        code.unwrap()
    }
}
impl Drop for Realm {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            self.child.kill().unwrap();
            self.child.wait().unwrap();
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn readings_changes_and_source_loss_reach_the_same_real_service_socket() {
    let mut r = Realm::start("updates", false);
    let pid = r.child.id();
    let until = now() + 2_000_000_000;
    within(until, || {
        r.snapshot().is_some_and(|v| {
            v["available"] == true && v["capacity_percent"] == 73 && v["status"] == "charging"
        })
    });
    fs::write(r.root.join("device/uevent"), Realm::data(51, "Discharging")).unwrap();
    let until = now() + 2_000_000_000;
    within(until, || {
        r.snapshot().is_some_and(|v| {
            v["available"] == true && v["capacity_percent"] == 51 && v["status"] == "discharging"
        })
    });
    fs::remove_file(r.root.join("device/uevent")).unwrap();
    let until = now() + 2_000_000_000;
    within(until, || {
        r.snapshot().is_some_and(|v| {
            v["available"] == false && v["capacity_percent"].is_null() && v["status"].is_null()
        })
    });
    assert_eq!(r.child.id(), pid);
    assert!(r.child.try_wait().unwrap().is_none());
    assert!(r.stop().success(), "Main must exit0 after owned drain");
}
#[test]
fn held_real_read_keeps_snapshots_and_term_responsive_and_is_actually_reaped() {
    let mut r = Realm::start("blocked", true);
    let until = now() + 500_000_000;
    let mut retained = Vec::new();
    within(until, || {
        retained = r.children();
        !retained.is_empty()
    });
    assert_eq!(retained.len(), 1);
    for _ in 0..5 {
        let s = r
            .snapshot()
            .expect("snapshot blocked behind real FIFO read");
        assert_eq!(s["available"], false);
    }
    // The FIFO has no writer. This is the production reader blocked in actual open/read.
    let worker = retained[0];
    assert!(
        fs::read_to_string(format!("/proc/{worker}/cmdline"))
            .unwrap()
            .contains("--read-supply")
    );
    assert!(r.stop().success(), "SIGTERM must drain before Main0");
    assert!(
        !PathBuf::from(format!("/proc/{worker}")).exists(),
        "owned reader must be reaped, not detached"
    );
}
#[test]
fn a_buffered_success_after_the_original_deadline_is_rejected_before_recovery() {
    let mut r = Realm::start("deadline", true);
    let until = now() + 500_000_000;
    let mut retained = Vec::new();
    within(until, || {
        retained = r.children();
        !retained.is_empty()
    });
    let worker = retained[0];
    // Pause the actual owner, then let its exact FIFO reader complete into the
    // owned PIPE. Resume after1s: the real queued success is already expired.
    let main = Pid::from_raw(r.child.id() as i32).unwrap();
    kill_process(main, Signal::STOP).unwrap();
    let until = now() + 1_100_000_000;
    let mut writer = None;
    let open_until = now() + 500_000_000;
    within(open_until, || {
        match fs::OpenOptions::new()
            .write(true)
            .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
            .open(r.root.join("device/uevent"))
        {
            Ok(file) => {
                writer = Some(file);
                true
            }
            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::NXIO.raw_os_error()) => {
                false
            }
            Err(error) => panic!("actual FIFO writer admission failed: {error}"),
        }
    });
    let mut writer = writer.unwrap();
    writer
        .write_all(Realm::data(99, "Charging").as_bytes())
        .unwrap();
    drop(writer);
    let completed = now() + 500_000_000;
    within(completed, || {
        fs::read_to_string(format!("/proc/{worker}/stat")).is_ok_and(|s| {
            s.rsplit_once(") ").is_some_and(|(_, tail)| {
                let f: Vec<_> = tail.split_whitespace().collect();
                f.first() == Some(&"Z") && f.get(49) == Some(&"0")
            })
        })
    });
    while now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    kill_process(main, Signal::CONT).unwrap();
    let until = now() + 500_000_000;
    within(until, || !PathBuf::from(format!("/proc/{worker}")).exists());
    let until = now() + 250_000_000;
    while now() < until {
        let v = r.snapshot().unwrap();
        assert_eq!(
            v["available"], false,
            "late buffered99% reading became visible"
        );
        assert!(v["capacity_percent"].is_null());
        std::thread::sleep(Duration::from_millis(5));
    }
    fs::remove_file(r.root.join("device/uevent")).unwrap();
    fs::write(r.root.join("device/uevent"), Realm::data(42, "Full")).unwrap();
    let until = now() + 2_500_000_000;
    within(until, || {
        r.snapshot()
            .is_some_and(|v| v["available"] == true && v["capacity_percent"] == 42)
    });
    assert!(r.stop().success());
}
