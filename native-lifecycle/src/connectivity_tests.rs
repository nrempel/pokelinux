//! Controlled Authority/Provider facts, actual self-pidfd and UNIX SEQPACKET.
//! session=None and logical UID11050/self-PID registration are authored test facts.
//! Disk grant replacement is real; no launched app, real Owner/backend, Source,
//! OS permission decision, GUI, network, ARM or hardware proof is claimed.
use super::connectivity_client_input;
use crate::{
    AppProfile, AppRequest, Permission, Reply,
    authority::Authority,
    boottime_ns,
    connectivity::{self, Event, Intent, Provider, Request, State},
    connectivity_session::Session,
    fresh_id, ipc,
    storage::Disk,
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, Shutdown, SocketFlags, SocketType},
};
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf, thread, time::Duration};
const CONNECTION: u64 = 81;
const LEASE_NS: u64 = 250_000_000;
fn observed(configured: bool) -> State {
    let at = boottime_ns();
    State {
        connected: true,
        available: true,
        configured: Some(configured),
        pending: false,
        observed_at_ns: at.to_string(),
        until_ns: (at + LEASE_NS).to_string(),
    }
}
struct ControlledProvider {
    state: State,
    sent: Vec<(u64, bool, u64)>,
    revoked: usize,
    incarnation: &'static str,
    own_inspect: bool,
    remote_pending: bool,
}
impl Provider for ControlledProvider {
    fn incarnation(&self) -> &str {
        self.incarnation
    }
    fn state(&self) -> State {
        self.state.clone()
    }
    fn readonly_in_flight(&self) -> bool {
        self.own_inspect && !self.remote_pending && self.state.is_current(boottime_ns())
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        self.sent
            .push((intent.id, intent.connected, intent.until_ns));
        self.remote_pending = true;
        self.state.pending = true;
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
        self.own_inspect = false;
        // Model only local withdrawal. Known remote work remains pending;
        // neither a Disconnect submission nor backend cleanup is fabricated.
        let pending = self.remote_pending;
        self.state = State::default();
        self.state.pending = pending;
    }
}
struct Case {
    authority: Authority,
    disk: Disk,
    session: Session,
    provider: ControlledProvider,
    server: OwnedFd,
    client: OwnedFd,
    directory: PathBuf,
}
impl Case {
    fn new(grant: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let directory = PathBuf::from(std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").unwrap())
            .join(format!("connectivity-{}", fresh_id().unwrap()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let (disk, durable) = Disk::open(&directory, &[connectivity::APP.into()]).unwrap();
        let mut authority = Authority::loading(vec![AppProfile {
            role: crate::AppRole::Application,
            app: connectivity::APP.into(),
            uid: connectivity::UID,
            executable: "/unused-component-fixture".into(),
            args: vec![],
        }])
        .unwrap();
        authority.activate(durable);
        authority
            .register(
                connectivity::APP,
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
            provider: ControlledProvider {
                state: observed(false),
                sent: vec![],
                revoked: 0,
                incarnation: "connectivity-component-one",
                own_inspect: false,
                remote_pending: false,
            },
            server,
            client,
            directory,
        };
        if grant {
            c.grant(true);
        }
        c
    }
    fn grant(&mut self, allowed: bool) {
        let intent = self
            .authority
            .plan_grant(
                connectivity::APP,
                Permission::NetworkControl,
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
            connected: true,
            until_ns: (boottime_ns() + 1_000_000_000).to_string(),
        }
    }
    fn begin(&mut self, request: Request) -> Result<u64, Reply> {
        self.session.begin(
            CONNECTION,
            std::process::id(),
            connectivity::UID,
            request,
            &mut self.authority,
            &self.provider,
        )
    }
    fn sent(&mut self) -> (u64, bool, u64) {
        self.begin(Self::request()).unwrap();
        assert!(
            self.session
                .drive(CONNECTION, &self.authority, &mut self.provider)
                .is_none()
        );
        assert_eq!(self.provider.sent.len(), 1);
        self.provider.sent[0]
    }
    fn complete(&mut self, id: u64) -> Option<(u64, Reply)> {
        let state = observed(true);
        self.provider.remote_pending = false;
        self.provider.state = state.clone();
        self.session.observe(
            Event::Completed { id, state },
            &self.authority,
            &mut self.provider,
        )
    }
    fn input(&mut self) -> io::Result<()> {
        connectivity_client_input(
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
fn connectivity_component_request_principal_grant_and_canonical_deadline() {
    for raw in [
        r#"{"connected":true,"until_ns":"1","path":"/sys/arbitrary"}"#,
        r#"{"connected":true,"until_ns":"1","uid":0}"#,
    ] {
        assert!(serde_json::from_str::<Request>(raw).is_err());
    }
    let mut c = Case::new(false);
    assert!(matches!(c.begin(Case::request()), Err(Reply::Denied)));
    c.grant(true);
    for raw in [
        "0".to_string(),
        "01".into(),
        "+1".into(),
        "18446744073709551616".into(),
        (boottime_ns() + 2_000_000_000).to_string(),
    ] {
        assert!(
            c.begin(Request {
                connected: true,
                until_ns: raw
            })
            .is_err()
        );
    }
    for (pid, uid) in [
        (std::process::id() + 1, connectivity::UID),
        (std::process::id(), connectivity::UID + 1),
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
    assert!(matches!(
        c.authority.plan_request(
            std::process::id(),
            connectivity::UID,
            AppRequest::Prepare {
                permission: Permission::NetworkControl,
                ttl_ms: 1000
            },
            boottime_ns()
        ),
        crate::authority::Decision::Immediate(Reply::Denied)
    ));
    assert!(c.provider.sent.is_empty());
}
#[test]
fn connectivity_component_readonly_reservation_preserves_private_occupancy() {
    let mut c = Case::new(true);
    c.provider.own_inspect = true;
    c.provider.state.pending = true;
    assert!(c.provider.readonly_in_flight());
    assert!(c.provider.state().pending);
    assert!(!c.session.state(CONNECTION, &c.provider).pending);
    c.begin(Case::request()).unwrap();
    assert!(c.session.state(CONNECTION, &c.provider).pending);
    assert!(
        c.session
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    assert!(c.provider.sent.is_empty());
    c.provider.own_inspect = false;
    c.provider.state = observed(false);
    assert!(
        c.session
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    assert_eq!(c.provider.sent.len(), 1);
    let mut busy = Case::new(true);
    busy.provider.own_inspect = true;
    busy.provider.remote_pending = true;
    busy.provider.state.pending = true;
    assert!(!busy.provider.readonly_in_flight());
    assert!(busy.session.state(CONNECTION, &busy.provider).pending);
    assert!(busy.begin(Case::request()).is_err());
    assert!(busy.provider.sent.is_empty());
}
#[test]
fn connectivity_component_revoke_before_send_and_late_completion_cannot_revive() {
    let mut c = Case::new(true);
    c.begin(Case::request()).unwrap();
    c.grant(false);
    assert!(matches!(
        c.session.drive(CONNECTION, &c.authority, &mut c.provider),
        Some(Reply::Denied)
    ));
    assert!(c.provider.sent.is_empty());
    for stopping in [false, true] {
        let mut c = Case::new(true);
        let (id, _, _) = c.sent();
        if !stopping {
            c.grant(false);
        }
        assert!(matches!(
            c.session
                .fence(&c.authority, &mut c.provider, stopping, |_| true),
            Some((CONNECTION, Reply::Denied))
        ));
        assert!(c.complete(id).is_none());
        assert!(
            c.session
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        assert_eq!(c.provider.sent.len(), 1);
        assert!(!c.session.owns(CONNECTION));
    }
}
#[test]
fn connectivity_component_actual_eof_or_extra_packet_before_send_prevents_submit() {
    for extra in [false, true] {
        let mut c = Case::new(true);
        c.begin(Case::request()).unwrap();
        if extra {
            ipc::send(&c.client, b"unexpected second command").unwrap();
        } else {
            net::shutdown(&c.client, Shutdown::Write).unwrap();
            ipc::send(&c.server, b"actual-request-half-close-control").unwrap();
            assert_eq!(
                ipc::receive(&c.client).unwrap().unwrap().0,
                b"actual-request-half-close-control"
            );
        }
        assert!(c.input().is_err());
        assert!(
            c.session
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        assert!(c.provider.sent.is_empty());
        c.assert_retired();
    }
}
#[test]
fn connectivity_component_actual_withdraw_after_send_never_submits_disconnect() {
    let mut c = Case::new(true);
    let (id, connected, _) = c.sent();
    assert!(connected);
    net::shutdown(&c.client, Shutdown::Write).unwrap();
    assert!(c.input().is_err());
    assert_eq!(c.provider.sent.len(), 1);
    assert!(c.provider.sent.iter().all(|(_, connected, _)| *connected));
    assert!(
        c.provider.state.pending,
        "controlled known remote custody is not cleanup proof"
    );
    assert!(c.complete(id).is_none());
    assert!(
        c.session
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    assert_eq!(c.provider.sent.len(), 1);
    c.assert_retired();
}
#[test]
fn connectivity_component_matching_reply_real_publication_and_wrong_id_refusal() {
    for wrong in [false, true] {
        let mut c = Case::new(true);
        let (id, _, _) = c.sent();
        let (connection, reply) = c.complete(id + u64::from(wrong)).unwrap();
        assert_eq!(connection, CONNECTION);
        if wrong {
            assert_eq!(reply, Reply::Unavailable);
            assert!(c.complete(id).is_none());
            c.assert_retired();
        } else {
            assert!(
                matches!(&reply, Reply::ConnectivityCompleted { state } if state.configured==Some(true))
            );
            c.input().unwrap();
            assert!(
                c.session
                    .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false)
                    .is_none()
            );
            let bytes = serde_json::to_vec(&reply).unwrap();
            ipc::send(&c.server, &bytes).unwrap();
            c.session.published(CONNECTION);
            let (actual, cred) = ipc::receive(&c.client).unwrap().unwrap();
            assert_eq!(actual, bytes);
            assert_eq!(cred.pid.as_raw_nonzero().get() as u32, std::process::id());
            assert_eq!(cred.uid.as_raw(), 0);
            assert!(!c.session.pending());
        }
    }
}
#[test]
fn connectivity_component_original_deadline_expires_unsent_and_sent_without_replay() {
    for sent in [false, true] {
        let mut c = Case::new(true);
        let end = boottime_ns() + 2_000_000;
        assert_eq!(
            c.begin(Request {
                connected: true,
                until_ns: end.to_string()
            })
            .unwrap(),
            end
        );
        if sent {
            assert!(
                c.session
                    .drive(CONNECTION, &c.authority, &mut c.provider)
                    .is_none()
            );
            assert_eq!(c.provider.sent[0].2, end);
        }
        thread::sleep(Duration::from_millis(3));
        assert!(boottime_ns() >= end);
        assert!(matches!(
            c.session.drive(CONNECTION, &c.authority, &mut c.provider),
            Some(Reply::Denied)
        ));
        if sent {
            let id = c.provider.sent[0].0;
            assert!(c.complete(id).is_none());
            assert_eq!(c.provider.sent.len(), 1);
        } else {
            assert!(c.provider.sent.is_empty());
        }
        assert!(!c.session.owns(CONNECTION));
    }
}
#[test]
fn connectivity_component_incarnation_change_fences_send_and_ready_publication() {
    let mut c = Case::new(true);
    c.begin(Case::request()).unwrap();
    c.provider.incarnation = "connectivity-component-two";
    assert!(matches!(
        c.session.drive(CONNECTION, &c.authority, &mut c.provider),
        Some(Reply::Denied)
    ));
    assert!(c.provider.sent.is_empty());
    let mut c = Case::new(true);
    let (id, _, _) = c.sent();
    assert!(matches!(
        c.complete(id),
        Some((CONNECTION, Reply::ConnectivityCompleted { .. }))
    ));
    c.provider.incarnation = "connectivity-component-two";
    assert!(matches!(
        c.session
            .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false),
        Some(Reply::Denied)
    ));
    c.assert_retired();
}
