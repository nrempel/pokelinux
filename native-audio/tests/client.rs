//! Actual Linux sockets and original Main wire; no fake IO or injected clock.
use pokelinux_audio::PreviewLevel;
use pokelinux_audio::client::{Client, Event, Failure, SendOutcome, Snapshot, SubmitError};
use pokelinux_power::clock::{BootClock, Clock};
use rustix::net::{RecvFlags, SendFlags, recv, send, sockopt};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn now() -> u64 {
    BootClock.now().elapsed().as_nanos().try_into().unwrap()
}
fn until() -> u64 {
    now() + 2_000_000_000
}
fn uid() -> u32 {
    rustix::process::getuid().as_raw()
}
fn pair() -> (Client, UnixStream, UnixStream) {
    let (a, b) = UnixStream::pair().unwrap();
    let alias = a.try_clone().unwrap();
    b.set_nonblocking(true).unwrap();
    (Client::from_stream(a, uid()).unwrap(), b, alias)
}
fn request(peer: &UnixStream) -> Value {
    let mut bytes = [0; 4096];
    let (_, n) = recv(peer, &mut bytes[..], RecvFlags::DONTWAIT).unwrap();
    assert!(
        n > 0 && bytes[n - 1] == b'\n',
        "actual complete one-shot request"
    );
    serde_json::from_slice(&bytes[..n - 1]).unwrap()
}
fn quiet(peer: &UnixStream) {
    let mut byte = [0];
    assert_eq!(
        recv(peer, &mut byte[..], RecvFlags::DONTWAIT),
        Err(rustix::io::Errno::AGAIN),
        "poll must never emit hidden requests"
    );
}
fn reply(peer: &mut UnixStream, value: Value) {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    peer.write_all(&bytes).unwrap();
}
fn catalog() -> Value {
    json!([{"id":"media-a7","label":"Speaker"},{"id":"media-z2","label":"Headphones"}])
}
fn status(
    lease: Option<u64>,
    route: &str,
    intent: u64,
    application: &str,
    inspected: bool,
) -> Value {
    let mut v = json!({"ok":true,"lease":lease,"peer_uid":uid(),"desired":if route.is_empty(){json!([])}else{json!([[7,"media",route]])},"observed":{"epoch":1,"revision":intent+1,"available":true,"links":[]},"intent_revision":intent,"application":application,"phase":"running","cleanup":"none","policy_settled":false,"preview":"idle","preview_generation":null,"preview_muted":null});
    if inspected {
        v["media_routes"] = catalog();
    }
    v
}
fn updated(c: &mut Client) -> Snapshot {
    match c.poll().expect("actual response available") {
        Event::Updated(s) => s,
        event => panic!("expected valid owner response, got {event:?}"),
    }
}
fn inspected(c: &mut Client, p: &mut UnixStream) {
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    assert_eq!(request(p), json!({"op":"status"}));
    reply(p, status(None, "", 0, "pending", true));
    assert_eq!(updated(c).media_routes.len(), 2);
}
fn applied(c: &mut Client, p: &mut UnixStream) -> Snapshot {
    inspected(c, p);
    assert_eq!(c.select("media-a7", until()), Ok(SendOutcome::Sent));
    assert_eq!(
        request(p),
        json!({"op":"acquire","role":"media","route":"media-a7"})
    );
    reply(p, status(Some(7), "media-a7", 1, "pending", false));
    let admitted = updated(c);
    assert!(admitted.applied.is_none());
    assert_eq!(admitted.admitted.as_ref().unwrap().lease, 7);
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    request(p);
    reply(p, status(None, "media-a7", 1, "observed", true));
    let current = updated(c);
    assert!(current.applied.is_some());
    current
}
fn failure(c: &mut Client, expected: Failure) {
    match c.poll() {
        Some(Event::Unavailable { reason, snapshot }) => {
            assert_eq!(reason, expected);
            assert!(!snapshot.connected && snapshot.uncertain);
            assert!(snapshot.admitted.is_none() && snapshot.applied.is_none());
        }
        event => panic!("actual expired/invalid transport must fail closed, got {event:?}"),
    }
}

