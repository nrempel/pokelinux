//! Real private SEQPACKET, process and FIFO controls; SyntheticTree only.
use rustix::{
    fd::OwnedFd,
    fs::{CWD, Mode, OFlags, mkfifoat},
    net::{self, AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType},
    pipe::{PipeFlags, pipe_with},
    process::{Pid, Signal, kill_process},
    time::{ClockId, clock_gettime},
};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
static NEXT: AtomicU32 = AtomicU32::new(0);
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
fn within(until: u64, mut predicate: impl FnMut() -> bool) {
    loop {
        if predicate() {
            return;
        }
        assert!(now() < until, "original BOOTTIME test cutoff");
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
    fn report(&mut self, cleanup: bool) {
        if !self.reported {
            println!(
                "{}",
                json!({"owned_service_actual_wait":true,"exit":self.status.unwrap().code(),"stdout_EOF":self.out_eof,"stderr_EOF":self.err_eof,"cleanup":cleanup,"stderr":String::from_utf8_lossy(&self.stderr)})
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
            "brightness-service-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.join("class")).unwrap();
        fs::create_dir(root.join("class/panel")).unwrap();
        fs::create_dir(root.join("runtime")).unwrap();
        fs::set_permissions(root.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("class/panel/max_brightness"), b"400\n").unwrap();
        fs::write(root.join("class/panel/brightness"), b"200\n").unwrap();
        fs::write(root.join("class/panel/actual_brightness"), b"120\n").unwrap();
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
        let until = work.min(now() + 1_000_000_000);
        within(until, || {
            assert!(
                realm.service.poll().is_none(),
                "service exited before private listener"
            );
            realm.socket().exists()
        });
        realm
    }
    fn socket(&self) -> PathBuf {
        self.root.join("runtime/actor.sock")
    }
    fn attr(&self, name: &str) -> PathBuf {
        self.root.join("class/panel").join(name)
    }
    fn connect(&self) -> OwnedFd {
        let fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::connect(&fd, &SocketAddrUnix::new(self.socket()).unwrap()).unwrap();
        let peer = net::sockopt::socket_peercred(&fd).unwrap();
        assert_eq!(peer.uid.as_raw(), 0);
        assert_eq!(peer.gid.as_raw(), 0);
        assert_eq!(peer.pid.as_raw_pid() as u32, self.service.child.id());
        fd
    }
    fn children(&self) -> Vec<u32> {
        fs::read_to_string(format!(
            "/proc/{}/task/{}/children",
            self.service.child.id(),
            self.service.child.id()
        ))
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect()
    }
    fn one_worker(&self, until: u64) -> u32 {
        let mut id = 0;
        within(until, || {
            let children = self.children();
            assert!(children.len() <= 1, "more than one retained worker");
            if let Some(pid) = children.first() {
                id = *pid;
                true
            } else {
                false
            }
        });
        assert!(
            fs::read(format!("/proc/{id}/cmdline"))
                .unwrap()
                .windows(19)
                .any(|s| s == b"--brightness-worker")
        );
        id
    }
    fn inspect(&self, fd: &OwnedFd, id: u64) -> Value {
        let until = self.work.min(now() + 800_000_000);
        send(
            fd,
            &json!({"op":"inspect","id":id,"until_ns":until.to_string()}),
        );
        reply(fd, until)
    }
    fn set(&self, fd: &OwnedFd, id: u64, revision: u64, percent: u8, until: u64) {
        send(
            fd,
            &json!({"op":"set","id":id,"percent":percent,"expected_revision":revision,"until_ns":until.to_string()}),
        );
    }
    fn fifo_max(&self) {
        fs::remove_file(self.attr("max_brightness")).unwrap();
        mkfifoat(CWD, self.attr("max_brightness"), Mode::RUSR | Mode::WUSR).unwrap();
    }
    fn stop(&mut self) {
        self.service.signal(Signal::TERM);
        assert!(
            self.service
                .finish(self.work.min(now() + 1_000_000_000))
                .success(),
            "Main must return0 after actual worker retirement"
        );
        assert!(!self.socket().exists(), "retired endpoint remains");
    }
}
impl Drop for Realm {
    fn drop(&mut self) {
        if self.service.status.is_none() {
            let _ = self.service.child.kill();
        } // Owned's Drop closes actual custody before temporary inputs disappear.
        // Main's PDEATHSIG workers are checked on every positive test; failure cleanup
        // preserves the child's wait/EOF receipt rather than claiming functional pass.
        if self.service.status.is_none() {
            let _ = self.service.finish(self.service.close);
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn send(fd: &OwnedFd, value: &Value) {
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(bytes.len() <= 512);
    assert_eq!(
        net::send(fd, &bytes, SendFlags::NOSIGNAL).unwrap(),
        bytes.len()
    );
}
fn reply(fd: &OwnedFd, until: u64) -> Value {
    let mut value = None;
    within(until, || {
        let mut b = [0; 1024];
        match net::recv(fd, &mut b, RecvFlags::DONTWAIT | RecvFlags::TRUNC) {
            Ok((0, _)) => panic!("service closed before reply"),
            Ok((n, total)) => {
                assert!(n == total && total <= 512);
                value = Some(serde_json::from_slice(&b[..n]).unwrap());
                true
            }
            Err(rustix::io::Errno::AGAIN) => false,
            Err(e) => panic!("reply: {e}"),
        }
    });
    let v: Value = value.unwrap();
    for field in [
        "schema",
        "id",
        "ok",
        "revision",
        "pending",
        "observation",
        "until_ns",
        "requested_percent",
        "last_set_uncertain",
    ] {
        assert!(v.get(field).is_some(), "required reply field {field}");
    }
    assert_eq!(v["schema"], 1);
    v
}
fn positive(v: &Value, requested: u32) {
    assert_eq!(v["ok"], true);
    assert_eq!(v["pending"], false);
    assert_eq!(
        v["observation"],
        json!({"max":400,"requested":requested,"actual":120})
    );
}
fn closed(fd: &OwnedFd, until: u64) {
    within(until, || {
        let mut b = [0; 513];
        match net::recv(fd, &mut b, RecvFlags::DONTWAIT | RecvFlags::TRUNC) {
            Ok((0, _)) | Err(rustix::io::Errno::CONNRESET) => true,
            Err(rustix::io::Errno::AGAIN) => false,
            other => panic!("invalid request was not closed: {other:?}"),
        }
    });
}
fn normal_updates(work: u64, close: u64) {
    let mut r = Realm::start("positive", work, close);
    assert_eq!(
        fs::metadata(r.root.join("runtime"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(r.socket()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let fd = r.connect();
    let initial = r.inspect(&fd, 1);
    positive(&initial, 200);
    let revision = initial["revision"].as_u64().unwrap();
    let until = work.min(now() + 800_000_000);
    r.set(&fd, 2, revision, 75, until);
    let changed = reply(&fd, until);
    positive(&changed, 300);
    assert_eq!(changed["requested_percent"], 75);
    assert_eq!(changed["last_set_uncertain"], false);
    assert!(changed["revision"].as_u64().unwrap() > revision);
    assert_eq!(fs::read(r.attr("brightness")).unwrap(), b"300\n");
    assert_eq!(fs::read(r.attr("actual_brightness")).unwrap(), b"120\n");
    let until = work.min(now() + 500_000_000);
    r.set(&fd, 3, revision, 25, until);
    let stale = reply(&fd, until);
    assert_eq!(stale["ok"], false);
    assert!(stale["requested_percent"].is_null());
    assert_eq!(fs::read(r.attr("brightness")).unwrap(), b"300\n");
    r.stop();
}
fn strict_packets(work: u64, close: u64) {
    let mut r = Realm::start("strict", work, close);
    for bytes in [
        br#"{"schema":1,"op":"inspect","id":1,"until_ns":"9"}"#.to_vec(),
        br#"{"op":"inspect","id":1,"until_ns":"9","extra":true}"#.to_vec(),
        br#"{"op":"inspect","id":1,"id":2,"until_ns":"9"}"#.to_vec(),
        br#"{"op":"inspect","id":1,"until_ns":"01"}"#.to_vec(),
        vec![b'x'; 513],
    ] {
        let fd = r.connect();
        assert_eq!(
            net::send(&fd, &bytes, SendFlags::NOSIGNAL).unwrap(),
            bytes.len()
        );
        closed(&fd, work.min(now() + 500_000_000));
        assert!(r.children().is_empty());
    }
    positive(&r.inspect(&r.connect(), 8), 200);
    r.stop();
}
fn held_inspect_stop(work: u64, close: u64) {
    let mut r = Realm::start("held-inspect", work, close);
    r.fifo_max();
    let fd = r.connect();
    let until = work.min(now() + 800_000_000);
    send(
        &fd,
        &json!({"op":"inspect","id":1,"until_ns":until.to_string()}),
    );
    let worker = r.one_worker(work.min(now() + 300_000_000));
    let other = r.connect();
    for id in 2..5 {
        let snapshot = r.inspect(&other, id);
        assert_eq!(snapshot["ok"], false);
        assert_eq!(snapshot["pending"], true);
        assert!(snapshot["observation"].is_null());
        assert_eq!(r.children(), vec![worker]);
    }
    r.stop();
    assert!(
        !Path::new(&format!("/proc/{worker}")).exists(),
        "blocked worker detached"
    );
}
fn disconnect_pending_set(work: u64, close: u64) {
    let mut r = Realm::start("disconnect", work, close);
    let fd = r.connect();
    let initial = r.inspect(&fd, 1);
    positive(&initial, 200);
    r.fifo_max();
    let until = work.min(now() + 800_000_000);
    r.set(&fd, 2, initial["revision"].as_u64().unwrap(), 75, until);
    let worker = r.one_worker(work.min(now() + 300_000_000));
    let other = r.connect();
    let pending = r.inspect(&other, 3);
    assert_eq!(pending["pending"], true);
    assert!(pending["observation"].is_null());
    assert_eq!(r.children(), vec![worker]);
    drop(fd);
    within(work.min(now() + 500_000_000), || {
        !Path::new(&format!("/proc/{worker}")).exists()
    });
    assert!(r.children().is_empty());
    fs::remove_file(r.attr("max_brightness")).unwrap();
    fs::write(r.attr("max_brightness"), b"400\n").unwrap();
    let current = r.inspect(&other, 4);
    positive(&current, 200);
    assert_eq!(current["last_set_uncertain"], true);
    assert!(current["requested_percent"].is_null());
    r.stop();
}
fn queued_late_set(work: u64, close: u64) {
    let mut r = Realm::start("late-set", work, close);
    let fd = r.connect();
    let initial = r.inspect(&fd, 1);
    positive(&initial, 200);
    r.fifo_max();
    let until = work.min(now() + 900_000_000);
    r.set(&fd, 2, initial["revision"].as_u64().unwrap(), 75, until);
    let worker = r.one_worker(work.min(now() + 300_000_000));
    r.service.signal(Signal::STOP);
    let mut writer = None;
    within(
        work.min(now() + 300_000_000),
        || match fs::OpenOptions::new()
            .write(true)
            .custom_flags(OFlags::NONBLOCK.bits() as i32)
            .open(r.attr("max_brightness"))
        {
            Ok(fd) => {
                writer = Some(fd);
                true
            }
            Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NXIO.raw_os_error()) => false,
            Err(e) => panic!("FIFO writer: {e}"),
        },
    );
    let mut writer = writer.unwrap();
    writer.write_all(b"400\n").unwrap();
    fs::remove_file(r.attr("max_brightness")).unwrap();
    fs::write(r.attr("max_brightness"), b"400\n").unwrap();
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
        "positive child must finish before its original deadline"
    );
    assert_eq!(fs::read(r.attr("brightness")).unwrap(), b"300\n");
    while now() < until + 30_000_000 {
        tick();
    }
    r.service.signal(Signal::CONT);
    let rejected = reply(&fd, work.min(now() + 500_000_000));
    assert_eq!(
        rejected["ok"], false,
        "queued late Set retroactively acknowledged"
    );
    assert!(rejected["observation"].is_null());
    assert!(rejected["requested_percent"].is_null());
    assert_eq!(rejected["last_set_uncertain"], true);
    within(work.min(now() + 300_000_000), || {
        !Path::new(&format!("/proc/{worker}")).exists()
    });
    let reconciled = r.inspect(&fd, 3);
    positive(&reconciled, 300);
    assert_eq!(reconciled["last_set_uncertain"], true);
    assert!(reconciled["requested_percent"].is_null());
    r.stop();
}
fn unprivileged_probe(socket: &str) {
    assert_eq!(rustix::process::getuid().as_raw(), 11010);
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    net::connect(&fd, &SocketAddrUnix::new(socket).unwrap()).unwrap();
    let until = now() + 500_000_000;
    let bytes =
        serde_json::to_vec(&json!({"op":"inspect","id":1,"until_ns":until.to_string()})).unwrap();
    let _ = net::send(&fd, &bytes, SendFlags::NOSIGNAL);
    closed(&fd, until);
}
fn actual_unprivileged_peer(work: u64, close: u64) {
    let mut r = Realm::start("peer", work, close);
    fs::set_permissions(&r.root, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(r.root.join("runtime"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(r.socket(), fs::Permissions::from_mode(0o666)).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--unprivileged-probe", r.socket().to_str().unwrap()])
        .uid(11010)
        .gid(11010);
    let mut probe = Owned::spawn(command, close);
    assert!(probe.finish(work.min(now() + 1_000_000_000)).success());
    assert!(r.children().is_empty());
    r.stop();
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if let [_, mode, socket] = args.as_slice()
        && mode == "--unprivileged-probe"
    {
        unprivileged_probe(socket);
        return;
    }
    if let [_, mode, revision] = args.as_slice()
        && mode == "--inherited-nonroot-probe"
    {
        inherited_nonroot_probe(revision);
        return;
    }
    if args.get(1).is_some_and(|v| v == "--preflight-only") {
        let start = now();
        let work = start + 25_000_000_000;
        let close = start + 30_000_000_000;
        let passed =
            std::panic::catch_unwind(|| late_preflight_must_not_write(work, close)).is_ok();
        println!(
            "{}",
            json!({"brightness_service_case":"late_preflight_never_writes_after_original_deadline","passed":passed})
        );
        println!(
            "{}",
            json!({"brightness_service_cases":1,"failed":u8::from(!passed),"SyntheticTree_only":true})
        );
        assert!(passed, "late preflight service regression");
        return;
    }
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let start = now();
    let work = start + 25_000_000_000;
    let close = start + 30_000_000_000;
    let mut failed = 0;
    for (name, case) in [
        (
            "same_socket_fresh_observe_set_and_stale_revision",
            normal_updates as fn(u64, u64),
        ),
        ("strict_packets_never_dispatch_backend", strict_packets),
        (
            "held_real_inspect_single_worker_and_Stop",
            held_inspect_stop,
        ),
        (
            "owner_disconnect_fences_real_Set_and_reconciles",
            disconnect_pending_set,
        ),
        (
            "original_deadline_rejects_real_queued_positive_Set",
            queued_late_set,
        ),
        (
            "actual_nonroot_peer_rejected_with_relaxed_fixture_modes",
            actual_unprivileged_peer,
        ),
        (
            "late_preflight_never_writes_after_original_deadline",
            late_preflight_must_not_write,
        ),
        (
            "inherited_root_socket_nonroot_Set_writer_denied",
            inherited_root_socket_writer_denied,
        ),
    ] {
        assert!(now() < work);
        let result = std::panic::catch_unwind(|| case(work, close));
        let passed = result.is_ok();
        println!(
            "{}",
            json!({"brightness_service_case":name,"passed":passed})
        );
        if !passed {
            failed += 1;
        }
    }
    println!(
        "{}",
        json!({"brightness_service_cases":8,"failed":failed,"SyntheticTree_only":true,"original_work_ns":work-start,"original_close_ns":close-start})
    );
    assert_eq!(failed, 0, "real brightness service cases failed");
}

fn late_preflight_must_not_write(work: u64, close: u64) {
    let mut r = Realm::start("late-preflight", work, close);
    let fd = r.connect();
    let initial = r.inspect(&fd, 1);
    positive(&initial, 200);
    fs::remove_file(r.attr("actual_brightness")).unwrap();
    mkfifoat(CWD, r.attr("actual_brightness"), Mode::RUSR | Mode::WUSR).unwrap();
    let until = work.min(now() + 250_000_000);
    r.set(&fd, 2, initial["revision"].as_u64().unwrap(), 75, until);
    let worker = r.one_worker(until);
    let mut blocked = String::new();
    within(until, || {
        blocked = fs::read_to_string(format!("/proc/{worker}/wchan")).unwrap();
        blocked.contains("wait_for_partner") || blocked.contains("pipe_read")
    });
    r.service.signal(Signal::STOP);
    while now() < until + 30_000_000 {
        tick();
    }
    let mut writer = None;
    within(
        work.min(now() + 500_000_000),
        || match fs::OpenOptions::new()
            .write(true)
            .custom_flags(OFlags::NONBLOCK.bits() as i32)
            .open(r.attr("actual_brightness"))
        {
            Ok(fd) => {
                writer = Some(fd);
                true
            }
            Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NXIO.raw_os_error()) => false,
            Err(e) => panic!("late preflight FIFO writer: {e}"),
        },
    );
    let mut writer = writer.unwrap();
    fs::remove_file(r.attr("actual_brightness")).unwrap();
    fs::write(r.attr("actual_brightness"), b"120\n").unwrap();
    writer.write_all(b"120\n").unwrap();
    drop(writer);
    let mut worker_exit = None;
    within(work.min(now() + 500_000_000), || {
        if let Ok(stat) = fs::read_to_string(format!("/proc/{worker}/stat"))
            && let Some((_, tail)) = stat.rsplit_once(") ")
        {
            let fields: Vec<_> = tail.split_whitespace().collect();
            if fields.first() == Some(&"Z") {
                worker_exit = fields.get(49).and_then(|v| v.parse::<u32>().ok());
                return worker_exit.is_some();
            }
        }
        false
    });
    let actual = fs::read_to_string(r.attr("brightness")).unwrap();
    r.service.signal(Signal::CONT);
    let rejected = reply(&fd, work.min(now() + 500_000_000));
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["last_set_uncertain"], true);
    within(work.min(now() + 500_000_000), || {
        !Path::new(&format!("/proc/{worker}")).exists()
    });
    r.stop();
    println!(
        "{}",
        json!({"late_preflight_actual_brightness_attribute":actual,"expected_unchanged":"200\n","preflight_original_until_elapsed_before_FIFO_release":true,"actual_preflight_wait_channel":blocked,"actual_worker_zombie_exit_code":worker_exit,"same_Main_resumed_and_retired_worker":true,"Main_Stop_actual_wait0_and_both_PIPE_EOFs":true})
    );
    assert_eq!(
        actual, "200\n",
        "a preflight read released after the original deadline must never write"
    );
}
fn inherited_nonroot_probe(revision: &str) {
    assert_eq!(rustix::process::getuid().as_raw(), 11010);
    let fd = rustix::io::fcntl_dupfd_cloexec(rustix::stdio::stdin(), 3).unwrap();
    let until = now() + 500_000_000;
    send(
        &fd,
        &json!({"op":"set","id":2,"percent":75,"expected_revision":revision.parse::<u64>().unwrap(),"until_ns":until.to_string()}),
    );
    closed(&fd, until);
}
fn inherited_root_socket_writer_denied(work: u64, close: u64) {
    let mut r = Realm::start("inherited-writer", work, close);
    let fd = r.connect();
    let initial = r.inspect(&fd, 1);
    positive(&initial, 200);
    let (out, out_write) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).unwrap();
    let (err, err_write) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--inherited-nonroot-probe",
            &initial["revision"].as_u64().unwrap().to_string(),
        ])
        .uid(11010)
        .gid(11010)
        .stdin(Stdio::from(fd))
        .stdout(Stdio::from(out_write))
        .stderr(Stdio::from(err_write))
        .spawn()
        .unwrap();
    let mut probe = Owned {
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
    };
    assert!(
        probe.finish(work.min(now() + 1_000_000_000)).success(),
        "nonroot inherited-FD Set was not denied"
    );
    assert_eq!(fs::read(r.attr("brightness")).unwrap(), b"200\n");
    assert!(r.children().is_empty());
    assert_eq!(
        fs::metadata(r.root.join("runtime"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(r.socket()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    positive(&r.inspect(&r.connect(), 3), 200);
    r.stop();
}
