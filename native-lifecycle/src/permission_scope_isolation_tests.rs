//! Actual controller credentials, storage Worker and retained audio UnixStream.
//! Source/app registration and network backend/remote state are authored facts.
//! Audio owner replies are authored observations decoded by the real Owner;
//! this does not claim a running audio Main, PCM, GUI or installed consent.
use super::*;
use crate::{audio::Provider as _, connectivity::Provider as _};
use std::os::unix::net::UnixStream;

const CONNECTION: u64 = 91;
const AUDIO_CONNECTION: u64 = 92;
const ROUTE: &str = "media-a7";
const GENERATION: u64 = 37;

fn focus(c: &mut ServiceCase, pid: u32, uid: u32) {
    let task = rustix::process::Pid::from_raw(pid as i32).unwrap();
    c.case.sample.revision += 1;
    c.case.sample.focus = Some(crate::session::OwnedFocus::new(
        crate::session::NativeFocus { pid, uid, gid: uid },
        rustix::process::pidfd_open(task, rustix::process::PidfdFlags::NONBLOCK).unwrap(),
    ));
    c.case.refresh();
}
fn register_authored_app(c: &mut ServiceCase, app: &str, uid: u32) {
    // Logical app identity/Source are authored; the retained self-pidfd is real.
    c.case
        .authority
        .register(
            app,
            std::process::id(),
            rustix::process::pidfd_open(
                rustix::process::getpid(),
                rustix::process::PidfdFlags::NONBLOCK,
            )
            .unwrap(),
        )
        .unwrap();
    focus(c, std::process::id(), uid);
}
fn write_network(
    c: &mut ServiceCase,
    allowed: bool,
    audio: &mut Option<&mut dyn crate::audio::Provider>,
) {
    let controller_pid = c.case.child.id();
    focus(c, controller_pid, crate::permission::UID);
    let until = boottime_ns() + 900_000_000;
    c.queue(allowed, until, audio);
    let reply = c.finish();
    let state = c.case.respond(reply);
    assert_eq!(state["target"], crate::connectivity::APP);
    assert_eq!(state["permission"], "network_control");
    assert_eq!(state["allowed"], allowed);
    assert!(boottime_ns() < until);
}
fn readback(configured: bool) -> crate::connectivity::State {
    let now = boottime_ns();
    crate::connectivity::State {
        connected: true,
        available: true,
        pending: false,
        configured: Some(configured),
        observed_at_ns: now.to_string(),
        until_ns: (now + 250_000_000).to_string(),
    }
}
struct Backend {
    state: crate::connectivity::State,
    sent: Vec<(u64, bool, u64)>,
    withdrawals: usize,
    remote_pending: bool,
    remote_configured: bool,
}
impl Backend {
    fn new() -> Self {
        Self {
            state: readback(false),
            sent: vec![],
            withdrawals: 0,
            remote_pending: false,
            remote_configured: false,
        }
    }
    fn complete(&mut self, configured: bool) -> crate::connectivity::State {
        assert!(self.remote_pending);
        self.remote_pending = false;
        self.remote_configured = configured;
        readback(configured)
    }
}
impl crate::connectivity::Provider for Backend {
    fn incarnation(&self) -> &str {
        "permission-scope-backend"
    }
    fn state(&self) -> crate::connectivity::State {
        self.state.clone()
    }
    fn readonly_in_flight(&self) -> bool {
        false
    }
    fn submit(&mut self, intent: &crate::connectivity::Intent) -> io::Result<bool> {
        self.sent
            .push((intent.id, intent.connected, intent.until_ns));
        self.remote_pending = true;
        self.state.pending = true;
        Ok(true)
    }
    fn inspect(&mut self, _: u64) -> io::Result<bool> {
        Ok(false)
    }
    fn poll(&mut self) -> Option<crate::connectivity::Event> {
        None
    }
    fn revoke(&mut self) {
        self.withdrawals += 1;
        self.state = crate::connectivity::State::default();
        // Local withdrawal cannot cancel transferred work or alter remote state.
    }
}
fn begin_network(
    c: &mut ServiceCase,
    session: &mut crate::connectivity_session::Session,
    backend: &Backend,
    connected: bool,
) -> u64 {
    session
        .begin(
            CONNECTION,
            std::process::id(),
            crate::connectivity::UID,
            crate::connectivity::Request {
                connected,
                until_ns: (boottime_ns() + 1_000_000_000).to_string(),
            },
            &mut c.case.authority,
            backend,
        )
        .unwrap()
}
fn connected_case() -> (ServiceCase, crate::connectivity_session::Session, Backend) {
    let mut c = ServiceCase::network_controller();
    write_network(&mut c, true, &mut None);
    register_authored_app(&mut c, crate::connectivity::APP, crate::connectivity::UID);
    (
        c,
        crate::connectivity_session::Session::default(),
        Backend::new(),
    )
}