#[test]
fn kernel_uid_and_fresh_connection_incarnation() {
    let (a, _) = UnixStream::pair().unwrap();
    assert_eq!(
        Client::from_stream(a, uid().checked_add(1).unwrap())
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let (first, _, _) = pair();
    let (second, _, _) = pair();
    assert_ne!(first.snapshot().incarnation, second.snapshot().incarnation);
    assert!(first.snapshot().media_routes.is_empty() && first.snapshot().admitted.is_none());
}
#[test]
fn accepted_intent_current_application_and_catalog_only_status() {
    let (mut c, mut p, _alias) = pair();
    let first = applied(&mut c, &mut p);
    assert_eq!(first.admitted, first.applied);
    quiet(&p);
    for _ in 0..4 {
        assert!(c.poll().is_none());
    }
    quiet(&p);
    assert_eq!(c.select("media-z2", until()), Ok(SendOutcome::Sent));
    assert_eq!(
        request(&p),
        json!({"op":"route","lease":7,"route":"media-z2"})
    );
    assert!(c.snapshot().applied.is_none());
    reply(&mut p, status(None, "media-z2", 2, "pending", false));
    let admitted = updated(&mut c);
    assert_eq!(admitted.media_routes, first.media_routes);
    assert_eq!(admitted.admitted.unwrap().route, "media-z2");
    assert!(admitted.applied.is_none());
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    request(&p);
    reply(&mut p, status(None, "media-z2", 2, "observed", true));
    assert_eq!(updated(&mut c).applied.unwrap().route, "media-z2");
}
#[test]
fn real_eagain_preserves_prior_lease_and_poll_never_flushes() {
    let (mut c, mut p, alias) = pair();
    let before = applied(&mut c, &mut p);
    sockopt::set_socket_send_buffer_size(&alias, 4096).unwrap();
    let mut filled = 0;
    loop {
        match send(
            &alias,
            &[b'F'; 1024],
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        ) {
            Ok(n) => filled += n,
            Err(rustix::io::Errno::AGAIN) => break,
            result => panic!("real send queue fill {result:?}"),
        }
    }
    loop {
        match send(&alias, b"F", SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
            Ok(1) => filled += 1,
            Err(rustix::io::Errno::AGAIN) => break,
            result => panic!("real final queue fill {result:?}"),
        }
    }
    assert!(filled > 0);
    assert_eq!(c.select("media-z2", until()), Ok(SendOutcome::WouldBlock));
    assert_eq!(c.snapshot(), before);
    assert!(c.poll().is_none());
    let mut drained = Vec::new();
    loop {
        let mut bytes = [0; 1024];
        match recv(&p, &mut bytes[..], RecvFlags::DONTWAIT) {
            Ok((_, 0)) => panic!("unexpected EOF"),
            Ok((_, n)) => drained.extend_from_slice(&bytes[..n]),
            Err(rustix::io::Errno::AGAIN) => break,
            result => panic!("actual drain {result:?}"),
        }
    }
    assert_eq!(drained, vec![b'F'; filled]);
    for _ in 0..5 {
        assert!(c.poll().is_none());
    }
    quiet(&p);
    assert_eq!(c.snapshot(), before);
    assert_eq!(c.select("media-z2", until()), Ok(SendOutcome::Sent));
    assert_eq!(request(&p)["op"], "route");
}
#[test]
fn presend_expiry_and_invalid_selection_send_nothing_and_keep_prior_approval() {
    let (mut c, mut p, _) = pair();
    let before = applied(&mut c, &mut p);
    assert_eq!(c.select("media-z2", now()), Err(SubmitError::Expired));
    assert_eq!(c.snapshot(), before);
    assert_eq!(
        c.select("private.node:port", until()),
        Err(SubmitError::InvalidRoute)
    );
    assert_eq!(c.snapshot(), before);
    quiet(&p);
}
#[test]
fn original_deadline_rejects_real_buffered_late_reply_without_timer_delivery() {
    let (mut c, mut p, _) = pair();
    let until_ns = now() + 100_000_000;
    assert_eq!(c.inspect(until_ns), Ok(SendOutcome::Sent));
    request(&p);
    sleep(Duration::from_millis(150));
    assert!(now() >= until_ns);
    reply(&mut p, status(None, "", 0, "pending", true));
    failure(&mut c, Failure::Deadline);
    let mut byte = [0];
    assert!(
        matches!(
            recv(&p, &mut byte[..], RecvFlags::DONTWAIT),
            Ok((_, 0)) | Err(rustix::io::Errno::CONNRESET)
        ),
        "closing with unread late bytes must terminate the peer connection"
    );
}
#[test]
fn original_deadline_expires_with_no_response_and_no_timer() {
    let (mut c, p, _) = pair();
    let until_ns = now() + 100_000_000;
    assert_eq!(c.inspect(until_ns), Ok(SendOutcome::Sent));
    request(&p);
    sleep(Duration::from_millis(150));
    failure(&mut c, Failure::Deadline);
}
#[test]
fn one_inflight_request_fragmented_reply_and_no_catalog_before_inspect() {
    let (mut c, mut p, _) = pair();
    assert_eq!(c.select("media-a7", until()), Err(SubmitError::NoCatalog));
    quiet(&p);
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    request(&p);
    assert_eq!(c.inspect(until()), Err(SubmitError::Busy));
    assert_eq!(c.select("media-a7", until()), Err(SubmitError::Busy));
    quiet(&p);
    let bytes = serde_json::to_vec(&status(None, "", 0, "pending", true)).unwrap();
    p.write_all(&bytes[..10]).unwrap();
    assert!(c.poll().is_none());
    quiet(&p);
    p.write_all(&bytes[10..]).unwrap();
    p.write_all(b"\n").unwrap();
    assert_eq!(updated(&mut c).media_routes.len(), 2);
}
#[test]
fn current_status_withdraws_applied_but_call_preemption_retains_admission() {
    let (mut c, mut p, _) = pair();
    let before = applied(&mut c, &mut p);
    c.inspect(until()).unwrap();
    request(&p);
    let mut v = status(None, "", 2, "observed", true);
    v["desired"] = json!([[8, "call", "call-only"]]);
    reply(&mut p, v);
    let during = updated(&mut c);
    assert_eq!(during.admitted, before.admitted);
    assert!(during.applied.is_none());
    c.inspect(until()).unwrap();
    request(&p);
    reply(&mut p, status(None, "media-a7", 3, "observed", true));
    assert_eq!(updated(&mut c).applied, before.applied);
    c.inspect(until()).unwrap();
    request(&p);
    reply(&mut p, status(None, "media-a7", 3, "pending", true));
    assert!(updated(&mut c).applied.is_none());
    c.inspect(until()).unwrap();
    request(&p);
    let mut unavailable = status(None, "media-a7", 3, "uncertain", true);
    unavailable["observed"]["available"] = json!(false);
    reply(&mut p, unavailable);
    let s = updated(&mut c);
    assert!(s.uncertain && !s.observation.unwrap().available && s.applied.is_none());
}
#[test]
fn revoke_is_terminal_eof_without_release_reconcile_or_successor_inheritance() {
    let (mut c, mut p, alias) = pair();
    let before = applied(&mut c, &mut p);
    c.revoke();
    let after = c.snapshot();
    assert!(
        !after.connected && !after.in_flight && after.admitted.is_none() && after.applied.is_none()
    );
    assert_eq!(c.inspect(until()), Err(SubmitError::Closed));
    assert_eq!(c.select("media-a7", until()), Err(SubmitError::Closed));
    assert!(c.poll().is_none());
    let mut byte = [0];
    assert_eq!(recv(&p, &mut byte[..], RecvFlags::DONTWAIT).unwrap().1, 0);
    drop(alias);
    let (successor, _, _) = pair();
    assert_ne!(successor.snapshot().incarnation, before.incarnation);
    assert!(
        successor.snapshot().admitted.is_none() && successor.snapshot().media_routes.is_empty()
    );
}
#[test]
fn real_peer_close_write_error_is_uncertain_without_sigpipe_or_retry() {
    let (mut c, mut p, _) = pair();
    applied(&mut c, &mut p);
    drop(p);
    assert_eq!(
        c.select("media-z2", until()),
        Err(SubmitError::Uncertain(Failure::Write))
    );
    assert!(!c.snapshot().connected && c.snapshot().uncertain);
    assert!(c.poll().is_none());
}
#[test]
fn bounded_catalog_and_frame_reject_malformed_actual_bytes() {
    let mut duplicate = status(None, "", 0, "pending", true);
    duplicate["media_routes"] = json!([{"id":"x","label":"One"},{"id":"x","label":"Two"}]);
    let mut private = status(None, "", 0, "pending", true);
    private["media_routes"] = json!([{"id":"logical","label":"One","target":"private.node"}]);
    let mut bad_label = status(None, "", 0, "pending", true);
    bad_label["media_routes"] = json!([{"id":"logical","label":"x".repeat(97)}]);
    let mut excessive = status(None, "", 0, "pending", true);
    excessive["media_routes"] = json!(
        (0..33)
            .map(|n| json!({"id":format!("route-{n}"),"label":"Choice"}))
            .collect::<Vec<_>>()
    );
    for value in [duplicate, private, bad_label, excessive, json!({"ok":true})] {
        let (mut c, mut p, _) = pair();
        c.inspect(until()).unwrap();
        request(&p);
        reply(&mut p, value);
        failure(&mut c, Failure::Protocol);
    }
    let (mut c, mut p, _) = pair();
    c.inspect(until()).unwrap();
    request(&p);
    p.write_all(&vec![b'x'; 65_537]).unwrap();
    let mut event = None;
    for _ in 0..20 {
        if let Some(next) = c.poll() {
            event = Some(next);
            break;
        }
    }
    assert!(matches!(
        event,
        Some(Event::Unavailable {
            reason: Failure::Protocol,
            ..
        })
    ));
}
#[test]
fn explicit_owner_rejection_is_settled_and_diagnostic_graph_names_not_exposed() {
    let (mut c, mut p, _) = pair();
    let before = applied(&mut c, &mut p);
    c.select("media-z2", until()).unwrap();
    request(&p);
    reply(&mut p, json!({"ok":false,"error":"unsupported_route"}));
    match c.poll().unwrap() {
        Event::Rejected { reason, snapshot } => {
            assert_eq!(reason, "unsupported_route");
            assert_eq!(snapshot.admitted, before.admitted);
            assert!(snapshot.connected && !snapshot.uncertain);
        }
        event => panic!("{event:?}"),
    }
    c.inspect(until()).unwrap();
    request(&p);
    let mut v = status(None, "media-a7", 1, "observed", true);
    v["observed"]["links"] =
        json!([{"source":"private.source.9","target":"private.output.21","state":"active"}]);
    reply(&mut p, v);
    let s = updated(&mut c);
    assert!(!format!("{s:?}").contains("private."));
    quiet(&p);
}

static DIRECTORY: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "audio-client-{}-{}",
            std::process::id(),
            DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o777)).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Owner(Child);
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            match self.0.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < end => sleep(Duration::from_millis(5)),
                result => panic!("owned original Main wait {result:?}"),
            }
        }
    }
}
#[test]
#[ignore = "requires isolated root and distinct-UID original Main"]
fn actual_listener_credentials_and_original_main_catalog_acquire_route_wire() {
    assert_eq!(
        uid(),
        0,
        "owned container test uses root only to run original owner as UID1000"
    );
    let dir = Directory::new();
    let socket = dir.0.join("audio.sock");
    let profile = dir.0.join("profile.json");
    fs::write(&profile,serde_json::to_vec(&json!({"sources":{"media":"fixture.source"},"routes":[{"id":"media-a7","label":"Speaker","target":"fixture.output.a","roles":["media"]},{"id":"media-z2","label":"Headphones","target":"fixture.output.z","roles":["media"]}]})).unwrap()).unwrap();
    fs::set_permissions(&profile, fs::Permissions::from_mode(0o644)).unwrap();
    let mut owner = Owner(
        Command::new(env!("CARGO_BIN_EXE_pokelinux-audio"))
            .args([
                socket.to_str().unwrap(),
                profile.to_str().unwrap(),
                "0",
                "1001",
            ])
            .uid(1000)
            .gid(1000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let end = Instant::now() + Duration::from_secs(3);
    while !socket.exists() {
        assert!(
            owner.0.try_wait().unwrap().is_none(),
            "original Main setup exit"
        );
        assert!(Instant::now() < end, "original Main socket setup");
        sleep(Duration::from_millis(5));
    }
    let mut c = Client::from_stream(UnixStream::connect(&socket).unwrap(), 1000).unwrap();
    fn settle(c: &mut Client) -> Snapshot {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(event) = c.poll() {
                match event {
                    Event::Updated(s) => return s,
                    event => panic!("actual Main wire {event:?}"),
                }
            }
            assert!(Instant::now() < end);
            sleep(Duration::from_millis(2));
        }
    }
    c.inspect(until()).unwrap();
    assert_eq!(settle(&mut c).media_routes.len(), 2);
    c.select("media-a7", until()).unwrap();
    let admitted = settle(&mut c);
    assert_eq!(admitted.media_routes.len(), 2);
    let lease = admitted.admitted.unwrap().lease;
    assert!(admitted.applied.is_none());
    c.select("media-z2", until()).unwrap();
    let changed = settle(&mut c);
    assert_eq!(changed.admitted.as_ref().unwrap().lease, lease);
    assert_eq!(changed.admitted.unwrap().route, "media-z2");
    c.revoke();
    // Explicit privileged fixture teardown only; the media Client has no Stop API.
    let mut stop = UnixStream::connect(&socket).unwrap();
    stop.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    stop.set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    stop.write_all(b"{\"op\":\"stop\"}\n").unwrap();
    let mut answer = Vec::new();
    let mut byte = [0];
    while stop.read(&mut byte).unwrap() == 1 {
        answer.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
        assert!(answer.len() < 4096);
    }
    assert!(
        serde_json::from_slice::<Value>(&answer).unwrap()["ok"]
            .as_bool()
            .unwrap()
    );
    drop(stop);
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = owner.0.try_wait().unwrap() {
            assert!(status.success(), "actual original Main Stop wait {status}");
            break;
        }
        assert!(
            Instant::now() < end,
            "actual original Main bounded Stop wait"
        );
        sleep(Duration::from_millis(5));
    }
    // A separate actual listener confirms that from_stream takes an already connected stream.
    let listener = UnixListener::bind(dir.0.join("listener.sock")).unwrap();
    let stream =
        UnixStream::connect(listener.local_addr().unwrap().as_pathname().unwrap()).unwrap();
    let (mut accepted, _) = listener.accept().unwrap();
    accepted
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut client = Client::from_stream(stream, uid()).unwrap();
    client.inspect(until()).unwrap();
    let mut byte = [0; 16];
    assert!(accepted.read(&mut byte).unwrap() > 0);
    client.revoke();
}

