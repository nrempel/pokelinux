//! Controlled Authority/Provider facts plus actual Linux SEQPACKET input/publication.
//! No backend, genuine Source producer, launched Settings, UI, ARM or hardware proof.
use super::brightness_client_input;
use crate::{
    AppProfile, AppRequest, Permission, Reply,
    authority::Authority,
    boottime_ns,
    brightness::{self, Event, Intent, Provider, Request, State},
    brightness_session::Session,
    fresh_id, ipc,
    storage::Disk,
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, Shutdown, SocketFlags, SocketType},
};
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf};
const CONNECTION: u64 = 71;
struct ObservedProvider {
    state: State,
    sent: Vec<Intent>,
    revoked: usize,
    fail_send: bool,
}
impl Provider for ObservedProvider {
    fn incarnation(&self) -> &str {
        "brightness-component-one"
    }
    fn state(&self) -> State {
        self.state.clone()
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        self.sent.push(intent.clone());
        if self.fail_send {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(true)
    }
    fn inspect(&mut self, _: u64) -> io::Result<bool> {
        Ok(false)
    }
    fn poll(&mut self) -> Option<Event> {
        None
    }
    fn revoke(&mut self) {
        self.revoked += 1;
        self.state.connected = false;
        self.state.available = false;
        self.state.requested_percent = None;
        self.state.observed_percent = None;
        self.state.actual_percent = None;
        self.state.until_ns = "0".into();
    }
}
struct Case {
    authority: Authority,
    disk: Disk,
    session: Session,
    provider: ObservedProvider,
    server: OwnedFd,
    client: OwnedFd,
    directory: PathBuf,
}
impl Case {
    fn new(grant: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let directory = PathBuf::from(std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").unwrap())
            .join(format!("brightness-{}", fresh_id().unwrap()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let (disk, durable) = Disk::open(&directory, &[brightness::APP.into()]).unwrap();
        let mut authority = Authority::loading(vec![AppProfile {
            app: brightness::APP.into(),
            uid: brightness::UID,
            executable: "/unused".into(),
            args: vec![],
        }])
        .unwrap();
        authority.activate(durable);
        authority
            .register(
                brightness::APP,
                std::process::id(),
                rustix::process::pidfd_open(
                    rustix::process::getpid(),
                    rustix::process::PidfdFlags::NONBLOCK,
                )
                .unwrap(),
            )
            .unwrap();
        authority.lifecycle(Some(false), None, None).unwrap();
        let (server, client) = net::socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&server, true).unwrap();
        net::sockopt::set_socket_passcred(&client, true).unwrap();
        let mut c = Self {
            authority,
            disk,
            session: Session::default(),
            provider: ObservedProvider {
                state: State {
                    connected: true,
                    available: true,
                    revision: 1,
                    pending: false,
                    requested_percent: None,
                    observed_percent: Some(25),
                    actual_percent: Some(24),
                    until_ns: (boottime_ns() + 2_000_000_000).to_string(),
                },
                sent: vec![],
                revoked: 0,
                fail_send: false,
            },
            server,
            client,
            directory,
        };
        if grant {
            c.grant(true)
        }
        c
    }
    fn grant(&mut self, allowed: bool) {
        let intent = self
            .authority
            .plan_grant(
                brightness::APP,
                Permission::DisplayBrightness,
                allowed,
                boottime_ns(),
            )
            .unwrap();
        self.disk.replace(&intent.next).unwrap();
        assert_eq!(
            self.authority.finish(intent, true, boottime_ns()),
            Reply::Ok
        );
    }
    fn request() -> Request {
        Request {
            percent: 75,
            expected_revision: 1,
            ttl_ms: 1000,
        }
    }
    fn begin(&mut self, request: Request) -> Result<u64, Reply> {
        self.session.begin(
            CONNECTION,
            std::process::id(),
            brightness::UID,
            request,
            &mut self.authority,
            &self.provider,
        )
    }
    fn sent(&mut self) -> Intent {
        self.begin(Self::request()).unwrap();
        assert!(
            self.session
                .drive(CONNECTION, &self.authority, &mut self.provider)
                .is_none()
        );
        assert_eq!(self.provider.sent.len(), 1);
        self.provider.sent[0].clone()
    }
    fn complete(&mut self, id: u64) -> Option<(u64, Reply)> {
        let state = State {
            connected: true,
            available: true,
            revision: 2,
            pending: false,
            requested_percent: Some(75),
            observed_percent: Some(74),
            actual_percent: Some(72),
            until_ns: (boottime_ns() + 1_000_000_000).to_string(),
        };
        self.provider.state = state.clone();
        self.session.observe(
            Event::Completed { id, state },
            &self.authority,
            &mut self.provider,
        )
    }
    fn ready(&mut self) -> Vec<u8> {
        let intent = self.sent();
        let (c, reply) = self.complete(intent.id).unwrap();
        assert_eq!(c, CONNECTION);
        assert!(
            matches!(&reply,Reply::BrightnessCompleted{state} if state.observed_percent==Some(74)&&state.actual_percent==Some(72))
        );
        serde_json::to_vec(&reply).unwrap()
    }
    fn check_input(&mut self) -> io::Result<()> {
        brightness_client_input(
            &mut self.session,
            CONNECTION,
            &self.server,
            &mut self.provider,
        )
    }
    fn assert_retired(&self) {
        assert!(!self.session.owns(CONNECTION));
        assert!(self.provider.revoked > 0);
        assert!(ipc::receive(&self.client).unwrap().is_none());
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}
#[test]
fn brightness_request_has_only_bounded_logical_fields() {
    for json in [
        r#"{"percent":75,"expected_revision":1,"ttl_ms":1000,"path":"/tmp/node"}"#,
        r#"{"percent":75,"expected_revision":1,"ttl_ms":1000,"uid":0}"#,
    ] {
        assert!(serde_json::from_str::<Request>(json).is_err());
    }
    let mut c = Case::new(true);
    for request in [
        Request {
            percent: 101,
            ..Case::request()
        },
        Request {
            expected_revision: 0,
            ..Case::request()
        },
        Request {
            expected_revision: 2,
            ..Case::request()
        },
        Request {
            ttl_ms: 0,
            ..Case::request()
        },
        Request {
            ttl_ms: 1001,
            ..Case::request()
        },
    ] {
        assert!(c.begin(request).is_err());
    }
    assert!(c.provider.sent.is_empty());
    let r = AppRequest::Prepare {
        permission: Permission::DisplayBrightness,
        ttl_ms: 1000,
    };
    assert!(
        matches!(
            c.authority
                .plan_request(std::process::id(), brightness::UID, r, boottime_ns()),
            crate::authority::Decision::Immediate(Reply::Denied)
        ),
        "generic permit must not admit a brightness effect"
    );
}
#[test]
fn brightness_exact_current_principal_and_independent_grant() {
    let mut c = Case::new(false);
    assert!(c.begin(Case::request()).is_err());
    c.grant(true);
    for (pid, uid) in [
        (std::process::id() + 1, brightness::UID),
        (std::process::id(), brightness::UID + 1),
    ] {
        assert!(
            c.session
                .begin(
                    CONNECTION,
                    pid,
                    uid,
                    Case::request(),
                    &mut c.authority,
                    &c.provider
                )
                .is_err()
        );
    }
    assert!(c.begin(Case::request()).is_ok());
    assert!(c.provider.sent.is_empty());
    assert!(
        c.session
            .begin(
                CONNECTION + 1,
                std::process::id(),
                brightness::UID,
                Case::request(),
                &mut c.authority,
                &c.provider
            )
            .is_err()
    );
}
#[test]
fn brightness_revoke_lock_background_suspend_source_loss_and_stop_fence_before_dispatch() {
    for loss in 0..6 {
        let mut c = Case::new(true);
        c.begin(Case::request()).unwrap();
        match loss {
            0 => c.grant(false),
            1 => c.authority.lifecycle(Some(true), None, None).unwrap(),
            2 => c
                .authority
                .lifecycle(None, None, Some((brightness::APP, true)))
                .unwrap(),
            3 => c.authority.lifecycle(None, Some(true), None).unwrap(),
            4 => c.authority.fail_session().unwrap(),
            _ => {}
        };
        let refused = c
            .session
            .fence(&c.authority, &mut c.provider, loss == 5, |_| true);
        assert!(matches!(refused, Some((CONNECTION, Reply::Denied))));
        assert!(c.provider.sent.is_empty());
        assert!(!c.session.owns(CONNECTION));
    }
}
#[test]
fn brightness_late_completion_never_revives_revoked_or_stopped_intent() {
    for stop in [false, true] {
        let mut c = Case::new(true);
        let intent = c.sent();
        if !stop {
            c.grant(false)
        };
        assert!(matches!(
            c.session
                .fence(&c.authority, &mut c.provider, stop, |_| true),
            Some((CONNECTION, Reply::Denied))
        ));
        assert!(c.complete(intent.id).is_none());
        assert!(
            c.session
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        assert_eq!(c.provider.sent.len(), 1);
    }
}
#[test]
fn brightness_original_expiry_without_timer_and_uncertain_send_have_no_resend() {
    let mut c = Case::new(true);
    c.begin(Request {
        ttl_ms: 1,
        ..Case::request()
    })
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));
    assert!(matches!(
        c.session.drive(CONNECTION, &c.authority, &mut c.provider),
        Some(Reply::Denied)
    ));
    assert!(c.provider.sent.is_empty());
    let mut c = Case::new(true);
    c.provider.fail_send = true;
    c.begin(Case::request()).unwrap();
    assert!(matches!(
        c.session.drive(CONNECTION, &c.authority, &mut c.provider),
        Some(Reply::Unavailable)
    ));
    assert!(
        c.session
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    assert_eq!(c.provider.sent.len(), 1);
}
#[test]
fn brightness_requires_fresh_readback_not_connected_or_requested_assumptions() {
    for bad in 0..5 {
        let mut c = Case::new(true);
        match bad {
            0 => c.provider.state.until_ns = "0".into(),
            1 => c.provider.state.until_ns = "01".into(),
            2 => c.provider.state.observed_percent = None,
            3 => c.provider.state.pending = true,
            _ => c.provider.state.connected = false,
        };
        assert!(c.begin(Case::request()).is_err());
        assert!(c.provider.sent.is_empty());
    }
    let mut c = Case::new(true);
    let _ = c.ready();
    assert_eq!(c.provider.state.observed_percent, Some(74));
    assert_eq!(c.provider.state.actual_percent, Some(72));
    assert!(!c.session.needs_drive(CONNECTION));
}
#[test]
fn brightness_actual_idle_request_half_permits_credentialed_readback_publication() {
    let mut c = Case::new(true);
    let reply = c.ready();
    c.check_input().unwrap();
    assert!(
        c.session
            .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false)
            .is_none()
    );
    ipc::send(&c.server, &reply).unwrap();
    c.session.published(CONNECTION);
    let (bytes, cred) = ipc::receive(&c.client).unwrap().unwrap();
    assert_eq!(bytes, reply);
    assert_eq!(cred.pid.as_raw_nonzero().get() as u32, std::process::id());
    assert_eq!(cred.uid.as_raw(), 0);
    assert!(!c.session.pending());
}
#[test]
fn brightness_actual_client_eof_and_extra_packet_cancel_queued_positive_reply() {
    for extra in [false, true] {
        let mut c = Case::new(true);
        let _ = c.ready();
        if extra {
            ipc::send(&c.client, b"unexpected second command").unwrap()
        } else {
            net::shutdown(&c.client, Shutdown::Write).unwrap();
            ipc::send(&c.server, b"kernel-half-close-control").unwrap();
            assert_eq!(
                ipc::receive(&c.client).unwrap().unwrap().0,
                b"kernel-half-close-control"
            );
        };
        assert!(c.check_input().is_err());
        c.assert_retired();
    }
}
#[test]
fn brightness_actual_eagain_rechecks_revocation_before_publication() {
    let mut c = Case::new(true);
    let reply = c.ready();
    let until = boottime_ns() + 100_000_000;
    let mut n = 0;
    loop {
        assert!(boottime_ns() < until);
        match ipc::send(&c.server, &reply) {
            Ok(()) => n += 1,
            Err(e) if e.raw_os_error() == Some(11) => break,
            Err(e) => panic!("{e}"),
        }
    }
    assert!(n > 0);
    for _ in 0..n {
        assert_eq!(ipc::receive(&c.client).unwrap().unwrap().0, reply)
    }
    c.grant(false);
    assert!(matches!(
        c.session
            .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false),
        Some(Reply::Denied)
    ));
    c.assert_retired();
}

#[test]
fn brightness_changed_unsent_revision_and_expired_readback_refuse_publication() {
    let mut c = Case::new(true);
    c.begin(Case::request()).unwrap();
    c.provider.state.revision = 2;
    let refusal = c.session.drive(CONNECTION, &c.authority, &mut c.provider);
    let stale_unsent_refused =
        matches!(refusal, Some(Reply::Denied | Reply::Unavailable)) && c.provider.sent.is_empty();
    let mut c = Case::new(true);
    let intent = c.sent();
    let mut state = c.provider.state.clone();
    state.revision = 2;
    state.requested_percent = Some(75);
    state.until_ns = (boottime_ns() + 10_000_000).to_string();
    c.provider.state = state.clone();
    assert!(matches!(
        c.session.observe(
            Event::Completed {
                id: intent.id,
                state
            },
            &c.authority,
            &mut c.provider
        ),
        Some((CONNECTION, Reply::BrightnessCompleted { .. }))
    ));
    std::thread::sleep(std::time::Duration::from_millis(15));
    let expired_readback_refused = c
        .session
        .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false)
        .is_some();
    println!(
        "stale_unsent_refused={stale_unsent_refused} expired_readback_refused={expired_readback_refused}"
    );
    assert!(
        stale_unsent_refused && expired_readback_refused,
        "BRIGHTNESS_ORIGINAL_READBACK_FRESHNESS_FENCE"
    );
}
