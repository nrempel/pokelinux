//! Actual UID/SCM_CREDENTIALS and live pidfd component boundary.
//! Source and installation facts are authored; this is not GUI or full actor proof.
use super::*;
use crate::{
    AppRole, ipc,
    session::{NativeFocus, OwnedFocus, Snapshot},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

const APP: &str = "org.omarchy.Permissions";
const UID: u32 = 11045;
const CLIENT: &str = r#"import os,socket,sys,json
os.setgroups([]);os.setresgid(11045,11045,11045);os.setresuid(11045,11045,11045)
s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(2);s.connect(sys.argv[1])
for line in sys.stdin:
 p=line.strip().encode();assert s.send(p)==len(p)
 control=sys.stdin.readline().strip()
 if control.startswith('drain:'):
  for _ in range(int(control.split(':')[1])): assert s.recv(4096)
  control=sys.stdin.readline().strip()
 assert control=='read'
 print(s.recv(4096).decode(),flush=True)
"#;

pub(crate) struct Case {
    pub(crate) authority: Authority,
    disk: crate::storage::Disk,
    pub(crate) child: Child,
    output: BufReader<std::process::ChildStdout>,
    pub(crate) fd: OwnedFd,
    pub(crate) root: PathBuf,
    pub(crate) sample: Snapshot,
}
impl Case {
    fn new(role: AppRole) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from("/run").join(format!("permission-{}", fresh_id().unwrap()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let listener = ipc::listener(&root.join("app.sock"), 0o666).unwrap();
        let mut child = Command::new("/usr/bin/python3")
            .args(["-I", "-c", CLIENT])
            .arg(root.join("app.sock"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let until = crate::boottime_ns() + 2_000_000_000;
        let fd = loop {
            match rustix::net::accept_with(
                &listener,
                rustix::net::SocketFlags::NONBLOCK | rustix::net::SocketFlags::CLOEXEC,
            ) {
                Ok(fd) => break fd,
                Err(rustix::io::Errno::AGAIN) if crate::boottime_ns() < until => {
                    std::thread::yield_now()
                }
                other => panic!("credentialed child setup {other:?}"),
            }
        };
        rustix::net::sockopt::set_socket_passcred(&fd, true).unwrap();
        let mut authority = Authority::loading(vec![
            AppProfile {
                role: AppRole::Application,
                app: crate::audio::APP.into(),
                uid: 11042,
                executable: "/usr/bin/true".into(),
                args: vec![],
            },
            AppProfile {
                role,
                app: APP.into(),
                uid: UID,
                executable: "/usr/bin/python3".into(),
                args: vec![],
            },
        ])
        .unwrap();
        authority.require_session().unwrap();
        fs::create_dir(root.join("policy")).unwrap();
        fs::set_permissions(root.join("policy"), fs::Permissions::from_mode(0o700)).unwrap();
        let (disk, durable) = crate::storage::Disk::open(
            &root.join("policy"),
            &[crate::audio::APP.into(), APP.into()],
        )
        .unwrap();
        authority.activate(durable);
        let pid = rustix::process::Pid::from_raw(child.id() as i32).unwrap();
        authority
            .register(
                APP,
                child.id(),
                rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::NONBLOCK).unwrap(),
            )
            .unwrap();
        let now = crate::boottime_ns();
        let sample = Snapshot {
            source: fresh_id().unwrap(),
            revision: 1,
            sequence: 1,
            sampled_at_ns: now,
            until: now + crate::session::SAMPLE_BUDGET_NS,
            locked: false,
            focus: Some(OwnedFocus::new(
                NativeFocus {
                    pid: child.id(),
                    uid: UID,
                    gid: UID,
                },
                rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::NONBLOCK).unwrap(),
            )),
            locker: None,
            lock_generation: None,
            protocol_unlock: None,
        };
        authority.observe_session(sample.clone(), now).unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            authority,
            disk,
            child,
            output,
            fd,
            root,
            sample,
        }
    }
    pub(crate) fn controller() -> Self {
        Self::new(AppRole::PermissionController {
            target: crate::audio::APP.into(),
            permission: Permission::MediaRoute,
        })
    }
    pub(crate) fn refresh(&mut self) {
        self.sample.sequence += 1;
        self.sample.sampled_at_ns = crate::boottime_ns();
        self.sample.until = self.sample.sampled_at_ns + crate::session::SAMPLE_BUDGET_NS;
        self.authority
            .observe_session(self.sample.clone(), self.sample.sampled_at_ns)
            .unwrap();
    }
    pub(crate) fn packet(&mut self, request: Value) -> (Vec<u8>, rustix::net::UCred) {
        writeln!(self.child.stdin.as_mut().unwrap(), "{request}").unwrap();
        let until = crate::boottime_ns() + 1_000_000_000;
        let (bytes, cred) = loop {
            match ipc::receive(&self.fd).unwrap() {
                Some(packet) => break packet,
                None if crate::boottime_ns() < until => std::thread::yield_now(),
                _ => panic!("actual credentialed packet deadline"),
            }
        };
        assert_eq!(cred.pid.as_raw_nonzero().get() as u32, self.child.id());
        assert_eq!(cred.uid.as_raw(), UID);
        assert_eq!(cred.gid.as_raw(), UID);
        self.refresh();
        (bytes, cred)
    }
    pub(crate) fn respond(&mut self, reply: Reply) -> Value {
        let until = crate::boottime_ns() + 1_000_000_000;
        loop {
            match ipc::send(&self.fd, &serde_json::to_vec(&reply).unwrap()) {
                Ok(()) => break,
                Err(e) if e.raw_os_error() == Some(11) && crate::boottime_ns() < until => {
                    std::thread::yield_now()
                }
                other => panic!("credentialed reply {other:?}"),
            }
        }
        writeln!(self.child.stdin.as_mut().unwrap(), "read").unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn wire(&mut self, request: Value) -> Value {
        let (bytes, _) = self.packet(request);
        let reply = match serde_json::from_slice::<AppRequest>(&bytes) {
            Ok(request) => match self.authority.plan_request(
                self.child.id(),
                UID,
                request,
                crate::boottime_ns(),
            ) {
                Decision::Immediate(reply) => reply,
                Decision::Persist(intent) => {
                    self.disk.replace(&intent.next).unwrap();
                    self.authority.finish(intent, true, crate::boottime_ns())
                }
            },
            Err(_) => Reply::Denied,
        };
        self.respond(reply)
    }
    fn status(&mut self) -> Value {
        self.wire(json!({"op":"permission_status","deadline_ns":crate::boottime_ns()+900_000_000}))
    }
    pub(crate) fn authored_media_lease(&mut self) -> MediaLeaseStamp {
        // Media principal/focus are authored component inputs; controller IPC
        // above still uses actual UID11045 and its retained live kernel task.
        self.authority
            .register(
                crate::audio::APP,
                std::process::id(),
                rustix::process::pidfd_open(
                    rustix::process::getpid(),
                    rustix::process::PidfdFlags::NONBLOCK,
                )
                .unwrap(),
            )
            .unwrap();
        let p = &self.authority.principals[crate::audio::APP];
        MediaLeaseStamp {
            app: crate::audio::APP.into(),
            generation: p.generation,
            pid: p.pid,
            uid: p.uid,
            backend: "permission-test-audio".into(),
            revocation: self
                .authority
                .media_revocations
                .get(crate::audio::APP)
                .copied()
                .unwrap_or(0),
        }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        self.child.stdin.take();
        let until = crate::boottime_ns() + 1_000_000_000;
        loop {
            if self.child.try_wait().unwrap().is_some() {
                break;
            }
            if crate::boottime_ns() >= until {
                self.child.kill().unwrap();
                self.child.wait().unwrap();
                break;
            }
            std::thread::yield_now();
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn permission_ingress_live_controller_reads_durable_fact() {
    let mut c = Case::controller();
    let state = c.status();
    assert_eq!(state["result"], "permission_state");
    assert_eq!(state["allowed"], false);
    assert_eq!(state["target"], crate::audio::APP);
    assert_eq!(state["permission"], "media_route");
}
#[test]
fn permission_ingress_live_controller_commits_scoped_grant() {
    let mut c = Case::controller();
    let request = json!({"op":"permission_set","allowed":true,"expected_owner":c.authority.incarnation,
        "expected_revision":c.authority.policy,"deadline_ns":crate::boottime_ns()+900_000_000});
    let state = c.wire(request);
    assert_eq!(state["result"], "permission_state");
    assert_eq!(state["allowed"], true);
    assert!(
        c.authority
            .durable
            .grants
            .iter()
            .any(|g| g.app == crate::audio::APP
                && g.permission == Permission::MediaRoute
                && g.allowed)
    );
    let durable: crate::storage::Durable =
        serde_json::from_slice(&fs::read(c.root.join("policy/policy.json")).unwrap()).unwrap();
    assert!(durable.grants.iter().any(|g| g.app == crate::audio::APP
        && g.permission == Permission::MediaRoute
        && g.allowed));
}
#[test]
fn permission_ingress_ordinary_or_wrong_scope_never_becomes_controller() {
    for role in [
        AppRole::Application,
        AppRole::PermissionController {
            target: crate::messages::APP.into(),
            permission: Permission::MediaRoute,
        },
        AppRole::PermissionController {
            target: crate::audio::APP.into(),
            permission: Permission::Capture,
        },
    ] {
        assert_eq!(Case::new(role).status()["result"], "denied");
    }
}
#[test]
fn permission_ingress_focus_deadline_and_unknown_identity_fields_refuse() {
    let mut c = Case::controller();
    c.sample.revision += 1;
    c.sample.focus = None;
    assert_eq!(c.status()["result"], "denied");
    for deadline in [1, crate::boottime_ns() + 2_000_000_000] {
        let mut c = Case::controller();
        assert_eq!(
            c.wire(json!({"op":"permission_status","deadline_ns":deadline}))["result"],
            "denied"
        );
    }
    let mut c = Case::controller();
    let malformed =
        json!({"op":"permission_status","deadline_ns":crate::boottime_ns()+900_000_000,"uid":0});
    assert!(serde_json::from_value::<AppRequest>(malformed.clone()).is_err());
    assert_eq!(c.wire(malformed)["result"], "denied");
}
#[test]
fn permission_ingress_ordinary_set_refuses_without_touching_disk() {
    let mut c = Case::new(AppRole::Application);
    let before = fs::read(c.root.join("policy/policy.json")).ok();
    let request = json!({"op":"permission_set","allowed":true,"expected_owner":c.authority.incarnation,
        "expected_revision":c.authority.policy,"deadline_ns":crate::boottime_ns()+900_000_000});
    assert_eq!(c.wire(request)["result"], "denied");
    assert_eq!(fs::read(c.root.join("policy/policy.json")).ok(), before);
}
#[test]
fn permission_ingress_stale_owner_or_revision_cannot_change_policy() {
    for stale_owner in [false, true] {
        let mut c = Case::controller();
        let before = fs::read(c.root.join("policy/policy.json")).ok();
        let request = json!({"op":"permission_set","allowed":true,
            "expected_owner":if stale_owner {"0".repeat(32)} else {c.authority.incarnation.clone()},
            "expected_revision":c.authority.policy+u64::from(!stale_owner),"deadline_ns":crate::boottime_ns()+900_000_000});
        assert_eq!(c.wire(request)["result"], "denied");
        assert_eq!(fs::read(c.root.join("policy/policy.json")).ok(), before);
    }
}
#[test]
fn permission_ingress_locked_suspended_or_missing_source_refuses() {
    for mode in 0..3 {
        let mut c = Case::controller();
        match mode {
            0 => c.authority.lifecycle(Some(true), None, None).unwrap(),
            1 => c.authority.lifecycle(None, Some(true), None).unwrap(),
            _ => {
                c.authority.session_required = false;
            }
        }
        assert_eq!(c.status()["result"], "denied");
    }
}