#[test]
fn explicit_preview_preserves_distinct_admission_and_startup_bounds() {
    use pokelinux_audio::PreviewState;
    let (mut c, mut peer, _) = pair();
    inspected(&mut c, &mut peer);
    let original = now() + 400_000_000;
    let startup = original + 500_000_000;
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            original,
            original - 1
        ),
        Err(SubmitError::InvalidPreview)
    );
    quiet(&peer);
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            original,
            startup
        ),
        Ok(SendOutcome::Sent)
    );
    assert_eq!(
        request(&peer),
        json!({"op":"preview_play","lease":null,"route":"media-a7","level_percent":50,"muted":false,"until_boottime_ns":original,"startup_until_boottime_ns":startup})
    );
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            original,
            startup
        ),
        Err(SubmitError::Busy)
    );
    quiet(&peer);
    let mut starting = status(Some(7), "media-a7", 1, "pending", false);
    starting["preview"] = json!("starting");
    starting["preview_generation"] = json!(17);
    starting["preview_muted"] = json!(false);
    starting["policy_settled"] = json!(true);
    reply(&mut peer, starting);
    let state = updated(&mut c);
    assert_eq!(state.preview, PreviewState::Starting);
    assert!(state.policy_settled && state.applied.is_none());
    for _ in 0..3 {
        assert!(c.poll().is_none());
    }
    quiet(&peer);
    assert_eq!(c.inspect(original), Ok(SendOutcome::Sent));
    request(&peer);
    let mut playing = status(None, "media-a7", 1, "observed", true);
    playing["preview"] = json!("playing");
    playing["preview_generation"] = json!(17);
    playing["preview_muted"] = json!(false);
    playing["policy_settled"] = json!(true);
    reply(&mut peer, playing);
    let state = updated(&mut c);
    assert_eq!(state.preview, PreviewState::Playing);
    assert_eq!(state.applied.as_ref().unwrap().lease, 7);
    assert_eq!(c.preview_stop(original), Ok(SendOutcome::Sent));
    assert_eq!(request(&peer), json!({"op":"preview_stop","lease":7}));
    let mut retiring = status(None, "media-a7", 2, "pending", false);
    retiring["preview"] = json!("retiring");
    reply(&mut peer, retiring);
    let state = updated(&mut c);
    assert_eq!(state.preview, PreviewState::Retiring);
    assert!(state.applied.is_none());
    quiet(&peer);
}

