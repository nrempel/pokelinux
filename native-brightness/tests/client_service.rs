//! Composed original-code Client -> normal Main -> retained worker execution.
//! Private SyntheticTree files only. Main is this test's real direct child;
//! workers remain Main's children and are never reported as our direct waits.
use pokelinux_brightness::client::{Client, Event, Snapshot};
use rustix::{
    fs::{CWD, Mode, OFlags, mkfifoat},
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType},
    pipe::{PipeFlags, pipe_with},
    process::{Pid, Signal, kill_process},
    time::{ClockId, clock_gettime},
};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
const SECOND: u64 = 1_000_000_000;
static NEXT: AtomicU32 = AtomicU32::new(0);
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * SECOND + t.tv_nsec as u64
}
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
fn within(until: u64, mut predicate: impl FnMut() -> bool) {
    loop {
        if predicate() {
            return;
        }
        assert!(now() < until, "original composed BOOTTIME cutoff");
        tick();
    }
}
struct Owned {
    child: Child,
    out: File,
    err: File,
    status: Option<ExitStatus>,
    out_eof: bool,
    err_eof: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    close: u64,
    reported: bool,
}
impl Owned {
    fn spawn(mut command: Command, close: u64) -> Self {
        let (out, ow) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).unwrap();
        let (err, ew) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).unwrap();
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::from(ow))
            .stderr(Stdio::from(ew))
            .spawn()
            .unwrap();
        Self {
            child,
            out: File::from(out),
            err: File::from(err),
            status: None,
            out_eof: false,
            err_eof: false,
            stdout: vec![],
            stderr: vec![],
            close,
            reported: false,
        }
    }
    fn pump(&mut self) {
        for (stream, eof, bytes) in [
            (&mut self.out, &mut self.out_eof, &mut self.stdout),
            (&mut self.err, &mut self.err_eof, &mut self.stderr),
        ] {
            for _ in 0..8 {
                let mut b = [0; 1024];
                match stream.read(&mut b) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        bytes.extend_from_slice(&b[..n]);
                        assert!(bytes.len() <= 8192, "bounded child output")
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        break;
                    }
                    Err(e) => panic!("child PIPE read: {e}"),
                }
            }
        }
    }
    fn poll(&mut self) -> Option<ExitStatus> {
        self.pump();
        if self.status.is_none() {
            self.status = self.child.try_wait().unwrap();
        }
        self.status
    }
    fn finish(&mut self, until: u64) -> ExitStatus {
        within(until.min(self.close), || {
            self.poll().is_some() && self.out_eof && self.err_eof
        });
        self.report(false);
        self.status.unwrap()
    }
    fn finish_cleanup(&mut self, until: u64) -> ExitStatus {
        within(until.min(self.close), || {
            self.poll().is_some() && self.out_eof && self.err_eof
        });
        self.report(true);
        self.status.unwrap()
    }
    fn report(&mut self, cleanup: bool) {
        if !self.reported {
            println!(
                "{}",
                json!({"owned_composed_Main_actual_wait":true,"exit":self.status.unwrap().code(),"stdout_EOF":self.out_eof,"stderr_EOF":self.err_eof,"cleanup":cleanup,"stderr":String::from_utf8_lossy(&self.stderr)})
            );
            self.reported = true;
        }
    }
    fn signal(&self, signal: Signal) {
        kill_process(Pid::from_raw(self.child.id() as i32).unwrap(), signal).unwrap();
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if self.status.is_none() {
            if self.child.try_wait().unwrap().is_none() {
                let _ = self.child.kill();
            }
            loop {
                self.pump();
                if let Some(status) = self.child.try_wait().unwrap() {
                    self.status = Some(status);
                    break;
                }
                if now() >= self.close {
                    std::process::abort();
                }
                tick();
            }
        }
        while !self.out_eof || !self.err_eof {
            self.pump();
            if now() >= self.close {
                std::process::abort();
            }
            tick();
        }
        self.report(true);
    }
}
struct Realm {
    root: PathBuf,
    service: Owned,
    work: u64,
}
impl Realm {
    fn start(name: &str, work: u64, close: u64) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brightness-client-service-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.join("class")).unwrap();
        fs::create_dir(root.join("class/panel")).unwrap();
        fs::create_dir(root.join("runtime")).unwrap();
        fs::set_permissions(root.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
        for (name, bytes) in [
            ("max_brightness", b"400\n".as_slice()),
            ("brightness", b"200\n".as_slice()),
            ("actual_brightness", b"120\n".as_slice()),
        ] {
            fs::write(root.join("class/panel").join(name), bytes).unwrap();
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_pokelinux-brightness"));
        command.args([
            "--serve",
            root.join("runtime/actor.sock").to_str().unwrap(),
            "synthetic-tree",
            root.join("class").to_str().unwrap(),
            "panel",
        ]);
        let service = Owned::spawn(command, close);
        let mut realm = Self {
            root,
            service,
            work,
        };
        within(work.min(now() + SECOND), || {
            assert!(
                realm.service.poll().is_none(),
                "Main exited before listener"
            );
            realm.socket().exists()
        });
        // Independent actual endpoint identity check. Client::inspect separately
        // performs its own connection/peer validation through the normal API.
        let fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::connect(&fd, &SocketAddrUnix::new(realm.socket()).unwrap()).unwrap();
        let peer = net::sockopt::socket_peercred(&fd).unwrap();
        assert_eq!(peer.pid.as_raw_pid() as u32, realm.service.child.id());
        assert_eq!(peer.uid.as_raw(), 0);
        assert_eq!(peer.gid.as_raw(), 0);
        drop(fd);
        realm
    }
    fn socket(&self) -> PathBuf {
        self.root.join("runtime/actor.sock")
    }
    fn attr(&self, name: &str) -> PathBuf {
        self.root.join("class/panel").join(name)
    }
    fn client(&self) -> Client {
        Client::new(self.socket()).unwrap()
    }
    fn children(&self) -> Vec<u32> {
        fs::read_to_string(format!(
            "/proc/{}/task/{}/children",
            self.service.child.id(),
            self.service.child.id()
        ))
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect()
    }
    fn one_worker(&mut self, until: u64) -> u32 {
        let mut worker = 0;
        within(until.min(self.work), || {
            assert!(
                self.service.poll().is_none(),
                "Main died with retained worker"
            );
            let ids = self.children();
            assert!(ids.len() <= 1, "overlapping actual workers");
            if let Some(id) = ids.first() {
                worker = *id;
                true
            } else {
                false
            }
        });
        let args = fs::read(format!("/proc/{worker}/cmdline")).unwrap();
        let args: Vec<_> = args.split(|b| *b == 0).filter(|s| !s.is_empty()).collect();
        assert_eq!(args[1], b"--brightness-worker");
        assert_eq!(args[2], self.service.child.id().to_string().as_bytes());
        assert_eq!(
            args[3],
            until.to_string().as_bytes(),
            "worker original deadline"
        );
        let stat = fs::read_to_string(format!("/proc/{worker}/stat")).unwrap();
        let fields: Vec<_> = stat
            .rsplit_once(") ")
            .unwrap()
            .1
            .split_whitespace()
            .collect();
        assert_eq!(fields[1].parse::<u32>().unwrap(), self.service.child.id());
        println!(
            "{}",
            json!({"observed_Main_owned_worker":worker,
            "actual_parent":self.service.child.id(),"original_until_ns":until,
            "test_direct_parent_wait":false})
        );
        worker
    }
    fn stopped(&mut self, until: u64) {
        self.service.signal(Signal::STOP);
        within(until.min(self.work), || {
            fs::read_to_string(format!("/proc/{}/stat", self.service.child.id()))
                .unwrap()
                .rsplit_once(") ")
                .unwrap()
                .1
                .starts_with("T ")
        });
    }
    fn worker_gone(&mut self, worker: u32, until: u64) {
        within(until.min(self.work), || {
            assert!(
                self.service.poll().is_none(),
                "Main exited during retirement"
            );
            !Path::new(&format!("/proc/{worker}")).exists() && self.children().is_empty()
        });
        println!(
            "{}",
            json!({"observed_Main_worker_retired":worker,
            "proc_absent":true,"Main_children_empty":true,"test_direct_parent_wait":false})
        );
    }
    fn fifo_max(&self) {
        fs::remove_file(self.attr("max_brightness")).unwrap();
        mkfifoat(CWD, self.attr("max_brightness"), Mode::RUSR | Mode::WUSR).unwrap();
    }
    fn restore_max(&self) {
        fs::remove_file(self.attr("max_brightness")).unwrap();
        fs::write(self.attr("max_brightness"), b"400\n").unwrap();
    }
    fn event(&mut self, client: &mut Client, until: u64) -> Event {
        let mut event = None;
        within(until.min(self.work), || {
            assert!(
                self.service.poll().is_none(),
                "Main exited before Client event"
            );
            event = client.poll();
            event.is_some()
        });
        event.unwrap()
    }
    fn inspect(&mut self, client: &mut Client) -> Snapshot {
        let until = self.work.min(now() + 800_000_000);
        assert!(
            client.inspect(until).unwrap(),
            "explicit Client Inspect admission"
        );
        assert!(matches!(self.event(client, until), Event::Observed));
        client.snapshot()
    }
    fn stop(&mut self) {
        self.service.signal(Signal::TERM);
        assert!(
            self.service.finish(self.work.min(now() + SECOND)).success(),
            "normal Main Stop must return0 after worker retirement"
        );
        assert!(!self.socket().exists());
    }
}
impl Drop for Realm {
    fn drop(&mut self) {
        if self.service.status.is_none() {
            // Failure cleanup releases an actual test SIGSTOP, then lets normal
            // Main Stop own its worker drain; it never waitpid's a grandchild.
            let pid = Pid::from_raw(self.service.child.id() as i32).unwrap();
            let _ = kill_process(pid, Signal::CONT);
            let _ = kill_process(pid, Signal::TERM);
            let _ = self.service.finish_cleanup(self.service.close);
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn positive(s: &Snapshot, requested: u32, actual: Option<u32>) {
    assert!(s.connected && !s.pending);
    let value = s.observation.expect("actual composed observed attributes");
    assert_eq!(value.max, 400);
    assert_eq!(value.requested, requested);
    assert_eq!(value.actual, actual);
}
fn set(realm: &mut Realm, client: &mut Client, id: u64, percent: u8, requested: u32) {
    let revision = client.snapshot().revision;
    let until = realm.work.min(now() + 800_000_000);
    assert!(client.set(id, percent, revision, until).unwrap());
    assert!(client.snapshot().pending && client.snapshot().observation.is_none());
    match realm.event(client, until) {
        Event::Completed {
            id: actual_id,
            reply,
        } => {
            assert_eq!(actual_id, id);
            assert!(reply.ok && !reply.pending && !reply.last_set_uncertain);
            assert_eq!(reply.requested_percent, Some(percent));
        }
        _ => panic!("missing actual composed Set completion"),
    }
    positive(&client.snapshot(), requested, Some(120));
    assert_eq!(
        fs::read(realm.attr("brightness")).unwrap(),
        format!("{requested}\n").as_bytes()
    );
    assert_eq!(fs::read(realm.attr("actual_brightness")).unwrap(), b"120\n");
    assert!(
        realm.children().is_empty(),
        "positive reply precedes retirement"
    );
}
fn persistent_updates(work: u64, close: u64) {
    let mut realm = Realm::start("persistent", work, close);
    let mut client = realm.client();
    assert!(!client.snapshot().connected);
    positive(&realm.inspect(&mut client), 200, Some(120));
    let incarnation = client.incarnation();
    assert!(incarnation > 0);
    set(&mut realm, &mut client, 11, 25, 100);
    set(&mut realm, &mut client, 12, 75, 300);
    fs::remove_file(realm.attr("actual_brightness")).unwrap();
    let optional = realm.inspect(&mut client);
    positive(&optional, 300, None);
    assert_eq!(optional.requested_percent, None);
    assert!(!optional.last_set_uncertain);
    assert!(!realm.attr("actual_brightness").exists());
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"300\n");
    assert_eq!(
        client.incarnation(),
        incarnation,
        "same persistent connection"
    );
    realm.stop();
}
fn held_cancel_reconciles(work: u64, close: u64) {
    let mut realm = Realm::start("cancel", work, close);
    let mut client = realm.client();
    positive(&realm.inspect(&mut client), 200, Some(120));
    let revision = client.snapshot().revision;
    let old_incarnation = client.incarnation();
    realm.fifo_max();
    let until = work.min(now() + 900_000_000);
    assert!(client.set(21, 75, revision, until).unwrap());
    let worker = realm.one_worker(until);
    let mut observer = realm.client();
    let remote_pending = realm.inspect(&mut observer);
    assert!(remote_pending.pending && remote_pending.observation.is_none());
    assert_eq!(realm.children(), vec![worker]);
    realm.stopped(until);
    client.revoke();
    let cancelled = client.snapshot();
    assert!(!cancelled.connected && cancelled.pending && cancelled.last_set_uncertain);
    assert!(cancelled.observation.is_none());
    assert!(!client.set(21, 75, revision, until).unwrap());
    assert!(client.poll().is_none());
    assert_eq!(realm.children(), vec![worker]);
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"200\n");
    realm.service.signal(Signal::CONT);
    realm.worker_gone(worker, work.min(now() + 500_000_000));
    assert!(
        client.snapshot().pending,
        "test-observed retirement is not Client knowledge"
    );
    realm.restore_max();
    let reconciled = realm.inspect(&mut client);
    positive(&reconciled, 200, Some(120));
    assert!(!reconciled.pending && reconciled.last_set_uncertain);
    assert_eq!(reconciled.requested_percent, None);
    assert!(client.incarnation() > old_incarnation);
    for _ in 0..4 {
        assert!(client.poll().is_none(), "poll must not replay a Set");
        assert!(realm.children().is_empty());
        assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"200\n");
        tick();
    }
    realm.stop();
}
fn record_observer(
    realm: &Realm,
    phase: &str,
    attempt: u32,
    set_until: u64,
    observer_until: u64,
    event: &Event,
    snapshot: &Snapshot,
) {
    let event = match event {
        Event::Observed => json!({"kind":"Observed"}),
        Event::Completed { id, reply } => json!({"kind":"Completed","id":id,"reply":reply}),
        Event::Rejected { id } => json!({"kind":"Rejected","id":id}),
        Event::Unavailable => json!({"kind":"Unavailable"}),
    };
    println!(
        "{}",
        json!({
            "composed_observer_phase":phase,"attempt":attempt,
            "original_Set_until_ns":set_until,"fixed_observer_until_ns":observer_until,
            "observed_boottime_ns":now(),"event":event,
            "snapshot":{"connected":snapshot.connected,"pending":snapshot.pending,
                "revision":snapshot.revision,"observation":snapshot.observation,
                "until_ns":snapshot.until_ns,"requested_percent":snapshot.requested_percent,
                "last_set_uncertain":snapshot.last_set_uncertain},
            "raw_max_bytes":fs::read(realm.attr("max_brightness")).unwrap(),
            "raw_requested_bytes":fs::read(realm.attr("brightness")).unwrap(),
            "raw_actual_bytes":fs::read(realm.attr("actual_brightness")).unwrap()
        })
    );
}
fn queued_success_expires(work: u64, close: u64) {
    let mut realm = Realm::start("queued", work, close);
    let mut client = realm.client();
    positive(&realm.inspect(&mut client), 200, Some(120));
    realm.fifo_max();
    let until = work.min(now() + 900_000_000);
    assert!(
        client
            .set(31, 75, client.snapshot().revision, until)
            .unwrap()
    );
    let worker = realm.one_worker(until);
    realm.stopped(until);
    let mut writer = None;
    within(
        until.min(now() + 300_000_000),
        || match fs::OpenOptions::new()
            .write(true)
            .custom_flags(OFlags::NONBLOCK.bits() as i32)
            .open(realm.attr("max_brightness"))
        {
            Ok(fd) => {
                writer = Some(fd);
                true
            }
            Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NXIO.raw_os_error()) => false,
            Err(e) => panic!("actual FIFO writer open: {e}"),
        },
    );
    let mut writer = writer.unwrap();
    writer.write_all(b"400\n").unwrap();
    realm.restore_max();
    drop(writer);
    within(until, || {
        fs::read_to_string(format!("/proc/{worker}/stat")).is_ok_and(|s| {
            s.rsplit_once(") ").is_some_and(|(_, tail)| {
                let fields: Vec<_> = tail.split_whitespace().collect();
                fields.first() == Some(&"Z") && fields.get(49) == Some(&"0")
            })
        })
    });
    assert!(
        now() < until,
        "actual worker success must precede original deadline"
    );
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"300\n");
    assert_eq!(fs::read(realm.attr("actual_brightness")).unwrap(), b"120\n");
    println!(
        "{}",
        json!({"actual_successful_worker_queued_while_Main_stopped":true,
        "worker":worker,"zombie_state":"Z","raw_exit_code":0,"before_original_deadline":true})
    );
    while now() < until + 30_000_000 {
        tick();
    }
    realm.service.signal(Signal::CONT);
    // Keep original Client open until Main has actually reaped the child.
    // Process disappearance alone does not establish its PIPE EOF or model
    // slot release; typed pending replies below remain honest busy state.
    realm.worker_gone(worker, work.min(now() + 500_000_000));
    let observer_until = work.min(now() + 800_000_000);
    let late_event = client.poll().expect("original pending Set deadline event");
    let late = client.snapshot();
    record_observer(
        &realm,
        "original_Set_after_deadline",
        0,
        until,
        observer_until,
        &late_event,
        &late,
    );
    assert!(matches!(late_event, Event::Unavailable));
    assert!(!late.connected && late.pending && late.last_set_uncertain);
    assert!(late.observation.is_none() && late.requested_percent.is_none());
    // Independent fresh Client has no local uncertain Set to mask actor state.
    // Every NEW readonly request shares this one original reconciliation
    // cutoff. No Set is resent and no original request deadline is renewed.
    let mut fresh = realm.client();
    let mut attempts = 0;
    let reconciled = loop {
        attempts += 1;
        assert!(
            attempts <= 16 && now() < observer_until,
            "bounded readonly observer"
        );
        assert!(fresh.inspect(observer_until).unwrap());
        let event = realm.event(&mut fresh, observer_until);
        let snapshot = fresh.snapshot();
        record_observer(
            &realm,
            "new_readonly_reconciliation",
            attempts,
            until,
            observer_until,
            &event,
            &snapshot,
        );
        assert!(matches!(event, Event::Observed));
        assert!(snapshot.connected);
        assert_eq!(snapshot.requested_percent, None);
        assert!(
            snapshot.last_set_uncertain,
            "late actor result was retroactively acknowledged"
        );
        if snapshot.pending {
            assert!(snapshot.observation.is_none());
            assert_eq!(snapshot.until_ns, 0);
            continue;
        }
        break snapshot;
    };
    positive(&reconciled, 300, Some(120));
    assert_eq!(reconciled.requested_percent, None);
    realm.stop();
}
fn unchanged_refresh_preserves_cached_revision(work: u64, close: u64) {
    let mut realm = Realm::start("unchanged-refresh", work, close);
    let mut cached = realm.client();
    let initial = realm.inspect(&mut cached);
    positive(&initial, 200, Some(120));
    let mut observer = realm.client();
    let first = realm.inspect(&mut observer);
    let second = realm.inspect(&mut observer);
    positive(&first, 200, Some(120));
    positive(&second, 200, Some(120));
    assert!(
        now() < initial.until_ns,
        "cached original observation must remain fresh"
    );
    assert!(
        realm.children().is_empty(),
        "completed refresh retains worker custody"
    );
    let until = work.min(initial.until_ns).min(now() + 800_000_000);
    // Exactly one NEW explicit Set uses A's still-fresh cached value revision.
    // Record the actual remote decision before asserting the new contract.
    assert!(cached.set(41, 75, initial.revision, until).unwrap());
    let event = realm.event(&mut cached, until);
    record_observer(
        &realm,
        "cached_Set_after_unchanged_refresh",
        1,
        until,
        until,
        &event,
        &cached.snapshot(),
    );
    println!(
        "{}",
        json!({"readonly_version_case":"unchanged",
        "cached_revision":initial.revision,"first_refresh_revision":first.revision,
        "second_refresh_revision":second.revision,"cached_original_until_ns":initial.until_ns,
        "first_refresh_until_ns":first.until_ns,"second_refresh_until_ns":second.until_ns,
        "exactly_one_explicit_Set":true})
    );
    assert!(
        matches!(&event, Event::Completed { id:41, reply } if reply.ok),
        "UNCHANGED_REFRESH_MUST_PRESERVE_CACHED_SET_ADMISSION"
    );
    assert_eq!(
        (first.revision, second.revision),
        (initial.revision, initial.revision)
    );
    assert!(first.until_ns > initial.until_ns && second.until_ns > first.until_ns);
    assert!(cached.snapshot().revision > initial.revision);
    positive(&cached.snapshot(), 300, Some(120));
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"300\n");
    realm.stop();
}
fn raw_change_invalidates_cached_revision(work: u64, close: u64) {
    let mut realm = Realm::start("changed-raw", work, close);
    let mut cached = realm.client();
    let initial = realm.inspect(&mut cached);
    positive(&initial, 200, Some(120));
    fs::write(realm.attr("brightness"), b"201\n").unwrap();
    let mut observer = realm.client();
    let changed = realm.inspect(&mut observer);
    positive(&changed, 201, Some(120));
    assert!(changed.revision > initial.revision);
    assert!(
        now() < initial.until_ns,
        "remote stale test needs fresh local cache"
    );
    let until = work.min(initial.until_ns).min(now() + 800_000_000);
    assert!(cached.set(42, 75, initial.revision, until).unwrap());
    let event = realm.event(&mut cached, until);
    record_observer(
        &realm,
        "stale_Set_after_raw201",
        1,
        until,
        until,
        &event,
        &cached.snapshot(),
    );
    assert!(matches!(event, Event::Rejected { id: 42 }));
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"201\n");
    assert_eq!(fs::read(realm.attr("actual_brightness")).unwrap(), b"120\n");
    assert!(realm.children().is_empty());
    for _ in 0..4 {
        assert!(cached.poll().is_none());
        assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"201\n");
        tick();
    }
    realm.stop();
}
fn positive_until(realm: &mut Realm, client: &mut Client, until: u64) -> Snapshot {
    for attempt in 1..=16 {
        assert!(now() < until, "original readonly gap reconciliation cutoff");
        assert!(client.inspect(until).unwrap());
        let event = realm.event(client, until);
        let view = client.snapshot();
        record_observer(
            realm,
            "gap_readonly_reconciliation",
            attempt,
            until,
            until,
            &event,
            &view,
        );
        assert!(matches!(event, Event::Observed));
        if !view.pending && view.observation.is_some() {
            return view;
        }
    }
    panic!("bounded readonly gap reconciliation exhausted");
}
fn gaps_never_restore_old_revision_or_cancelled_ack(work: u64, close: u64) {
    let mut realm = Realm::start("version-gaps", work, close);
    let mut client = realm.client();
    let initial = realm.inspect(&mut client);
    positive(&initial, 200, Some(120));
    while now() < initial.until_ns {
        assert!(now() < work);
        assert!(realm.service.poll().is_none());
        tick();
    }
    assert!(client.snapshot().observation.is_none());
    assert!(
        !client
            .set(51, 75, initial.revision, work.min(now() + 800_000_000))
            .unwrap()
    );
    let expired_recovery = realm.inspect(&mut client);
    positive(&expired_recovery, 200, Some(120));
    assert!(
        expired_recovery.revision > initial.revision
            && expired_recovery.requested_percent.is_none()
    );
    fs::remove_file(realm.attr("max_brightness")).unwrap();
    let unavailable = realm.inspect(&mut client);
    assert!(!unavailable.pending && unavailable.observation.is_none() && unavailable.until_ns == 0);
    assert!(
        !client
            .set(
                52,
                75,
                expired_recovery.revision,
                work.min(now() + 800_000_000)
            )
            .unwrap()
    );
    fs::write(realm.attr("max_brightness"), b"400\n").unwrap();
    let recovered = realm.inspect(&mut client);
    positive(&recovered, 200, Some(120));
    assert!(
        recovered.revision > expired_recovery.revision && recovered.requested_percent.is_none()
    );
    let before_cancel = recovered.revision;
    realm.fifo_max();
    let until = work.min(now() + 900_000_000);
    assert!(client.set(53, 75, before_cancel, until).unwrap());
    let worker = realm.one_worker(until);
    realm.stopped(until);
    client.revoke();
    assert!(
        client.snapshot().pending
            && client.snapshot().last_set_uncertain
            && client.snapshot().observation.is_none()
    );
    assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"200\n");
    realm.service.signal(Signal::CONT);
    realm.worker_gone(worker, work.min(now() + 500_000_000));
    realm.restore_max();
    let final_view = positive_until(&mut realm, &mut client, work.min(now() + 800_000_000));
    positive(&final_view, 200, Some(120));
    assert!(
        final_view.revision > before_cancel
            && final_view.last_set_uncertain
            && final_view.requested_percent.is_none()
    );
    assert!(
        !client
            .set(53, 75, before_cancel, work.min(now() + 800_000_000))
            .unwrap()
    );
    for _ in 0..4 {
        assert!(client.poll().is_none());
        assert!(realm.children().is_empty());
        assert_eq!(fs::read(realm.attr("brightness")).unwrap(), b"200\n");
        tick();
    }
    realm.stop();
}

