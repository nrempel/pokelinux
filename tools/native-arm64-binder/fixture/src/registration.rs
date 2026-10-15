//! Fixed actual Binder registration-wire publication contract.
//! Candidate3012 must be separately qualified by the original raw-routing oracle.
//! This test's peer and all packets are authored; no IMS/modem meaning is claimed.
use crate::*;
use pokelinux_radio::{
    indication::IndicationKind,
    owner::{CommittedObservation, Observation, PreparedEvent, SessionKey},
    registration::RegistrationWire,
};
use std::io::Write;
#[path = "registration_vectors.rs"]
mod vectors;

#[derive(Debug)]
struct MissingPublication;
impl std::fmt::Display for MissingPublication {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str("registration-wire publication required; actual Unsupported after exact callback receipt")
    }
}
impl std::error::Error for MissingPublication {}

fn require(ok: bool, text: &str) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(std::io::Error::other(text).into())
    }
}
fn before(until: Duration) -> TestResult {
    require(
        BootClock.now() < until,
        "original registration-wire case cutoff",
    )
}
fn next(
    session: &mut TransportSession,
    until: Duration,
) -> Result<PreparedEvent, Box<dyn std::error::Error>> {
    loop {
        before(until)?;
        let event = session
            .prepare_callback_until(Instant::now())
            .map_err(|error| format!("prepare callback: {error:?}"))?;
        before(until)?;
        if let Some(event) = event {
            return Ok(event);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn trigger(peer: &SIBinder, code: u32, until: Duration) -> TestResult {
    before(until)?;
    latch(peer, code)?;
    before(until)
}
fn exact_indication(event: &PreparedEvent, control: u32) -> TestResult {
    let (message, slot, fragment, bytes) =
        vectors::callback(control).ok_or("unknown fixed vector")?;
    let RadioEvent::Indication(actual) = event.event() else {
        return Err("expected actual indication endpoint callback".into());
    };
    require(
        actual.message_id == message && actual.slot_id == slot,
        "actual indication route metadata",
    )?;
    require(
        actual.payload == bytes,
        "actual indication full payload bytes",
    )?;
    require(
        if fragment {
            matches!(actual.kind, IndicationKind::Segment { index: 0, total_length } if total_length == bytes.len() as i32)
        } else {
            actual.kind == IndicationKind::Raw
        },
        "actual indication raw/fragment kind",
    )
}
fn fields(wire: &RegistrationWire) -> TestResult {
    require(
        wire.registration_state_wire == 255
            && wire.feature_wire == 128
            && wire.feature_mask_wire == i32::MIN
            && wire.status_code_wire == u16::MAX,
        "exact opaque registration scalar fields",
    )?;
    require(
        wire.uri_bytes() == b"sip:route" && wire.reason_bytes() == b"ok",
        "exact owned URI/reason bytes",
    )?;
    require(
        wire.feature_mask_ext_wire == Some(0x11223344)
            && wire.sip_delegate_mask_wire == Some(0x0102030405060708)
            && wire.feature_ext_wire == Some(0x3344),
        "exact present registration extension fields",
    )
}
fn typed(observed: &CommittedObservation) -> TestResult {
    let Observation::RegistrationWire {
        slot_id,
        message_id,
        wire,
    } = &observed.kind
    else {
        return Err("latest observation is not registration wire".into());
    };
    require(
        *slot_id == 0 && *message_id == vectors::CANDIDATE_MESSAGE_ID,
        "committed registration route",
    )?;
    fields(wire)
}
fn positive(session: &mut TransportSession, peer: &SIBinder, until: Duration) -> TestResult {
    let previous = session.snapshot();
    trigger(peer, vectors::VALID, until)?;
    let event = next(session, until)?;
    exact_indication(&event, vectors::VALID)?;
    let prepared_base_revision = event.base_revision();
    require(
        prepared_base_revision == previous.revision,
        "actual preparation base revision",
    )?;
    // This proves actual receiver consumption before the new capability assertion.
    println!(
        "{{\"schema\":1,\"event\":\"registration-wire-received\",\"client_pid\":{},\"selected_slot\":{},\"candidate_message_id\":{},\"raw\":true,\"payload_length\":{},\"payload_hex\":\"ff8000000080ffff090002007369703a726f7574656f6b4433221108070605040302014433\",\"original_route_qualification_required\":true,\"prepared_base_revision\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}}}",
        std::process::id(),
        session.selected_slot(),
        vectors::CANDIDATE_MESSAGE_ID,
        vectors::valid().len(),
        prepared_base_revision,
        BootClock.now().as_nanos(),
        until.as_nanos()
    );
    std::io::stdout().flush()?;
    before(until)?;
    let publication = session.publish_callback(event);
    before(until)?;
    let after = session.snapshot();
    let history_unchanged = match (&previous.observation, &after.observation) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    println!(
        "{{\"schema\":1,\"event\":\"registration-wire-publication\",\"actual_result\":\"{:?}\",\"before_revision\":{},\"after_revision\":{},\"history_unchanged\":{},\"owned_worker_occupied\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}}}",
        publication,
        previous.revision,
        after.revision,
        history_unchanged,
        session.worker_occupied(),
        BootClock.now().as_nanos(),
        until.as_nanos()
    );
    std::io::stdout().flush()?;
    before(until)?;
    let revision = match publication {
        Ok(revision) => revision,
        Err(error) => {
            require(
                error == OwnerError::Unsupported,
                "unexpected baseline publication error",
            )?;
            require(
                after.revision == previous.revision,
                "refused callback changed owner revision",
            )?;
            require(
                history_unchanged,
                "refused callback changed committed history",
            )?;
            require(
                after.phase == previous.phase,
                "refused callback changed transport phase",
            )?;
            return Err(Box::new(MissingPublication));
        }
    };
    require(
        after.phase == Phase::Active { client_id: 42 },
        "registration wire changed transport phase",
    )?;
    require(
        after.revision == revision && revision > previous.revision,
        "registration commit revision",
    )?;
    let observed = after
        .observation
        .ok_or("missing committed registration observation")?;
    require(
        observed.revision == revision,
        "observation owns exact commit revision",
    )?;
    typed(&observed)
}
fn negative(
    session: &mut TransportSession,
    peer: &SIBinder,
    code: u32,
    until: Duration,
) -> TestResult {
    let previous = session.snapshot();
    trigger(peer, code, until)?;
    let event = next(session, until)?;
    exact_indication(&event, code)?;
    let publication = session.publish_callback(event);
    before(until)?;
    require(
        publication.is_err(),
        "unselected or malformed indication published",
    )?;
    if matches!(
        code,
        vectors::WRONG_SLOT | vectors::WRONG_ID | vectors::FRAGMENT
    ) {
        require(
            publication == Err(OwnerError::Unsupported),
            "unselected indication refusal kind",
        )?;
    }
    let after = session.snapshot();
    require(
        after.phase == previous.phase && after.revision == previous.revision,
        "refused indication changed transport/revision",
    )?;
    let before_observation = previous.observation.ok_or("missing prior typed history")?;
    let after_observation = after
        .observation
        .ok_or("refusal cleared prior typed history")?;
    require(
        Arc::ptr_eq(&before_observation, &after_observation),
        "refusal replaced historical observation",
    )?;
    typed(&after_observation)?;
    println!(
        "REGISTRATION-WIRE-REFUSAL control={} actual={:?}",
        code, publication
    );
    std::io::stdout().flush()?;
    before(until)
}
fn raw_replacement(
    session: &mut TransportSession,
    peer: &SIBinder,
    key: &SessionKey,
    until: Duration,
) -> TestResult {
    let previous = session.snapshot();
    let historical = previous
        .observation
        .ok_or("missing typed history before request")?;
    let request = session
        .prepare_request(key, 137, 0, vec![9, 8], Deadline::at(until))
        .map_err(|error| format!("prepare authored public request: {error:?}"))?;
    session
        .submit_request(request)
        .map_err(|error| format!("submit authored public request: {error:?}"))?;
    let report = loop {
        before(until)?;
        if let Some(report) = session.poll_request() {
            before(until)?;
            break report.map_err(|_| "authored request worker panicked")?;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    require(
        report.submitted_parts == 1
            && report.submission.is_ok()
            && report.outcome == Outcome::OutcomeUnknown,
        "authored request submission is uncertainty only",
    )?;
    let receipt_only = session.snapshot();
    require(
        receipt_only.revision == previous.revision,
        "request-only receipt changed callback revision",
    )?;
    require(
        Arc::ptr_eq(
            &historical,
            receipt_only
                .observation
                .as_ref()
                .ok_or("request cleared typed history")?,
        ),
        "request-only receipt republished historical observation",
    )?;
    let event = next(session, until)?;
    require(
        matches!(event.event(), RadioEvent::Response(response)
        if response.kind == pokelinux_radio::response::ResponseKind::Raw && response.payload == [9, 8]),
        "actual authored raw response callback",
    )?;
    let revision = session
        .publish_callback(event)
        .map_err(|error| format!("publish raw replacement: {error:?}"))?;
    let snapshot = session.snapshot();
    let observed = snapshot
        .observation
        .ok_or("missing raw replacement history")?;
    require(
        observed.revision == revision && revision > historical.revision,
        "raw replacement commit revision",
    )?;
    require(
        observed
            .kind
            .raw_response()
            .is_some_and(|response| response.payload == [9, 8]),
        "typed-to-raw replacement",
    )?;
    positive(session, peer, until)
}
fn cleanup(session: &mut TransportSession, key: &SessionKey, until: Duration) -> TestResult {
    session
        .stop(key)
        .map_err(|error| format!("normal session Stop: {error:?}"))?;
    while session.worker_occupied() {
        let _ = session.poll_registration();
        let _ = session.poll_request();
        if BootClock.now() >= until {
            // An unjoined worker cannot be detached or reported as a qualified RED.
            // The external process owner must retain and close this failed case.
            std::process::abort();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    require(
        session.snapshot().observation.is_none(),
        "Stop did not withdraw owner observation",
    )?;
    before(until)
}
pub fn run(peer: &SIBinder) -> TestResult {
    let setup_until = BootClock.now() + Duration::from_secs(10);
    let mut session = TransportSession::new(n(2), 0);
    let session_until = BootClock.now() + Duration::from_secs(30);
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "pokelinux-radio".into(),
            n(8),
            Deadline::at(session_until),
        )
        .map_err(|error| format!("start actual registration session: {error:?}"))?;
    let result = (|| -> TestResult {
        let report = loop {
            before(setup_until)?;
            if let Some(report) = session.poll_registration() {
                before(setup_until)?;
                break report;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        require(
            report.dispatched && matches!(report.result, Ok(42)) && report.activation == Ok(()),
            "actual registration must activate before publication oracle",
        )?;
        require(
            !session.worker_occupied() && session.selected_slot() == 0,
            "joined registration and explicit immutable slot",
        )?;
        let registration = crate::report(peer)?;
        require(
            registration[0] == 1
                && registration[3] == std::process::id() as i32
                && registration[4] == rustix::process::geteuid().as_raw() as i32
                && registration[5] == 0,
            "one actual peer registration/caller and no clear",
        )?;
        let raw = next(&mut session, setup_until)?;
        require(
            matches!(raw.event(), RadioEvent::Response(response)
            if response.kind == pokelinux_radio::response::ResponseKind::Raw && response.payload == [7]),
            "actual existing raw callback prerequisite",
        )?;
        let raw_revision = session
            .publish_callback(raw)
            .map_err(|error| format!("existing raw publication: {error:?}"))?;
        let raw = session
            .snapshot()
            .observation
            .ok_or("existing raw history missing")?;
        require(
            raw.revision == raw_revision
                && raw
                    .kind
                    .raw_response()
                    .is_some_and(|response| response.payload == [7]),
            "existing raw payload through latest committed enum",
        )?;
        before(setup_until)?;
        println!(
            "{{\"schema\":1,\"event\":\"registration-wire-setup\",\"client_pid\":{},\"client_uid\":{},\"registration_count\":1,\"client_id\":42,\"selected_slot\":0,\"owner_active\":true,\"raw_history_payload_hex\":\"07\",\"raw_history_revision\":{},\"worker_empty\":true,\"observed_at_ns\":{},\"original_setup_until_ns\":{},\"session_original_until_ns\":{}}}",
            std::process::id(),
            rustix::process::geteuid().as_raw(),
            raw_revision,
            BootClock.now().as_nanos(),
            setup_until.as_nanos(),
            session_until.as_nanos()
        );
        std::io::stdout().flush()?;
        before(setup_until)?;
        let until = BootClock.now() + Duration::from_secs(3);
        positive(&mut session, peer, until)?;
        raw_replacement(&mut session, peer, &key, until)?;
        for code in [
            vectors::WRONG_SLOT,
            vectors::WRONG_ID,
            vectors::FRAGMENT,
            vectors::TRUNCATED,
            vectors::TRAILING,
        ] {
            negative(&mut session, peer, code, until)?;
        }
        trigger(peer, vectors::VALID, until)?;
        let held = next(&mut session, until)?;
        exact_indication(&held, vectors::VALID)?;
        let prepared_base_revision = held.base_revision();
        require(
            session.snapshot().revision == prepared_base_revision,
            "cancellation preparation base revision",
        )?;
        println!(
            "{{\"schema\":1,\"event\":\"registration-wire-received\",\"client_pid\":{},\"selected_slot\":{},\"candidate_message_id\":{},\"raw\":true,\"payload_length\":{},\"payload_hex\":\"ff8000000080ffff090002007369703a726f7574656f6b4433221108070605040302014433\",\"original_route_qualification_required\":true,\"prepared_base_revision\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}}}",
            std::process::id(),
            session.selected_slot(),
            vectors::CANDIDATE_MESSAGE_ID,
            vectors::valid().len(),
            prepared_base_revision,
            BootClock.now().as_nanos(),
            until.as_nanos()
        );
        std::io::stdout().flush()?;
        before(until)?;
        session
            .stop(&key)
            .map_err(|error| format!("cancel held publication: {error:?}"))?;
        let previous = session.snapshot();
        require(
            previous.observation.is_none() && previous.revision == prepared_base_revision + 1,
            "Stop must clear history and advance revision once",
        )?;
        let publication = session.publish_callback(held);
        before(until)?;
        let after = session.snapshot();
        let history_unchanged = previous.observation.is_none() && after.observation.is_none();
        println!(
            "{{\"schema\":1,\"event\":\"registration-wire-publication\",\"actual_result\":\"{:?}\",\"before_revision\":{},\"after_revision\":{},\"history_unchanged\":{},\"owned_worker_occupied\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}}}",
            publication,
            previous.revision,
            after.revision,
            history_unchanged,
            session.worker_occupied(),
            BootClock.now().as_nanos(),
            until.as_nanos()
        );
        std::io::stdout().flush()?;
        before(until)?;
        require(
            publication == Err(OwnerError::Failed(ReceiverFailure::Cancelled)),
            "consumed valid indication survived cancellation",
        )?;
        require(
            history_unchanged && after.revision == previous.revision,
            "cancelled preparation changed history/revision",
        )?;
        before(until)?;
        Ok(())
    })();
    let desired_unsupported_red = result
        .as_ref()
        .err()
        .is_some_and(|error| error.as_ref().is::<MissingPublication>());
    // The baseline Unsupported result is retained while normal owned cleanup runs.
    let cleanup_until = BootClock.now() + Duration::from_secs(2);
    cleanup(&mut session, &key, cleanup_until)?;
    drop(session); // Registration/request workers are empty; retained watch drops normally.
    before(cleanup_until)?;
    let witness = crate::report(peer)?;
    require(
        witness[0] == 1 && witness[5] == 0,
        "cleanup must not retry registration or clear vendor state",
    )?;
    println!(
        "{{\"schema\":1,\"event\":\"registration-wire-session-closed\",\"client_pid\":{},\"worker_empty\":true,\"registration_worker_joined\":true,\"owned_worker_occupied\":false,\"watch_dropped\":true,\"peer_registration_count\":1,\"peer_clear_count\":0,\"desired_unsupported_red\":{},\"requirement_passed\":{},\"closed_at_ns\":{},\"original_cleanup_until_ns\":{},\"vendor_cleanup\":false}}",
        std::process::id(),
        desired_unsupported_red,
        result.is_ok(),
        BootClock.now().as_nanos(),
        cleanup_until.as_nanos()
    );
    std::io::stdout().flush()?;
    before(cleanup_until)?;
    if result.is_ok() {
        marker("registration-wire-actual-callback-history-and-refusal-controls");
    }
    result
}