#[test]
fn idle_policy_selection_does_not_invent_observed_audio_application() {
    let (mut c, mut peer, _) = pair();
    inspected(&mut c, &mut peer);
    assert_eq!(c.select("media-a7", until()), Ok(SendOutcome::Sent));
    request(&peer);
    let mut selected = status(Some(7), "media-a7", 1, "pending", false);
    selected["policy_settled"] = json!(true);
    reply(&mut peer, selected);
    let state = updated(&mut c);
    assert!(state.policy_settled && state.admitted.is_some() && state.applied.is_none());
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    request(&peer);
    let mut other = status(None, "media-z2", 2, "pending", true);
    other["desired"] = json!([[8, "media", "media-z2"]]);
    other["policy_settled"] = json!(true);
    reply(&mut peer, other);
    let state = updated(&mut c);
    assert!(!state.policy_settled && state.applied.is_none());
}

#[test]
fn expired_preview_cannot_publish_buffered_start_or_retry_itself() {
    let (mut c, mut peer, _) = pair();
    inspected(&mut c, &mut peer);
    let original = now() + 40_000_000;
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            original,
            original + 800_000_000
        ),
        Ok(SendOutcome::Sent)
    );
    assert_eq!(request(&peer)["until_boottime_ns"], original);
    let mut late = status(Some(7), "media-a7", 1, "pending", false);
    late["preview"] = json!("starting");
    late["preview_generation"] = json!(17);
    late["preview_muted"] = json!(false);
    reply(&mut peer, late);
    while now() < original {
        sleep(Duration::from_millis(1));
    }
    failure(&mut c, Failure::Deadline);
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            now() + 800_000_000,
            now() + 900_000_000
        ),
        Err(SubmitError::Closed)
    );
    assert!(!c.snapshot().policy_settled);
}

