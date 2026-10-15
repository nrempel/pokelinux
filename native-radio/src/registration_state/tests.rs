//! Fixed conditional contract; prerequisite injection exists only in lib tests.
use super::*;
use crate::{
    indication::{IndicationKind, RadioIndication},
    owner::{Observation, OwnerError, Phase, RadioOwner, SessionKey},
    receiver::{CallbackQueue, RadioEvent, ReceiverFailure},
    transport::RadioTransport,
    worker::TransportSession,
};
use pokelinux_phone_state::{RegistrationAvailability as Availability, RegistrationObservation};
use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

fn n(v: usize) -> NonZeroUsize {
    NonZeroUsize::new(v).unwrap()
}
struct Controlled(AtomicU64);
impl Clock for Controlled {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::SeqCst))
    }
}
fn setup() -> (RadioOwner, SessionKey, Arc<CallbackQueue>, Arc<Controlled>) {
    let clock = Arc::new(Controlled(AtomicU64::new(10)));
    let mut owner = RadioOwner::with_clock(n(8), 1, clock.clone());
    let reg = owner
        .begin_until(None, n(8), Deadline::at(Duration::from_nanos(100)))
        .unwrap();
    owner.activate(&reg.key, 42).unwrap();
    (owner, reg.key, reg.endpoints.control.queue(), clock)
}
fn packet(state: u8, feature: u8, slot: i32) -> RadioEvent {
    let mut payload = vec![0; 12];
    payload[0] = state;
    payload[1] = feature;
    RadioEvent::Indication(RadioIndication {
        message_id: 3012,
        slot_id: slot,
        payload,
        kind: IndicationKind::Raw,
    })
}
fn commit(owner: &mut RadioOwner, queue: &CallbackQueue, event: RadioEvent) -> u64 {
    queue.deliver(event).unwrap();
    let prepared = owner.prepare_until(Instant::now()).unwrap().unwrap();
    owner.publish(prepared).unwrap()
}
fn positive(
    owner: &mut RadioOwner,
    key: &SessionKey,
    queue: &CallbackQueue,
) -> RegistrationObservation {
    owner.test_registration_context(key, 1, true, true).unwrap();
    let revision = commit(owner, queue, packet(2, 0, 1));
    let expected = RegistrationObservation {
        source_generation: 1,
        source_revision: revision,
    };
    assert_eq!(
        owner.registration_view().availability,
        Availability::RegistrationObserved(expected)
    );
    assert_eq!(owner.registration_view().last_observed, Some(expected));
    expected
}
#[test]
fn conditional_positive_requires_exact_context_and_complete_qualified_commit() {
    let (mut owner, key, queue, _) = setup();
    for (entity, request, slot) in [
        (false, false, 1),
        (false, true, 1),
        (true, false, 1),
        (true, true, 0),
    ] {
        owner
            .test_registration_context(&key, slot, entity, request)
            .unwrap();
        commit(&mut owner, &queue, packet(2, 0, 1));
        assert_eq!(owner.registration_view(), RegistrationView::default());
    }
    let observation = positive(&mut owner, &key, &queue);
    assert_eq!(owner.registration_view().revision, 1);
    assert_eq!(observation.source_revision, 7);
    assert!(owner.snapshot().request.is_none());
}
#[test]
fn conservative_loss_retains_original_history_and_distinct_view_revision() {
    let (mut owner, key, queue, _) = setup();
    for (state, feature) in [(1, 0), (255, 0), (3, 0), (2, 1)] {
        let observed = positive(&mut owner, &key, &queue);
        let before = owner.registration_view();
        commit(&mut owner, &queue, packet(state, feature, 1));
        let after = owner.registration_view();
        assert_eq!(after.availability, Availability::Unavailable);
        assert_eq!(after.last_observed, Some(observed));
        assert_eq!(after.revision, before.revision + 1);
    }
}
#[test]
fn wrong_slot_and_raw_do_not_withdraw_but_matching_malformed_does() {
    let (mut owner, key, queue, _) = setup();
    positive(&mut owner, &key, &queue);
    let before = owner.registration_view();
    queue.deliver(packet(2, 0, 0)).unwrap();
    let event = owner.prepare_until(Instant::now()).unwrap().unwrap();
    assert_eq!(owner.publish(event), Err(OwnerError::Unsupported));
    assert_eq!(owner.registration_view(), before);
    let raw = RadioEvent::Response(crate::response::RadioResponse {
        info: crate::RadioExternalResponseInfo::default(),
        payload: vec![8],
        kind: crate::response::ResponseKind::Raw,
    });
    commit(&mut owner, &queue, raw);
    assert_eq!(owner.registration_view(), before);
    let wire_revision = owner.snapshot().revision;
    let mut malformed = packet(2, 0, 1);
    if let RadioEvent::Indication(ref mut e) = malformed {
        e.payload.truncate(11);
    }
    queue.deliver(malformed).unwrap();
    let event = owner.prepare_until(Instant::now()).unwrap().unwrap();
    assert_eq!(owner.publish(event), Err(OwnerError::InvalidIndication));
    assert_eq!(owner.snapshot().revision, wire_revision);
    let after = owner.registration_view();
    assert_eq!(after.availability, Availability::Unavailable);
    assert_eq!(after.last_observed, before.last_observed);
    assert_eq!(after.revision, before.revision + 1);
}
#[test]
fn stop_death_and_suspend_expiry_withdraw_without_relabelling_history() {
    for reason in [
        ReceiverFailure::Cancelled,
        ReceiverFailure::ServiceDied,
        ReceiverFailure::Expired,
    ] {
        let (mut owner, key, queue, clock) = setup();
        let observed = positive(&mut owner, &key, &queue);
        let retained = owner.registration_view();
        match reason {
            ReceiverFailure::Cancelled => owner.stop(&key).unwrap(),
            ReceiverFailure::Expired => clock.0.store(100, Ordering::SeqCst),
            _ => owner.fail_current(&key, reason),
        }
        let after = owner.registration_view();
        assert_eq!(after.availability, Availability::Unavailable);
        assert_eq!(after.last_observed, Some(observed));
        assert_eq!(after.revision, retained.revision + 1);
        assert_eq!(owner.snapshot().phase, Phase::Failed(reason));
        assert!(matches!(
            retained.availability,
            Availability::RegistrationObserved(_)
        ));
    }
}
#[test]
fn replacement_revokes_context_and_keeps_history_origin() {
    let (mut owner, key, queue, _) = setup();
    let observed = positive(&mut owner, &key, &queue);
    let reg = owner
        .begin_until(Some(&key), n(8), Deadline::at(Duration::from_nanos(100)))
        .unwrap();
    owner.activate(&reg.key, 43).unwrap();
    assert_eq!(
        owner.test_registration_context(&key, 1, true, true),
        Err(OwnerError::StaleSession)
    );
    let queue2 = reg.endpoints.control.queue();
    commit(&mut owner, &queue2, packet(2, 0, 1));
    assert_eq!(
        owner.registration_view().availability,
        Availability::Unavailable
    );
    assert_eq!(owner.registration_view().last_observed, Some(observed));
    owner
        .test_registration_context(&reg.key, 1, true, true)
        .unwrap();
    let revision = commit(&mut owner, &queue2, packet(2, 0, 1));
    assert_eq!(
        owner.registration_view().availability,
        Availability::RegistrationObserved(RegistrationObservation {
            source_generation: 5,
            source_revision: revision
        })
    );
}
#[test]
fn held_candidate_cannot_use_changed_context_or_survive_stop_or_expiry() {
    for expire in [false, true] {
        let (mut owner, key, queue, clock) = setup();
        let observed = positive(&mut owner, &key, &queue);
        queue.deliver(packet(2, 0, 1)).unwrap();
        let held = owner.prepare_until(Instant::now()).unwrap().unwrap();
        assert_eq!(
            owner.test_registration_context(&key, 1, true, true),
            Err(OwnerError::Preparing)
        );
        if expire {
            clock.0.store(100, Ordering::SeqCst);
        } else {
            owner.stop(&key).unwrap();
        }
        assert!(matches!(owner.publish(held), Err(OwnerError::Failed(_))));
        assert_eq!(
            owner.registration_view().availability,
            Availability::Unavailable
        );
        assert_eq!(owner.registration_view().last_observed, Some(observed));
    }
}

