//! Persistent wireless consumer: normal Settings SDK → actor → normal Main.
//! No normal Qt/C ABI/window proof; SDK owns its private transport and thread.
//! Authored session=None/unlocked/foreground facts; no Source, GUI or auth claim.
//! Register only as a child of copied service_restore_tests.rs, never production.
use super::ActorControl;
use crate::boottime_ns as boot_time_ns;
use crate::{Reply, boottime_ns, connectivity, fresh_id, ipc};
use rustix::{
    fd::OwnedFd,
    net::sockopt::socket_peercred,
    process::{Pid, PidfdFlags},
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    env,
    fs::{self, File},
    io::{self, Read, Write},
    net::{Ipv4Addr, UdpSocket},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        process::ExitStatusExt,
    },
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
const APP: &str = "org.omarchy.Settings";
const PEER: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 2);
const PORT: u16 = 36042;
const APP_ERROR_LIMIT: u64 = 4096;
const DIAGNOSTIC_LIMIT: usize = 32768;
fn pause() {
    thread::sleep(Duration::from_millis(2));
}
fn until(case: u64) -> u64 {
    case.min(boottime_ns().checked_add(1_000_000_000).unwrap())
}
fn publish(root: &Path, name: &str, value: &Value) {
    let temporary = root.join(format!("{name}.tmp"));
    fs::write(&temporary, serde_json::to_vec(value).unwrap()).unwrap();
    fs::rename(temporary, root.join(name)).unwrap();
}
fn gate(root: &Path, name: &str, end: u64) -> Value {
    while boottime_ns() < end {
        match fs::read(root.join(name)) {
            Ok(bytes) => return serde_json::from_slice(&bytes).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => pause(),
            Err(e) => panic!("gate {name}: {e}"),
        }
    }
    panic!("original cutoff waiting for {name}");
}
fn diagnostic_once(root: &Path, name: &str, value: &Value) {
    // Error-only evidence must not replace the original assertion or renew any
    // deadline. One bounded write attempt; no per-getter files, waits or retry.
    if let Ok(bytes) = serde_json::to_vec(value)
        && bytes.len() <= DIAGNOSTIC_LIMIT
    {
        let _ = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(name))
            .and_then(|mut file| file.write_all(&bytes));
    }
}
fn app_error_report(control: &Path, pid: u32) -> Value {
    let path = control
        .parent()
        .unwrap()
        .join(format!("fixture-error-{pid}.json"));
    let report = (|| -> io::Result<Value> {
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != 11050
            || metadata.gid() != 11050
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o644
            || metadata.len() > APP_ERROR_LIMIT
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut bytes = vec![];
        file.take(APP_ERROR_LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > APP_ERROR_LIMIT {
            return Err(io::ErrorKind::InvalidData.into());
        }
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(report) => Ok(json!({"present":true,"report":report})),
            Err(error) => Ok(json!({"present":true,"parse_error":error.to_string(),
                "raw_report_hex":hex(&bytes)})),
        }
    })();
    match report {
        Ok(mut report) => {
            report["path"] = json!(path);
            report["collected_at_ns"] = json!(boottime_ns());
            report
        }
        Err(error) => {
            json!({"path":path,"present":false,"read_error_kind":format!("{:?}",error.kind()),
            "raw_os_error":error.raw_os_error(),"collected_at_ns":boottime_ns()})
        }
    }
}
fn unavailable_matches(row: &Value, after: u64) -> Value {
    let view = &row["sdk_view"];
    json!({"sdk_view_is_object":view.is_object(),"sdk_view_is_null":view.is_null(),
        "getter_after_loss":row["sdk_call_before_ns"].as_u64().is_some_and(|at|at >= after),
        "available_false":view["available"] == false,"configured_null":view["configured"].is_null(),
        "pending_false":view["pending"] == false,"can_control_false":view["can_control"] == false,
        "observed_at_zero":view["observed_at_ns"] == 0,"until_zero":view["until_ns"] == 0,
        "status_unavailable":view["status"] == "unavailable"})
}
struct CommandContext<'a> {
    operation: &'a str,
    expected_sequence: u64,
    original_until_ns: u64,
    started_at_ns: u64,
}
struct App {
    pid: u32,
    alive: OwnedFd,
    control: PathBuf,
    sdk_stop: Option<Value>,
    next_sequence: u64,
}
struct Case {
    diagnostic_root: PathBuf,
    diagnostics: RefCell<Value>,
    failed_app: Cell<Option<u32>>,
    runtime: PathBuf,
    policy: PathBuf,
    app_control: PathBuf,
    args: Vec<PathBuf>,
    executable: PathBuf,
    actor: Option<JoinHandle<()>>,
    done: Receiver<io::Result<()>>,
    app_waits: Receiver<(u32, ExitStatus)>,
    reaped: Vec<(u32, ExitStatus)>,
    case: u64,
    close: u64,
    closed: bool,
}
impl Case {
    fn new(socket: &Path, case: u64, close: u64, diagnostic_root: &Path) -> Self {
        let id = fresh_id().unwrap();
        let install = PathBuf::from("/opt/connectivity-actor-fixture").join(&id);
        let runtime = PathBuf::from("/run/connectivity-actor-fixture").join(&id);
        let policy = PathBuf::from("/var/lib/connectivity-actor-fixture").join(&id);
        for path in [&install, &runtime, &policy] {
            fs::create_dir_all(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o700)).unwrap();
        let home = install.join("home");
        let themes = install.join("themes");
        for path in [&home, &themes] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let controls = runtime.join("controls");
        fs::create_dir(&controls).unwrap();
        fs::set_permissions(&controls, fs::Permissions::from_mode(0o1733)).unwrap();
        let application = controls.join("application");
        let args = vec![
            home.clone(),
            application.clone(),
            runtime.join("wayland.sock"),
            runtime.join("appearance.sock"),
            runtime.join("app.sock"),
            themes,
        ];
        let executable = PathBuf::from(
            env::var_os("NATIVE_SETTINGS_CONNECTIVITY_APP_FIXTURE")
                .expect("selected actual native app"),
        );
        let entry = PathBuf::from(
            env::var_os("NATIVE_LIFECYCLE_LAUNCHER_HELPER").expect("selected normal launcher"),
        );
        let storage = PathBuf::from(
            env::var_os("NATIVE_LIFECYCLE_STORAGE_HELPER").expect("selected normal storage helper"),
        );
        assert!(executable.is_absolute() && entry.is_absolute() && storage.is_absolute());
        fs::write(
            home.join("fixture.json"),
            serde_json::to_vec(
                &json!({"actor_pid":std::process::id(),"original_case_until_ns":case.to_string()}),
            )
            .unwrap(),
        )
        .unwrap();
        fs::set_permissions(home.join("fixture.json"), fs::Permissions::from_mode(0o644)).unwrap();
        let profile = install.join("profile.json");
        fs::write(
            &profile,
            serde_json::to_vec(
                &json!({"apps":[{"role":{"kind":"application"},"app":APP,"uid":11050,"executable":executable,"args":args}]}),
            )
            .unwrap(),
        )
        .unwrap();
        let (tx, done) = mpsc::sync_channel(1);
        let (app_wait_observer, app_waits) = mpsc::sync_channel(4);
        let (commands, inbox) = mpsc::sync_channel(1);
        drop(commands);
        let sr = runtime.clone();
        let sp = policy.clone();
        let endpoint = socket.to_owned();
        let actor = thread::spawn(move || {
            let mut control = ActorControl {
                inbox,
                attempt: None,
                observed_revision: Arc::new(AtomicU64::new(0)),
                source_barrier: None,
                storage_executable: storage,
                app_wait_observer,
                gui_replies: None,
                gui_observer_failed: Arc::new(AtomicBool::new(false)),
            };
            let result = connectivity::Owner::new(&endpoint).and_then(|mut provider| {
                super::super::run_with_device_services_inner(
                    &sr,
                    &sp,
                    &profile,
                    &entry,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(&mut provider),
                    None,
                    Some(&mut control),
                )
            });
            let _ = tx.try_send(result);
        });
        Self {
            diagnostic_root: diagnostic_root.to_owned(),
            diagnostics: RefCell::new(json!({})),
            failed_app: Cell::new(None),
            runtime,
            policy,
            app_control: application.join("control.sock"),
            args,
            executable,
            actor: Some(actor),
            done,
            app_waits,
            reaped: vec![],
            case,
            close,
            closed: false,
        }
    }
    fn admin(&self, request: Value, end: u64) -> Reply {
        let fd = ipc::connect(&self.runtime.join("admin.sock")).unwrap();
        ipc::call_until_from(fd, &request, end, None).unwrap()
    }
    fn ready(&self, end: u64) {
        while boottime_ns() < end {
            if fs::symlink_metadata(self.runtime.join("admin.sock"))
                .is_ok_and(|m| m.file_type().is_socket())
                && let Reply::Snapshot { ready: true, .. } =
                    self.admin(json!({"op":"snapshot"}), end)
            {
                return;
            }
            pause();
        }
        panic!("setup: normal actor storage readiness");
    }
    fn launch(&self) -> App {
        let end = until(self.case);
        assert_eq!(self.admin(json!({"op":"launch","app":APP}), end), Reply::Ok);
        while boottime_ns() < end {
            if fs::symlink_metadata(&self.app_control).is_ok_and(|m| m.file_type().is_socket()) {
                assert_eq!(fs::metadata(&self.app_control).unwrap().uid(), 11050);
                // Do not open-and-close a fake control request to learn identity.
                // Reuse the desktop fixture's actual normal-child /proc matching.
                let mut children = std::collections::BTreeSet::new();
                for task in fs::read_dir("/proc/self/task").unwrap() {
                    if let Ok(text) = fs::read_to_string(task.unwrap().path().join("children")) {
                        children.extend(text.split_whitespace().map(|v| v.parse::<u32>().unwrap()));
                    }
                }
                let mut expected = Vec::new();
                for arg in std::iter::once(&self.executable).chain(self.args.iter()) {
                    expected.extend_from_slice(arg.as_os_str().as_encoded_bytes());
                    expected.push(0);
                }
                for pid in children {
                    let Ok(actual) = fs::read(format!("/proc/{pid}/cmdline")) else {
                        continue;
                    };
                    if actual != expected {
                        continue;
                    }
                    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
                    assert!(
                        status
                            .lines()
                            .any(|l| l == format!("PPid:\t{}", std::process::id()))
                    );
                    assert!(status.lines().any(|l| l.starts_with("Uid:")
                        && l.split_whitespace().skip(1).all(|v| v == "11050")));
                    assert!(
                        status
                            .lines()
                            .any(|l| l.starts_with("Groups:") && l.split_whitespace().count() == 1)
                    );
                    let alive = rustix::process::pidfd_open(
                        Pid::from_raw(pid as i32).unwrap(),
                        PidfdFlags::NONBLOCK,
                    )
                    .unwrap();
                    assert!(crate::authority::living(&alive).unwrap());
                    return App {
                        pid,
                        alive,
                        control: self.app_control.clone(),
                        sdk_stop: None,
                        next_sequence: 0,
                    };
                }
            }
            pause();
        }
        panic!("setup: actual UID11050 native control socket");
    }
    fn command(&self, app: &mut App, op: &str, connected: Option<bool>, end: u64) -> Value {
        assert!(boottime_ns() < end && end <= self.case);
        let context = CommandContext {
            operation: op,
            expected_sequence: app.next_sequence.saturating_add(1),
            original_until_ns: end,
            started_at_ns: boottime_ns(),
        };
        let fd = self.command_result(app, &context, "connect", ipc::connect(&app.control));
        let cred = self.command_result(
            app,
            &context,
            "peer_credentials",
            socket_peercred(&fd).map_err(io::Error::from),
        );
        assert_eq!(cred.pid.as_raw_nonzero().get() as u32, app.pid);
        assert_eq!(cred.uid.as_raw(), 11050);
        assert_eq!(cred.gid.as_raw(), 11050);
        let mut packet = json!({"op":op,"until_ns":end.to_string()});
        if let Some(v) = connected {
            packet["connected"] = json!(v);
        }
        self.command_result(
            app,
            &context,
            "send",
            ipc::send(&fd, &serde_json::to_vec(&packet).unwrap()),
        );
        while boottime_ns() < end {
            if let Some((bytes, c)) =
                self.command_result(app, &context, "receive", ipc::receive(&fd))
            {
                assert_eq!(c.pid, cred.pid);
                assert_eq!(c.uid, cred.uid);
                assert_eq!(c.gid, cred.gid);
                let row: Value = serde_json::from_slice(&bytes).unwrap();
                assert!(boottime_ns() < end);
                assert_eq!(row["actual_caps"]["NoNewPrivs"], "1");
                for key in ["CapEff", "CapPrm", "CapInh", "CapAmb"] {
                    assert_eq!(
                        u64::from_str_radix(row["actual_caps"][key].as_str().unwrap(), 16).unwrap(),
                        0
                    );
                }
                assert_eq!(
                    u64::from_str_radix(row["actual_caps"]["CapBnd"].as_str().unwrap(), 16)
                        .unwrap(),
                    (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8)
                );
                assert_eq!(row["pid"], app.pid);
                assert_eq!(row["uid"], 11050);
                app.next_sequence += 1;
                assert_eq!(row["sequence"], app.next_sequence);
                assert!(
                    row["sdk_call_returned_ns"].as_u64().unwrap() < end
                        && row["completed_ns"].as_u64().unwrap() < end
                );
                assert_eq!(row["original_until_ns"], end);
                assert_eq!(row["operation"], op);
                assert!(
                    row["sdk_call_before_ns"].as_u64().unwrap()
                        <= row["sdk_call_returned_ns"].as_u64().unwrap()
                );
                if op == "set" {
                    assert_eq!(row["choice_connected"], connected.unwrap());
                    assert!(row["choice_accepted"].is_boolean());
                } else {
                    assert!(row["choice_accepted"].is_null());
                    assert!(row["choice_connected"].is_null());
                }
                return row;
            }
            pause();
        }
        self.command_failure(
            app,
            &context,
            "receive_deadline",
            &io::ErrorKind::TimedOut.into(),
        );
        panic!("original native app control deadline expired");
    }
    fn command_result<T>(
        &self,
        app: &App,
        context: &CommandContext<'_>,
        stage: &str,
        result: io::Result<T>,
    ) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                self.command_failure(app, context, stage, &error);
                panic!("called `Result::unwrap()` on an `Err` value: {error:?}");
            }
        }
    }
    fn command_failure(
        &self,
        app: &App,
        context: &CommandContext<'_>,
        stage: &str,
        error: &io::Error,
    ) {
        let failed_at = boottime_ns();
        let mut diagnostics = self.diagnostics.borrow_mut();
        if diagnostics.get("command_failure").is_some() {
            return;
        }
        self.failed_app.set(Some(app.pid));
        let value = json!({"schema":1,"operation":context.operation,"app_pid":app.pid,
            "expected_sequence":context.expected_sequence,"original_until_ns":context.original_until_ns,
            "started_at_ns":context.started_at_ns,"failed_at_ns":failed_at,"ipc_stage":stage,
            "error_kind":format!("{:?}",error.kind()),"raw_os_error":error.raw_os_error(),
            "app_error_before_cleanup":app_error_report(&app.control,app.pid)});
        diagnostic_once(&self.diagnostic_root, "actor-command-failure.json", &value);
        diagnostics["command_failure"] = value;
    }
    fn failure_diagnostics(&self) -> Value {
        let mut value = self.diagnostics.borrow().clone();
        if let Some(pid) = self.failed_app.get() {
            value["app_error_after_existing_wait"] = if let Some((_, status)) =
                self.reaped.iter().find(|(waited, _)| *waited == pid)
            {
                json!({"existing_Apps_wait":true,"app_pid":pid,"code":status.code(),"signal":status.signal(),
                    "report":app_error_report(&self.app_control,pid)})
            } else {
                json!({"existing_Apps_wait":false,"app_pid":pid,"read_attempted":false})
            };
        }
        value
    }
    fn state(&self, app: &mut App, configured: bool, can: bool, end: u64) -> Value {
        while boottime_ns() < end {
            let row = self.command(app, "status", None, end);
            let v = &row["sdk_view"];
            let now = boottime_ns();
            if let (Some(at), Some(expires)) =
                (v["observed_at_ns"].as_u64(), v["until_ns"].as_u64())
            {
                if v["transport_connected"] == true
                    && v["available"] == true
                    && v["pending"] == false
                    && v["configured"] == configured
                    && v["can_control"] == can
                    && at > 0
                    && at <= now
                    && now < expires
                    && at.checked_add(250_000_000) == Some(expires)
                    && matches!(v["status"].as_str(), Some("ready" | "controls_unavailable"))
                {
                    return row;
                }
            }
            // Readonly getter/control observations only, all under the SAME end.
            // An uncertain outcome cannot qualify completion via later readback.
            pause();
        }
        panic!("original fresh normal SDK getter deadline expired");
    }
    fn unavailable_after(&self, app: &mut App, after: u64, end: u64) -> Value {
        assert!(after > 0 && after <= boottime_ns() && boottime_ns() < end);
        let entered_at = boottime_ns();
        let mut attempts = 0_u64;
        let mut first = None;
        let mut last = None;
        while boottime_ns() < end {
            let row = self.command(app, "status", None, end);
            attempts += 1;
            if first.is_none() {
                first = Some(row.clone());
            }
            let view = &row["sdk_view"];
            if row["sdk_call_before_ns"].as_u64().unwrap() >= after
                && view["available"] == false
                && view["configured"].is_null()
                && view["pending"] == false
                && view["can_control"] == false
                && view["observed_at_ns"] == 0
                && view["until_ns"] == 0
                && view["status"] == "unavailable"
            {
                return row;
            }
            last = Some(row);
            // Fresh read-only SDK getters share the frozen loss cutoff. No
            // choice, Disconnect, replacement or action retry is submitted.
            pause();
        }
        self.failed_app.set(Some(app.pid));
        let value = json!({"schema":1,"app_pid":app.pid,"after_loss_ns":after,
            "original_until_ns":end,"entered_at_ns":entered_at,"failed_at_ns":boottime_ns(),
            "attempt_count":attempts,"first_predicate_matches":first.as_ref().map(|row|unavailable_matches(row,after)),
            "last_predicate_matches":last.as_ref().map(|row|unavailable_matches(row,after)),
            "first_sdk_row":first,"last_sdk_row":last});
        diagnostic_once(
            &self.diagnostic_root,
            "actor-unavailable-timeout.json",
            &value,
        );
        self.diagnostics.borrow_mut()["unavailable_timeout"] = value;
        panic!("original AP-loss cutoff: actual SDK did not become unavailable");
    }
    fn collect_waits(&mut self) {
        self.reaped.extend(self.app_waits.try_iter());
    }
    fn exit_app(&mut self, app: &mut App) {
        let end = until(self.case);
        let exited = self.command(app, "exit", None, end);
        let stop = &exited["sdk_stop"];
        assert_eq!(stop["stop_requested"], true);
        assert_eq!(stop["joined"], true);
        assert_eq!(stop["actual_task_count_after_join"], 1);
        assert_eq!(stop["join_status"], 0);
        assert!(
            stop["stop_call_before_ns"].as_u64().unwrap() <= stop["joined_at_ns"].as_u64().unwrap()
        );
        assert!(
            stop["joined_at_ns"].as_u64().unwrap()
                < stop["fixture_stop_cutoff_ns"].as_u64().unwrap()
        );
        assert!(stop["fixture_stop_cutoff_ns"].as_u64().unwrap() <= end);
        app.sdk_stop = Some(stop.clone());
        while boottime_ns() < end {
            self.collect_waits();
            if self.reaped.iter().any(|(pid, _)| *pid == app.pid) {
                let status = &self
                    .reaped
                    .iter()
                    .find(|(pid, _)| *pid == app.pid)
                    .unwrap()
                    .1;
                assert!(status.success());
                assert!(!crate::authority::living(&app.alive).unwrap());
                assert!(!Path::new(&format!("/proc/{}", app.pid)).exists());
                assert!(!self.app_control.exists());
                return;
            }
            pause();
        }
        panic!("original deadline: normal Apps actual child wait missing");
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        let fd = ipc::connect(&self.runtime.join("admin.sock")).unwrap();
        let reply: Reply =
            ipc::call_until_from(fd, &json!({"op":"stop"}), self.close, None).unwrap();
        assert_eq!(reply, Reply::Ok);
        while boottime_ns() < self.close {
            match self.done.try_recv() {
                Ok(result) => {
                    result.unwrap();
                    self.actor.take().unwrap().join().unwrap();
                    self.collect_waits();
                    self.closed = true;
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => pause(),
                Err(e) => panic!("actor close channel {e}"),
            }
        }
        std::process::abort();
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.closed {
            self.close();
        }
    }
}
fn nonce(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0_u8; 37];
    bytes[..4].copy_from_slice(b"PKL1");
    bytes[4] = tag;
    File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut bytes[5..])
        .unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn roundtrip(root: &Path, bytes: &[u8], case: u64) -> Value {
    let end = until(case);
    let tag = bytes[4];
    publish(
        root,
        &format!("nonce-{tag}-request.json"),
        &json!({"tag":tag,"nonce_hex":hex(bytes),"requested_at_ns":boot_time_ns(),"original_until_ns":end}),
    );
    let value = gate(root, &format!("nonce-{tag}-complete.json"), end);
    assert_eq!(value["passed"], true);
    assert_eq!(value["payload_hex"], hex(bytes));
    assert_eq!(value["original_until_ns"], end);
    assert!(value["actual_cap0_client_wait_EOF"].as_bool().unwrap());
    json!({"nonce_hex":hex(bytes),"cap0_client":value,"original_until_ns":end})
}