#[test]
fn exact_preview_level_socket_wire_and_owner_refusal_never_retry() {
    for (percent, muted) in [(0, false), (50, false), (100, false), (50, true)] {
        let (mut c, mut peer, _) = pair();
        inspected(&mut c, &mut peer);
        let original = now() + 900_000_000;
        let level = PreviewLevel::new(percent, muted).unwrap();
        assert_eq!(
            c.preview_play("media-a7", level, original, original),
            Ok(SendOutcome::Sent)
        );
        assert_eq!(
            request(&peer),
            json!({
                "op":"preview_play", "lease":null, "route":"media-a7",
                "level_percent":percent, "muted":muted, "until_boottime_ns":original,"startup_until_boottime_ns":original
            })
        );
        // Actual socket response control, not evidence that Main rejected input.
        reply(&mut peer, json!({"ok":false,"error":"call_active"}));
        match c.poll().expect("actual refusal reply") {
            Event::Rejected { reason, snapshot } => {
                assert_eq!(reason, "call_active");
                assert!(!snapshot.in_flight && snapshot.admitted.is_none());
            }
            event => panic!("owner refusal must remain a refusal: {event:?}"),
        }
        for _ in 0..3 {
            assert!(c.poll().is_none());
        }
        quiet(&peer);
    }
}