#[test]
fn permission_scope_connectivity_forwards_original_deadline_over_private_channel() {
    use pokelinux_connectivity_control::protocol;
    use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType};

    // Real production Owner/Client and root credentialed channel. The Source,
    // app registration and backend observations remain authored test facts.
    let (mut c, mut session, _) = connected_case();
    let path = c.case.root.join("conn.sock");
    let listener = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&listener, true).unwrap();
    net::bind(&listener, &SocketAddrUnix::new(&path).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    net::listen(&listener, 1).unwrap();
    let mut owner = crate::connectivity::Owner::new(&path).unwrap();
    assert!(owner.inspect(boottime_ns() + protocol::SECOND).unwrap());
    let peer = net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
    let (bytes, credentials) = crate::ipc::receive(&peer).unwrap().unwrap();
    assert_eq!(credentials.uid.as_raw(), 0);
    assert_eq!(credentials.gid.as_raw(), 0);
    assert_eq!(
        credentials.pid.as_raw_nonzero().get() as u32,
        std::process::id()
    );
    let inspect: protocol::Request = serde_json::from_slice(&bytes).unwrap();
    assert!(matches!(inspect, protocol::Request::Inspect { .. }));
    let at = boottime_ns();
    let reply = protocol::Reply {
        schema: 1,
        id: inspect.id(),
        status: protocol::Status::Observed,
        state: protocol::State {
            available: true,
            configured: Some(false),
            observed_at_ns: at.to_string(),
            until_ns: (at + protocol::LEASE).to_string(),
            pending: false,
            terminal_uncertain: false,
            worker: "1".repeat(32),
        },
    };
    crate::ipc::send(&peer, &serde_json::to_vec(&reply).unwrap()).unwrap();
    assert!(matches!(
        owner.poll(),
        Some(crate::connectivity::Event::Observed)
    ));
    c.case.refresh();
    let captured_source_until = c.case.sample.until;
    let original = boottime_ns() + protocol::SECOND;
    session
        .begin(
            CONNECTION,
            std::process::id(),
            crate::connectivity::UID,
            crate::connectivity::Request {
                connected: true,
                until_ns: original.to_string(),
            },
            &mut c.case.authority,
            &owner,
        )
        .unwrap();
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut owner)
            .is_none()
    );
    let (bytes, _) = crate::ipc::receive(&peer).unwrap().unwrap();
    let apply: protocol::Request = serde_json::from_slice(&bytes).unwrap();
    let forwarded = match apply {
        protocol::Request::Apply {
            choice: protocol::Choice::Connect,
            until_ns,
            ..
        } => protocol::decimal(&until_ns).unwrap(),
        _ => panic!("expected one explicit Connect"),
    };
    session.retire(&mut owner);
    // Withdrawal adds no implicit Disconnect/retry packet.
    let mut trailing = [0; protocol::LIMIT];
    assert!(matches!(
        net::recv(&peer, &mut trailing, net::RecvFlags::DONTWAIT),
        Ok((0, _)) | Err(rustix::io::Errno::AGAIN)
    ));
    drop(owner);
    drop(peer);
    drop(listener);
    c.clean_reopen();
    drop(c);
    eprintln!(
        "connectivity deadline wire: original={original} captured_source_until={captured_source_until} forwarded={forwarded}"
    );
    assert!(captured_source_until < original);
    assert_eq!(
        forwarded, original,
        "transferred action must retain the original app cutoff"
    );
}

