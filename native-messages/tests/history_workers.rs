//! Normal production executable's real credentialed local history helpers.
//! These storage-child tests do not establish composed-owner GUI or app authority.
use pokelinux_lifecycle::{
    boottime_ns,
    messages::{Event, Provider},
};
use pokelinux_messages::worker::Projector;
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
static NEXT: AtomicU32 = AtomicU32::new(0);
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
fn children() -> BTreeSet<u32> {
    fs::read_to_string("/proc/thread-self/children")
        .unwrap()
        .split_whitespace()
        .map(|p| p.parse().unwrap())
        .collect()
}
struct Case {
    root: PathBuf,
    provider: Option<Projector>,
    until: u64,
    close: u64,
    before: BTreeSet<u32>,
    known: Vec<String>,
}
impl Case {
    fn new(seed: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from(std::env::var_os("POKELINUX_SESSION_CONFIG_TEST_ROOT").unwrap())
            .join(format!(
                "history-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for n in ["runtime", "raw", "inbox"] {
            fs::create_dir(root.join(n)).unwrap();
            fs::set_permissions(root.join(n), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut known = vec![];
        if seed {
            let mut raw = ReceiptOwner::open(&root.join("raw")).unwrap();
            let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
            let h = "000008915155100000006201213141510005e8329bfd06";
            let bytes: Vec<_> = (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect();
            for _ in 0..2 {
                known.push(
                    raw.admit(source, IncomingKind::Deliver, &bytes)
                        .unwrap()
                        .to_string(),
                );
            }
        }
        let started = boottime_ns();
        Self {
            root,
            provider: None,
            until: started + 4_000_000_000,
            close: started + 6_000_000_000,
            before: children(),
            known,
        }
    }
    fn spawn(&mut self) {
        self.provider = Some(
            Projector::spawn(
                &self.root.join("runtime"),
                &self.root.join("raw"),
                &self.root.join("inbox"),
                std::path::Path::new(env!("CARGO_BIN_EXE_messages-service")),
            )
            .unwrap(),
        );
    }
    fn p(&mut self) -> &mut Projector {
        self.provider.as_mut().unwrap()
    }
    fn ready(&mut self) {
        while boottime_ns() < self.until {
            self.p().poll().unwrap();
            if self.p().available() {
                assert!(!self.p().receiving());
                return;
            }
            if !self.p().occupied() {
                break;
            }
            tick();
        }
        panic!(
            "normal daemon did not serve history worker entries before original readiness cutoff"
        );
    }
    fn query(&mut self, q: Value) -> Value {
        let until = self.until.min(boottime_ns() + 1_000_000_000);
        let token = self.p().start(q, until).unwrap();
        while boottime_ns() < until {
            if let Some(event) = self.p().poll().unwrap() {
                match event {
                    Event::Complete {
                        token: seen,
                        packet,
                    } => {
                        assert_eq!(seen, token);
                        return serde_json::from_slice(&packet).unwrap();
                    }
                    Event::Failed { .. } => panic!("history read failed"),
                }
            }
            tick();
        }
        panic!("original read deadline");
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if let Some(p) = &mut self.provider {
            let owned: Vec<_> = children().difference(&self.before).copied().collect();
            assert!(p.drain_until(self.close));
            assert!(!p.occupied());
            assert!(!p.read_occupied());
            for pid in &owned {
                assert_eq!(
                    rustix::process::waitpid(
                        rustix::process::Pid::from_raw(*pid as i32),
                        rustix::process::WaitOptions::NOHANG
                    )
                    .unwrap_err(),
                    rustix::io::Errno::CHILD
                );
            }
            println!(
                "HISTORY_HELPERS_DRAINED children={} occupied=false actual_wait_in_Projector=true observed_at_ns={} original_until_ns={} stdout=DEVNULL stderr=DEVNULL",
                owned.len(),
                boottime_ns(),
                self.close
            );
        }
        assert_eq!(children(), self.before);
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
#[ignore = "isolated root credentialed child IPC"]
fn normal_daemon_reads_exact_committed_history_with_independent_reader() {
    let mut c = Case::new(true);
    c.spawn();
    c.ready();
    assert_eq!(children().difference(&c.before).count(), 2);
    let page = c.query(json!({"query":"conversations","after":null,"limit":8}));
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let conversation = page["items"][0]["id"].clone();
    let page = c
        .query(json!({"query":"messages","conversation":conversation,"after_ordinal":0,"limit":2}));
    let found: BTreeSet<_> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["receipt"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(found, c.known.iter().cloned().collect());
}
#[test]
#[ignore = "isolated root credentialed child IPC"]
fn failed_storage_bootstrap_reaps_retained_child() {
    let mut c = Case::new(false);
    fs::remove_dir(c.root.join("raw")).unwrap();
    fs::write(c.root.join("raw"), b"not a directory").unwrap();
    c.spawn();
    while c.p().occupied() && boottime_ns() < c.until {
        let _ = c.p().poll();
        tick();
    }
    assert!(!c.p().available());
    assert!(!c.p().receiving());
    assert!(!c.p().occupied());
}
#[test]
#[ignore = "isolated root credentialed child IPC"]
fn stopped_helpers_cannot_publish_or_reattach_receiving() {
    let mut c = Case::new(false);
    c.spawn();
    c.ready();
    c.p().stop();
    assert!(!c.p().available());
    assert!(!c.p().receiving());
    assert!(c.p().occupied());
    let until = c.until;
    assert!(
        c.p()
            .start(
                json!({"query":"conversations","after":null,"limit":8}),
                until
            )
            .is_err()
    );
}