fn main() {
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let start = now();
    let work = start + 25 * SECOND;
    let close = start + 30 * SECOND;
    let mut failed = 0;
    for (name, case) in [
        (
            "unchanged_refresh_preserves_cached_revision",
            unchanged_refresh_preserves_cached_revision as fn(u64, u64),
        ),
        (
            "raw_change_invalidates_cached_revision",
            raw_change_invalidates_cached_revision,
        ),
        (
            "gaps_never_restore_old_revision_or_cancelled_ack",
            gaps_never_restore_old_revision_or_cancelled_ack,
        ),
        (
            "persistent_Client_Inspect_25_75_independent_optional_actual",
            persistent_updates as fn(u64, u64),
        ),
        (
            "held_Set_revoke_pending_readonly_reconcile_no_replay",
            held_cancel_reconciles,
        ),
        (
            "original_deadline_refuses_real_queued_success",
            queued_success_expires,
        ),
    ] {
        assert!(now() < work);
        let passed = std::panic::catch_unwind(|| case(work, close)).is_ok();
        println!(
            "{}",
            json!({"brightness_client_service_case":name,"passed":passed})
        );
        if !passed {
            failed += 1;
        }
    }
    println!(
        "{}",
        json!({"brightness_client_service_cases":6,"failed":failed,
        "SyntheticTree_only":true,"original_work_ns":work-start,"original_close_ns":close-start})
    );
    assert_eq!(failed, 0, "composed actual Client/Main/worker cases failed");
}