// These peers exercise the native owner's exact socket protocol and client state,
// not a running PCM producer. The actual producer is qualified separately.
fn observed_preview(c: &mut Client, peer: &mut UnixStream, generation: u64) -> Snapshot {
    inspected(c, peer);
    assert_eq!(
        c.preview_play(
            "media-a7",
            PreviewLevel::new(50, false).unwrap(),
            now() + 800_000_000,
            now() + 900_000_000
        ),
        Ok(SendOutcome::Sent)
    );
    request(peer);
    let mut starting = status(Some(7), "media-a7", 1, "pending", false);
    starting["preview"] = json!("starting");
    starting["preview_generation"] = json!(generation);
    starting["preview_muted"] = json!(false);
    starting["policy_settled"] = json!(true);
    reply(peer, starting);
    updated(c);
    assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
    request(peer);
    reply(peer, observed_preview_reply(generation, false, true));
    let state = updated(c);
    assert!(state.applied.is_some() && state.policy_settled);
    state
}
fn observed_preview_reply(generation: u64, muted: bool, inspected: bool) -> Value {
    let mut playing = status(None, "media-a7", 1, "observed", inspected);
    playing["preview"] = json!("playing");
    playing["preview_generation"] = json!(generation);
    playing["preview_muted"] = json!(muted);
    playing["policy_settled"] = json!(true);
    playing
}
#[test]
fn live_mute_exact_socket_wire_preserves_route_and_waits_for_confirmation() {
    let generation = 9_007_199_254_740_993;
    let (mut c, mut peer, _) = pair();
    let before = observed_preview(&mut c, &mut peer, generation);
    let original = now() + 900_000_000;
    assert_eq!(
        c.preview_mute(generation, true, original),
        Ok(SendOutcome::Sent)
    );
    assert_eq!(
        request(&peer),
        json!({"op":"preview_mute","lease":7,
        "generation":generation,"muted":true,"until_boottime_ns":original})
    );
    let pending = c.snapshot();
    assert_eq!(pending.applied, before.applied);
    assert_eq!(pending.admitted, before.admitted);
    assert_eq!(pending.preview_muted, Some(false));
    assert_eq!(
        c.preview_mute(generation, false, original),
        Err(SubmitError::Busy)
    );
    quiet(&peer);
    reply(&mut peer, observed_preview_reply(generation, true, false));
    let confirmed = updated(&mut c);
    assert_eq!(confirmed.preview_muted, Some(true));
    assert_eq!(confirmed.preview_generation, Some(generation));
    assert_eq!(confirmed.admitted, before.admitted);
    assert_eq!(confirmed.applied, before.applied);
    for _ in 0..3 {
        assert!(c.poll().is_none());
    }
    quiet(&peer);
}
#[test]
fn live_mute_wrong_generation_or_boolean_ack_closes_without_replay() {
    for (ack_generation, ack_muted) in [(18, true), (17, false)] {
        let (mut c, mut peer, _) = pair();
        observed_preview(&mut c, &mut peer, 17);
        assert_eq!(
            c.preview_mute(17, true, now() + 900_000_000),
            Ok(SendOutcome::Sent)
        );
        request(&peer);
        reply(
            &mut peer,
            observed_preview_reply(ack_generation, ack_muted, false),
        );
        failure(&mut c, Failure::Protocol);
        assert!(c.snapshot().preview_generation.is_none());
        assert!(c.snapshot().preview_muted.is_none());
        assert!(c.poll().is_none());
        let mut byte = [0];
        assert_eq!(
            peer.read(&mut byte).unwrap(),
            0,
            "failed channel actually closed"
        );
    }
}
#[test]
fn live_mute_stale_or_expired_input_emits_nothing_and_does_not_change_play() {
    let (mut c, mut peer, _) = pair();
    let before = observed_preview(&mut c, &mut peer, 17);
    for stale in [0, 16, 18] {
        assert_eq!(
            c.preview_mute(stale, true, now() + 900_000_000),
            Err(SubmitError::InvalidPreview)
        );
        quiet(&peer);
        assert_eq!(c.snapshot(), before);
    }
    assert_eq!(c.preview_mute(17, true, now()), Err(SubmitError::Expired));
    quiet(&peer);
    assert_eq!(c.snapshot(), before);
}
#[test]
fn live_identity_fields_are_required_and_consistent_on_actual_socket() {
    for malformed in 0..7 {
        let (mut c, mut peer, _) = pair();
        assert_eq!(c.inspect(until()), Ok(SendOutcome::Sent));
        request(&peer);
        let mut value = status(None, "", 0, "pending", true);
        match malformed {
            0 => {
                value.as_object_mut().unwrap().remove("preview_generation");
            }
            1 => {
                value.as_object_mut().unwrap().remove("preview_muted");
            }
            2 => {
                value["preview_generation"] = json!(17);
            }
            3 => {
                value["preview_muted"] = json!(false);
            }
            4 => {
                value["preview"] = json!("playing");
            }
            5 => {
                value["preview"] = json!("playing");
                value["preview_generation"] = json!(0);
                value["preview_muted"] = json!(false);
            }
            6 => {
                value["preview"] = json!("playing");
                value["preview_generation"] = json!(17);
                value["preview_muted"] = json!(0);
            }
            _ => unreachable!(),
        }
        reply(&mut peer, value);
        failure(&mut c, Failure::Protocol);
    }
}
