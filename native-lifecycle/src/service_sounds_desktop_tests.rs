//! Actual fixed desktop sockets/UIDs and existing Apps custody only.
//! No Source/audio provider, normal GUI, compositor, grant or auth is selected.
use super::ActorControl;
use crate::{Reply, boottime_ns, fresh_id, ipc, messages};
use rustix::{
    fd::OwnedFd,
    process::{Pid, PidfdFlags},
};
use std::{
    collections::BTreeSet,
    fs, io,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
const SOUNDS: &[u8] = br#"{"op":"launch_sounds"}"#;
const MESSAGES: &[u8] = br#"{"op":"launch_messages"}"#;
const CLIENT: &str = r#"import os,sys,socket,struct,json,time
uid=int(sys.argv[1]);os.setgroups([]);os.setresgid(uid,uid,uid);os.setresuid(uid,uid,uid)
assert os.getuid()==os.geteuid()==uid and os.getgid()==os.getegid()==uid and os.getgroups()==[]
until=time.clock_gettime_ns(time.CLOCK_BOOTTIME)+2_000_000_000
s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1)
def budget():
 left=until-time.clock_gettime_ns(time.CLOCK_BOOTTIME)
 assert left>0
 s.settimeout(left/1e9)
budget();s.connect(sys.argv[2]);peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(int(sys.argv[4]),0,0)
p=sys.argv[3].encode();budget();assert s.send(p)==len(p)
budget();b,a,f,_=s.recvmsg(4096,socket.CMSG_SPACE(12));assert b and not f&(socket.MSG_TRUNC|socket.MSG_CTRUNC)
assert len(a)==1 and a[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS) and struct.unpack('3i',a[0][2])==peer
reply=json.loads(b);assert time.clock_gettime_ns(time.CLOCK_BOOTTIME)<until
print(json.dumps({'actual_uid':uid,'groups':os.getgroups(),'packet':sys.argv[3],'reply':reply,'original_until_ns':until,'completed_ns':time.clock_gettime_ns(time.CLOCK_BOOTTIME)}),flush=True)
"#;
struct UnavailableMessages;
impl messages::Provider for UnavailableMessages {
    fn incarnation(&self) -> &str {
        "desktop-host-messages-setup"
    }
    fn occupied(&self) -> bool {
        false
    }
    fn read_occupied(&self) -> bool {
        false
    }
    fn busy(&self) -> bool {
        false
    }
    fn available(&self) -> bool {
        false
    }
    fn start(&mut self, _: serde_json::Value, _: u64) -> io::Result<u64> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn poll(&mut self) -> io::Result<Option<messages::Event>> {
        Ok(None)
    }
    fn cancel_read(&mut self, _: u64) {}
    fn stop(&mut self) {}
    fn restart(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn drain_until(&mut self, _: u64) -> bool {
        true
    }
}
struct Case {
    runtime: PathBuf,
    install: PathBuf,
    done: Receiver<io::Result<()>>,
    actor: Option<JoinHandle<()>>,
    app_waits: Receiver<(u32, ExitStatus)>,
    reaped: Vec<(u32, ExitStatus)>,
    work_until: u64,
    close_until: u64,
    closed: bool,
}
impl Case {
    fn new(with_messages: bool, sounds_profile: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let id = fresh_id().unwrap();
        let install = PathBuf::from("/opt/sounds-desktop-tests").join(&id);
        let runtime = PathBuf::from("/run/sounds-desktop-tests").join(&id);
        let policy = PathBuf::from("/var/lib/sounds-desktop-tests").join(&id);
        for p in [&install, &runtime, &policy] {
            fs::create_dir_all(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o700)).unwrap();
        for (env, name) in [
            ("NATIVE_LIFECYCLE_STORAGE_HELPER", "lifecycle-daemon"),
            ("NATIVE_LIFECYCLE_LAUNCHER_HELPER", "lifecycle-launcher"),
        ] {
            fs::copy(
                std::env::var_os(env).expect("explicit built helper"),
                install.join(name),
            )
            .unwrap();
            fs::set_permissions(install.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(install.join("client.py"), CLIENT).unwrap();
        let mut profiles = vec![];
        if sounds_profile {
            profiles.push(serde_json::json!({"role":{"kind":"application"},"app":"org.omarchy.Sounds","uid":11042,"executable":"/usr/bin/sleep","args":["120"]}));
        }
        if with_messages || !sounds_profile {
            profiles.push(serde_json::json!({"role":{"kind":"application"},"app":"org.omarchy.Messages","uid":11010,"executable":"/usr/bin/sleep","args":["120"]}));
        }
        let profile = install.join("profile.json");
        fs::write(
            &profile,
            serde_json::to_vec(&serde_json::json!({"apps":profiles})).unwrap(),
        )
        .unwrap();
        let (actor_done, done) = mpsc::sync_channel(1);
        let (app_wait_observer, app_waits) = mpsc::sync_channel(4);
        let (commands, inbox) = mpsc::sync_channel(1);
        drop(commands);
        let sr = runtime.clone();
        let si = install.clone();
        let actor = thread::spawn(move || {
            let mut control = ActorControl {
                inbox,
                attempt: None,
                observed_revision: Arc::new(AtomicU64::new(0)),
                source_barrier: None,
                storage_executable: si.join("lifecycle-daemon"),
                app_wait_observer,
                gui_replies: None,
                gui_observer_failed: Arc::new(AtomicBool::new(false)),
            };
            let mut provider = UnavailableMessages;
            let result = super::super::run_with_services_inner(
                &sr,
                &policy,
                &profile,
                &si.join("lifecycle-launcher"),
                None,
                if with_messages {
                    Some(&mut provider)
                } else {
                    None
                },
                None,
                None,
                None,
                None,
                None,
                Some(&mut control),
            );
            let _ = actor_done.try_send(result);
        });
        let now = boottime_ns();
        let c = Self {
            runtime,
            install,
            done,
            actor: Some(actor),
            app_waits,
            reaped: vec![],
            work_until: now + 12_000_000_000,
            close_until: now + 15_000_000_000,
            closed: false,
        };
        while boottime_ns() < c.work_until {
            if c.runtime.join("admin.sock").exists()
                && let Ok(Reply::Snapshot { ready: true, .. }) =
                    c.admin(serde_json::json!({"op":"snapshot"}))
            {
                return c;
            }
            thread::sleep(Duration::from_millis(2));
        }
        panic!("setup: actual actor storage readiness");
    }
    fn admin(&self, value: serde_json::Value) -> io::Result<Reply> {
        let fd = ipc::connect(&self.runtime.join("admin.sock"))?;
        ipc::call_until_from(fd, &value, self.work_until, None)
    }
    fn wait_child(&self, mut child: Child) -> (ExitStatus, Vec<u8>) {
        let pid = child.id();
        let mut killed = false;
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break s;
            }
            if boottime_ns() >= self.work_until && !killed {
                child.kill().unwrap();
                killed = true;
            }
            if boottime_ns() >= self.close_until {
                std::process::abort();
            }
            thread::sleep(Duration::from_millis(1));
        };
        let waited = boottime_ns();
        let output = child.wait_with_output().unwrap();
        let eof = boottime_ns();
        assert!(
            waited < self.work_until
                && eof < self.work_until
                && output.stdout.len() < 8192
                && output.stderr.len() < 8192
        );
        eprintln!(
            "ACTUAL_SOUNDS_CLIENT_WAIT {}",
            serde_json::json!({"pid":pid,"status":status.code(),"waited_ns":waited,"stdout_and_stderr_EOF_ns":eof,"original_until_ns":self.work_until})
        );
        assert!(
            status.success(),
            "setup: client failed {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        (status, output.stdout)
    }
    fn request(&self, uid: u32, endpoint: &str, bytes: &[u8]) -> Reply {
        let child = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(self.install.join("client.py"))
            .arg(uid.to_string())
            .arg(self.runtime.join(endpoint))
            .arg(std::str::from_utf8(bytes).unwrap())
            .arg(std::process::id().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (_, out) = self.wait_child(child);
        let row: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(row["actual_uid"], uid);
        assert_eq!(row["groups"], serde_json::json!([]));
        assert!(row["completed_ns"].as_u64().unwrap() < row["original_until_ns"].as_u64().unwrap());
        serde_json::from_value(row["reply"].clone()).unwrap()
    }
    fn app(&self, uid: u32) -> (u32, OwnedFd) {
        while boottime_ns() < self.work_until {
            let mut children = BTreeSet::new();
            for t in fs::read_dir("/proc/self/task").unwrap() {
                if let Ok(raw) = fs::read_to_string(t.unwrap().path().join("children")) {
                    for id in raw.split_whitespace() {
                        children.insert(id.parse::<u32>().unwrap());
                    }
                }
            }
            for pid in children {
                let Ok(status) = fs::read_to_string(format!("/proc/{pid}/status")) else {
                    continue;
                };
                let expected = uid.to_string();
                if status.lines().any(|l| {
                    l.starts_with("Uid:") && l.split_whitespace().skip(1).all(|x| x == expected)
                }) && fs::read(format!("/proc/{pid}/cmdline"))
                    .is_ok_and(|b| b == b"/usr/bin/sleep\x00120\x00")
                {
                    assert!(
                        status
                            .lines()
                            .any(|l| l.starts_with("Groups:") && l.split_whitespace().count() == 1)
                    );
                    assert!(
                        status
                            .lines()
                            .any(|l| l == format!("PPid:\t{}", std::process::id()))
                    );
                    let fd = rustix::process::pidfd_open(
                        Pid::from_raw(pid as i32).unwrap(),
                        PidfdFlags::NONBLOCK,
                    )
                    .unwrap();
                    assert!(crate::authority::living(&fd).unwrap());
                    return (pid, fd);
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        panic!("setup: launched app identity/exec not observed");
    }
    fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        let fd = ipc::connect(&self.runtime.join("admin.sock"))?;
        let reply: Reply = ipc::call_until_from(
            fd,
            &serde_json::json!({"op":"stop"}),
            self.close_until,
            None,
        )?;
        assert_eq!(reply, Reply::Ok);
        loop {
            match self.done.try_recv() {
                Ok(result) => {
                    result?;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) if boottime_ns() < self.close_until => {
                    thread::sleep(Duration::from_millis(2))
                }
                _ => return Err(io::ErrorKind::TimedOut.into()),
            }
        }
        self.actor.take().unwrap().join().unwrap();
        while let Ok((pid, status)) = self.app_waits.try_recv() {
            let observed = boottime_ns();
            assert!(observed < self.close_until);
            eprintln!(
                "ACTUAL_SOUNDS_APP_WAIT {}",
                serde_json::json!({"pid":pid,"code":status.code(),"signal":status.signal(),"observed_ns":observed,"original_until_ns":self.close_until,"witness":"existing Apps child.try_wait Some -> bounded cfg(test) observer"})
            );
            self.reaped.push((pid, status));
        }
        assert!(!self.runtime.join("launch-sounds.sock").exists());
        assert!(!self.runtime.join("admin.sock").exists());
        self.closed = true;
        Ok(())
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            eprintln!("owned Sounds case cleanup failed: {e}");
            std::process::abort();
        }
    }
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_actual_wrong_uids_denied() {
    let c = Case::new(false, true);
    for uid in [0, 1001, 11042] {
        assert_eq!(c.request(uid, "launch-sounds.sock", SOUNDS), Reply::Denied);
    }
    assert!(matches!(
        c.admin(serde_json::json!({"op":"snapshot"})).unwrap(),
        Reply::Snapshot { principals: 0, .. }
    ));
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_strict_verbs_and_actual_messages_child_setup_control() {
    let mut c = Case::new(true, true);
    for bytes in [
        MESSAGES,
        br#"{"op":"launch_sounds","app":"org.omarchy.Messages"}"#,
        br#"{"op":"grant","app":"org.omarchy.Sounds","permission":"media_route","allowed":true}"#,
        b"{",
    ] {
        assert_eq!(c.request(1000, "launch-sounds.sock", bytes), Reply::Denied);
    }
    assert_eq!(
        c.request(1000, "launch-messages.sock", SOUNDS),
        Reply::Denied
    );
    assert_eq!(c.request(1000, "launch-messages.sock", MESSAGES), Reply::Ok);
    let (pid, fd) = c.app(11010);
    c.close().unwrap();
    assert_eq!(c.reaped.len(), 1);
    assert_eq!(c.reaped[0].0, pid);
    assert!(!crate::authority::living(&fd).unwrap());
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_missing_fixed_profile_unavailable() {
    let c = Case::new(false, false);
    assert_eq!(
        c.request(1000, "launch-sounds.sock", SOUNDS),
        Reply::Unavailable
    );
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_actual_uid1000_launches_without_messages() {
    let c = Case::new(false, true);
    assert!(!c.runtime.join("launch-messages.sock").exists());
    assert_eq!(c.request(1000, "launch-sounds.sock", SOUNDS), Reply::Ok);
    let _owned = c.app(11042);
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_duplicate_retains_one_actor_child() {
    let c = Case::new(false, true);
    assert_eq!(c.request(1000, "launch-sounds.sock", SOUNDS), Reply::Ok);
    let (pid, _fd) = c.app(11042);
    assert_eq!(c.request(1000, "launch-sounds.sock", SOUNDS), Reply::Denied);
    assert_eq!(c.app(11042).0, pid);
}
#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn sounds_stop_reaps_exact_owned_child() {
    let mut c = Case::new(false, true);
    assert_eq!(c.request(1000, "launch-sounds.sock", SOUNDS), Reply::Ok);
    let (pid, fd) = c.app(11042);
    c.close().unwrap();
    assert_eq!(c.reaped.len(), 1);
    assert_eq!(c.reaped[0].0, pid);
    assert!(!crate::authority::living(&fd).unwrap());
}