#[test]
fn permission_scope_unsent_connectivity_stays_unsent_after_revoke_and_regrant() {
    let (mut c, mut session, mut backend) = connected_case();
    let original = begin_network(&mut c, &mut session, &backend, true);
    assert!(session.pending() && backend.sent.is_empty());
    assert!(original > c.case.sample.until);
    write_network(&mut c, false, &mut None);
    assert_eq!(
        session.drive(CONNECTION, &c.case.authority, &mut backend),
        Some(Reply::Denied)
    );
    assert!(backend.sent.is_empty() && !backend.remote_pending);
    assert_eq!(backend.withdrawals, 1);
    assert!(!session.owns(CONNECTION));
    write_network(&mut c, true, &mut None);
    focus(&mut c, std::process::id(), crate::connectivity::UID);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert!(
        backend.sent.is_empty(),
        "REGRANT_MUST_NOT_REPLAY_UNSENT_CONNECTIVITY"
    );
    c.clean_reopen();
}

#[test]
fn permission_scope_sent_connectivity_retains_remote_custody_without_publication_or_retry() {
    let (mut c, mut session, mut backend) = connected_case();
    let original = begin_network(&mut c, &mut session, &backend, true);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(backend.sent.len(), 1);
    let sent = backend.sent[0];
    assert_eq!(sent.2, original);
    assert!(sent.1 && backend.remote_pending);
    write_network(&mut c, false, &mut None);
    assert_eq!(
        session.fence(&c.case.authority, &mut backend, false, |_| true),
        Some((CONNECTION, Reply::Denied))
    );
    assert!(
        backend.remote_pending,
        "WITHDRAWAL_MUST_NOT_FABRICATE_REMOTE_CANCELLATION"
    );
    let late = backend.complete(true);
    assert!(
        session
            .observe(
                crate::connectivity::Event::Completed {
                    id: sent.0,
                    state: late
                },
                &c.case.authority,
                &mut backend
            )
            .is_none()
    );
    assert!(
        session
            .revalidate_reply(CONNECTION, &c.case.authority, &mut backend, false)
            .is_none()
    );
    assert!(!session.owns(CONNECTION));
    assert!(backend.remote_configured && !backend.state().available);
    write_network(&mut c, true, &mut None);
    focus(&mut c, std::process::id(), crate::connectivity::UID);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(
        backend.sent,
        vec![sent],
        "REVOKE_OR_REGRANT_MUST_NOT_SEND_DISCONNECT_OR_RETRY"
    );
    c.clean_reopen();
}

#[test]
fn permission_scope_established_connectivity_survives_revoke_until_new_explicit_action() {
    let (mut c, mut session, mut backend) = connected_case();
    let original = begin_network(&mut c, &mut session, &backend, true);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    let sent = backend.sent[0];
    assert_eq!(sent.2, original);
    let state = backend.complete(true);
    backend.state = state.clone();
    assert!(matches!(
        session.observe(
            crate::connectivity::Event::Completed { id: sent.0, state },
            &c.case.authority,
            &mut backend
        ),
        Some((CONNECTION, Reply::ConnectivityCompleted { .. }))
    ));
    assert!(
        session
            .revalidate_reply(CONNECTION, &c.case.authority, &mut backend, false)
            .is_none()
    );
    session.published(CONNECTION);
    assert!(session.owns(CONNECTION) && !session.pending());
    write_network(&mut c, false, &mut None);
    assert!(
        session
            .fence(&c.case.authority, &mut backend, false, |_| true)
            .is_none()
    );
    assert!(!session.owns(CONNECTION));
    assert!(backend.remote_configured && !backend.remote_pending);
    write_network(&mut c, true, &mut None);
    focus(&mut c, std::process::id(), crate::connectivity::UID);
    // A fresh readonly backend fact is authored; it is not a replayed action.
    backend.state = readback(backend.remote_configured);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(
        backend.sent,
        vec![sent],
        "REGRANT_MUST_NOT_MUTATE_ESTABLISHED_ASSOCIATION"
    );
    let explicit = begin_network(&mut c, &mut session, &backend, false);
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(backend.sent.len(), 2);
    assert!(!backend.sent[1].1 && backend.sent[1].2 == explicit);
    assert!(backend.remote_configured && backend.remote_pending);
    let state = backend.complete(false);
    backend.state = state.clone();
    assert!(matches!(
        session.observe(
            crate::connectivity::Event::Completed {
                id: backend.sent[1].0,
                state
            },
            &c.case.authority,
            &mut backend
        ),
        Some((CONNECTION, Reply::ConnectivityCompleted { .. }))
    ));
    session.published(CONNECTION);
    assert!(!backend.remote_configured && !backend.remote_pending);
    c.clean_reopen();
}