#[test]
fn unqualified_fields_and_present_zero_extensions_withdraw() {
    let (mut owner, key, queue, _) = setup();
    let mut packets = Vec::new();
    for index in [1, 2, 6] {
        let mut bytes = vec![0; 12];
        bytes[0] = 2;
        bytes[index] = 1;
        packets.push(bytes);
    }
    for index in [8, 10] {
        let mut bytes = vec![0; 12];
        bytes[0] = 2;
        bytes[index] = 1;
        bytes.push(b'x');
        packets.push(bytes);
    }
    for extension in [4, 12, 14] {
        let mut bytes = vec![0; 12 + extension];
        bytes[0] = 2;
        packets.push(bytes);
    }
    for payload in packets {
        let observed = positive(&mut owner, &key, &queue);
        commit(
            &mut owner,
            &queue,
            RadioEvent::Indication(RadioIndication {
                message_id: 3012,
                slot_id: 1,
                payload,
                kind: IndicationKind::Raw,
            }),
        );
        assert_eq!(
            owner.registration_view().availability,
            Availability::Unavailable
        );
        assert_eq!(owner.registration_view().last_observed, Some(observed));
    }
}
#[test]
fn revision_exhaustion_reserves_withdrawal_and_never_reopens() {
    let session = SessionKey::new();
    let observed = RegistrationObservation {
        source_generation: 9,
        source_revision: 11,
    };
    let mut projection = Projection {
        generation: 9,
        context: Some(Context {
            session: session.clone(),
            slot: 1,
            live_entity: true,
            request_membership: true,
        }),
        view: RegistrationView {
            revision: u64::MAX - 1,
            availability: Availability::RegistrationObserved(observed),
            last_observed: Some(observed),
        },
    };
    projection.withdraw();
    assert_eq!(projection.view().revision, u64::MAX);
    let bytes = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let wire = RegistrationWire::new(omarchy_ims::registration::decode(&bytes).unwrap()).unwrap();
    projection.commit(&session, 1, 12, &wire);
    assert_eq!(projection.view().revision, u64::MAX);
    assert_eq!(projection.view().availability, Availability::Unavailable);
    assert_eq!(projection.view().last_observed, Some(observed));
}