fn quiet(root: &Path, case: u64, expected_packets: usize) -> Value {
    let loss_nonce = nonce(3);
    let quiet_until = boottime_ns().checked_add(800_000_000).unwrap();
    assert!(quiet_until < case);
    publish(
        root,
        "quiet-request.json",
        &json!({"original_until_ns":quiet_until,"probe_hex":hex(&loss_nonce),"requested_at_ns":boottime_ns(),"expected_packets":expected_packets}),
    );
    let armed = gate(root, "peer-quiet-armed.json", quiet_until);
    assert_eq!(armed["original_until_ns"], quiet_until);
    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let sent_at = boottime_ns();
    assert!(sent_at < quiet_until);
    let sent = udp.send_to(&loss_nonce, (PEER, PORT));
    assert!(
        matches!(&sent,Err(e) if e.raw_os_error()==Some(101)),
        "fresh disconnected loss socket must return exact ENETUNREACH101: {sent:?}"
    );
    udp.set_nonblocking(true).unwrap();
    let mut buf = [0_u8; 128];
    while boottime_ns() < quiet_until {
        match udp.recv_from(&mut buf) {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            other => panic!("disconnected socket not quiet: {other:?}"),
        }
        pause();
    }
    let result = gate(root, "peer-quiet.json", case);
    assert_eq!(result["received_datagrams"], 0);
    assert_eq!(result["quiet_end_actual_EAGAIN"], true);
    assert_eq!(result["original_until_ns"], quiet_until);
    assert!(result["armed_at_ns"].as_u64().unwrap() <= sent_at);
    assert!(result["quiet_end_EAGAIN_at_ns"].as_u64().unwrap() >= quiet_until);
    json!({"peer":result,"loss_send":format!("{sent:?}"),"sent_at_ns":sent_at})
}
fn quiet_without_probe(root: &Path, case: u64) -> Value {
    let requested_at = boottime_ns();
    let quiet_until = requested_at.checked_add(800_000_000).unwrap();
    assert!(quiet_until < case);
    publish(
        root,
        "quiet-request.json",
        &json!({"original_until_ns":quiet_until,"requested_at_ns":requested_at,"expected_packets":2}),
    );
    let armed = gate(root, "peer-quiet-armed.json", quiet_until);
    assert_eq!(armed["original_until_ns"], quiet_until);
    let armed_at = armed["armed_at_ns"].as_u64().unwrap();
    assert!(requested_at <= armed_at && armed_at < quiet_until);
    while boottime_ns() < quiet_until {
        pause();
    }
    let result = gate(root, "peer-quiet.json", case);
    assert_eq!(result["received_datagrams"], 0);
    assert_eq!(result["quiet_end_actual_EAGAIN"], true);
    assert_eq!(result["original_until_ns"], quiet_until);
    assert_eq!(result["armed_at_ns"], armed_at);
    assert!(result["quiet_end_EAGAIN_at_ns"].as_u64().unwrap() >= quiet_until);
    json!({"peer":result,"requested_at_ns":requested_at,"no_post_loss_probe_submitted":true})
}
fn independent(value: &Value, configured: bool, index: u64) {
    assert_eq!(value["ifindex"], index);
    assert_eq!(value["default_route_absent"], true);
    let expected = if configured {
        json!([{"ifindex":index,"local":"192.0.2.1","prefix":30}])
    } else {
        json!([])
    };
    assert_eq!(value["ipv4"], expected);
}
fn desired(c: &mut Case, root: &Path, scenario: &str) -> Value {
    let case = c.case;
    let ready = gate(root, "ready.json", case);
    assert_eq!(ready["scenario"], scenario);
    assert_eq!(ready["provisioner_reaped_before_packet_peer"], true);
    let index = ready["initial_link_ifindex"].as_u64().unwrap();
    assert!(index > 1);
    let mut first = c.launch();
    if ready["startup_profile_rejected_candidate"] == true {
        let end = until(case);
        let unavailable = c.command(&mut first, "status", None, end);
        assert_eq!(unavailable["sdk_view"]["available"], false);
        assert_eq!(unavailable["sdk_view"]["configured"], Value::Null);
        assert_eq!(unavailable["sdk_view"]["pending"], false);
        c.exit_app(&mut first);
        return json!({"functional_green":false,"genuine_refused_candidate":false,
            "startup_profile_rejected_candidate":true,"actual_SDK_unavailable":unavailable,
            "first_app_pid":first.pid,"first_sdk_stop_join":first.sdk_stop,
            "functional_finished_at_ns":boottime_ns()});
    }
    let initial = c.state(&mut first, false, false, until(case));
    publish(root, "initial-proof.json", &initial);
    independent(&gate(root, "initial-readback.json", case), false, index);
    let denied = c.command(&mut first, "set", Some(true), until(case));
    assert_eq!(denied["choice_accepted"], false);
    publish(root, "no-grant-proof.json", &denied);
    independent(&gate(root, "no-grant-readback.json", case), false, index);
    assert_eq!(
        c.admin(json!({"op":"locked","locked":false}), until(case)),
        Reply::Ok
    );
    assert_eq!(
        c.admin(
            json!({"op":"background","app":APP,"background":false}),
            until(case)
        ),
        Reply::Ok
    );
    assert_eq!(
        c.admin(
            json!({"op":"grant","app":APP,"permission":"network_control","allowed":true}),
            until(case)
        ),
        Reply::Ok
    );
    let durable: Value =
        serde_json::from_slice(&fs::read(c.policy.join("policy.json")).unwrap()).unwrap();
    assert!(
        durable["grants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["app"] == APP
                && v["permission"] == "network_control"
                && v["allowed"] == true)
    );
    let granted = c.state(&mut first, false, true, until(case));
    let connect_until = until(case);
    publish(
        root,
        "connect-attempt.json",
        &json!({"attempted_at_ns":boottime_ns(),"original_until_ns":connect_until,"granted_status":granted}),
    );
    let connected = c.command(&mut first, "set", Some(true), connect_until);
    publish(
        root,
        "connect-return.json",
        &json!({"choice_accepted":connected["choice_accepted"],"returned_at_ns":connected["sdk_call_returned_ns"],"actual_app_control":connected}),
    );
    if connected["choice_accepted"] == false {
        let after = c.state(&mut first, false, true, until(case));
        publish(root, "refused-proof.json", &after);
        let readback = gate(root, "refused-readback.json", case);
        independent(&readback, false, index);
        let silent = quiet(root, case, 0);
        c.exit_app(&mut first);
        return json!({"functional_green":false,"genuine_refused_candidate":true,"configured_positive":false,"no_grant_refused_by_SDK":denied,
            "granted_status":granted,"returned":connected,"refused_readback":readback,"quiet":silent,
            "first_app_pid":first.pid,"first_sdk_stop_join":first.sdk_stop,"functional_finished_at_ns":boottime_ns()});
    }
    assert_eq!(connected["choice_accepted"], true);
    let connected_observation = c.state(&mut first, true, true, connect_until);
    assert!(
        connected_observation["sdk_view"]["observed_at_ns"]
            .as_u64()
            .unwrap()
            > connected["sdk_call_before_ns"].as_u64().unwrap()
    );
    publish(root, "connected-proof.json", &connected_observation);
    let readback = gate(root, "connected-readback.json", case);
    independent(&readback, true, index);
    let first_packet = roundtrip(root, &nonce(1), case);
    c.exit_app(&mut first);
    let first_waited = boottime_ns();
    publish(
        root,
        "first-requester-closed.json",
        &json!({"pid":first.pid,
        "actual_Apps_wait_observed_at_ns":first_waited,"SDK_stop_join":first.sdk_stop}),
    );
    let retained = gate(root, "persistent-after-first-wait.json", case);
    assert_eq!(retained["same_worker_radio_profile_confirmed"], true);
    assert!(retained["observed_at_ns"].as_u64().unwrap() > first_waited);
    let mut second = c.launch();
    assert_ne!(first.pid, second.pid);
    let first_second_status = c.state(&mut second, true, true, until(case));
    let mut idle = vec![];
    let idle_until = until(case);
    let mut last = first_waited;
    while boottime_ns() < idle_until {
        let row = c.state(&mut second, true, true, idle_until);
        let at = row["sdk_view"]["observed_at_ns"].as_u64().unwrap();
        if at > last {
            last = at;
            idle.push(row);
            if idle.len() == 2 {
                break;
            }
        }
        pause();
    }
    assert_eq!(
        idle.len(),
        2,
        "two fresh background observations after actual first app wait"
    );
    let background_packet = roundtrip(root, &nonce(2), case);
    publish(
        root,
        "before-disconnect.json",
        &json!({"second_app_pid":second.pid,
        "first_app_actual_wait_observed_at_ns":first_waited,
        "fresh_idle_observations":idle,"requested_at_ns":boottime_ns()}),
    );
    if scenario == "ap_loss" {
        let intent = gate(root, "loss-intent.json", until(case));
        assert_eq!(intent["scenario"], scenario);
        assert_eq!(intent["original_worker_identity"], retained["worker"]);
        let loss_requested_at = intent["loss_requested_at_ns"].as_u64().unwrap();
        let loss_until = intent["original_until_ns"].as_u64().unwrap();
        assert!(loss_requested_at > 0 && loss_requested_at <= boottime_ns());
        assert!(loss_requested_at < loss_until && loss_until <= case);
        assert!(loss_until - loss_requested_at <= 1_000_000_000);
        let lost = gate(root, "loss-ready.json", loss_until);
        assert_eq!(lost["original_until_ns"], loss_until);
        assert_eq!(lost["loss_requested_at_ns"], loss_requested_at);
        for key in [
            "independent_radio_loss_confirmed",
            "profile_retained",
            "same_target_identity",
            "IFF_UP",
            "worker_pidfd_terminal",
            "no_replacement",
            "Main_unavailable",
            "terminal_uncertain",
        ] {
            assert_eq!(lost[key], true, "actual AP-loss prerequisite: {key}");
        }
        assert!(lost["independent_loss_proof"].is_object());
        let worker_id = lost["original_worker_identity"].as_str().unwrap();
        assert!(worker_id.len() == 32 && worker_id.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(worker_id, retained["worker"].as_str().unwrap());
        let observed_at = lost["loss_observed_at_ns"].as_u64().unwrap();
        assert!(loss_requested_at <= observed_at && observed_at < loss_until);
        let main = &lost["Main_observation"];
        assert_eq!(main["status"], "unavailable");
        assert_eq!(main["state"]["available"], false);
        assert!(main["state"]["configured"].is_null());
        assert_eq!(main["state"]["pending"], false);
        assert_eq!(main["state"]["terminal_uncertain"], true);
        assert_eq!(main["state"]["observed_at_ns"], "0");
        assert_eq!(main["state"]["until_ns"], "0");
        assert_eq!(main["state"]["worker"], lost["original_worker_identity"]);
        let unavailable = c.unavailable_after(&mut second, observed_at, loss_until);
        publish(
            root,
            "loss-SDK-unavailable.json",
            &json!({
            "original_until_ns":loss_until,"loss_observed_at_ns":observed_at,
            "actual_SDK_unavailable":unavailable,"observed_at_ns":boottime_ns()}),
        );
        let silent = quiet_without_probe(root, case);
        c.exit_app(&mut second);
        let second_waited = boottime_ns();
        return json!({"functional_green":false,"functional_ap_loss_green":true,
            "genuine_refused_candidate":false,"configured_positive":true,
            "no_grant_refused_by_SDK":denied,"granted_status":granted,
            "connected_choice":connected,"connected_observation":connected_observation,
            "first_packet":first_packet,"background_packet_after_withdraw":background_packet,
            "first_app_pid":first.pid,"second_app_pid":second.pid,
            "first_sdk_stop_join":first.sdk_stop,"second_sdk_stop_join":second.sdk_stop,
            "first_app_actual_wait_observed_at_ns":first_waited,
            "second_app_actual_wait_observed_at_ns":second_waited,
            "fresh_app_status":first_second_status,"fresh_idle_observations":idle,
            "loss_intent":intent,"loss_ready":lost,"actual_SDK_unavailable":unavailable,
            "loss_requested_at_ns":loss_requested_at,"loss_observed_at_ns":observed_at,
            "original_loss_until_ns":loss_until,"post_loss_Set_commands":0,
            "Disconnect_commands_admitted":0,"peer_quiet":silent,
            "functional_finished_at_ns":boottime_ns()});
    }
    assert_eq!(
        gate(root, "disconnect-ready.json", case)["same_worker_radio_profile_confirmed"],
        true
    );
    let disconnect_until = until(case);
    publish(
        root,
        "disconnect-attempt.json",
        &json!({"attempted_at_ns":boottime_ns(),
        "original_until_ns":disconnect_until,"second_app_pid":second.pid}),
    );
    let disconnected = c.command(&mut second, "set", Some(false), disconnect_until);
    assert_eq!(disconnected["choice_accepted"], true);
    let disconnected_observation = c.state(&mut second, false, true, disconnect_until);
    assert!(
        disconnected_observation["sdk_view"]["observed_at_ns"]
            .as_u64()
            .unwrap()
            > disconnected["sdk_call_before_ns"].as_u64().unwrap()
    );
    publish(root, "disconnected-proof.json", &disconnected_observation);
    let readback = gate(root, "disconnected-readback.json", case);
    independent(&readback, false, index);
    let silent = quiet(root, case, 2);
    c.exit_app(&mut second);
    json!({"functional_green":true,"genuine_refused_candidate":false,"configured_positive":true,"no_grant_refused_by_SDK":denied,"granted_status":granted,
        "connected_choice":connected,"connected_observation":connected_observation,"disconnected_choice":disconnected,"disconnected_observation":disconnected_observation,"first_packet":first_packet,"background_packet_after_withdraw":background_packet,
        "first_app_pid":first.pid,"second_app_pid":second.pid,"first_sdk_stop_join":first.sdk_stop,"second_sdk_stop_join":second.sdk_stop,
        "first_app_actual_wait_observed_at_ns":first_waited,"fresh_app_status":first_second_status,"fresh_idle_observations":idle,
        "peer_quiet":silent,"functional_finished_at_ns":boottime_ns()})
}
#[test]
#[ignore = "requires explicitly selected isolated actual Main and built lifecycle/app helpers"]
fn settings_sdk_wireless_packet_lifecycle() {
    assert_eq!(
        env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),
        Ok("1")
    );
    let scenario = env::var("CONNECTIVITY_WIRELESS_SCENARIO")
        .expect("selected explicit persistent or ap_loss scenario");
    assert!(matches!(scenario.as_str(), "persistent" | "ap_loss"));
    let root = PathBuf::from(env::var_os("CONNECTIVITY_CASE_GATES").unwrap());
    let endpoint = PathBuf::from(env::var_os("CONNECTIVITY_ACTOR_ENDPOINT").unwrap());
    let case = env::var("CONNECTIVITY_CASE_UNTIL_NS")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let close = env::var("CONNECTIVITY_CASE_CLOSE_UNTIL_NS")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let start = env::var("CONNECTIVITY_START_UNTIL_NS")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let main_pid = env::var("CONNECTIVITY_MAIN_PID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(boottime_ns() < start && start <= case && case < close);
    let mut c = Case::new(&endpoint, case, close, &root);
    publish(
        &root,
        "created.json",
        &json!({"driver_pid":std::process::id(),"main_pid":main_pid,"authored_session_none":true,"scenario":scenario}),
    );
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        c.ready(start);
        desired(&mut c, &root, &scenario)
    }));
    let mut result = match result {
        Ok(v) => v,
        Err(_) => {
            json!({"functional_green":false,"functional_ap_loss_green":false,"genuine_refused_candidate":false,"setup_error":"consumer panic; see actual libtest stderr","functional_finished_at_ns":boottime_ns()})
        }
    };
    result["scenario"] = json!(scenario);
    c.close();
    if result.get("setup_error").is_some() {
        // Operational gate files are not exported by the enclosing VM owner.
        // Carry error-only evidence through its existing functional receipt.
        result["diagnostics"] = c.failure_diagnostics();
    }
    let waits:Vec<_>=c.reaped.iter().map(|(pid,s)|json!({"pid":pid,"code":s.code(),"signal":s.signal(),"witness":"existing Apps Child::try_wait Some observer","PIPE_EOF_claim":false})).collect();
    assert!(!waits.is_empty());
    publish(
        &root,
        "actor-closed.json",
        &json!({"actor_stopped_and_joined":true,"actual_Apps_waits":waits,"completed_at_ns":boottime_ns(),"app_stdout_stderr":"null"}),
    );
    publish(&root, "functional-finished.json", &result);
    let cleanup = gate(&root, "cleanup-complete.json", close);
    assert_eq!(cleanup["main_worker_packet_actual_wait_EOF_closed"], true);
    assert_eq!(cleanup["all_namespace_handles_retained"], true);
    publish(
        &root,
        "driver-boundary-after-service-cleanup.json",
        &json!({"reported_after_service_cleanup_at_ns":boottime_ns(),"cleanup_completed_at_ns":cleanup["completed_at_ns"],
        "genuine_refused_candidate":result["genuine_refused_candidate"],"functional_green":result["functional_green"],
        "functional_ap_loss_green":result["functional_ap_loss_green"],"scenario":scenario,"original_close_until_ns":close}),
    );
    // The enclosing hwsim owner makes the sole desired assertion only after
    // AP/radio/namespace teardown. This test reports its actual boundary result.
}
