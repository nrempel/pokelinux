//! Actual native radio owner/worker through guest kernel Binder to synthetic native peer.
use pokelinux_radio::{
    deadline::{BootClock, Clock, Deadline},
    owner::{OwnerError, Phase},
    receiver::{RadioEvent, ReceiverFailure, new_radio_callbacks},
    request::Outcome,
    transport::RadioTransport,
    worker::{LifecycleError, RegistrationFailure, RegistrationReport, TransportSession},
};
use rsbinder::{Interface, Parcel, ProcessState, SIBinder, Status, StatusCode};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn n(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn marker(name: &str) {
    println!("arm64-{name}\tPASS");
}
fn end() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn deadline() -> Deadline {
    Deadline::at(BootClock.now() + Duration::from_secs(120))
}
fn control(peer: &SIBinder, code: u32) -> rsbinder::Result<Parcel> {
    let remote = peer.as_remote().ok_or(StatusCode::BadType)?;
    let data = remote.prepare_transact(true)?;
    let mut reply = remote
        .submit_transact(code, &data, 0)?
        .ok_or(StatusCode::UnexpectedNull)?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(StatusCode::from(status));
    }
    Ok(reply)
}
fn latch(peer: &SIBinder, code: u32) -> rsbinder::Result<()> {
    let mut reply = control(peer, code)?;
    if reply.read_i32()? != 1 || reply.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(())
}
fn report(peer: &SIBinder) -> rsbinder::Result<[i32; 6]> {
    let mut reply = control(peer, 100)?;
    let mut report = [0; 6];
    for value in &mut report {
        *value = reply.read_i32()?;
    }
    if reply.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    Ok(report)
}
fn registered(session: &mut TransportSession) -> RegistrationReport {
    let end = end();
    loop {
        if let Some(report) = session.poll_registration() {
            return report;
        }
        assert!(Instant::now() < end, "registration completion timeout");
        std::thread::yield_now();
    }
}
fn publish(session: &mut TransportSession) {
    let prepared = session
        .prepare_callback_until(end())
        .unwrap()
        .expect("callback timeout");
    session.publish_callback(prepared).unwrap();
}
fn callbacks(peer: &SIBinder) -> TestResult {
    let transport = RadioTransport::from_binder(peer.clone())?;
    let callbacks = new_radio_callbacks(n(8));
    let _watch = transport.watch_death(callbacks.control.clone())?;
    assert_eq!(
        transport.register(
            &callbacks.response.as_binder(),
            &callbacks.indication.as_binder(),
            "callbacks"
        )?,
        42
    );
    let mut response = false;
    let mut indication = false;
    for _ in 0..2 {
        match callbacks
            .events
            .recv_until(end())
            .unwrap()
            .expect("callback timeout")
        {
            RadioEvent::Response(event) => {
                assert_eq!(event.info.serial, 7);
                assert_eq!(event.payload, vec![7]);
                response = true;
            }
            RadioEvent::Indication(event) => {
                assert_eq!(event.message_id, 33);
                assert_eq!(event.payload, vec![1, 2, 3]);
                indication = true;
            }
        }
    }
    assert!(response && indication);
    let r = report(peer)?;
    assert_eq!(r[0], 1);
    assert_eq!(r[3], std::process::id() as i32);
    assert_eq!(r[4], 1000);
    assert_eq!(r[5], 0);
    marker("kernel-object-transfer-nested-metadata-response-indication");
    Ok(())
}
struct Controlled(AtomicU64);
impl Clock for Controlled {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::SeqCst))
    }
}
fn held(peer: &SIBinder, expire: bool) -> TestResult {
    latch(peer, 199)?; // Executed fixture-capability red before held controls existed.
    let clock = Arc::new(Controlled(AtomicU64::new(10)));
    let mut session = TransportSession::with_clock(n(2), clock.clone());
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "hold".into(),
            n(4),
            Deadline::at(Duration::from_nanos(20)),
        )
        .unwrap();
    latch(peer, 110)?;
    assert!(session.worker_occupied());
    assert_eq!(session.snapshot().phase, Phase::Registering);
    let terminal = if expire {
        clock.0.store(20, Ordering::SeqCst);
        ReceiverFailure::Expired
    } else {
        session.stop(&key).unwrap();
        ReceiverFailure::Cancelled
    };
    let before = Instant::now();
    for _ in 0..16 {
        assert_eq!(session.snapshot().phase, Phase::Failed(terminal));
        assert!(session.snapshot().response.is_none());
        assert!(session.poll_registration().is_none());
        assert!(session.worker_occupied());
        assert_eq!(
            session.start(
                RadioTransport::from_binder(peer.clone())?,
                "retry".into(),
                n(4),
                Deadline::at(Duration::from_nanos(30))
            ),
            Err(LifecycleError::WorkerOccupied)
        );
        session.stop(&key).unwrap();
    }
    assert!(
        before.elapsed() < Duration::from_secs(2),
        "stop/snapshot blocked on actual Binder registration"
    );
    marker("held-registration-stop-snapshot-capacity");
    latch(peer, 109)?;
    let completion = registered(&mut session);
    assert!(completion.dispatched);
    assert!(matches!(completion.result, Ok(42)));
    assert_eq!(completion.activation, Err(OwnerError::Failed(terminal)));
    assert!(!session.worker_occupied());
    assert!(session.snapshot().response.is_none());
    assert_eq!(
        session.start(
            RadioTransport::from_binder(peer.clone())?,
            "retry".into(),
            n(4),
            deadline()
        ),
        Err(LifecycleError::AlreadyAttempted)
    );
    assert_eq!(report(peer)?[0], 1);
    assert_eq!(report(peer)?[5], 0);
    marker(if expire {
        "expired-late-registration-success-rejected-no-timer"
    } else {
        "cancelled-late-registration-success-rejected"
    });
    marker("no-implicit-registration-retry-or-clear");
    Ok(())
}
fn death(peer: &SIBinder, during: bool) -> TestResult {
    let mut session = TransportSession::new(n(2));
    session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            if during { "die-register" } else { "active" }.into(),
            n(8),
            deadline(),
        )
        .unwrap();
    let result = registered(&mut session);
    if during {
        assert!(
            matches!(result.result,Err(RegistrationFailure::Binder(status)) if status.transaction_error()==StatusCode::DeadObject)
        );
        assert_eq!(
            result.activation,
            Err(OwnerError::Failed(ReceiverFailure::ServiceDied))
        );
    } else {
        assert!(matches!(result.result, Ok(42)));
        assert_eq!(result.activation, Ok(()));
        publish(&mut session);
        assert_eq!(session.snapshot().response.unwrap().payload, vec![7]);
        assert_eq!(control(peer, 107).err(), Some(StatusCode::DeadObject));
    }
    let end = end();
    while session.snapshot().phase != Phase::Failed(ReceiverFailure::ServiceDied) {
        assert!(Instant::now() < end, "death watch timeout");
        std::thread::yield_now();
    }
    assert!(!session.worker_occupied());
    assert!(session.snapshot().response.is_none());
    marker(if during {
        "owned-registration-death"
    } else {
        "retained-watch-service-death-withdraws-publication"
    });
    Ok(())
}
fn requests(peer: &SIBinder, scenario: &str) -> TestResult {
    let clock = Arc::new(Controlled(AtomicU64::new(10)));
    let mut session = TransportSession::with_clock(n(2), clock.clone());
    let key = session
        .start(
            RadioTransport::from_binder(peer.clone())?,
            "active".into(),
            n(8),
            Deadline::at(Duration::from_nanos(100)),
        )
        .unwrap();
    assert_eq!(registered(&mut session).activation, Ok(()));
    publish(&mut session);
    let bytes: Vec<u8> = if scenario == "segments" {
        (0..4096).map(|i| (i % 251) as u8).collect()
    } else {
        vec![9, 8]
    };
    if scenario == "late-request" {
        latch(peer, 111)?;
    }
    let prepared = session
        .prepare_request(
            &key,
            137,
            0,
            bytes.clone(),
            Deadline::at(Duration::from_nanos(20)),
        )
        .unwrap();
    let request = prepared.key();
    if scenario == "expired-request" {
        clock.0.store(20, Ordering::SeqCst);
    }
    session.submit_request(prepared).unwrap();
    let end = end();
    let result = loop {
        if let Some(report) = session.poll_request() {
            break report.unwrap();
        }
        assert!(Instant::now() < end);
        std::thread::yield_now();
    };
    if scenario == "expired-request" {
        assert_eq!(result.submitted_parts, 0);
        assert_eq!(result.outcome, Outcome::NotSent);
        assert_eq!(report(peer)?[1..3], [0, 0]);
        marker("owned-request-expiry-withheld-timer-not-sent");
    } else {
        assert_eq!(
            result.submitted_parts,
            if scenario == "segments" { 2 } else { 1 }
        );
        assert_eq!(result.outcome, Outcome::OutcomeUnknown);
        assert!(result.submission.is_ok());
        if scenario == "late-request" {
            latch(peer, 112)?;
            session.cancel_request(&request).unwrap();
            latch(peer, 113)?;
        }
        publish(&mut session);
        assert_eq!(session.snapshot().response.unwrap().payload, bytes);
        assert_eq!(
            session.snapshot().request.unwrap().outcome,
            Outcome::OutcomeUnknown
        );
        assert_eq!(
            report(peer)?[if scenario == "segments" { 2 } else { 1 }],
            if scenario == "segments" { 2 } else { 1 }
        );
        marker(if scenario == "segments" {
            "owned-two-part-request-and-4096-byte-callback"
        } else {
            "oneway-and-held-late-callback-retain-unknown"
        });
    }
    session.stop(&key).unwrap();
    assert!(session.snapshot().response.is_none());
    assert_eq!(report(peer)?[5], 0);
    Ok(())
}
fn peer(service_manager: bool) -> Result<SIBinder, Box<dyn std::error::Error>> {
    let process = ProcessState::init("/dev/binderfs/radio", 3)?;
    ProcessState::start_thread_pool();
    let bootstrap = process.context_object()?;
    let mut reply = if service_manager {
        let remote = bootstrap.as_remote().ok_or("local manager")?;
        let mut data = remote.prepare_transact(true)?;
        data.write("org.pokelinux.fixture.arm64-radio")?;
        let mut reply = remote
            .submit_transact(3, &data, 0)?
            .ok_or("missing service")?;
        let status: Status = reply.read()?;
        if !status.is_ok() {
            return Err(status.into());
        }
        reply
    } else {
        control(&bootstrap, 1)?
    };
    let peer: SIBinder = reply.read()?;
    assert_eq!(reply.data_avail(), 0);
    assert!(peer.is_remote());
    assert!(
        peer != bootstrap,
        "bootstrap returned context manager instead of transferred endpoint"
    );
    marker("real-kernel-binder-handle-transfer");
    Ok(peer)
}
#[path = "../bootstrap_controls.rs"]
mod bootstrap_controls;
#[path = "../supervised.rs"]
mod supervised;
fn main() -> TestResult {
    let cooperative =
        std::env::args().nth(1).as_deref() == Some(pokelinux_radio::bootstrap::WORKER_SELECTOR);
    if cooperative
        && (std::env::var(bootstrap_controls::CANARY_ENV).as_deref() == Ok("1")
            || std::env::args().nth(2).as_deref() == Some("bootstrap-canary"))
    {
        return bootstrap_controls::child_canary();
    }
    let _parent = if cooperative {
        Some(pokelinux_radio::bootstrap::BoundParent::from_stdin()?)
    } else {
        None
    };
    let scenario = std::env::args()
        .nth(if cooperative { 2 } else { 1 })
        .ok_or("missing scenario")?;
    if scenario == "parent-death-supervisor" {
        return supervised::parent_death_supervisor();
    }
    if scenario == "parent-death-refuse" {
        return supervised::parent_death_refuse();
    }
    if scenario == "driver" {
        ProcessState::init("/dev/binderfs/radio", 3)?;
        marker("binder-driver-open");
        return Ok(());
    }
    if scenario == "supervised-permanent" {
        return supervised::parent();
    }
    let ndk = scenario.starts_with("ndk-");
    let scenario = scenario.strip_prefix("ndk-").unwrap_or(&scenario);
    let peer = peer(ndk)?;
    match scenario {
        "bootstrap-late-go" => bootstrap_controls::late_go(&peer),
        "bootstrap-withheld-poll" => bootstrap_controls::policy_control(&peer, true),
        "bootstrap-stop-initializing" => bootstrap_controls::policy_control(&peer, false),
        "bootstrap-wrong-parent" => bootstrap_controls::wrong_parent(
            &peer,
            std::env::args()
                .nth(2)
                .ok_or("missing actual independent peer PID")?
                .parse()?,
        ),
        "permanent-child" => supervised::child(&peer),
        "callbacks" => callbacks(&peer),
        "hold-cancel" => held(&peer, false),
        "hold-expire" => held(&peer, true),
        "death-register" => death(&peer, true),
        "death-active" => death(&peer, false),
        "expired-request" | "segments" | "late-request" => requests(&peer, scenario),
        _ => Err("unknown scenario".into()),
    }
}
