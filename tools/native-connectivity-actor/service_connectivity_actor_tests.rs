//! Actual Settings UID11050 → lifecycle actor → real connectivity Owner/Main.
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
    env,
    fs::{self, File},
    io::{self, Read},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
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
const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
const PEER: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 2);
const PORT: u16 = 36042;
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
struct App {
    pid: u32,
    alive: OwnedFd,
    control: PathBuf,
    actor_fd: Option<i64>,
    next_sequence: u64,
}
struct Case {
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
    fn new(socket: &Path, case: u64, close: u64) -> Self {
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
                &json!({"apps":[{"app":APP,"uid":11050,"executable":executable,"args":args}]}),
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
                        actor_fd: None,
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
        let fd = ipc::connect(&app.control).unwrap();
        let cred = socket_peercred(&fd).unwrap();
        assert_eq!(cred.pid.as_raw_nonzero().get() as u32, app.pid);
        assert_eq!(cred.uid.as_raw(), 11050);
        assert_eq!(cred.gid.as_raw(), 11050);
        let mut packet = json!({"op":op,"until_ns":end.to_string()});
        if let Some(v) = connected {
            packet["connected"] = json!(v);
        }
        ipc::send(&fd, &serde_json::to_vec(&packet).unwrap()).unwrap();
        while boottime_ns() < end {
            if let Some((bytes, c)) = ipc::receive(&fd).unwrap() {
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
                assert_eq!(row["actor_peer_pid"], std::process::id());
                let raw = row["actor_fd"].as_i64().unwrap();
                if let Some(old) = app.actor_fd {
                    assert_eq!(raw, old);
                } else {
                    app.actor_fd = Some(raw);
                }
                app.next_sequence += 1;
                assert_eq!(row["sequence"], app.next_sequence);
                assert!(
                    row["reply_ns"].as_u64().unwrap() < end
                        && row["completed_ns"].as_u64().unwrap() < end
                );
                assert_eq!(row["original_until_ns"], end);
                if op == "set" {
                    let actual: Value =
                        serde_json::from_str(row["actual_packet"].as_str().unwrap()).unwrap();
                    assert_eq!(
                        actual,
                        json!({"op":"connectivity_set","request":{"connected":connected.unwrap(),"until_ns":end.to_string()}})
                    );
                }
                return row;
            }
            pause();
        }
        panic!("original native app control deadline expired");
    }
    fn state(&self, app: &mut App, configured: bool, can: bool, end: u64) -> Value {
        while boottime_ns() < end {
            let row = self.command(app, "status", None, end);
            match serde_json::from_value::<Reply>(row["reply"].clone()).unwrap() {
                Reply::ConnectivityState { state, can_control }
                    if state.is_current(boottime_ns()) && !state.pending =>
                {
                    assert_eq!(state.configured, Some(configured));
                    assert_eq!(can_control, can);
                    return row;
                }
                Reply::ConnectivityState { .. } | Reply::Unavailable => pause(),
                other => panic!("setup: registered app status rejected {other:?}"),
            }
        }
        panic!("original fresh status deadline expired");
    }
    fn collect_waits(&mut self) {
        self.reaped.extend(self.app_waits.try_iter());
    }
    fn exit_app(&mut self, app: &mut App) {
        let end = until(self.case);
        self.command(app, "exit", None, end);
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
fn roundtrip(bytes: &[u8], case: u64) -> Value {
    let end = until(case);
    let udp = UdpSocket::bind((LOCAL, 0)).unwrap();
    let sampled = boot_time_ns();
    assert!(sampled < end);
    udp.set_read_timeout(Some(Duration::from_nanos(end - sampled)))
        .unwrap();
    let source = udp.local_addr().unwrap();
    let target = SocketAddr::from((PEER, PORT));
    let before = boot_time_ns();
    assert_eq!(
        udp.send_to(bytes, target).unwrap(),
        bytes.len(),
        "one nonce, no retry"
    );
    let mut answer = [0_u8; 128];
    let (length, from) = udp.recv_from(&mut answer).unwrap();
    assert_eq!(from, target);
    assert_eq!(&answer[..4], b"ACK1");
    assert_eq!(&answer[4..length], bytes);
    assert!(boot_time_ns() < end);
    json!({"nonce_hex":hex(bytes),"source":source.to_string(),"peer":target.to_string(),
        "send_before_ns":before,"received_at_ns":boot_time_ns(),"received_bytes":length,"original_until_ns":end})
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
fn desired(c: &mut Case, root: &Path) -> Value {
    let case = c.case;
    let ready = gate(root, "ready.json", case);
    assert_eq!(ready["provisioner_reaped_before_packet_peer"], true);
    let index = ready["initial_link_ifindex"].as_u64().unwrap();
    assert!(index > 1);
    let mut first = c.launch();
    let initial = c.state(&mut first, false, false, until(case));
    publish(root, "initial-proof.json", &initial);
    independent(&gate(root, "initial-readback.json", case), false, index);
    let denied = c.command(&mut first, "set", Some(true), until(case));
    assert_eq!(denied["reply"]["result"], "denied");
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
        &json!({"reply":connected["reply"],"returned_at_ns":connected["reply_ns"],"actual_app_control":connected}),
    );
    if connected["reply"]["result"] == "denied" {
        let after = c.state(&mut first, false, true, until(case));
        publish(root, "refused-proof.json", &after);
        let readback = gate(root, "refused-readback.json", case);
        independent(&readback, false, index);
        let silent = quiet(root, case, 0);
        c.exit_app(&mut first);
        return json!({"functional_green":false,"genuine_refused_candidate":true,"configured_positive":false,"no_grant_denied":denied,
            "granted_status":granted,"returned":connected,"refused_readback":readback,"quiet":silent,
            "first_app_pid":first.pid,"first_actor_fd":first.actor_fd,"functional_finished_at_ns":boottime_ns()});
    }
    match serde_json::from_value::<Reply>(connected["reply"].clone()).unwrap() {
        Reply::ConnectivityCompleted { state } => {
            assert!(state.is_current(boottime_ns()));
            assert_eq!(state.configured, Some(true));
        }
        other => panic!("unexpected Connect result {other:?}"),
    }
    publish(root, "connected-proof.json", &connected);
    let readback = gate(root, "connected-readback.json", case);
    independent(&readback, true, index);
    let first_packet = roundtrip(&nonce(1), case);
    c.exit_app(&mut first);
    let first_waited = boottime_ns();
    let mut second = c.launch();
    assert_ne!(first.pid, second.pid);
    let first_second_status = c.state(&mut second, true, true, until(case));
    let mut idle = vec![];
    let idle_until = until(case);
    let mut last = first_waited;
    while boottime_ns() < idle_until {
        let row = c.state(&mut second, true, true, idle_until);
        let at = row["reply"]["state"]["observed_at_ns"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
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
    let background_packet = roundtrip(&nonce(2), case);
    let disconnected = c.command(&mut second, "set", Some(false), until(case));
    match serde_json::from_value::<Reply>(disconnected["reply"].clone()).unwrap() {
        Reply::ConnectivityCompleted { state } => {
            assert!(state.is_current(boottime_ns()));
            assert_eq!(state.configured, Some(false));
        }
        other => panic!("unexpected Disconnect result {other:?}"),
    }
    publish(root, "disconnected-proof.json", &disconnected);
    let readback = gate(root, "disconnected-readback.json", case);
    independent(&readback, false, index);
    let silent = quiet(root, case, 2);
    c.exit_app(&mut second);
    json!({"functional_green":true,"genuine_refused_candidate":false,"configured_positive":true,"no_grant_denied":denied,"granted_status":granted,
        "connected":connected,"disconnected":disconnected,"first_packet":first_packet,"background_packet_after_withdraw":background_packet,
        "first_app_pid":first.pid,"second_app_pid":second.pid,"first_actor_fd":first.actor_fd,"second_actor_fd":second.actor_fd,
        "first_app_actual_wait_observed_at_ns":first_waited,"fresh_app_status":first_second_status,"fresh_idle_observations":idle,
        "peer_quiet":silent,"functional_finished_at_ns":boottime_ns()})
}
#[test]
#[ignore = "requires explicitly selected isolated actual Main and built lifecycle/app helpers"]
fn settings_actual_connect_packet_lifecycle() {
    assert_eq!(
        env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),
        Ok("1")
    );
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
    let mut c = Case::new(&endpoint, case, close);
    publish(
        &root,
        "created.json",
        &json!({"driver_pid":std::process::id(),"main_pid":main_pid,"authored_session_none":true}),
    );
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        c.ready(start);
        desired(&mut c, &root)
    }));
    let result = match result {
        Ok(v) => v,
        Err(_) => {
            json!({"functional_green":false,"genuine_refused_candidate":false,"setup_error":"consumer panic; see actual libtest stderr","functional_finished_at_ns":boottime_ns()})
        }
    };
    c.close();
    let waits:Vec<_>=c.reaped.iter().map(|(pid,s)|json!({"pid":pid,"code":s.code(),"signal":s.signal(),"witness":"existing Apps Child::try_wait Some observer","PIPE_EOF_claim":false})).collect();
    assert!(!waits.is_empty());
    publish(
        &root,
        "actor-closed.json",
        &json!({"actor_stopped_and_joined":true,"actual_Apps_waits":waits,"completed_at_ns":boottime_ns(),"app_stdout_stderr":"null"}),
    );
    publish(&root, "functional-finished.json", &result);
    let cleanup = gate(&root, "cleanup-complete.json", close);
    assert_eq!(
        cleanup["main_worker_packet_keeper_actual_wait_EOF_closed"],
        true
    );
    assert_eq!(cleanup["all_namespace_handles_retained"], true);
    publish(
        &root,
        "desired-assertion.json",
        &json!({"asserted_after_cleanup_at_ns":boottime_ns(),"cleanup_completed_at_ns":cleanup["completed_at_ns"],
        "genuine_refused_candidate":result["genuine_refused_candidate"],"functional_green":result["functional_green"],"original_close_until_ns":close}),
    );
    if result["genuine_refused_candidate"] == true {
        assert!(
            result["configured_positive"] == true,
            "REGISTERED_GRANTED_SETTINGS_CONNECT_REQUIRES_REAL_OBSERVED_TRUE"
        );
    }
    assert!(
        result["functional_green"] == true,
        "actual actor consumer failed outside desired Connect assertion: {result}"
    );
}
