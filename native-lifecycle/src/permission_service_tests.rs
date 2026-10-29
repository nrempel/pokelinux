//! Shared service ingress/publication functions with actual credentialed packets,
//! actual owned Worker and synced files. Source/media facts are authored. This
//! component does not run the complete actor loop, GUI or a real audio backend.
use super::*;
use crate::{authority::permission_ingress_tests::Case, storage::Durable};
use serde_json::{Value, json};
use std::{io::Write, os::unix::fs::PermissionsExt};

struct ServiceCase {
    case: Case,
    worker: Worker,
    connection: Connection,
    operation: Option<Operation>,
    policy: PathBuf,
}
impl ServiceCase {
    fn new(pause: bool) -> Self {
        let case = Case::controller();
        let runtime = case.root.join("worker");
        let policy = case.root.join("worker-policy");
        for path in [&runtime, &policy] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let connection = Connection {
            id: 71,
            awaiting: None,
            fd: rustix::io::dup(&case.fd).unwrap(),
            admin: false,
            desktop: None,
            lock: false,
            reading: None,
            read_stamp: None,
            permission_stamp: None,
            deadline: boottime_ns() + 3_000_000_000,
            pending: None,
            lock_ack: None,
            launching: None,
        };
        let worker = Worker::spawn(
            &runtime,
            &policy,
            &PathBuf::from(std::env::var_os("NATIVE_LIFECYCLE_STORAGE_HELPER").unwrap()),
            pause.then_some("pause:1"),
        )
        .unwrap();
        let mut c = Self {
            case,
            worker,
            connection,
            operation: None,
            policy,
        };
        let until = boottime_ns() + 1_000_000_000;
        let token = c
            .worker
            .start_load(
                vec![crate::audio::APP.into(), crate::permission::APP.into()],
                until,
            )
            .unwrap();
        loop {
            c.case.refresh();
            match c.worker.poll().unwrap() {
                Some(Event::Complete {
                    token: t,
                    success: true,
                    state: Some(state),
                }) if t == token => {
                    c.case.authority.activate(state);
                    break;
                }
                None if boottime_ns() < until => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                _ => panic!("owned worker load"),
            }
        }
        c
    }
    fn status(&mut self) -> Value {
        let (bytes, cred) = self
            .case
            .packet(json!({"op":"permission_status","deadline_ns":boottime_ns()+900_000_000}));
        let reply = permission_request(
            &mut self.connection,
            &mut self.case.authority,
            &mut self.worker,
            &mut self.operation,
            serde_json::from_slice(&bytes).unwrap(),
            cred.pid.as_raw_nonzero().get() as u32,
            cred.uid.as_raw(),
            &mut None,
        )
        .unwrap();
        self.case.respond(reply)
    }
    fn queue(
        &mut self,
        allowed: bool,
        until: u64,
        audio: &mut Option<&mut dyn crate::audio::Provider>,
    ) {
        let state = self.status();
        assert_eq!(state["result"], "permission_state");
        let (bytes,cred)=self.case.packet(json!({"op":"permission_set","allowed":allowed,
            "expected_owner":state["owner"],"expected_revision":state["revision"],"deadline_ns":until}));
        assert!(
            permission_request(
                &mut self.connection,
                &mut self.case.authority,
                &mut self.worker,
                &mut self.operation,
                serde_json::from_slice(&bytes).unwrap(),
                cred.pid.as_raw_nonzero().get() as u32,
                cred.uid.as_raw(),
                audio
            )
            .is_none()
        );
        assert_eq!(self.operation.as_ref().unwrap().until, until);
        assert_eq!(
            self.connection.permission_stamp.as_ref().unwrap().until,
            until
        );
    }
    fn focus_lost(&mut self) {
        self.case.sample.revision += 1;
        self.case.sample.focus = None;
        self.case.refresh();
    }
    fn cancel_queued(&mut self) {
        revalidate_permission_write(
            &self.case.authority,
            &mut self.worker,
            &mut self.operation,
            std::slice::from_mut(&mut self.connection),
        );
    }
    fn finish(&mut self) -> Reply {
        let until = self.operation.as_ref().unwrap().until;
        loop {
            self.case.refresh();
            match self.worker.poll().unwrap() {
                Some(Event::Complete {
                    token,
                    success,
                    state,
                }) => {
                    let mut op = self.operation.take().unwrap();
                    assert_eq!(token, op.token);
                    assert!(success && state.is_none());
                    let reply =
                        self.case
                            .authority
                            .finish(op.intent.take().unwrap(), true, boottime_ns());
                    if op.connection == Some(self.connection.id) {
                        self.connection.awaiting = None;
                        self.connection.pending = Some(serde_json::to_vec(&reply).unwrap());
                    }
                    return reply;
                }
                None if boottime_ns() < until => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                _ => panic!("original owned write deadline"),
            }
        }
    }
    fn held(&mut self) -> rustix::process::Pid {
        let until = self.operation.as_ref().unwrap().until;
        loop {
            self.case.refresh();
            assert!(self.worker.poll().unwrap().is_none());
            if fs::read(self.policy.join("policy.json"))
                .ok()
                .is_some_and(|b| {
                    serde_json::from_slice::<Durable>(&b)
                        .unwrap()
                        .grants
                        .iter()
                        .any(|g| g.allowed)
                })
            {
                for task in fs::read_dir("/proc/self/task").unwrap() {
                    let children =
                        fs::read_to_string(task.unwrap().path().join("children")).unwrap();
                    for child in children.split_whitespace() {
                        let command =
                            fs::read(format!("/proc/{child}/cmdline")).unwrap_or_default();
                        if command.split(|b| *b == 0).any(|v| v == b"--storage-worker")
                            && command
                                .windows(self.case.root.as_os_str().as_encoded_bytes().len())
                                .any(|v| v == self.case.root.as_os_str().as_encoded_bytes())
                        {
                            let status =
                                fs::read_to_string(format!("/proc/{child}/status")).unwrap();
                            if status
                                .lines()
                                .any(|l| l.starts_with("State:") && l.contains('T'))
                            {
                                return rustix::process::Pid::from_raw(child.parse().unwrap())
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            assert!(boottime_ns() < until);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    fn clean_reopen(&mut self) -> Durable {
        let until = boottime_ns() + 1_000_000_000;
        let token = self.worker.start_shutdown(until).unwrap();
        let mut reply_observed = false;
        while self.worker.occupied() {
            match self.worker.poll().unwrap() {
                Some(Event::Complete {
                    token: t,
                    success: true,
                    state: None,
                }) if t == token => reply_observed = true,
                Some(Event::Failed { token: t }) if t == token => {}
                Some(Event::Reaped) => {}
                None if boottime_ns() < until => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                _ => panic!("original clean worker reap deadline"),
            }
        }
        assert!(self.policy.join("owner.clean").is_file());
        println!(
            "PERMISSION_SERVICE_CLEAN_REOPEN worker_waited=true shutdown_reply_observed={reply_observed}"
        );
        let durable = crate::storage::Disk::open(
            &self.policy,
            &[crate::audio::APP.into(), crate::permission::APP.into()],
        )
        .unwrap()
        .1;
        assert!(
            !durable.lock_pending,
            "clean marker, not unclean grant retention, qualifies reopen"
        );
        durable
    }
}
impl Drop for ServiceCase {
    fn drop(&mut self) {
        self.worker.drain();
    }
}

#[test]
fn permission_ingress_service_unsent_focus_loss_cancels_exact_token() {
    let mut c = ServiceCase::new(false);
    let before = fs::read(c.policy.join("policy.json")).ok();
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    let token = c.operation.as_ref().unwrap().token;
    assert!(!c.worker.cancel_queued(token + 1));
    c.focus_lost();
    c.cancel_queued();
    assert!(
        c.operation.is_none()
            && !c.worker.busy()
            && c.worker.occupied()
            && c.case.authority.ready()
    );
    assert_eq!(fs::read(c.policy.join("policy.json")).ok(), before);
    assert_eq!(
        serde_json::from_slice::<Reply>(c.connection.pending.as_ref().unwrap()).unwrap(),
        Reply::Denied
    );
    assert_eq!(c.case.respond(Reply::Denied)["result"], "denied");
}
#[test]
fn permission_ingress_service_unsent_expiry_keeps_healthy_writer() {
    let mut c = ServiceCase::new(false);
    c.queue(true, boottime_ns() + 20_000_000, &mut None);
    std::thread::sleep(std::time::Duration::from_millis(25));
    c.case.refresh();
    c.cancel_queued();
    assert!(
        c.operation.is_none()
            && !c.worker.busy()
            && c.worker.occupied()
            && c.case.authority.ready()
    );
    assert!(!c.policy.join("policy.json").exists());
    c.case.respond(Reply::Denied);
}
#[test]
fn permission_ingress_service_submitted_focus_loss_keeps_commit_busy_is_inert() {
    let mut c = ServiceCase::new(true);
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    let pid = c.held();
    let token = c.operation.as_ref().unwrap().token;
    let sequence = c.worker.admission_sequence();
    let busy = permission_request(
        &mut c.connection,
        &mut c.case.authority,
        &mut c.worker,
        &mut c.operation,
        AppRequest::PermissionStatus {
            deadline_ns: boottime_ns() + 500_000_000,
        },
        c.case.child.id(),
        crate::permission::UID,
        &mut None,
    );
    assert_eq!(busy, Some(Reply::Unavailable));
    assert_eq!(c.operation.as_ref().unwrap().token, token);
    assert_eq!(c.worker.admission_sequence(), sequence);
    c.focus_lost();
    c.cancel_queued();
    assert!(c.operation.is_some() && c.worker.busy());
    rustix::process::kill_process(pid, rustix::process::Signal::CONT).unwrap();
    assert_eq!(c.finish(), Reply::Denied);
    assert!(!c.case.authority.is_degraded());
    c.case.respond(Reply::Denied);
    assert!(
        c.clean_reopen()
            .grants
            .iter()
            .any(|g| g.app == crate::audio::APP && g.allowed)
    );
}
#[test]
fn permission_ingress_service_disconnect_retains_submitted_writer_and_commit() {
    let mut c = ServiceCase::new(true);
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    let pid = c.held();
    c.case.child.kill().unwrap();
    c.case.child.wait().unwrap();
    assert!(ipc::receive(&c.connection.fd).is_err());
    assert!(detach_permission_writer(c.connection.id, &mut c.operation));
    assert!(c.operation.as_ref().unwrap().connection.is_none() && c.worker.busy());
    assert!(matches!(
        c.case.authority.snapshot(true, true),
        Reply::Snapshot {
            ready: true,
            storage_busy: true,
            ..
        }
    ));
    c.focus_lost();
    rustix::process::kill_process(pid, rustix::process::Signal::CONT).unwrap();
    assert_eq!(c.finish(), Reply::Denied);
    assert!(c.connection.pending.is_none());
    assert!(c.clean_reopen().grants.iter().any(|g| g.allowed));
}
#[test]
fn permission_ingress_service_eagain_positive_is_downgraded_before_send() {
    let mut c = ServiceCase::new(false);
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    assert!(matches!(
        c.finish(),
        Reply::PermissionState { allowed: true, .. }
    ));
    net::sockopt::set_socket_send_buffer_size(&c.connection.fd, 4096).unwrap();
    let pressure = vec![b'x'; 2048];
    let mut count = 0;
    loop {
        match ipc::send(&c.connection.fd, &pressure) {
            Ok(()) => {
                count += 1;
                assert!(count < 128)
            }
            Err(e) if e.raw_os_error() == Some(11) => break,
            other => panic!("pressure {other:?}"),
        }
    }
    revalidate_permission_reply(&mut c.connection, &c.case.authority, false, false);
    assert_eq!(
        ipc::send(&c.connection.fd, c.connection.pending.as_ref().unwrap())
            .unwrap_err()
            .raw_os_error(),
        Some(11)
    );
    c.focus_lost();
    revalidate_permission_reply(&mut c.connection, &c.case.authority, false, false);
    assert!(c.connection.permission_stamp.is_none());
    let reply: Reply = serde_json::from_slice(c.connection.pending.as_ref().unwrap()).unwrap();
    assert_eq!(reply, Reply::Denied);
    writeln!(c.case.child.stdin.as_mut().unwrap(), "drain:{count}").unwrap();
    assert_eq!(c.case.respond(reply)["result"], "denied");
    assert!(c.clean_reopen().grants.iter().any(|g| g.allowed));
}

#[derive(Default)]
struct Audio {
    revokes: usize,
    effects: usize,
}
impl crate::audio::Provider for Audio {
    fn incarnation(&self) -> &str {
        "permission-test-audio"
    }
    fn state(&self) -> crate::audio::State {
        Default::default()
    }
    fn routes(&self) -> Vec<crate::audio::Route> {
        vec![]
    }
    fn submit(&mut self, _: &crate::audio::Intent) -> io::Result<bool> {
        self.effects += 1;
        Ok(true)
    }
    fn play(&mut self, _: &crate::audio::PreviewIntent) -> io::Result<bool> {
        self.effects += 1;
        Ok(true)
    }
    fn mute_preview(&mut self, _: &crate::audio::MuteIntent) -> io::Result<bool> {
        self.effects += 1;
        Ok(true)
    }
    fn stop_preview(&mut self, _: u64, _: u64) -> io::Result<bool> {
        Ok(true)
    }
    fn inspect(&mut self, _: u64) -> io::Result<bool> {
        Ok(false)
    }
    fn poll(&mut self) -> Option<crate::audio::Event> {
        None
    }
    fn revoke(&mut self) {
        self.revokes += 1;
    }
    fn reconnect(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn permission_ingress_service_cancelled_revoke_retains_allowed_fact_but_fences_old_lease() {
    let mut c = ServiceCase::new(false);
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    let reply = c.finish();
    c.case.respond(reply);
    let old = c.case.authored_media_lease();
    assert!(
        c.case
            .authority
            .current_media_lease(&old, "permission-test-audio")
    );
    let focus = c.case.sample.focus.clone();
    let mut audio = Audio::default();
    c.queue(false, boottime_ns() + 900_000_000, &mut Some(&mut audio));
    assert_eq!(audio.revokes, 1);
    assert_eq!(audio.effects, 0);
    assert!(
        !c.case
            .authority
            .current_media_lease(&old, "permission-test-audio")
    );
    c.focus_lost();
    c.cancel_queued();
    assert!(c.operation.is_none() && !c.worker.busy());
    c.case.respond(Reply::Denied);
    c.case.sample.revision += 1;
    c.case.sample.focus = focus;
    c.case.refresh();
    let state = c.status();
    assert_eq!(state["allowed"], true);
    assert_eq!(state["result"], "permission_state");
    assert_eq!(
        audio.effects, 0,
        "no automatic Play or replacement of revoked lease"
    );
    c.case.sample.revision += 1;
    c.case.sample.focus = Some(crate::session::OwnedFocus::new(
        crate::session::NativeFocus {
            pid: std::process::id(),
            uid: 11042,
            gid: 11042,
        },
        rustix::process::pidfd_open(
            rustix::process::getpid(),
            rustix::process::PidfdFlags::NONBLOCK,
        )
        .unwrap(),
    ));
    c.case.refresh();
    let now = boottime_ns();
    assert!(
        c.case
            .authority
            .begin_media_preview(
                std::process::id(),
                11042,
                "permission-test-audio",
                &crate::audio::PreviewRequest {
                    route: "speaker".into(),
                    ttl_ms: 1000,
                    until_boottime_ns: now + 900_000_000,
                    level_percent: 10,
                    muted: false
                },
                now
            )
            .is_ok()
    );
    assert_eq!(audio.effects, 0);
    assert!(c.clean_reopen().grants.iter().any(|g| g.allowed));
}
#[test]
fn permission_ingress_service_generation_and_original_deadline_fence_queued_status() {
    for successor in [false, true] {
        let mut c = ServiceCase::new(false);
        let until = boottime_ns() + 20_000_000;
        let (bytes, cred) = c
            .case
            .packet(json!({"op":"permission_status","deadline_ns":until}));
        let reply = permission_request(
            &mut c.connection,
            &mut c.case.authority,
            &mut c.worker,
            &mut c.operation,
            serde_json::from_slice(&bytes).unwrap(),
            cred.pid.as_raw_nonzero().get() as u32,
            cred.uid.as_raw(),
            &mut None,
        )
        .unwrap();
        assert!(matches!(reply, Reply::PermissionState { .. }));
        c.connection.pending = Some(serde_json::to_vec(&reply).unwrap());
        if successor {
            c.case
                .authority
                .register(
                    crate::permission::APP,
                    c.case.child.id(),
                    rustix::process::pidfd_open(
                        rustix::process::Pid::from_raw(c.case.child.id() as i32).unwrap(),
                        rustix::process::PidfdFlags::NONBLOCK,
                    )
                    .unwrap(),
                )
                .unwrap();
        } else {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        c.case.refresh();
        revalidate_permission_reply(&mut c.connection, &c.case.authority, false, false);
        assert!(c.connection.permission_stamp.is_none());
        assert_eq!(c.case.respond(Reply::Denied)["result"], "denied");
        assert_eq!(
            serde_json::from_slice::<Reply>(c.connection.pending.as_ref().unwrap()).unwrap(),
            Reply::Denied
        );
    }
}
#[test]
fn permission_ingress_service_fresh_heartbeat_keeps_semantic_stamp() {
    let mut c = ServiceCase::new(false);
    let until = boottime_ns() + 900_000_000;
    let (bytes, cred) = c
        .case
        .packet(json!({"op":"permission_status","deadline_ns":until}));
    let reply = permission_request(
        &mut c.connection,
        &mut c.case.authority,
        &mut c.worker,
        &mut c.operation,
        serde_json::from_slice(&bytes).unwrap(),
        cred.pid.as_raw_nonzero().get() as u32,
        cred.uid.as_raw(),
        &mut None,
    )
    .unwrap();
    c.connection.pending = Some(serde_json::to_vec(&reply).unwrap());
    let old_until = c.case.sample.until;
    std::thread::sleep(std::time::Duration::from_millis(50));
    c.case.refresh();
    std::thread::sleep(std::time::Duration::from_millis(55));
    c.case.refresh();
    assert!(boottime_ns() > old_until);
    revalidate_permission_reply(&mut c.connection, &c.case.authority, false, false);
    assert!(c.connection.permission_stamp.is_some());
    assert_eq!(c.connection.permission_stamp.as_ref().unwrap().until, until);
    assert_eq!(c.case.respond(reply)["result"], "permission_state");
}
#[test]
fn permission_ingress_service_stop_during_held_write_is_bounded_and_never_clean_seals() {
    let mut c = ServiceCase::new(true);
    c.queue(true, boottime_ns() + 900_000_000, &mut None);
    c.held();
    let mut apps = crate::apps::Apps::default();
    let mut stopping = false;
    let mut stop_reply = None;
    let mut cleanup = None;
    let started = boottime_ns();
    let reply = begin_stop(
        &mut c.case.authority,
        &mut apps,
        &mut c.worker,
        &mut c.operation,
        &mut None,
        &mut None,
        false,
        &mut stopping,
        &mut stop_reply,
        &mut cleanup,
    );
    assert_eq!(reply, Reply::Ok);
    assert!(stopping && boottime_ns() - started < 100_000_000);
    let original = cleanup;
    assert_eq!(
        begin_stop(
            &mut c.case.authority,
            &mut apps,
            &mut c.worker,
            &mut c.operation,
            &mut None,
            &mut None,
            false,
            &mut stopping,
            &mut stop_reply,
            &mut cleanup
        ),
        reply
    );
    assert_eq!(cleanup, original);
    assert!(c.operation.as_ref().unwrap().retired);
    c.worker.drain();
    assert!(!c.policy.join("owner.clean").exists());
    c.case.respond(Reply::Unavailable);
    assert!(
        crate::storage::Disk::open(
            &c.policy,
            &[crate::audio::APP.into(), crate::permission::APP.into()]
        )
        .is_err(),
        "unclean writer must refuse reopen, never silently restore authority"
    );
    let durable: Durable =
        serde_json::from_slice(&fs::read(c.policy.join("policy.json")).unwrap()).unwrap();
    assert!(durable.grants.iter().any(|g| g.allowed));
}
