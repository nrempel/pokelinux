//! Actual credentialed Linux SEQPACKET EOF/extra-packet/publication controls.
//! Authority facts and the audio provider are components; this is not a full
//! actor, PipeWire, Qt, ARM or physical audio test.
use super::media_client_input;
use crate::{
    AppProfile, Permission, Reply,
    audio::{self, Provider},
    audio_session::Session,
    authority::Authority,
    boottime_ns, fresh_id, ipc,
    storage::Disk,
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, Shutdown, SocketFlags, SocketType},
};
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf};

const CONNECTION: u64 = 71;
struct ObservedProvider {
    revoked: usize,
    sent: Option<u64>,
    played: Option<u64>,
    preview_intent: Option<audio::PreviewIntent>,
    preview_calls: usize,
    stopped: Option<u64>,
}
impl Provider for ObservedProvider {
    fn incarnation(&self) -> &str {
        "media-input-component"
    }
    fn state(&self) -> audio::State {
        audio::State {
            connected: self.revoked == 0,
            available: true,
            ..Default::default()
        }
    }
    fn routes(&self) -> Vec<audio::Route> {
        vec![audio::Route {
            id: "speaker".into(),
            label: "Speaker".into(),
        }]
    }
    fn submit(&mut self, intent: &audio::Intent) -> io::Result<bool> {
        self.sent = Some(intent.id);
        Ok(true)
    }
    fn play(&mut self, intent: &audio::PreviewIntent) -> io::Result<bool> {
        self.played = Some(intent.route_intent.id);
        self.preview_intent = Some(intent.clone());
        self.preview_calls += 1;
        Ok(true)
    }
    fn stop_preview(&mut self, id: u64, _: u64) -> io::Result<bool> {
        self.stopped = Some(id);
        Ok(true)
    }
    fn inspect(&mut self, _: u64) -> io::Result<bool> {
        Ok(false)
    }
    fn poll(&mut self) -> Option<audio::Event> {
        None
    }
    fn revoke(&mut self) {
        self.revoked += 1;
    }
    fn reconnect(&mut self) -> io::Result<()> {
        panic!("input check must not reconnect")
    }
}
struct Ready {
    authority: Authority,
    media: Session,
    provider: ObservedProvider,
    server: OwnedFd,
    client: OwnedFd,
    reply: Vec<u8>,
    directory: PathBuf,
}
impl Ready {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let directory = PathBuf::from(std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").unwrap())
            .join(format!("media-input-{}", fresh_id().unwrap()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let (mut disk, durable) = Disk::open(&directory, &[audio::APP.into()]).unwrap();
        let mut authority = Authority::loading(vec![AppProfile {
            app: audio::APP.into(),
            uid: 11042,
            executable: "/unused".into(),
            args: vec![],
        }])
        .unwrap();
        authority.activate(durable);
        authority
            .register(
                audio::APP,
                std::process::id(),
                rustix::process::pidfd_open(
                    rustix::process::getpid(),
                    rustix::process::PidfdFlags::NONBLOCK,
                )
                .unwrap(),
            )
            .unwrap();
        authority.lifecycle(Some(false), None, None).unwrap();
        let intent = authority
            .plan_grant(audio::APP, Permission::MediaRoute, true, boottime_ns())
            .unwrap();
        disk.replace(&intent.next).unwrap();
        assert_eq!(authority.finish(intent, true, boottime_ns()), Reply::Ok);
        let (server, client) = net::socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&server, true).unwrap();
        net::sockopt::set_socket_passcred(&client, true).unwrap();
        let mut provider = ObservedProvider {
            revoked: 0,
            sent: None,
            played: None,
            preview_intent: None,
            preview_calls: 0,
            stopped: None,
        };
        let mut media = Session::default();
        media
            .begin(
                CONNECTION,
                std::process::id(),
                11042,
                audio::Request {
                    route: "speaker".into(),
                    ttl_ms: 1000,
                },
                &mut authority,
                &provider,
            )
            .unwrap();
        assert!(media.drive(CONNECTION, &authority, &mut provider).is_none());
        let id = provider.sent.expect("explicit selected intent");
        let (connection, reply) = media
            .observe(
                audio::Event::Selected {
                    id,
                    state: provider.state(),
                },
                &authority,
                &mut provider,
            )
            .unwrap();
        assert_eq!(connection, CONNECTION);
        assert!(matches!(&reply, Reply::MediaSelected { .. }));
        assert!(
            !media.needs_drive(CONNECTION),
            "fixture must reach queued positive reply"
        );
        Self {
            authority,
            media,
            provider,
            server,
            client,
            reply: serde_json::to_vec(&reply).unwrap(),
            directory,
        }
    }
    fn check(&mut self) -> io::Result<()> {
        media_client_input(
            &mut self.media,
            CONNECTION,
            &self.server,
            &mut self.provider,
        )
    }
    fn assert_retired(&self) {
        assert!(
            !self.media.owns(CONNECTION),
            "pending connection survived observable client retirement"
        );
        assert_eq!(
            self.provider.revoked, 1,
            "provider connection was not withdrawn"
        );
        assert!(
            ipc::receive(&self.client).unwrap().is_none(),
            "positive reply was published"
        );
    }
}
impl Drop for Ready {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}
#[test]
fn media_ready_idle_request_half_allows_qualified_publication() {
    let mut c = Ready::new();
    c.check().unwrap();
    assert!(
        c.media
            .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false)
            .is_none()
    );
    ipc::send(&c.server, &c.reply).unwrap();
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    let (bytes, cred) = ipc::receive(&c.client).unwrap().unwrap();
    assert_eq!(bytes, c.reply);
    assert_eq!(cred.pid.as_raw_nonzero().get() as u32, std::process::id());
    assert!(c.media.owns(CONNECTION));
    assert_eq!(c.provider.revoked, 0);
}
#[test]
fn media_ready_write_half_close_cancels_before_positive_publication() {
    let mut c = Ready::new();
    net::shutdown(&c.client, Shutdown::Write).unwrap();
    // Kernel control: request EOF does not prevent a server send to the read half.
    ipc::send(&c.server, b"half-close-control").unwrap();
    assert_eq!(
        ipc::receive(&c.client).unwrap().unwrap().0,
        b"half-close-control"
    );
    assert_eq!(c.check().unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    c.assert_retired();
}
#[test]
fn media_ready_extra_credentialed_packet_cancels_before_publication() {
    let mut c = Ready::new();
    ipc::send(&c.client, br#"{"unexpected":"second request"}"#).unwrap();
    assert_eq!(c.check().unwrap_err().kind(), io::ErrorKind::InvalidData);
    c.assert_retired();
}
#[test]
fn media_ready_real_reply_eagain_rechecks_client_half_close() {
    let mut c = Ready::new();
    c.check().unwrap();
    let until = boottime_ns() + 100_000_000;
    let mut queued = 0;
    loop {
        assert!(boottime_ns() < until, "did not reach bounded real EAGAIN");
        match ipc::send(&c.server, &c.reply) {
            Ok(()) => queued += 1,
            Err(e) if e.raw_os_error() == Some(11) => break,
            Err(e) => panic!("unexpected send error: {e}"),
        }
    }
    assert!(queued > 0);
    assert_eq!(
        ipc::send(&c.server, &c.reply).unwrap_err().raw_os_error(),
        Some(11)
    );
    // The filled packets are transport controls, not session.published calls.
    // Drain them so only an illicit retry could yield another positive reply.
    for _ in 0..queued {
        assert_eq!(ipc::receive(&c.client).unwrap().unwrap().0, c.reply);
    }
    assert!(ipc::receive(&c.client).unwrap().is_none());
    net::shutdown(&c.client, Shutdown::Write).unwrap();
    assert_eq!(c.check().unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    c.assert_retired();
}

#[test]
fn preview_actual_request_half_close_cancels_before_dispatch() {
    let mut c = Ready::new();
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    c.media
        .begin_preview(
            CONNECTION,
            std::process::id(),
            11042,
            audio::PreviewRequest {
                route: "speaker".into(),
                ttl_ms: 1000,
                until_boottime_ns: boottime_ns() + 900_000_000,
                level_percent: 50,
                muted: false,
            },
            &mut c.authority,
            &c.provider,
        )
        .unwrap();
    net::shutdown(&c.client, Shutdown::Write).unwrap();
    assert_eq!(c.check().unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert!(c.provider.played.is_none());
    c.assert_retired();
}

#[test]
fn preview_stop_is_owned_cleanup_and_does_not_publish_a_new_play_approval() {
    let mut c = Ready::new();
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    assert_eq!(c.media.stop_preview(CONNECTION + 1), Err(Reply::Denied));
    c.media.stop_preview(CONNECTION).unwrap();
    c.check().unwrap();
    assert!(
        c.media
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    let id = c.provider.stopped.expect("explicit connection-owned Stop");
    assert!(c.provider.played.is_none());
    let mut state = c.provider.state();
    state.preview = audio::PreviewState::Retiring;
    let (_, reply) = c
        .media
        .observe(
            audio::Event::Preview { id, state },
            &c.authority,
            &mut c.provider,
        )
        .unwrap();
    assert!(
        matches!(reply, Reply::MediaPreview { state } if state.preview == audio::PreviewState::Retiring)
    );
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    assert!(c.media.owns(CONNECTION) && !c.media.pending());
    assert!(
        c.media
            .fence(&c.authority, &mut c.provider, false, |_| true)
            .is_none()
    );
    assert_eq!(c.provider.revoked, 0);
}

#[test]
fn preview_dispatch_preserves_admitted_level_route_and_absolute_deadline() {
    for muted in [false, true] {
        let mut c = Ready::new();
        c.media.published(CONNECTION, &c.authority, &mut c.provider);
        let until = boottime_ns() + 900_000_000;
        c.media
            .begin_preview(
                CONNECTION,
                std::process::id(),
                11042,
                audio::PreviewRequest {
                    route: "speaker".into(),
                    ttl_ms: 1000,
                    until_boottime_ns: until,
                    level_percent: 50,
                    muted,
                },
                &mut c.authority,
                &c.provider,
            )
            .unwrap();
        c.check().unwrap();
        assert!(
            c.media
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        let dispatched = c.provider.preview_intent.as_ref().unwrap();
        assert_eq!(
            dispatched.level,
            audio::PreviewLevel::new(50, muted).unwrap()
        );
        assert_eq!(dispatched.route_intent.route, "speaker");
        assert_eq!(dispatched.route_intent.until_ns, until);
        assert_eq!(c.provider.preview_calls, 1);
        assert!(
            c.media
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        assert_eq!(c.provider.preview_calls, 1);
    }
}