// Exact normal audio socket response shapes from native-audio/tests/client.rs.
// Both wire observations and Source identity below remain authored test facts.
fn audio_status(lease: Option<u64>, inspected: bool, playing: bool) -> Value {
    let admitted = lease.is_some() || playing;
    let mut value = json!({"ok":true,"lease":lease,"peer_uid":rustix::process::getuid().as_raw(),
        "desired":if admitted { json!([[7,"media",ROUTE]]) } else { json!([]) },
        "observed":{"epoch":1,"revision":if admitted{2}else{1},"available":true,"links":[]},
        "intent_revision":if admitted{1}else{0},"application":if playing{"observed"}else{"pending"},
        "phase":"running","cleanup":"none","policy_settled":admitted,
        "preview":if playing{"playing"}else if admitted{"starting"}else{"idle"},
        "preview_generation":if admitted{json!(GENERATION)}else{Value::Null},
        "preview_muted":if admitted{json!(false)}else{Value::Null}});
    if inspected {
        value["media_routes"] = json!([{"id":ROUTE,"label":"Speaker"}]);
    }
    value
}
fn audio_request(peer: &UnixStream) -> Value {
    let mut bytes = [0; 4096];
    let (_, n) = rustix::net::recv(peer, &mut bytes[..], rustix::net::RecvFlags::DONTWAIT).unwrap();
    assert!(n > 0 && bytes[n - 1] == b'\n');
    serde_json::from_slice(&bytes[..n - 1]).unwrap()
}
fn audio_reply(peer: &mut UnixStream, value: Value) {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    peer.write_all(&bytes).unwrap();
}
fn audio_quiet_and_open(peer: &UnixStream) {
    let mut byte = [0];
    assert_eq!(
        rustix::net::recv(peer, &mut byte[..], rustix::net::RecvFlags::DONTWAIT),
        Err(rustix::io::Errno::AGAIN),
        "NETWORK_SCOPE_MUST_NOT_CLOSE_OR_WRITE_AUDIO_TRANSPORT"
    );
}

