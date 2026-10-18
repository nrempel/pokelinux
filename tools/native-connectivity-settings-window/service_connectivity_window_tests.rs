//! Authored root authority + real ordinary Settings/Wayland Factory + packet case.
//! No Source, authentication, installed session-owner Main or production app hooks.
use super::{ActorControl, GuiReply};
use crate::{Reply, boottime_ns, connectivity, fresh_id, ipc};
use rustix::{
    fd::OwnedFd,
    process::{Pid, PidfdFlags},
};
use serde_json::{Value, json};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::{
        fs::{FileTypeExt, PermissionsExt},
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
fn boot_time_ns() -> u64 {
    boottime_ns()
}
fn publish(root: &Path, name: &str, value: &Value) {
    let tmp = root.join(format!("{name}.tmp"));
    fs::write(&tmp, serde_json::to_vec(value).unwrap()).unwrap();
    fs::rename(tmp, root.join(name)).unwrap();
}
fn gate(root: &Path, name: &str, end: u64) -> Value {
    while boottime_ns() < end {
        // Only the private retaining controller can publish this fixture gate.
        // Unwind the functional body while the actor still owns its children;
        // the final cleanup acknowledgement must remain available afterwards.
        assert!(
            name == "cleanup-complete.json" || !root.join("cancel-case.json").exists(),
            "private controller cancelled functional gate {name}"
        );
        match fs::read(root.join(name)) {
            Ok(b) => return serde_json::from_slice(&b).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => pause(),
            Err(e) => panic!("gate {name}: {e}"),
        }
    }
    panic!("original cutoff gate {name}");
}
struct App {
    pid: u32,
    alive: OwnedFd,
    sdk_pid: u32,
    sdk_alive: OwnedFd,
    listener: PathBuf,
    argv: Vec<PathBuf>,
}
struct Case {
    runtime: PathBuf,
    policy: PathBuf,
    args: Vec<PathBuf>,
    executable: PathBuf,
    helper: PathBuf,
    display_parent: PathBuf,
    actor: Option<JoinHandle<()>>,
    done: Receiver<io::Result<()>>,
    app_waits: Receiver<(u32, ExitStatus)>,
    gui: Receiver<GuiReply>,
    gui_failed: Arc<AtomicBool>,
    publications: Vec<Value>,
    reaped: Vec<(u32, ExitStatus)>,
    case: u64,
    close: u64,
    closed: bool,
}
impl Case {
    fn new(endpoint: &Path, case: u64, close: u64) -> Self {
        let id = fresh_id().unwrap();
        let install = PathBuf::from("/opt/connectivity-window-fixture").join(&id);
        let runtime = PathBuf::from("/run/connectivity-window-fixture").join(&id);
        let policy = PathBuf::from("/var/lib/connectivity-window-fixture").join(&id);
        for p in [&install, &runtime, &policy] {
            fs::create_dir_all(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o700)).unwrap();
        let home = install.join("home");
        let application = runtime.join("application");
        for p in [&home, &application] {
            fs::create_dir(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
            rustix::fs::chown(
                p,
                Some(rustix::process::Uid::from_raw(11050)),
                Some(rustix::process::Gid::from_raw(11050)),
            )
            .unwrap();
        }
        let get = |n| PathBuf::from(env::var_os(n).expect("selected absolute input"));
        let args = vec![
            home,
            application,
            get("CONNECTIVITY_WAYLAND"),
            get("CONNECTIVITY_APPEARANCE"),
            runtime.join("app.sock"),
            get("CONNECTIVITY_THEME_DIR"),
        ];
        let executable = get("NATIVE_SETTINGS_NORMAL_APP");
        let helper = get("CONNECTIVITY_DISPLAY_HELPER");
        let display_parent = get("CONNECTIVITY_DISPLAY_PARENT");
        let entry = get("NATIVE_LIFECYCLE_LAUNCHER_HELPER");
        let storage = get("NATIVE_LIFECYCLE_STORAGE_HELPER");
        let profile = install.join("profile.json");
        fs::write(
            &profile,
            serde_json::to_vec(
                &json!({"apps":[{"app":APP,"uid":11050,"executable":executable,"args":args}]}),
            )
            .unwrap(),
        )
        .unwrap();
        let compositor = Pid::from_raw(
            env::var("CONNECTIVITY_COMPOSITOR_PID")
                .unwrap()
                .parse::<i32>()
                .unwrap(),
        )
        .unwrap();
        let pidfd = rustix::process::pidfd_open(compositor, PidfdFlags::NONBLOCK).unwrap();
        let mut factory = crate::installed_display::Factory::new(
            display_parent.clone(),
            args[2].clone(),
            pidfd,
            pokelinux_display::SourceIdentity {
                pid: compositor,
                uid: 1000,
                gid: 1000,
            },
            helper.clone(),
        )
        .unwrap();
        let (tx, done) = mpsc::sync_channel(1);
        let (waiter, app_waits) = mpsc::sync_channel(4);
        let (sender, gui) = mpsc::sync_channel(128);
        let gui_failed = Arc::new(AtomicBool::new(false));
        let failed = gui_failed.clone();
        let (commands, inbox) = mpsc::sync_channel(1);
        drop(commands);
        let sr = runtime.clone();
        let sp = policy.clone();
        let endpoint = endpoint.to_owned();
        let actor = thread::spawn(move || {
            let mut control = ActorControl {
                inbox,
                attempt: None,
                observed_revision: Arc::new(AtomicU64::new(0)),
                source_barrier: None,
                storage_executable: storage,
                app_wait_observer: waiter,
                gui_replies: Some(sender),
                gui_observer_failed: failed,
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
                    Some(&mut factory),
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
            args,
            executable,
            helper,
            display_parent,
            actor: Some(actor),
            done,
            app_waits,
            gui,
            gui_failed,
            publications: vec![],
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
                && matches!(
                    self.admin(json!({"op":"snapshot"}), end),
                    Reply::Snapshot { ready: true, .. }
                )
            {
                return;
            }
            pause();
        }
        panic!("actor storage readiness");
    }
    fn publications(&mut self) {
        assert!(!self.gui_failed.load(std::sync::atomic::Ordering::Acquire));
        for r in self.gui.try_iter() {
            if r.uid == 11050 {
                let reply: Reply = serde_json::from_slice(&r.bytes).unwrap();
                // Retain actual actor replies, including refusals, for failed
                // functional-body diagnosis. State/completion predicates below
                // still select their exact variants and original deadlines.
                assert!(self.publications.len() < 512);
                self.publications.push(json!({"pid":r.pid,"uid":r.uid,"connection_id":r.connection_id,"sent_at_ns":r.sent_at_ns,"reply":reply}));
            }
        }
    }
    fn state(&mut self, app: &App, configured: bool, can: bool, after: u64, end: u64) -> Value {
        while boottime_ns() < end {
            self.publications();
            for r in self.publications.iter().rev() {
                if r["pid"] == app.pid
                    && r["sent_at_ns"].as_u64().unwrap() >= after
                    && r["reply"]["result"] == "connectivity_state"
                {
                    let state: connectivity::State =
                        serde_json::from_value(r["reply"]["state"].clone()).unwrap();
                    if state.is_current(boottime_ns())
                        && !state.pending
                        && state.configured == Some(configured)
                        && r["reply"]["can_control"] == can
                    {
                        return r.clone();
                    }
                }
            }
            pause();
        }
        panic!("fresh actual actor publication absent");
    }
    fn completed(&mut self, app: &App, configured: bool, after: u64, end: u64) -> Value {
        while boottime_ns() < end {
            self.publications();
            for r in self.publications.iter().rev() {
                if r["pid"] == app.pid
                    && r["sent_at_ns"].as_u64().unwrap() >= after
                    && r["reply"]["result"] == "connectivity_completed"
                {
                    let s: connectivity::State =
                        serde_json::from_value(r["reply"]["state"].clone()).unwrap();
                    if s.is_current(boottime_ns()) && !s.pending && s.configured == Some(configured)
                    {
                        return r.clone();
                    }
                }
            }
            pause();
        }
        panic!("actual current matching completed reply absent; no readonly substitution");
    }
    fn children() -> Vec<u32> {
        let mut ids = std::collections::BTreeSet::new();
        for task in fs::read_dir("/proc/self/task").unwrap() {
            if let Ok(s) = fs::read_to_string(task.unwrap().path().join("children")) {
                ids.extend(s.split_whitespace().map(|v| v.parse::<u32>().unwrap()));
            }
        }
        ids.into_iter().collect()
    }
    fn launch(&self) -> App {
        let end = until(self.case);
        assert_eq!(self.admin(json!({"op":"launch","app":APP}), end), Reply::Ok);
        while boottime_ns() < end {
            let mut found = None;
            let mut sdk = None;
            for pid in Self::children() {
                let Ok(raw) = fs::read(format!("/proc/{pid}/cmdline")) else {
                    continue;
                };
                let words: Vec<_> = raw
                    .split(|b| *b == 0)
                    .filter(|v| !v.is_empty())
                    .map(|b| PathBuf::from(std::str::from_utf8(b).unwrap()))
                    .collect();
                if words
                    == [
                        self.helper.clone(),
                        PathBuf::from("--display-context-worker"),
                    ]
                {
                    assert!(sdk.replace(pid).is_none());
                }
                if words.len() == 7
                    && words[0] == self.executable
                    && [0, 1, 3, 4, 5]
                        .iter()
                        .all(|i| words[*i + 1] == self.args[*i])
                {
                    let listener = &words[3];
                    assert_eq!(listener.file_name().unwrap(), "wayland.sock");
                    assert_eq!(
                        listener.parent().unwrap().parent(),
                        Some(self.display_parent.as_path())
                    );
                    let name = listener
                        .parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    assert!(
                        name.len() == 32
                            && name
                                .bytes()
                                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
                    );
                    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
                    assert!(status.lines().any(|l| l.starts_with("Uid:")
                        && l.split_whitespace().skip(1).all(|v| v == "11050")));
                    assert!(
                        status
                            .lines()
                            .any(|l| l.starts_with("Groups:") && l.split_whitespace().count() == 1)
                    );
                    for key in ["CapEff:", "CapPrm:", "CapInh:", "CapAmb:"] {
                        let v = status
                            .lines()
                            .find(|l| l.starts_with(key))
                            .unwrap()
                            .split_whitespace()
                            .nth(1)
                            .unwrap();
                        assert_eq!(u64::from_str_radix(v, 16).unwrap(), 0);
                    }
                    assert!(status.lines().any(|l| l == "NoNewPrivs:\t1"));
                    found = Some((pid, listener.clone(), words));
                }
            }
            if let (Some((pid, listener, argv)), Some(sdk_pid)) = (found, sdk) {
                let open = |p| {
                    rustix::process::pidfd_open(
                        Pid::from_raw(p as i32).unwrap(),
                        PidfdFlags::NONBLOCK,
                    )
                    .unwrap()
                };
                return App {
                    pid,
                    alive: open(pid),
                    sdk_pid,
                    sdk_alive: open(sdk_pid),
                    listener,
                    argv,
                };
            }
            pause();
        }
        panic!("normal Settings/SDK children not identified");
    }
    fn exited(&mut self, app: &App, end: u64) -> Value {
        while boottime_ns() < end {
            self.reaped.extend(self.app_waits.try_iter());
            self.publications();
            if let Some((_, status)) = self.reaped.iter().find(|(pid, _)| *pid == app.pid) {
                assert!(status.success());
                if !crate::authority::living(&app.alive).unwrap()
                    && !crate::authority::living(&app.sdk_alive).unwrap()
                    && !Path::new(&format!("/proc/{}", app.pid)).exists()
                    && !Path::new(&format!("/proc/{}", app.sdk_pid)).exists()
                    && !app.listener.parent().unwrap().exists()
                {
                    return json!({"pid":app.pid,"exit_code":status.code(),"exit_signal":status.signal(),"actual_Apps_Child_try_wait_observed":true,"app_stdout_stderr":"null","PIPE_EOF_claim":false,
       "sdk_pid":app.sdk_pid,"sdk_pidfd_terminal":true,"sdk_proc_absent":true,"listener_and_directory_absent":true,"SDK_reap_owned_by_existing_Context_Supervisor":true,"at_ns":boottime_ns()});
                }
            }
            pause();
        }
        panic!("original close deadline normal app/SDK/listener not retired");
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
                Ok(v) => {
                    v.unwrap();
                    self.actor.take().unwrap().join().unwrap();
                    self.reaped.extend(self.app_waits.try_iter());
                    self.publications();
                    self.closed = true;
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => pause(),
                Err(e) => panic!("actor close {e}"),
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

// Exact unchanged packet/quiet/readback helpers from the public SDK case.
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
    let ready = gate(root, "ready.json", c.case);
    let index = ready["initial_link_ifindex"].as_u64().unwrap();
    assert!(index > 1);
    let first = c.launch();
    publish(
        root,
        "first-window.json",
        &json!({"pid":first.pid,"argv":first.argv,"sdk_pid":first.sdk_pid,"listener":first.listener}),
    );
    gate(root, "first-window-qualified.json", c.case);
    let initial = c.state(&first, false, false, 0, until(c.case));
    publish(root, "initial-proof.json", &initial);
    independent(&gate(root, "initial-readback.json", c.case), false, index);
    assert_eq!(
        c.admin(json!({"op":"locked","locked":false}), until(c.case)),
        Reply::Ok
    );
    assert_eq!(
        c.admin(
            json!({"op":"background","app":APP,"background":false}),
            until(c.case)
        ),
        Reply::Ok
    );
    assert_eq!(
        c.admin(
            json!({"op":"grant","app":APP,"permission":"network_control","allowed":true}),
            until(c.case)
        ),
        Reply::Ok
    );
    let policy_before = fs::read(c.policy.join("policy.json")).unwrap();
    let policy: Value = serde_json::from_slice(&policy_before).unwrap();
    assert!(
        policy["grants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["app"] == APP
                && v["permission"] == "network_control"
                && v["allowed"] == true)
    );
    let grant_at = boottime_ns();
    let granted = c.state(&first, false, true, grant_at, until(c.case));
    publish(root, "first-action-ready.json", &granted);
    // The GUI setup and following effect phase are separately frozen before use.
    let clock = gate(root, "functional-clock.json", c.case);
    let start = clock["start_ns"].as_u64().unwrap();
    let end = clock["until_ns"].as_u64().unwrap();
    assert_eq!(start.checked_add(12_000_000_000), Some(end));
    assert!(start <= boottime_ns() && boottime_ns() < end && end < c.close);
    c.case = end;
    let connect_until = until(c.case);
    publish(
        root,
        "enable-input-request.json",
        &json!({"pid":first.pid,"original_until_ns":connect_until}),
    );
    let click = gate(root, "enable-input-sent.json", connect_until);
    assert!(
        click["before_ns"].as_u64().unwrap() < click["after_ns"].as_u64().unwrap()
            && click["after_ns"].as_u64().unwrap() < connect_until
    );
    let connected = c.completed(
        &first,
        true,
        click["before_ns"].as_u64().unwrap(),
        connect_until,
    );
    publish(root, "connected-proof.json", &connected);
    independent(&gate(root, "connected-readback.json", c.case), true, index);
    let first_packet = roundtrip(&nonce(1), c.case);
    let exit_until = (boottime_ns() + 2_000_000_000).min(c.case);
    publish(
        root,
        "close-first-request.json",
        &json!({"pid":first.pid,"original_until_ns":exit_until}),
    );
    gate(root, "close-first-sent.json", exit_until);
    let first_closed = c.exited(&first, exit_until);
    publish(root, "first-closed.json", &first_closed);
    independent(&gate(root, "background-readback.json", c.case), true, index);
    let second = c.launch();
    assert_ne!(first.pid, second.pid);
    assert_ne!(first.sdk_pid, second.sdk_pid);
    assert_ne!(first.listener, second.listener);
    publish(
        root,
        "second-window.json",
        &json!({"pid":second.pid,"argv":second.argv,"sdk_pid":second.sdk_pid,"listener":second.listener}),
    );
    gate(root, "second-window-qualified.json", c.case);
    let mut statuses = Vec::new();
    let idle_until = until(c.case);
    let mut at = first_closed["at_ns"].as_u64().unwrap();
    while boottime_ns() < idle_until {
        let row = c.state(&second, true, true, at, idle_until);
        let next = row["reply"]["state"]["observed_at_ns"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        if next > at {
            at = next;
            statuses.push(row);
            if statuses.len() == 2 {
                break;
            }
        }
        pause();
    }
    assert_eq!(statuses.len(), 2);
    assert_eq!(
        fs::read(c.policy.join("policy.json")).unwrap(),
        policy_before,
        "no regrant/reset between app generations"
    );
    let background_packet = roundtrip(&nonce(2), c.case);
    let disconnect_until = until(c.case);
    publish(
        root,
        "disable-input-request.json",
        &json!({"pid":second.pid,"original_until_ns":disconnect_until}),
    );
    let click2 = gate(root, "disable-input-sent.json", disconnect_until);
    assert!(
        click2["before_ns"].as_u64().unwrap() < click2["after_ns"].as_u64().unwrap()
            && click2["after_ns"].as_u64().unwrap() < disconnect_until
    );
    let disconnected = c.completed(
        &second,
        false,
        click2["before_ns"].as_u64().unwrap(),
        disconnect_until,
    );
    publish(root, "disconnected-proof.json", &disconnected);
    independent(
        &gate(root, "disconnected-readback.json", c.case),
        false,
        index,
    );
    let silent = quiet(root, c.case, 2);
    let exit_until = (boottime_ns() + 2_000_000_000).min(c.case);
    publish(
        root,
        "close-second-request.json",
        &json!({"pid":second.pid,"original_until_ns":exit_until}),
    );
    gate(root, "close-second-sent.json", exit_until);
    let second_closed = c.exited(&second, exit_until);
    json!({"functional_green":true,"first_pid":first.pid,"second_pid":second.pid,"first_closed":first_closed,"second_closed":second_closed,
  "connected_completed":connected,"disconnected_completed":disconnected,"first_packet":first_packet,"background_packet_after_withdraw":background_packet,
  "two_fresh_second_generation_states":statuses,"quiet":silent,"durable_grant_unchanged":true,"actual_actor_publications":c.publications,
  "normal_app_test_control_endpoint_selected":false,"actual_Qt_clicked_count_claimed":false,"functional_finished_at_ns":boottime_ns()})
}
#[test]
#[ignore = "isolated generic x86 VM; normal compositor/window and explicit authored root authority"]
fn normal_settings_window_connect_packet_lifecycle() {
    assert_eq!(
        env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),
        Ok("1")
    );
    let get = |k| env::var(k).unwrap().parse::<u64>().unwrap();
    let case = get("CONNECTIVITY_CASE_UNTIL_NS");
    let close = get("CONNECTIVITY_CASE_CLOSE_UNTIL_NS");
    let start = get("CONNECTIVITY_START_UNTIL_NS");
    let root = PathBuf::from(env::var_os("CONNECTIVITY_CASE_GATES").unwrap());
    let endpoint = PathBuf::from(env::var_os("CONNECTIVITY_ACTOR_ENDPOINT").unwrap());
    assert!(boottime_ns() < start && start <= case && case < close);
    let mut c = Case::new(&endpoint, case, close);
    publish(
        &root,
        "created.json",
        &json!({"driver_pid":std::process::id(),"actor_runtime":c.runtime,"authored_session_none":true,"normal_app_test_control_endpoint_selected":false}),
    );
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        c.ready(start);
        desired(&mut c, &root)
    }));
    let result = match result {
        Ok(v) => v,
        Err(_) => {
            json!({"functional_green":false,"consumer_error":"see actual libtest stderr","functional_finished_at_ns":boottime_ns()})
        }
    };
    c.close();
    publish(&root, "actor-publications.json", &json!(c.publications));
    let waits:Vec<_>=c.reaped.iter().map(|(pid,s)|json!({"pid":pid,"code":s.code(),"signal":s.signal(),"actual_Apps_Child_try_wait_observer":true,"PIPE_EOF_claim":false})).collect();
    publish(
        &root,
        "actor-closed.json",
        &json!({"actor_stopped_and_joined":true,"actual_Apps_waits":waits,"app_stdout_stderr":"null","completed_at_ns":boottime_ns()}),
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
        &json!({"asserted_after_cleanup_at_ns":boottime_ns(),"cleanup_completed_at_ns":cleanup["completed_at_ns"],"functional_green":result["functional_green"]}),
    );
    assert!(
        result["functional_green"] == true,
        "NORMAL_SETTINGS_WINDOW_REQUIRES_REAL_ENABLE_BACKGROUND_PACKET_DISABLE_AND_CUSTODY: {result}"
    );
}
