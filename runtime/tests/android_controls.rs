//! Accepted controls against a call injected into the isolated emulator radio.
#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    Identity,
    android_protocol::{Controls, decode_controls},
    coordinator::Coordinator,
    enrollment::Enrollment,
    ledger::{Ledger, Outcome},
    rpc,
};
use std::{
    io::Write,
    os::unix::net::UnixStream,
    path::Path,
    time::{Duration, Instant},
};

fn controls(enrollment: &Enrollment) -> Controls {
    let id = Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,"op":"controls"})).unwrap();
    let reply = enrollment
        .client()
        .exchange(enrollment.address(), &bytes, Duration::from_secs(5))
        .unwrap();
    decode_controls(&reply, &id).unwrap()
}
fn observe(enrollment: &Enrollment, state: Option<u8>) -> Controls {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let value = controls(enrollment);
        assert!(value.allowed(), "fixture must enable desktop controls");
        if match state {
            Some(state) => value.calls().len() == 1 && value.calls()[0].state() == state,
            None => value.calls().is_empty(),
        } {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "Telecom state did not reach expectation"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
#[test]
#[ignore = "requires the isolated emulator with incoming call and controls enabled"]
fn rust_owner_answers_and_ends_emulator_call() {
    let path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT").unwrap();
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "android-controls-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let owner = Coordinator::new(
        Ledger::open(&directory).unwrap(),
        enrollment.client().clone(),
        enrollment.address(),
    )
    .unwrap();
    for (state, state_name, action, after) in [
        (2, "ringing", "answer", Some(4)),
        (4, "active", "disconnect", None),
    ] {
        let snapshot = observe(&enrollment, Some(state));
        let (mut app, mut service) = UnixStream::pair().unwrap();
        let (epoch, session) = owner.hello(&service).unwrap();
        let (admission, _) = owner.fence(&service, &epoch, &session).unwrap();
        let id = Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap();
        let request = serde_json::json!({"v":1,"op":"submit","id":id,"epoch":epoch,"session":session,"admission":admission,
            "command":{"kind":"control","generation":snapshot.generation(),"call_id":snapshot.calls()[0].id(),"revision":snapshot.revision(),"state":state_name,"action":action}});
        writeln!(app, "{request}").unwrap();
        rpc::serve(&owner, &mut service).unwrap();
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            let receipt = owner.lookup(&service, &epoch, &id).unwrap().unwrap();
            if receipt.outcome() != Outcome::Pending {
                assert_eq!(receipt.outcome(), Outcome::ControlSubmitted);
                break;
            }
            assert!(Instant::now() < deadline, "receipt deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
        // A submission receipt alone is insufficient: verify the real callback.
        observe(&enrollment, after);
    }
    drop(owner);
    std::fs::remove_dir_all(directory).unwrap();
}