#[test]
fn permission_scope_network_grant_revoke_keeps_live_settled_audio_lease_and_transport() {
    let mut c = ServiceCase::network_controller();
    c.authored_audio_history();
    register_authored_app(&mut c, crate::audio::APP, 11042);
    let (stream, mut peer) = UnixStream::pair().unwrap();
    peer.set_nonblocking(true).unwrap();
    let mut owner =
        crate::audio::Owner::from_stream(stream, Path::new("/unused-scope-audio.sock"), 0).unwrap();
    assert!(owner.inspect(boottime_ns() + 900_000_000).unwrap());
    assert_eq!(audio_request(&peer), json!({"op":"status"}));
    audio_reply(&mut peer, audio_status(None, true, false));
    assert!(matches!(owner.poll(), Some(crate::audio::Event::Observed)));
    let mut media = crate::audio_session::Session::default();
    let app_until = boottime_ns() + 900_000_000;
    media
        .begin_preview(
            AUDIO_CONNECTION,
            std::process::id(),
            11042,
            crate::audio::PreviewRequest {
                route: ROUTE.into(),
                ttl_ms: 1000,
                until_boottime_ns: app_until,
                level_percent: 50,
                muted: false,
            },
            &mut c.case.authority,
            &owner,
        )
        .unwrap();
    assert!(
        media
            .drive(AUDIO_CONNECTION, &c.case.authority, &mut owner)
            .is_none()
    );
    let request = audio_request(&peer);
    assert_eq!(request["op"], "preview_play");
    assert_eq!(request["route"], ROUTE);
    assert!(request["until_boottime_ns"].as_u64().unwrap() <= app_until);
    audio_reply(&mut peer, audio_status(Some(7), false, false));
    let event = owner.poll().expect("real Owner decoded preview admission");
    assert!(matches!(
        media.observe(event, &c.case.authority, &mut owner),
        Some((AUDIO_CONNECTION, Reply::MediaPreview { .. }))
    ));
    assert!(
        media
            .revalidate_reply(AUDIO_CONNECTION, &c.case.authority, &mut owner, false)
            .is_none()
    );
    media.published(AUDIO_CONNECTION, &c.case.authority, &mut owner);
    assert!(owner.inspect(boottime_ns() + 900_000_000).unwrap());
    assert_eq!(audio_request(&peer), json!({"op":"status"}));
    audio_reply(&mut peer, audio_status(None, true, true));
    assert!(matches!(owner.poll(), Some(crate::audio::Event::Observed)));
    let initial = owner.state();
    assert!(initial.connected && initial.available && !initial.pending && initial.policy_settled);
    assert_eq!(initial.applied.as_deref(), Some(ROUTE));
    assert_eq!(initial.preview, crate::audio::PreviewState::Playing);
    assert_eq!(initial.preview_generation, Some(GENERATION));
    assert!(media.owns(AUDIO_CONNECTION) && !media.pending_on(AUDIO_CONNECTION));
    let incarnation = owner.incarnation().to_owned();
    // published() obtained the actual retained lease through approve_media;
    // each fence below validates that same lease via current_media_lease.
    for allowed in [true, false, true] {
        write_network(&mut c, allowed, &mut Some(&mut owner));
        assert_eq!(owner.incarnation(), incarnation);
        assert_eq!(
            owner.state(),
            initial,
            "NETWORK_SCOPE_MUST_KEEP_SETTLED_AUDIO_TRANSPORT"
        );
        assert!(
            media
                .fence(&c.case.authority, &mut owner, false, |_| true)
                .is_none()
        );
        assert!(media.owns(AUDIO_CONNECTION) && !media.pending_on(AUDIO_CONNECTION));
        assert_eq!(media.state(AUDIO_CONNECTION, &owner), initial);
        assert!(owner.poll().is_none());
        audio_quiet_and_open(&peer);
        let durable: Durable =
            serde_json::from_slice(&fs::read(c.policy.join("policy.json")).unwrap()).unwrap();
        assert!(durable.grants.iter().any(|g| g.app == crate::audio::APP
            && g.permission == crate::Permission::MediaRoute
            && g.allowed));
    }
    c.clean_reopen();
    assert_eq!(owner.state(), initial);
    audio_quiet_and_open(&peer);
}

