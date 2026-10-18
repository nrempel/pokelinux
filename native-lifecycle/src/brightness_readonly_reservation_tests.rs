//! Controlled current Authority + production Session/Owner + actual root SEQPACKET.
//! No app.sock ingress, actual UID11050 app, Source, daemon worker, GUI or hardware.
//! The four cases retain one original first-send intent; no Set retry is selected.
use super::inspect_brightness;
use crate::{
    AppProfile, Permission, Reply,
    authority::Authority,
    boottime_ns,
    brightness::{self, Event, Owner, Provider, Request},
    brightness_session::Session,
    fresh_id, ipc,
    storage::Disk,
};
use pokelinux_brightness::protocol::{
    Attributes, LIMIT, Reply as WireReply, Request as WireRequest,
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, SendFlags, SocketAddrUnix, SocketFlags, SocketType},
};
use serde_json::{Value, json};
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

const CONNECTION: u64 = 71;
const REVISION: u64 = 7;
const SECOND: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug)]
enum CaseKind {
    Unchanged,
    ReplyRemotePending,
    ChangedRevision,
    OriginalExpiry,
}
struct Case {
    directory: PathBuf,
    authority: Authority,
    disk: Disk,
    session: Session,
    owner: Owner,
    peer: OwnedFd,
    listener: OwnedFd,
    held: WireRequest,
    incarnation: String,
}
fn attrs(requested: u32) -> Attributes {
    Attributes {
        max: 100,
        requested,
        actual: Some(24),
    }
}
fn reply(id: u64, revision: u64, requested: u32, percent: Option<u8>) -> WireReply {
    WireReply {
        schema: 1,
        id,
        ok: true,
        revision,
        pending: false,
        observation: Some(attrs(requested)),
        until_ns: (boottime_ns() + SECOND).to_string(),
        requested_percent: percent,
        last_set_uncertain: false,
    }
}
fn send_reply(peer: &OwnedFd, reply: &WireReply) -> bool {
    assert!(
        reply.valid(boottime_ns()),
        "fixture setup: strict reply shape"
    );
    let bytes = serde_json::to_vec(reply).unwrap();
    assert!(bytes.len() <= LIMIT, "fixture setup: bounded reply");
    matches!(
        net::send(peer, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL),
        Ok(n) if n == bytes.len()
    )
}
fn receive_request(peer: &OwnedFd) -> Option<WireRequest> {
    match ipc::receive(peer) {
        Ok(Some((bytes, cred))) => {
            assert!(bytes.len() <= LIMIT, "fixture setup: bounded request");
            assert_eq!(
                (
                    cred.pid.as_raw_nonzero().get() as u32,
                    cred.uid.as_raw(),
                    cred.gid.as_raw()
                ),
                (std::process::id(), 0, 0),
                "fixture setup: actual root request credential"
            );
            Some(serde_json::from_slice(&bytes).expect("strict production request"))
        }
        Ok(None) => None,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset
            ) =>
        {
            None
        }
        Err(error) => panic!("fixture receive error: {error}"),
    }
}
impl Case {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0, "fixture setup: root");
        let directory = PathBuf::from(
            std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT")
                .expect("explicit short RAM-backed fixture root"),
        )
        .join(format!("br-{}", fresh_id().unwrap()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let (mut disk, durable) = Disk::open(&directory, &[brightness::APP.into()]).unwrap();
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
        let grant = authority
            .plan_grant(
                brightness::APP,
                Permission::DisplayBrightness,
                true,
                boottime_ns(),
            )
            .unwrap();
        disk.replace(&grant.next).unwrap();
        assert_eq!(
            authority.finish(grant, true, boottime_ns()),
            Reply::Ok,
            "fixture setup: controlled grant"
        );

        let path = directory.join("owner.sock");
        let listener = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&listener, true).unwrap();
        net::bind(
            &listener,
            &SocketAddrUnix::new(&path).expect("short fixture pathname"),
        )
        .unwrap();
        net::listen(&listener, 4).unwrap();
        let mut owner = Owner::new(&path).unwrap();
        assert!(
            owner.inspect(boottime_ns() + SECOND).unwrap(),
            "fixture setup: seed Inspect sent"
        );
        let peer =
            net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
        net::sockopt::set_socket_passcred(&peer, true).unwrap();
        let cred = net::sockopt::socket_peercred(&peer).unwrap();
        assert_eq!(
            (
                cred.pid.as_raw_nonzero().get() as u32,
                cred.uid.as_raw(),
                cred.gid.as_raw()
            ),
            (std::process::id(), 0, 0),
            "fixture setup: concrete Owner root peer"
        );
        let seed = receive_request(&peer).expect("fixture setup: actual seed packet");
        assert!(
            matches!(seed, WireRequest::Inspect { .. }),
            "fixture setup: seed is readonly"
        );
        assert!(
            send_reply(&peer, &reply(seed.id(), REVISION, 25, None)),
            "fixture setup: seed response"
        );
        assert!(
            matches!(owner.poll(), Some(Event::Observed)),
            "fixture setup: seed observed"
        );
        let seeded = owner.state();
        assert!(
            seeded.is_current(boottime_ns()) && !seeded.pending && seeded.revision == REVISION,
            "fixture setup: fresh current baseline"
        );
        let incarnation = owner.incarnation().to_owned();

        // Invoke the production periodic helper, with no actor intent, then hold
        // its actual packet. No invented Provider pending bit or future API.
        let mut last = 0;
        inspect_brightness(&mut owner, false, false, &mut last);
        let held = receive_request(&peer).expect("fixture setup: actual periodic Inspect");
        assert!(
            matches!(held, WireRequest::Inspect { .. }),
            "fixture setup: periodic packet readonly"
        );
        let held_state = owner.state();
        assert!(
            held_state.is_current(boottime_ns())
                && held_state.pending
                && held_state.revision == REVISION,
            "fixture setup: own readonly request overlaps fresh values"
        );
        Self {
            directory,
            authority,
            disk,
            session: Session::default(),
            owner,
            peer,
            listener,
            held,
            incarnation,
        }
    }
    fn close(self, mut report: Value) -> Value {
        let Self {
            directory,
            authority,
            disk,
            session,
            owner,
            peer,
            listener,
            held: _,
            incarnation: _,
        } = self;
        // No child or helper thread exists. Every kernel socket/pidfd/lock owner
        // is dropped before writing the record and the final behavioral assert.
        drop(session);
        drop(owner);
        drop(peer);
        drop(listener);
        drop(authority);
        drop(disk);
        report["all_fixture_socket_pidfd_disk_owners_dropped_before_assert"] = json!(true);
        report["fixture_children_or_threads_started"] = json!(false);
        fs::write(
            directory.join("result.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        report
    }
    fn exercise(mut self, kind: CaseKind) -> Value {
        let ttl_ms = if matches!(kind, CaseKind::OriginalExpiry) {
            80
        } else {
            800
        };
        let begin_at = boottime_ns();
        let admission = self.session.begin(
            CONNECTION,
            std::process::id(),
            brightness::UID,
            Request {
                percent: 75,
                expected_revision: REVISION,
                ttl_ms,
            },
            &mut self.authority,
            &self.owner,
        );
        let (original_until, actual_refusal) = match admission {
            Ok(until) => (Some(until), None),
            Err(reply) => (None, Some(format!("{reply:?}"))),
        };
        let no_packet_at_admission = receive_request(&self.peer).is_none();
        let drive_while_held = self
            .session
            .drive(CONNECTION, &self.authority, &mut self.owner);
        let no_set_while_held = receive_request(&self.peer).is_none();
        let reservation_retained = self.session.pending();
        let mut new_inspection_at = 0;
        inspect_brightness(
            &mut self.owner,
            self.session.pending(),
            false,
            &mut new_inspection_at,
        );
        let successor_inspect_suppressed =
            new_inspection_at == 0 && receive_request(&self.peer).is_none();
        let no_reply_while_held = self.owner.poll().is_none();

        let mut expired_fence = None;
        if matches!(kind, CaseKind::OriginalExpiry) {
            // On the baseline refusal only, the fallback permits orderly cleanup
            // and records that the reservation-dependent control was unexercised.
            let until = original_until.unwrap_or(begin_at + ttl_ms * 1_000_000);
            let remaining = until.saturating_sub(boottime_ns());
            std::thread::sleep(Duration::from_nanos(remaining.saturating_add(2_000_000)));
            expired_fence = self
                .session
                .fence(&self.authority, &mut self.owner, false, |_| true);
        }
        let readonly_reply = match kind {
            CaseKind::ReplyRemotePending => WireReply {
                schema: 1,
                id: self.held.id(),
                ok: false,
                revision: REVISION,
                pending: true,
                observation: None,
                until_ns: "0".into(),
                requested_percent: None,
                last_set_uncertain: false,
            },
            CaseKind::ChangedRevision => reply(self.held.id(), REVISION + 1, 26, None),
            _ => reply(self.held.id(), REVISION, 25, None),
        };
        let readonly_reply_sent = send_reply(&self.peer, &readonly_reply);
        let observed_event = self.owner.poll();
        let readonly_observed = matches!(observed_event, Some(Event::Observed));
        let unsolicited_completion = observed_event.and_then(|event| {
            self.session
                .observe(event, &self.authority, &mut self.owner)
        });
        let after_read = self.owner.state();
        let incarnation_unchanged = self.owner.incarnation() == self.incarnation;
        let first_send_result = self
            .session
            .drive(CONNECTION, &self.authority, &mut self.owner);
        let first_packet = receive_request(&self.peer);
        let first_wire = first_packet
            .as_ref()
            .map(|request| serde_json::to_value(request).unwrap());
        let first_send_exact = matches!(
            first_packet.as_ref(),
            Some(WireRequest::Set { percent: 75, expected_revision: REVISION, until_ns, .. })
                if original_until.is_some_and(|until| until_ns == &until.to_string())
                    && boottime_ns() < original_until.unwrap()
        );
        let mut completion_ok = false;
        if matches!(kind, CaseKind::Unchanged) && first_send_exact {
            let packet = first_packet.as_ref().unwrap();
            let mut completed = reply(packet.id(), REVISION + 1, 75, Some(75));
            completed.observation.as_mut().unwrap().actual = Some(72);
            if send_reply(&self.peer, &completed)
                && let Some(event) = self.owner.poll()
            {
                completion_ok = matches!(
                    self.session.observe(event, &self.authority, &mut self.owner),
                    Some((CONNECTION, Reply::BrightnessCompleted { state }))
                        if state.requested_percent == Some(75)
                            && state.observed_percent == Some(75)
                            && state.actual_percent == Some(72)
                            && !state.pending && state.is_current(boottime_ns())
                );
            }
            self.session.published(CONNECTION);
        }
        let no_second_packet = receive_request(&self.peer).is_none();
        let no_duplicate_drive_reply = self
            .session
            .drive(CONNECTION, &self.authority, &mut self.owner)
            .is_none();
        let no_duplicate_packet = receive_request(&self.peer).is_none();
        let shared = original_until.is_some()
            && no_packet_at_admission
            && drive_while_held.is_none()
            && no_set_while_held
            && reservation_retained
            && successor_inspect_suppressed
            && no_reply_while_held
            && unsolicited_completion.is_none();
        let pass = shared
            && match kind {
                CaseKind::Unchanged => {
                    readonly_reply_sent
                        && readonly_observed
                        && after_read.is_current(boottime_ns())
                        && !after_read.pending
                        && after_read.revision == REVISION
                        && incarnation_unchanged
                        && first_send_result.is_none()
                        && first_send_exact
                        && completion_ok
                        && no_second_packet
                        && no_duplicate_drive_reply
                        && no_duplicate_packet
                }
                CaseKind::ReplyRemotePending => {
                    readonly_reply_sent
                        && readonly_observed
                        && after_read.pending
                        && !after_read.available
                        && first_packet.is_none()
                        && !self.session.pending()
                        && no_duplicate_packet
                }
                CaseKind::ChangedRevision => {
                    readonly_reply_sent
                        && readonly_observed
                        && after_read.revision == REVISION + 1
                        && first_packet.is_none()
                        && !self.session.pending()
                        && no_duplicate_packet
                }
                CaseKind::OriginalExpiry => {
                    expired_fence.is_some()
                        && first_packet.is_none()
                        && !self.session.pending()
                        && no_duplicate_packet
                }
            };
        let report = json!({
            "case": format!("{kind:?}"), "passed": pass,
            "boundary": "controlled current Authority + production Session/Owner + real root SEQPACKET",
            "actual_refusal": actual_refusal, "reservation_admitted": original_until.is_some(),
            "reservation_dependent_continuation_exercised": original_until.is_some(),
            "begin_at_ns": begin_at, "original_until_ns": original_until,
            "request_percent": 75, "captured_revision": REVISION, "original_ttl_ms": ttl_ms,
            "held_Inspect": self.held, "no_packet_at_admission": no_packet_at_admission,
            "drive_while_held": format!("{drive_while_held:?}"),
            "no_Set_while_held": no_set_while_held, "reservation_retained": reservation_retained,
            "successor_periodic_Inspect_suppressed": successor_inspect_suppressed,
            "no_reply_while_held": no_reply_while_held,
            "readonly_reply": readonly_reply, "readonly_reply_sent": readonly_reply_sent,
            "readonly_observed": readonly_observed, "state_after_readonly_reply": after_read,
            "incarnation_unchanged_after_readonly_reply": incarnation_unchanged,
            "first_send_result": format!("{first_send_result:?}"), "first_wire_packet": first_wire,
            "one_first_Set_has_original_deadline_and_revision": first_send_exact,
            "qualified_completion": completion_ok, "expiry_fence_reply": format!("{expired_fence:?}"),
            "no_second_packet": no_second_packet, "no_duplicate_drive_reply": no_duplicate_drive_reply,
            "no_duplicate_packet": no_duplicate_packet,
            "actual_app_UID11050_or_app_sock_ingress_exercised": false,
            "Source_daemon_worker_GUI_hardware_exercised": false,
        });
        self.close(report)
    }
}
#[test]
fn actor_own_inspect_reservation_has_one_original_first_send() {
    let reports: Vec<_> = [
        CaseKind::Unchanged,
        CaseKind::ReplyRemotePending,
        CaseKind::ChangedRevision,
        CaseKind::OriginalExpiry,
    ]
    .into_iter()
    .map(|kind| Case::new().exercise(kind))
    .collect();
    // All cases have closed their sockets, pidfd and disk lock before asserting.
    println!("{}", serde_json::to_string(&reports).unwrap());
    assert!(
        reports.iter().all(|report| report["passed"] == true),
        "BRIGHTNESS_FIRST_SEND_RESERVATION_BEHIND_OWN_READONLY_INSPECT"
    );
}

// Additional published-readiness contract. The original fixture above is unchanged.
fn projection_can_set(case: &mut Case, state: &brightness::State) -> bool {
    // The real Status predicate with no unrelated actor operation in this fixture.
    case.session.available_to(CONNECTION)
        && state.available
        && !state.pending
        && case.authority.brightness_allowed(
            std::process::id(),
            brightness::UID,
            case.owner.incarnation(),
        )
}
fn projection_idle_then_active() -> Value {
    let mut case = Case::new();
    let private_held = case.owner.state();
    let published_idle = case.session.state(CONNECTION, &case.owner);
    let idle_can_set = projection_can_set(&mut case, &published_idle);
    let own_readonly = case.owner.readonly_in_flight();
    let no_active_at_read = !case.session.pending();
    let admitted = case.session.begin(
        CONNECTION,
        std::process::id(),
        brightness::UID,
        Request {
            percent: 75,
            expected_revision: REVISION,
            ttl_ms: 800,
        },
        &mut case.authority,
        &case.owner,
    );
    let original_until = admitted.ok();
    let published_active_held = case.session.state(CONNECTION, &case.owner);
    let drive_held = case
        .session
        .drive(CONNECTION, &case.authority, &mut case.owner);
    let no_set_held = receive_request(&case.peer).is_none();
    let replied = send_reply(&case.peer, &reply(case.held.id(), REVISION, 25, None));
    let observed = matches!(case.owner.poll(), Some(Event::Observed));
    let private_after_read = case.owner.state();
    let active_before_drive = case.session.pending();
    let published_active_before_drive = case.session.state(CONNECTION, &case.owner);
    let active_can_set = projection_can_set(&mut case, &published_active_before_drive);
    let drive_first = case
        .session
        .drive(CONNECTION, &case.authority, &mut case.owner);
    let first = receive_request(&case.peer);
    let original_first_set = matches!(first.as_ref(),
        Some(WireRequest::Set { percent: 75, expected_revision: REVISION, until_ns, .. })
        if original_until.is_some_and(|until| until_ns == &until.to_string()
            && boottime_ns() < until));
    let drive_second = case
        .session
        .drive(CONNECTION, &case.authority, &mut case.owner);
    let no_second = receive_request(&case.peer).is_none();
    let pass = private_held.pending
        && private_held.is_current(boottime_ns())
        && own_readonly
        && no_active_at_read
        && published_idle.available
        && !published_idle.pending
        && idle_can_set
        && published_idle.revision == REVISION
        && published_idle.requested_percent.is_none()
        && published_idle.observed_percent == private_held.observed_percent
        && published_idle.actual_percent == private_held.actual_percent
        && published_idle.until_ns == private_held.until_ns
        && original_until.is_some()
        && published_active_held.pending
        && drive_held.is_none()
        && no_set_held
        && replied
        && observed
        && !private_after_read.pending
        && active_before_drive
        && published_active_before_drive.pending
        && !active_can_set
        && drive_first.is_none()
        && original_first_set
        && drive_second.is_none()
        && no_second;
    case.close(json!({"case":"OwnReadonlyIdleThenActive","passed":pass,
        "private_held_state":private_held,"own_readonly_in_flight":own_readonly,
        "no_active_effect_at_read":no_active_at_read,
        "published_idle_state":published_idle,"idle_can_set":idle_can_set,
        "admitted_original_until_ns":original_until,
        "published_active_while_read_held":published_active_held,
        "no_Set_while_held":no_set_held,"actual_readonly_reply_sent":replied,
        "actual_readonly_reply_observed":observed,"private_state_after_read":private_after_read,
        "session_pending_before_first_drive":active_before_drive,
        "published_active_before_first_drive":published_active_before_drive,
        "active_can_set":active_can_set,"one_original_first_Set":original_first_set,
        "first_wire_packet":first,"no_second_Set":no_second}))
}
fn projection_known_remote_pending() -> Value {
    let mut case = Case::new();
    let busy = WireReply {
        schema: 1,
        id: case.held.id(),
        ok: false,
        revision: REVISION,
        pending: true,
        observation: None,
        until_ns: "0".into(),
        requested_percent: None,
        last_set_uncertain: false,
    };
    let replied = send_reply(&case.peer, &busy);
    let observed = matches!(case.owner.poll(), Some(Event::Observed));
    let private = case.owner.state();
    let own_readonly = case.owner.readonly_in_flight();
    let published = case.session.state(CONNECTION, &case.owner);
    let can_set = projection_can_set(&mut case, &published);
    let refused = case.session.begin(
        CONNECTION,
        std::process::id(),
        brightness::UID,
        Request {
            percent: 75,
            expected_revision: REVISION,
            ttl_ms: 800,
        },
        &mut case.authority,
        &case.owner,
    );
    let unavailable = matches!(refused, Err(Reply::Unavailable));
    let no_set = receive_request(&case.peer).is_none();
    let pass = replied
        && observed
        && private.pending
        && !own_readonly
        && published.pending
        && !published.available
        && !can_set
        && unavailable
        && !case.session.pending()
        && no_set;
    case.close(json!({"case":"KnownRemotePending","passed":pass,
        "actual_readonly_reply_sent":replied,"actual_readonly_reply_observed":observed,
        "private_state":private,"own_readonly_in_flight":own_readonly,
        "published_state":published,"can_set":can_set,
        "begin_Unavailable":unavailable,"no_Set":no_set}))
}
fn projection_fresh_revision_requires_new_choice() -> Value {
    let mut case = Case::new();
    let replied = send_reply(&case.peer, &reply(case.held.id(), REVISION + 1, 26, None));
    let observed = matches!(case.owner.poll(), Some(Event::Observed));
    let published = case.session.state(CONNECTION, &case.owner);
    let fresh_can_set = projection_can_set(&mut case, &published);
    let stale = case.session.begin(
        CONNECTION,
        std::process::id(),
        brightness::UID,
        Request {
            percent: 75,
            expected_revision: REVISION,
            ttl_ms: 800,
        },
        &mut case.authority,
        &case.owner,
    );
    let stale_denied = matches!(stale, Err(Reply::Denied));
    let stale_emitted_no_set = receive_request(&case.peer).is_none();
    let fresh = case.session.begin(
        CONNECTION,
        std::process::id(),
        brightness::UID,
        Request {
            percent: 75,
            expected_revision: REVISION + 1,
            ttl_ms: 800,
        },
        &mut case.authority,
        &case.owner,
    );
    let original_until = fresh.ok();
    let private_idle = !case.owner.state().pending;
    let active_state = case.session.state(CONNECTION, &case.owner);
    let active_can_set = projection_can_set(&mut case, &active_state);
    let driven = case
        .session
        .drive(CONNECTION, &case.authority, &mut case.owner);
    let first = receive_request(&case.peer);
    let one_fresh_explicit_set = matches!(first.as_ref(),
        Some(WireRequest::Set { percent: 75, expected_revision, until_ns, .. })
        if *expected_revision == REVISION + 1
            && original_until.is_some_and(|until| until_ns == &until.to_string()
                && boottime_ns() < until));
    let no_second = receive_request(&case.peer).is_none();
    let pass = replied
        && observed
        && published.is_current(boottime_ns())
        && published.revision == REVISION + 1
        && !published.pending
        && fresh_can_set
        && stale_denied
        && stale_emitted_no_set
        && original_until.is_some()
        && private_idle
        && active_state.pending
        && !active_can_set
        && driven.is_none()
        && one_fresh_explicit_set
        && no_second;
    case.close(
        json!({"case":"FreshRevisionNeedsNewExplicitChoice","passed":pass,
        "actual_readonly_reply_sent":replied,"actual_readonly_reply_observed":observed,
        "published_fresh_state":published,"fresh_can_set":fresh_can_set,
        "stale_R7_Denied":stale_denied,"no_Set_for_stale_R7":stale_emitted_no_set,
        "fresh_R8_original_until_ns":original_until,"private_idle_before_fresh_Set":private_idle,
        "published_active_before_first_drive":active_state,"active_can_set":active_can_set,
        "one_new_explicit_R8_Set":one_fresh_explicit_set,"first_wire_packet":first,
        "no_second_Set":no_second}),
    )
}
#[test]
fn published_readiness_distinguishes_own_readonly_from_choice_custody() {
    let reports = [
        projection_idle_then_active(),
        projection_known_remote_pending(),
        projection_fresh_revision_requires_new_choice(),
    ];
    // Each Case dropped its actual socket, pidfd and disk owners before this assert.
    println!("{}", serde_json::to_string(&reports).unwrap());
    assert!(
        reports.iter().all(|report| report["passed"] == true),
        "BRIGHTNESS_PUBLISHED_READINESS_DISTINGUISHES_READONLY_AND_EFFECT"
    );
}
