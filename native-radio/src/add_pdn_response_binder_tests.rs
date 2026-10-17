//! Frozen actual ARM Binder response-attribution contract; authored peer only.
//! Existing typed request, full original bytes and owned cleanup remain required.
use crate::{
    owner::{OwnerError, Phase},
    receiver::{RadioEvent, ReceiverFailure},
    request::{Outcome, RequestFailure},
    transport::RadioTransport,
    worker::{AddPdnFailure, LifecycleError, TransportSession},
};
use omarchy_ims::add_pdn::{AddPdn, EncodeError, WireTime, encode};
use pokelinux_process_owner::deadline::{BootClock, Clock, Deadline};
use rsbinder::{FLAG_ONEWAY, Parcel, ProcessState, SIBinder, Status};
use std::{
    io::Write,
    net::{Ipv4Addr, Ipv6Addr},
    num::NonZeroUsize,
    time::{Duration, Instant},
};
type TestResult<T = ()> = Result<T, String>;
const VECTOR_ID: &str = "empty-pcscf-wire2-fixed-time";
const EXPECTED_TSV_SHA: &str = "d5bd8106f83b3cc260a04dfa4087f35fe6d65adc4abd8c0afb0c45359f66465c";
fn require(ok: bool, text: &str) -> TestResult {
    if ok { Ok(()) } else { Err(text.into()) }
}
fn before(until: Duration) -> TestResult {
    require(BootClock.now() < until, "original add-PDN cutoff")
}
fn n(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn emit(event: &str, fields: String) -> TestResult {
    let line = format!("{{\"schema\":1,\"event\":\"{event}\",{fields}}}\n");
    require(line.len() <= 4096, "bounded authored metadata")?;
    std::io::stdout()
        .write_all(line.as_bytes())
        .map_err(|e| format!("stdout {e}"))?;
    std::io::stdout().flush().map_err(|e| format!("flush {e}"))
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 15) as usize] as char);
    }
    result
}
fn original() -> TestResult<Vec<u8>> {
    let row = include_str!("../../native-ims/tests/fixtures/shannon-add-pdn.tsv")
        .lines()
        .find(|line| line.split('\t').next() == Some(VECTOR_ID))
        .ok_or("missing original vector")?;
    let fields: Vec<_> = row.split('\t').collect();
    require(
        fields.len() == 6 && fields[1..5] == ["2", "0", "0", "0"] && fields[5].len() == 1326,
        "fixed original vector recipe",
    )?;
    fields[5]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|e| format!("hex UTF8 {e}"))?;
            u8::from_str_radix(text, 16).map_err(|e| format!("hex {e}"))
        })
        .collect()
}
fn config() -> AddPdn<'static> {
    AddPdn {
        pdn_type_wire: 0,
        ip_type_wire: 3,
        pdn_ipv4: Ipv4Addr::new(192, 0, 2, 10),
        pdn_ipv6: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10),
        hd_enabled_wire: 1,
        operator_id_wire: 7,
        registration_style_wire: 2,
        new_pdn_wire: 1,
        volte_auto_config_wire: 0,
        rcs_auto_config_wire: 1,
        offering_type_wire: 0x1234,
        registration_type_wire: 4,
        init_type_wire: 0,
        ims_net_type_wire: 2,
        mac: Some([2, 0, 0, 0, 0, 1]),
        pcscf_ipv4: &[],
        pcscf_ipv6: &[],
        primary_dns_ipv4: Ipv4Addr::new(192, 0, 2, 53),
        primary_dns_ipv6: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x53),
        secondary_dns_ipv4: Ipv4Addr::new(198, 51, 100, 53),
        secondary_dns_ipv6: Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 0x53),
        last_access_network_wire: 7,
        last_access_time: Some(WireTime {
            year: 2020,
            month: 1,
            day: 2,
            hour: 3,
            minute: 4,
            second: 5,
            utc_offset_minutes: -210,
        }),
        time: Some(WireTime {
            year: 2024,
            month: 2,
            day: 29,
            hour: 23,
            minute: 58,
            second: 59,
            utc_offset_minutes: 330,
        }),
        tac_or_lac_wire: 0x12345678,
        cell_id32_wire: 0x01020304,
        remaining_timer_wire: 0x01020304,
        mcc_mnc: Some("310260"),
        country: Some("US"),
        mtu_wire: 1500,
        rcs_capability_wire: 0x10203040,
        vendor_capability_wire: 0x1234,
        extended_rcs_capability_wire: 0x55667788,
        e911_aid: Some("ORACLE"),
        band_wire: 0x01020304,
        cell_id64_wire: 0x0102030405060708,
        network_interface_index_wire: 0x00010203,
        sip_delegate_capability_wire: 0x1112131415161718,
    }
}
fn control(peer: &SIBinder, code: u32) -> TestResult<Parcel> {
    let remote = peer.as_remote().ok_or("local endpoint")?;
    let data = remote
        .prepare_transact(true)
        .map_err(|e| format!("prepare {e:?}"))?;
    let mut reply = remote
        .submit_transact(code, &data, 0)
        .map_err(|e| format!("control{code} {e:?}"))?
        .ok_or("missing reply")?;
    let status: Status = reply.read().map_err(|e| format!("status {e:?}"))?;
    require(status.is_ok(), "fixture status")?;
    Ok(reply)
}
#[derive(Debug)]
struct Capture {
    epoch: i32,
    count: i32,
    total: i32,
    args: [i32; 4],
    owner: [i32; 2],
    calling: [i32; 2],
    payload: Vec<u8>,
}
fn capture(peer: &SIBinder, until: Duration) -> TestResult<Capture> {
    before(until)?;
    let mut reply = control(peer, 127)?;
    let version = reply
        .read_i32()
        .map_err(|e| format!("capture version {e:?}"))?;
    let epoch = reply
        .read_i32()
        .map_err(|e| format!("capture epoch {e:?}"))?;
    let count = reply
        .read_i32()
        .map_err(|e| format!("capture count {e:?}"))?;
    let total = reply
        .read_i32()
        .map_err(|e| format!("capture total {e:?}"))?;
    let mut args = [0; 4];
    for v in &mut args {
        *v = reply.read_i32().map_err(|e| format!("args {e:?}"))?;
    }
    let mut owner = [0; 2];
    for v in &mut owner {
        *v = reply.read_i32().map_err(|e| format!("owner {e:?}"))?;
    }
    let mut calling = [0; 2];
    for v in &mut calling {
        *v = reply.read_i32().map_err(|e| format!("calling {e:?}"))?;
    }
    let payload: Vec<u8> = reply.read().map_err(|e| format!("capture payload {e:?}"))?;
    require(
        version == 1
            && (0..=1).contains(&epoch)
            && (0..=1).contains(&count)
            && (0..=2).contains(&total)
            && payload.len() <= 2048
            && reply.data_avail() == 0,
        "fixed enabled capture contract and bounds",
    )?;
    require(
        owner
            == [
                std::process::id() as i32,
                rustix::process::geteuid().as_raw() as i32,
            ],
        "capture bound to registered owner",
    )?;
    if count == 0 {
        require(
            args == [0; 4] && calling == [0; 2] && payload.is_empty(),
            "empty fixture capture",
        )?;
    }
    before(until)?;
    Ok(Capture {
        epoch,
        count,
        total,
        args,
        owner,
        calling,
        payload,
    })
}
fn exact_capture(actual: &Capture, expected: &[u8], epoch: i32, total: i32) -> TestResult {
    require(
        actual.epoch == epoch
            && actual.count == 1
            && actual.total == total
            && actual.args == [1, 42, 105, 1]
            && actual.payload == expected
            && actual.calling == [0, 1000]
            && actual.owner == [std::process::id() as i32, 1000],
        "independent exact oneway argument/payload capture",
    )
}
fn empty_capture(peer: &SIBinder, until: Duration, epoch: i32, total: i32) -> TestResult {
    let actual = capture(peer, until)?;
    require(
        actual.epoch == epoch && actual.count == 0 && actual.total == total,
        "exact capture epoch/count/total",
    )
}
fn preflight(peer: &SIBinder, expected: &[u8], until: Duration) -> TestResult {
    before(until)?;
    let remote = peer.as_remote().ok_or("local preflight endpoint")?;
    let mut data = remote
        .prepare_transact(true)
        .map_err(|e| format!("preflight prepare {e:?}"))?;
    for value in [1, 42, 105, 1] {
        data.write_i32(value)
            .map_err(|e| format!("preflight argument {e:?}"))?;
    }
    data.write(expected)
        .map_err(|e| format!("preflight bytes {e:?}"))?;
    let reply = remote
        .submit_transact(2, &data, FLAG_ONEWAY)
        .map_err(|e| format!("preflight submit {e:?}"))?;
    require(reply.is_none(), "oneway preflight has no invented response")?;
    loop {
        let actual = capture(peer, until)?;
        if actual.count == 1 {
            exact_capture(&actual, expected, 0, 1)?;
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut reset = control(peer, 128)?;
    require(
        reset
            .read_i32()
            .map_err(|e| format!("fixture-only reset ack {e:?}"))?
            == 1
            && reset.data_avail() == 0,
        "exact fixture-only reset acknowledgement",
    )?;
    empty_capture(peer, until, 1, 1)?;
    before(until)
}
fn report(peer: &SIBinder, until: Duration) -> TestResult {
    before(until)?;
    let mut reply = control(peer, 100)?;
    let mut values = [0; 6];
    for v in &mut values {
        *v = reply.read_i32().map_err(|e| format!("report {e:?}"))?;
    }
    require(
        reply.data_avail() == 0
            && values
                == [
                    1,
                    0,
                    0,
                    std::process::id() as i32,
                    rustix::process::geteuid().as_raw() as i32,
                    0,
                ],
        "one registration, no unrelated request, segment or clear",
    )?;
    before(until)
}
fn outcome(value: Option<Outcome>) -> &'static str {
    match value {
        None => "none",
        Some(Outcome::Queued) => "queued",
        Some(Outcome::NotSent) => "not-sent",
        Some(Outcome::OutcomeUnknown) => "outcome-unknown",
    }
}
#[test]
#[ignore = "requires isolated generic ARM Binder, public manager/peer and fixed original byte fixture"]
fn actual_binder_add_pdn_response() -> TestResult {
    println!();
    let started = BootClock.now();
    let setup_until = started + Duration::from_secs(10);
    let until = started + Duration::from_secs(20);
    let session_until = started + Duration::from_secs(30);
    let group = std::env::var("POKELINUX_ADD_PDN_TEST_GROUP").map_err(|_| "group missing")?;
    require(
        matches!(group.as_str(), "sent" | "cancel-before" | "expiry-before"),
        "fixed group",
    )?;
    let expected = original()?;
    let config = config();
    let encoded = encode(&config).map_err(|e| format!("encoder {e:?}"))?;
    require(
        encoded == expected && expected.len() == 663,
        "complete original TSV byte comparison",
    )?;
    let device = std::env::var("POKELINUX_ADD_PDN_TEST_DEVICE").map_err(|_| "device missing")?;
    before(setup_until)?;
    let process = ProcessState::init(&device, 3).map_err(|e| format!("Binder {e:?}"))?;
    ProcessState::start_thread_pool();
    let manager = process
        .context_object()
        .map_err(|e| format!("manager {e:?}"))?;
    require(
        manager.is_remote() && manager.descriptor() == "android.os.IServiceManager",
        "actual manager interface",
    )?;
    let remote = manager.as_remote().ok_or("local manager")?;
    let mut data = remote
        .prepare_transact(true)
        .map_err(|e| format!("lookup {e:?}"))?;
    data.write("org.pokelinux.fixture.arm64-radio")
        .map_err(|e| format!("name {e:?}"))?;
    let mut reply = remote
        .submit_transact(3, &data, 0)
        .map_err(|e| format!("lookup {e:?}"))?
        .ok_or("no service")?;
    let status: Status = reply.read().map_err(|e| format!("lookup status {e:?}"))?;
    require(status.is_ok(), "named lookup")?;
    let peer: SIBinder = reply.read().map_err(|e| format!("peer {e:?}"))?;
    require(
        reply.data_avail() == 0 && peer.is_remote() && peer != manager,
        "distinct remote Radio endpoint",
    )?;
    let transport =
        RadioTransport::from_binder(peer.clone()).map_err(|e| format!("transport {e:?}"))?;
    let mut session = TransportSession::new(n(4), 1);
    before(setup_until)?;
    let key = session
        .start(
            transport,
            "pokelinux-ipc-test".into(),
            n(8),
            Deadline::at(session_until),
        )
        .map_err(|e| format!("start {e:?}"))?;
    let mut setup_passed = false;
    let mut prepared_queued = false;
    let mut request_joined = false;
    let mut desired_red = false;
    let mut captured_count = 0;
    let mut preflight_passed = false;
    let result = (|| -> TestResult<bool> {
        loop {
            before(setup_until)?;
            if let Some(r) = session.poll_registration() {
                require(
                    r.dispatched && matches!(r.result, Ok(42)) && r.activation == Ok(()),
                    "real registration activation",
                )?;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        require(
            !session.worker_occupied(),
            "registration worker actually joined",
        )?;
        loop {
            before(setup_until)?;
            if let Some(raw) = session
                .prepare_callback_until(Instant::now())
                .map_err(|e| format!("initial callback {e:?}"))?
            {
                require(
                    matches!(raw.event(),RadioEvent::Response(r) if r.payload==[7]),
                    "actual initial raw callback",
                )?;
                session
                    .publish_callback(raw)
                    .map_err(|e| format!("initial publish {e:?}"))?;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        report(&peer, setup_until)?;
        empty_capture(&peer, setup_until, 0, 0)?;
        require(
            session.snapshot().request.is_none()
                && session.registration_view() == Default::default(),
            "no operation or registration readiness",
        )?;
        let mut invalid = config.clone();
        invalid.ims_net_type_wire = 1;
        require(
            matches!(
                session.prepare_add_pdn(&key, &invalid, Deadline::at(until)),
                Err(AddPdnFailure::Encode(EncodeError::UnsupportedSelector))
            ),
            "invalid typed config refused before admission",
        )?;
        require(
            session.snapshot().request.is_none(),
            "invalid config consumed no admission",
        )?;
        empty_capture(&peer, setup_until, 0, 0)?;
        before(setup_until)?;
        setup_passed = true;
        emit(
            "add-pdn-transport-setup",
            format!(
                "\"group\":\"{}\",\"client_pid\":{},\"client_uid\":{},\"selected_slot\":1,\"registration_count\":1,\"client_id\":42,\"worker_empty\":true,\"capture_version\":1,\"capture_count\":0,\"invalid_config_refused\":true,\"request_none\":true,\"registration_unavailable\":true,\"reference_vector_id\":\"{}\",\"expected_tsv_sha256\":\"{}\",\"payload_length\":663,\"payload_hex\":\"{}\",\"original_setup_until_ns\":{},\"original_oracle_until_ns\":{},\"session_original_until_ns\":{},\"observed_at_ns\":{}",
                group,
                std::process::id(),
                rustix::process::geteuid().as_raw(),
                VECTOR_ID,
                EXPECTED_TSV_SHA,
                hex(&expected),
                setup_until.as_nanos(),
                until.as_nanos(),
                session_until.as_nanos(),
                BootClock.now().as_nanos()
            ),
        )?;
        preflight(&peer, &expected, until)?;
        preflight_passed = true;
        require(
            session.snapshot().request.is_none() && !session.worker_occupied(),
            "fixture preflight/reset changed no owner request or worker",
        )?;
        report(&peer, until)?;
        emit(
            "add-pdn-transport-preflight",
            format!(
                "\"group\":\"{}\",\"serial\":1,\"client_id\":42,\"request\":105,\"slot\":1,\"owner_pid\":{},\"owner_uid\":1000,\"calling_pid\":0,\"calling_uid\":1000,\"payload_length\":663,\"exact_fixture_capture\":true,\"fixture_only_reset128\":true,\"preflight_capture_epoch\":0,\"preflight_capture_count\":1,\"capture_epoch_after_reset\":1,\"capture_count_after_reset\":0,\"total_wire105_after_reset\":1,\"owner_request_none\":true,\"owner_worker_empty\":true,\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                group,
                std::process::id(),
                BootClock.now().as_nanos(),
                until.as_nanos()
            ),
        )?;
        let request_until = if group == "expiry-before" {
            BootClock.now() + Duration::from_millis(250)
        } else {
            until
        };
        let prepared = match session.prepare_add_pdn(&key, &config, Deadline::at(request_until)) {
            Ok(prepared) => prepared,
            Err(AddPdnFailure::Lifecycle(LifecycleError::Owner(OwnerError::Unsupported)))
                if group == "sent" =>
            {
                require(
                    session.snapshot().request.is_none() && !session.worker_occupied(),
                    "safe baseline refusal has no owned dispatch or receipt",
                )?;
                empty_capture(&peer, until, 1, 1)?;
                desired_red = true;
                emit(
                    "add-pdn-transport-prepared",
                    format!(
                        "\"group\":\"sent\",\"adapter\":\"unsupported\",\"desired_boundary_red\":true,\"request_outcome\":\"none\",\"capture_count\":0,\"capture_epoch\":1,\"total_wire105\":1,\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                        BootClock.now().as_nanos(),
                        until.as_nanos()
                    ),
                )?;
                return Ok(false);
            }
            Err(error) => return Err(format!("typed preparation {error:?}")),
        };
        let request_key = prepared.key();
        let receipt = session.snapshot().request.ok_or("queued receipt missing")?;
        require(
            receipt.key == request_key
                && receipt.serial == 1
                && receipt.outcome == Outcome::Queued
                && receipt.revision == 0,
            "current queued exact request receipt",
        )?;
        empty_capture(&peer, until, 1, 1)?;
        prepared_queued = true;
        emit(
            "add-pdn-transport-prepared",
            format!(
                "\"group\":\"{}\",\"adapter\":\"queued\",\"desired_boundary_red\":false,\"request_outcome\":\"queued\",\"capture_count\":0,\"capture_epoch\":1,\"total_wire105\":1,\"original_request_until_ns\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                group,
                request_until.as_nanos(),
                BootClock.now().as_nanos(),
                until.as_nanos()
            ),
        )?;
        require(group == "sent", "response integration selects exactly sent")?;
        // Synchronous fixture ACK returns only after the real callback handler's
        // gate/deliver has completed. It is not a vendor sender-flags claim.
        let mut queued_ack = control(&peer, 140)?;
        require(
            queued_ack
                .read_i32()
                .map_err(|e| format!("queued ack {e:?}"))?
                == 1
                && queued_ack.data_avail() == 0,
            "actual receiver admission acknowledgement",
        )?;
        let before_dispatch_ack = BootClock.now();
        emit(
            "add-pdn-response-prepermission",
            format!(
                "\"receiver_callback_completed\":true,\"request_still_queued\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                session
                    .snapshot()
                    .request
                    .ok_or("prepermission receipt")?
                    .outcome
                    == Outcome::Queued,
                before_dispatch_ack.as_nanos(),
                until.as_nanos()
            ),
        )?;
        let reason = if group == "cancel-before" {
            session
                .cancel_request(&request_key)
                .map_err(|e| format!("cancel {e:?}"))?;
            Some(ReceiverFailure::Cancelled)
        } else if group == "expiry-before" {
            while BootClock.now() < request_until {
                before(until)?;
                std::thread::sleep(Duration::from_millis(1));
            }
            Some(ReceiverFailure::Expired)
        } else {
            None
        };
        if reason.is_some() {
            require(
                session.snapshot().request.ok_or("cancel receipt")?.outcome == Outcome::NotSent,
                "pre-dispatch cancellation/expiry remains NotSent",
            )?;
        }
        session
            .submit_request(prepared)
            .map_err(|e| format!("submit {e:?}"))?;
        let request_report = loop {
            before(until)?;
            if let Some(report) = session.poll_request() {
                break report.map_err(|_| "request worker panicked")?;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        request_joined = true;
        require(
            request_report.key == request_key && !session.worker_occupied(),
            "actual request worker joined",
        )?;
        if let Some(reason) = reason {
            require(
                request_report.submitted_parts == 0
                    && matches!(request_report.submission,Err(RequestFailure::Admission(OwnerError::Failed(actual))) if actual==reason)
                    && request_report.outcome == Outcome::NotSent,
                "not-sent admission report",
            )?;
            empty_capture(&peer, until, 1, 1)?;
            require(
                session
                    .snapshot()
                    .request
                    .ok_or("final NotSent receipt")?
                    .outcome
                    == Outcome::NotSent,
                "no owned peer capture after pre-dispatch refusal",
            )?;
            report(&peer, until)?;
            emit(
                "add-pdn-transport-result",
                format!(
                    "\"group\":\"{}\",\"request_outcome\":\"not-sent\",\"submitted_parts\":0,\"capture_count\":0,\"capture_epoch\":1,\"total_wire105\":1,\"exact_capture\":false,\"request_worker_joined\":true,\"peer_control107\":false,\"registration_unavailable\":true,\"original_request_until_ns\":{},\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                    group,
                    request_until.as_nanos(),
                    BootClock.now().as_nanos(),
                    until.as_nanos()
                ),
            )?;
        } else {
            require(
                request_report.submitted_parts == 1
                    && request_report.submission.is_ok()
                    && request_report.outcome == Outcome::OutcomeUnknown,
                "one-way submission creates uncertainty only",
            )?;
            let actual = loop {
                let c = capture(&peer, until)?;
                if c.count == 1 {
                    break c;
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            exact_capture(&actual, &expected, 1, 2)?;
            captured_count = 1;
            let receive = |session: &mut TransportSession| -> TestResult<u64> {
                before(until)?;
                let event = session
                    .prepare_callback_until(Instant::now() + Duration::from_millis(100))
                    .map_err(|e| format!("actual response drain {e:?}"))?
                    .ok_or("synchronous callback missing from actual queue")?;
                session
                    .publish_callback(event)
                    .map_err(|e| format!("actual response publish {e:?}"))
            };
            receive(&mut session)?;
            require(
                session
                    .snapshot()
                    .request
                    .ok_or("queued response receipt")?
                    .response
                    .is_none(),
                "prepermission callback must not bind after later dispatch",
            )?;
            // Every rejected callback really traverses Binder -> decoder -> queue -> owner.
            for control_code in 141..=144 {
                let mut ack = control(&peer, control_code)?;
                require(
                    ack.read_i32().map_err(|e| format!("response ack {e:?}"))? == 1
                        && ack.data_avail() == 0,
                    "actual rejected callback completion",
                )?;
                receive(&mut session)?;
                require(
                    session
                        .snapshot()
                        .request
                        .ok_or("rejected response receipt")?
                        .response
                        .is_none(),
                    "wrong serial/slot/code/length cannot bind",
                )?;
            }
            let mut ack = control(&peer, 145)?;
            require(
                ack.read_i32().map_err(|e| format!("matching ack {e:?}"))? == 1
                    && ack.data_avail() == 0,
                "actual matching callback completion",
            )?;
            let committed_revision = receive(&mut session)?;
            let receipt = session.snapshot().request.ok_or("matching receipt")?;
            let first = receipt.response.clone();
            desired_red = first.is_none();
            if let Some(observed) = &first {
                require(
                    observed.request_key() == &request_key
                        && observed.session_key() == &key
                        && observed.route()
                            == (crate::request::ResponseRoute {
                                serial: 1,
                                request_id: 105,
                                slot_id: 1,
                            })
                        && observed.vendor_error() == -27
                        && observed.payload() == [0x70, 0, 0xff, 0x31]
                        && observed.received_at() >= before_dispatch_ack
                        && observed.received_at() < until
                        && observed.ingress_sequence() > 0
                        && observed.revision() == committed_revision
                        && receipt.revision == 2,
                    "exact immutable original keys, route, bytes and real ingress",
                )?;
            }
            let mut duplicate = control(&peer, 146)?;
            require(
                duplicate
                    .read_i32()
                    .map_err(|e| format!("duplicate ack {e:?}"))?
                    == 1
                    && duplicate.data_avail() == 0,
                "actual duplicate callback completion",
            )?;
            receive(&mut session)?;
            let after = session.snapshot().request.ok_or("duplicate receipt")?;
            match (&first, &after.response) {
                (Some(first), Some(after)) => require(
                    std::sync::Arc::ptr_eq(first, after),
                    "first response immutable after duplicate",
                )?,
                (None, None) => {}
                _ => return Err("response identity changed on duplicate".into()),
            }
            emit(
                "add-pdn-response-attribution",
                format!(
                    "\"client_pid\":{},\"client_uid\":1000,\"response_bound\":{},\"prepermission_rejected\":true,\"wrong_route_length_rejected\":true,\"duplicate_preserves_first\":true,\"serial\":1,\"request\":105,\"slot\":1,\"vendor_error\":-27,\"payload_hex\":\"7000ff31\",\"request_outcome\":\"outcome-unknown\",\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                    std::process::id(),
                    first.is_some(),
                    BootClock.now().as_nanos(),
                    until.as_nanos()
                ),
            )?;
            require(
                session
                    .snapshot()
                    .request
                    .as_ref()
                    .is_some_and(|r| r.key == request_key && r.outcome == Outcome::OutcomeUnknown),
                "captured receipt stays Unknown",
            )?;
            require(
                matches!(
                    session.prepare_add_pdn(&key, &config, Deadline::at(until)),
                    Err(AddPdnFailure::Lifecycle(LifecycleError::Owner(
                        OwnerError::OperationPending
                    )))
                ),
                "unknown operation cannot be replaced/retried",
            )?;
            session
                .cancel_request(&request_key)
                .map_err(|e| format!("post-dispatch cancel {e:?}"))?;
            require(
                session
                    .snapshot()
                    .request
                    .ok_or("post-cancel receipt")?
                    .outcome
                    == Outcome::OutcomeUnknown,
                "cancel cannot invent NotSent after dispatch",
            )?;
            report(&peer, until)?;
            require(
                control(&peer, 107).is_err(),
                "peer exit control has no reply",
            )?;
            loop {
                before(until)?;
                if session.snapshot().phase == Phase::Failed(ReceiverFailure::ServiceDied) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            let stop_at = BootClock.now();
            session
                .stop(&key)
                .map_err(|e| format!("post-death stop {e:?}"))?;
            let stop_elapsed = BootClock.now() - stop_at;
            require(
                stop_elapsed < Duration::from_millis(500),
                "responsive Stop after death with unresolved request",
            )?;
            require(
                session
                    .snapshot()
                    .request
                    .ok_or("post-death receipt")?
                    .outcome
                    == Outcome::OutcomeUnknown
                    && session.registration_view() == Default::default(),
                "death/Stop retain Unknown without readiness",
            )?;
            emit(
                "add-pdn-transport-result",
                format!(
                    "\"group\":\"sent\",\"request_outcome\":\"outcome-unknown\",\"submitted_parts\":1,\"capture_count\":1,\"capture_epoch\":1,\"total_wire105\":2,\"exact_capture\":true,\"serial\":1,\"client_id\":42,\"request\":105,\"slot\":1,\"owner_pid\":{},\"owner_uid\":1000,\"calling_pid\":0,\"calling_uid\":1000,\"payload_length\":663,\"payload_hex\":\"{}\",\"operation_pending_refused\":true,\"post_dispatch_cancel_stays_unknown\":true,\"actual_service_death\":true,\"stop_after_death\":true,\"stop_elapsed_ns\":{},\"request_worker_joined\":true,\"peer_control107\":true,\"registration_unavailable\":true,\"observed_at_ns\":{},\"original_oracle_until_ns\":{}",
                    std::process::id(),
                    hex(&actual.payload),
                    stop_elapsed.as_nanos(),
                    BootClock.now().as_nanos(),
                    until.as_nanos()
                ),
            )?;
        }
        require(
            session.registration_view() == Default::default(),
            "authored codec inputs never grant readiness",
        )?;
        before(until)?;
        Ok(!desired_red)
    })();
    let cleanup_until = BootClock.now() + Duration::from_secs(2);
    session
        .stop(&key)
        .map_err(|e| format!("cleanup stop {e:?}"))?;
    while session.worker_occupied() {
        before(cleanup_until)?;
        let _ = session.poll_registration();
        let _ = session.poll_request();
        std::thread::sleep(Duration::from_millis(1));
    }
    let final_outcome = session.snapshot().request.map(|r| r.outcome);
    require(
        session.registration_view() == Default::default(),
        "cleanup remains unavailable",
    )?;
    drop(session);
    before(cleanup_until)?;
    let passed = matches!(result, Ok(true));
    emit(
        "add-pdn-transport-closed",
        format!(
            "\"group\":\"{}\",\"requirement_passed\":{},\"desired_boundary_red\":{},\"setup_passed\":{},\"prepared_queued\":{},\"preflight_passed\":{},\"registration_worker_joined\":true,\"request_worker_joined\":{},\"owned_worker_occupied\":false,\"watch_dropped\":true,\"request_outcome\":\"{}\",\"capture_count\":{},\"capture_epoch\":1,\"total_wire105\":{},\"no_remote_cleanup_claim\":true,\"closed_at_ns\":{},\"original_cleanup_until_ns\":{},\"original_oracle_until_ns\":{}",
            group,
            passed,
            desired_red,
            setup_passed,
            prepared_queued,
            preflight_passed,
            request_joined,
            outcome(final_outcome),
            captured_count,
            1 + captured_count,
            BootClock.now().as_nanos(),
            cleanup_until.as_nanos(),
            until.as_nanos()
        ),
    )?;
    match result { Ok(true)=>Ok(()),Ok(false)=>Err("MISSING_ADD_PDN_RESPONSE_ATTRIBUTION after exact codec + real Binder request/callback + normal cleanup".into()),Err(error)=>Err(error) }
}