// Authored Source/backend controls only. The separate real Owner/SEQPACKET
// test above qualifies deadline forwarding; these isolate old stamp expiry.
fn refresh_past_captured_connectivity_source(
    c: &mut ServiceCase,
    captured_until: u64,
    original_until: u64,
) {
    let source = c.case.sample.source.clone();
    let revision = c.case.sample.revision;
    let sequence = c.case.sample.sequence;
    assert!(boottime_ns() < captured_until && captured_until < original_until);
    while boottime_ns() < captured_until {
        assert!(boottime_ns() < original_until);
        c.case.refresh();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // refresh() uses the actual Source SAMPLE_BUDGET_NS (100ms). Updating the
    // current sample must not extend the authority captured by this action.
    c.case.refresh();
    let now = boottime_ns();
    assert!(captured_until <= now && now < original_until);
    assert!(c.case.sample.sampled_at_ns >= captured_until && now < c.case.sample.until);
    assert_eq!(c.case.sample.source, source);
    assert_eq!(c.case.sample.revision, revision);
    assert!(c.case.sample.sequence > sequence);
    assert_eq!(
        c.case.sample.until,
        c.case.sample.sampled_at_ns + crate::session::SAMPLE_BUDGET_NS
    );
}

#[test]
fn permission_scope_captured_source_expiry_before_send_keeps_current_inputs_unsent() {
    let (mut c, mut session, mut backend) = connected_case();
    c.case.refresh();
    let captured_until = c.case.sample.until;
    let original_until = boottime_ns() + 1_000_000_000;
    assert_eq!(
        session.begin(
            CONNECTION,
            std::process::id(),
            crate::connectivity::UID,
            crate::connectivity::Request {
                connected: true,
                until_ns: original_until.to_string(),
            },
            &mut c.case.authority,
            &backend,
        ),
        Ok(original_until)
    );
    assert!(session.pending() && backend.sent.is_empty());
    refresh_past_captured_connectivity_source(&mut c, captured_until, original_until);
    backend.state = readback(false);
    assert!(backend.state().is_current(boottime_ns()) && !backend.state().pending);
    assert!(c.case.authority.connectivity_allowed(
        std::process::id(),
        crate::connectivity::UID,
        backend.incarnation()
    ));
    assert_eq!(
        session.drive(CONNECTION, &c.case.authority, &mut backend),
        Some(Reply::Denied)
    );
    assert!(backend.sent.is_empty() && !backend.remote_pending);
    assert_eq!(backend.withdrawals, 1);
    assert!(!session.owns(CONNECTION) && !session.pending());
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert!(
        backend.sent.is_empty(),
        "fresh current Source must not replay old intent"
    );
    assert!(boottime_ns() < original_until);
    c.clean_reopen();
}

#[test]
fn permission_scope_captured_source_expiry_after_send_keeps_remote_custody() {
    let (mut c, mut session, mut backend) = connected_case();
    c.case.refresh();
    let captured_until = c.case.sample.until;
    let original_until = boottime_ns() + 1_000_000_000;
    assert_eq!(
        session.begin(
            CONNECTION,
            std::process::id(),
            crate::connectivity::UID,
            crate::connectivity::Request {
                connected: true,
                until_ns: original_until.to_string(),
            },
            &mut c.case.authority,
            &backend,
        ),
        Ok(original_until)
    );
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(backend.sent.len(), 1);
    let sent = backend.sent[0];
    assert!(sent.1 && sent.2 == original_until && backend.remote_pending);
    refresh_past_captured_connectivity_source(&mut c, captured_until, original_until);
    backend.state = readback(false);
    backend.state.pending = backend.remote_pending;
    assert!(backend.state().is_current(boottime_ns()) && backend.state().pending);
    assert!(c.case.authority.connectivity_allowed(
        std::process::id(),
        crate::connectivity::UID,
        backend.incarnation()
    ));
    assert_eq!(
        session.fence(&c.case.authority, &mut backend, false, |_| true),
        Some((CONNECTION, Reply::Denied))
    );
    assert!(backend.remote_pending && !backend.remote_configured);
    assert_eq!(backend.withdrawals, 1);
    let late = backend.complete(true);
    assert!(late.is_current(boottime_ns()));
    assert!(
        session
            .observe(
                crate::connectivity::Event::Completed {
                    id: sent.0,
                    state: late
                },
                &c.case.authority,
                &mut backend,
            )
            .is_none()
    );
    assert!(
        session
            .revalidate_reply(CONNECTION, &c.case.authority, &mut backend, false)
            .is_none()
    );
    session.published(CONNECTION);
    assert!(!session.owns(CONNECTION) && !session.pending());
    assert!(
        session
            .drive(CONNECTION, &c.case.authority, &mut backend)
            .is_none()
    );
    assert_eq!(
        backend.sent,
        vec![sent],
        "no Disconnect or retry after withdrawal"
    );
    assert!(backend.remote_configured && !backend.state().available);
    assert!(boottime_ns() < original_until);
    c.clean_reopen();
}