// Actual generic ARM kernel Binder integration. Explicitly selected by the
// private guest; never exercised by ordinary host tests or a production ELF.
fn before(until: Duration) -> Result<(), String> {
    if BootClock.now() < until {
        Ok(())
    } else {
        Err("original case cutoff elapsed".into())
    }
}
fn require(ok: bool, text: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(text.into()) }
}
fn control(peer: &rsbinder::SIBinder, code: u32) -> Result<rsbinder::Parcel, String> {
    let remote = peer.as_remote().ok_or("local peer")?;
    let data = remote
        .prepare_transact(true)
        .map_err(|e| format!("prepare {e:?}"))?;
    let mut reply = remote
        .submit_transact(code, &data, 0)
        .map_err(|e| format!("control {code}: {e:?}"))?
        .ok_or("missing reply")?;
    let status: rsbinder::Status = reply.read().map_err(|e| format!("status {e:?}"))?;
    require(status.is_ok(), "fixture status")?;
    Ok(reply)
}
fn trigger(peer: &rsbinder::SIBinder, code: u32, until: Duration) -> Result<(), String> {
    before(until)?;
    let mut reply = control(peer, code)?;
    require(
        reply.read_i32().map_err(|e| format!("ack {e:?}"))? == 1 && reply.data_avail() == 0,
        "exact fixture ack",
    )?;
    before(until)
}
fn next(
    session: &mut TransportSession,
    until: Duration,
) -> Result<crate::owner::PreparedEvent, String> {
    loop {
        before(until)?;
        let event = session
            .prepare_callback_until(Instant::now())
            .map_err(|e| format!("callback {e:?}"))?;
        before(until)?;
        if let Some(event) = event {
            return Ok(event);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn actual_commit(
    session: &mut TransportSession,
    peer: &rsbinder::SIBinder,
    code: u32,
    until: Duration,
) -> Result<u64, String> {
    trigger(peer, code, until)?;
    let event = next(session, until)?;
    let RadioEvent::Indication(indication) = event.event() else {
        return Err("not actual indication".into());
    };
    let mut expected = vec![0; 12];
    expected[0] = if code == 122 {
        1
    } else if code == 123 {
        255
    } else {
        2
    };
    if code == 124 {
        expected[1] = 1;
    }
    require(
        indication.kind == IndicationKind::Raw
            && indication.message_id == 3012
            && indication.slot_id == 1
            && indication.payload == expected,
        "exact real callback kind/route/full bytes",
    )?;
    let base_revision = event.base_revision();
    require(
        base_revision == session.snapshot().revision,
        "exact preparation base revision",
    )?;
    let revision = session
        .publish_callback(event)
        .map_err(|e| format!("publish {e:?}"))?;
    before(until)?;
    let snapshot = session.snapshot();
    let observed = snapshot.observation.as_ref().ok_or("no typed history")?;
    require(
        snapshot.revision == revision
            && observed.revision == revision
            && base_revision.checked_add(1) == Some(revision),
        "exact new committed revision",
    )?;
    let Observation::RegistrationWire {
        slot_id,
        message_id,
        wire,
    } = &observed.kind
    else {
        return Err("committed history is not typed registration".into());
    };
    require(
        *slot_id == 1
            && *message_id == 3012
            && wire.registration_state_wire == expected[0]
            && wire.feature_wire == expected[1]
            && wire.feature_mask_wire == 0
            && wire.status_code_wire == 0
            && wire.uri_bytes().is_empty()
            && wire.reason_bytes().is_empty()
            && wire.feature_mask_ext_wire.is_none()
            && wire.sip_delegate_mask_wire.is_none()
            && wire.feature_ext_wire.is_none(),
        "complete exact typed history at new revision",
    )?;
    Ok(revision)
}
fn json(event: &str, fields: String) {
    println!("{{\"schema\":1,\"event\":\"{event}\",{fields}}}");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
#[test]
#[ignore = "requires isolated generic ARM kernel Binder, private manager and synthetic peer"]
fn actual_binder_conditional_registration() -> Result<(), String> {
    use rsbinder::ProcessState;
    println!();
    let started = BootClock.now();
    let setup_until = started + Duration::from_secs(10);
    let until = started + Duration::from_secs(20);
    let session_until = started + Duration::from_secs(30);
    let device = std::env::var("POKELINUX_REGISTRATION_TEST_DEVICE")
        .map_err(|_| "private Binder device missing")?;
    let process = ProcessState::init(&device, 3).map_err(|e| format!("Binder init {e:?}"))?;
    ProcessState::start_thread_pool();
    let manager = process
        .context_object()
        .map_err(|e| format!("context {e:?}"))?;
    let remote = manager.as_remote().ok_or("local context manager")?;
    let mut data = remote
        .prepare_transact(true)
        .map_err(|e| format!("lookup prepare {e:?}"))?;
    data.write("org.pokelinux.fixture.arm64-radio")
        .map_err(|e| format!("lookup name {e:?}"))?;
    let mut reply = remote
        .submit_transact(3, &data, 0)
        .map_err(|e| format!("lookup {e:?}"))?
        .ok_or("lookup absent")?;
    let status: rsbinder::Status = reply.read().map_err(|e| format!("lookup status {e:?}"))?;
    require(status.is_ok(), "lookup succeeded")?;
    let peer: rsbinder::SIBinder = reply.read().map_err(|e| format!("handle {e:?}"))?;
    require(
        reply.data_avail() == 0 && peer.is_remote() && peer != manager,
        "actual distinct remote endpoint",
    )?;
    let transport =
        RadioTransport::from_binder(peer.clone()).map_err(|e| format!("transport {e:?}"))?;
    let mut session = TransportSession::new(n(4), 1);
    let key = session
        .start(
            transport,
            "registration-state-test".into(),
            n(8),
            Deadline::at(session_until),
        )
        .map_err(|e| format!("start {e:?}"))?;
    let mut setup_passed = false;
    let mut positive_receipt = false;
    let mut controls_passed = false;
    let result = (|| -> Result<bool, String> {
        loop {
            before(setup_until)?;
            if let Some(report) = session.poll_registration() {
                require(
                    report.dispatched
                        && matches!(report.result, Ok(42))
                        && report.activation == Ok(()),
                    "actual registration activation",
                )?;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        require(
            !session.worker_occupied(),
            "registration worker actually joined",
        )?;
        let raw = next(&mut session, setup_until)?;
        require(
            matches!(raw.event(),RadioEvent::Response(r) if r.payload==[7]),
            "initial actual raw callback",
        )?;
        let raw_revision = session
            .publish_callback(raw)
            .map_err(|e| format!("initial publish {e:?}"))?;
        let mut report = control(&peer, 100)?;
        let mut values = [0; 6];
        for v in &mut values {
            *v = report.read_i32().map_err(|e| format!("report {e:?}"))?;
        }
        require(
            report.data_avail() == 0
                && values
                    == [
                        1,
                        0,
                        0,
                        std::process::id() as i32,
                        rustix::process::geteuid().as_raw() as i32,
                        0,
                    ],
            "one registration, actual caller and no commands or clear",
        )?;
        require(
            session.registration_view() == RegistrationView::default()
                && session.snapshot().request.is_none(),
            "production default unavailable, no operation",
        )?;
        before(setup_until)?;
        setup_passed = true;
        json(
            "registration-state-setup",
            format!(
                "\"client_pid\":{},\"client_uid\":{},\"selected_slot\":1,\"registration_count\":1,\"client_id\":42,\"worker_empty\":true,\"raw_history_revision\":{},\"source_generation\":1,\"original_setup_until_ns\":{},\"session_original_until_ns\":{},\"original_oracle_until_ns\":{},\"observed_at_ns\":{}",
                std::process::id(),
                rustix::process::geteuid().as_raw(),
                raw_revision,
                setup_until.as_nanos(),
                session_until.as_nanos(),
                until.as_nanos(),
                BootClock.now().as_nanos()
            ),
        );
        for (entity, request) in [(false, false), (false, true), (true, false)] {
            session
                .test_registration_context(&key, 1, entity, request)
                .map_err(|e| format!("private context {e:?}"))?;
            actual_commit(&mut session, &peer, 121, until)?;
            require(
                session.registration_view() == RegistrationView::default(),
                "missing prerequisite must not register",
            )?;
        }
        session
            .test_registration_context(&key, 1, true, true)
            .map_err(|e| format!("private admitted context {e:?}"))?;
        let revision = actual_commit(&mut session, &peer, 121, until)?;
        let expected = RegistrationObservation {
            source_generation: 1,
            source_revision: revision,
        };
        let view = session.registration_view();
        let observed = view.availability == Availability::RegistrationObserved(expected)
            && view.last_observed == Some(expected)
            && view.revision == 1;
        let desired_red = view == RegistrationView::default();
        require(
            observed || desired_red,
            "unexpected projection shape, not desired RED",
        )?;
        positive_receipt = true;
        json(
            "registration-state-positive",
            format!(
                "\"exact_typed_history\":true,\"source_revision\":{},\"availability\":\"{}\",\"last_observed\":{},\"desired_boundary_red\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                revision,
                if observed {
                    "registration-observed"
                } else {
                    "unavailable"
                },
                view.last_observed.is_some(),
                desired_red,
                BootClock.now().as_nanos(),
                until.as_nanos()
            ),
        );
        if desired_red {
            return Ok(false);
        }
        let mut last = expected;
        for code in [122, 123, 124, 125] {
            if code == 125 {
                let before_view = session.registration_view();
                let source_before = session.snapshot().revision;
                trigger(&peer, 125, until)?;
                let malformed = next(&mut session, until)?;
                require(
                    matches!(malformed.event(),RadioEvent::Indication(i) if i.message_id==3012 && i.slot_id==1 && i.payload==[2,0,0,0,0,0,0,0,0,0,0]),
                    "actual matching truncated bytes",
                )?;
                require(
                    session.publish_callback(malformed) == Err(OwnerError::InvalidIndication),
                    "matching malformed refusal",
                )?;
                require(
                    session.snapshot().revision == source_before
                        && session.registration_view().revision == before_view.revision + 1,
                    "wire revision preserved, view withdrawal revision advances",
                )?;
            } else {
                actual_commit(&mut session, &peer, code, until)?;
            }
            require(
                session.registration_view().availability == Availability::Unavailable
                    && session.registration_view().last_observed == Some(last),
                "conservative withdrawal retains exact last positive",
            )?;
            let revision = actual_commit(&mut session, &peer, 121, until)?;
            last = RegistrationObservation {
                source_generation: 1,
                source_revision: revision,
            };
            require(
                session.registration_view().availability
                    == Availability::RegistrationObserved(last),
                "new qualified callback restores conditional observation",
            )?;
        }
        let before_wrong = session.registration_view();
        trigger(&peer, 126, until)?;
        let wrong = next(&mut session, until)?;
        require(
            matches!(wrong.event(),RadioEvent::Indication(i) if i.slot_id==0 && i.message_id==3012 && i.payload==[2,0,0,0,0,0,0,0,0,0,0,0]),
            "actual wrong-slot callback",
        )?;
        require(
            session.publish_callback(wrong) == Err(OwnerError::Unsupported)
                && session.registration_view() == before_wrong,
            "wrong-slot leaves view unchanged",
        )?;
        let mut report = control(&peer, 100)?;
        let mut values = [0; 6];
        for v in &mut values {
            *v = report
                .read_i32()
                .map_err(|e| format!("final report {e:?}"))?;
        }
        require(
            report.data_avail() == 0
                && values
                    == [
                        1,
                        0,
                        0,
                        std::process::id() as i32,
                        rustix::process::geteuid().as_raw() as i32,
                        0,
                    ],
            "no retry, request or vendor clear",
        )?;
        require(
            control(&peer, 107).is_err(),
            "fixture peer exits without a reply",
        )?;
        loop {
            before(until)?;
            if session.snapshot().phase == Phase::Failed(ReceiverFailure::ServiceDied) {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let after = session.registration_view();
        require(
            after.availability == Availability::Unavailable
                && after.last_observed == Some(last)
                && after.revision == before_wrong.revision + 1,
            "real death-watch withdrawal retains exact source history",
        )?;
        require(
            session.snapshot().request.is_none(),
            "no OutcomeUnknown without an operation",
        )?;
        controls_passed = true;
        json(
            "registration-state-controls",
            format!(
                "\"disconnected\":true,\"unknown\":true,\"unqualified\":true,\"malformed\":true,\"wrong_slot\":true,\"actual_service_death\":true,\"history_retained\":true,\"no_commands_or_retry\":true,\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                BootClock.now().as_nanos(),
                until.as_nanos()
            ),
        );
        Ok(true)
    })();
    let cleanup_until = BootClock.now() + Duration::from_secs(2);
    session
        .stop(&key)
        .map_err(|e| format!("cleanup stop {e:?}"))?;
    while session.worker_occupied() {
        before(cleanup_until)?;
        let _ = session.poll_registration();
        std::thread::sleep(Duration::from_millis(1));
    }
    require(
        session.registration_view().availability == Availability::Unavailable,
        "cleanup withdrawal",
    )?;
    let request_none = session.snapshot().request.is_none();
    drop(session);
    before(cleanup_until)?;
    let desired_red = matches!(result, Ok(false));
    let passed = matches!(result, Ok(true));
    json(
        "registration-state-closed",
        format!(
            "\"requirement_passed\":{},\"desired_boundary_red\":{},\"setup_passed\":{},\"positive_receipt\":{},\"controls_passed\":{},\"registration_worker_joined\":true,\"owned_worker_occupied\":false,\"watch_dropped\":true,\"request_none\":{},\"no_vendor_cleanup_claim\":true,\"closed_at_ns\":{},\"original_cleanup_until_ns\":{},\"original_oracle_until_ns\":{}",
            passed,
            desired_red,
            setup_passed,
            positive_receipt,
            controls_passed,
            request_none,
            BootClock.now().as_nanos(),
            cleanup_until.as_nanos(),
            until.as_nanos()
        ),
    );
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(
            "required conditional registration observation missing after exact actual typed commit"
                .into(),
        ),
        Err(error) => Err(error),
    }
}
