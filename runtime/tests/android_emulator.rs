//! Explicit integration test. Run only inside the owned emulator's network
//! namespace with its disposable enrollment. No discovery or physical ADB.
#![cfg(target_os = "linux")]
use omarchy_phone_core::android_protocol::{
    ReceiptOutcome, decode_controls, decode_dial_status, decode_status, encode_command,
};
use omarchy_phone_core::enrollment::Enrollment;
use omarchy_phone_core::{Identity, authorization::Session};
use omarchy_phone_core::{
    dispatch::execute,
    ledger::{Ledger, Outcome},
};
use std::{io::Write, os::unix::net::UnixStream};
use std::{path::Path, time::Duration};

#[test]
#[ignore = "requires the isolated Android emulator and disposable enrollment"]
fn typed_watch_reads_real_android_framework() {
    use omarchy_phone_core::{tls::WatchCancel, watch::WatchQuery};
    let path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT").unwrap();
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let query = |after, wait| {
        WatchQuery::new(
            Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap(),
            after,
            wait,
        )
        .unwrap()
    };
    let first = enrollment
        .client()
        .watch(
            enrollment.address(),
            &query(None, 0),
            &WatchCancel::default(),
        )
        .unwrap();
    assert!(first.ringing().is_empty(), "fixture must have no calls");
    let second = enrollment
        .client()
        .watch(
            enrollment.address(),
            &query(Some(first.cursor()), 250),
            &WatchCancel::default(),
        )
        .unwrap();
    assert_eq!(first.cursor(), second.cursor());
    assert_eq!(first.source_state(), second.source_state());
    assert!(second.ringing().is_empty());
}

#[test]
#[ignore = "requires the isolated Android emulator and disposable enrollment"]
fn rust_tls_reads_real_android_framework() {
    let path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT")
        .expect("explicit emulator enrollment required");
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let client = enrollment.client();
    let address = enrollment.address();
    // This test deliberately uses Serde's sorted JSON objects, plus whitespace
    // and reordered fields below. The server must validate structure, not text.
    // Read-only interoperability: each operation gets a fresh authenticated
    // connection, using the same immutable client configuration.
    for op in ["status", "controls", "dial_status", "status"] {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let body = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,"op":op})).unwrap();
        let reply = client
            .exchange(address, &body, Duration::from_secs(5))
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(value["id"], id);
        assert_eq!(value["v"], 1);
        assert_eq!(value["generation"].as_str().unwrap().len(), 32);
        match op {
            "status" => {
                decode_status(&reply, &Identity::try_from(id.clone()).unwrap()).unwrap();
            }
            "controls" => {
                decode_controls(&reply, &Identity::try_from(id.clone()).unwrap()).unwrap();
            }
            "dial_status" => {
                decode_dial_status(&reply, &Identity::try_from(id.clone()).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
    }
    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let reordered = format!(" {{ \"op\": \"status\", \"id\": \"{id}\", \"v\": 1 }} ");
    let response = client
        .exchange(address, reordered.as_bytes(), Duration::from_secs(5))
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"],
        id
    );
    let journal = std::env::temp_dir().join(format!(
        "emulator-journal-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let mut ledger = Ledger::open(&journal).unwrap();
    let service_session = Session::default();
    for (snapshot_op, command_op, expected) in [
        ("controls", "control", ReceiptOutcome::ControlRejected),
        ("dial_status", "dial", ReceiptOutcome::DialNotInvoked),
    ] {
        let snapshot_request =
            serde_json::to_vec(&serde_json::json!({"v":1,"id":id,"op":snapshot_op})).unwrap();
        let snapshot = client
            .exchange(address, &snapshot_request, Duration::from_secs(5))
            .unwrap();
        let identity = Identity::try_from(id.to_string()).unwrap();
        let command = if command_op == "control" {
            let controls = decode_controls(&snapshot, &identity).unwrap();
            assert!(!controls.allowed(), "test requires controls disabled");
            serde_json::json!({"kind":"control","generation":controls.generation(),"call_id":1,"revision":controls.revision(),"state":"ringing","action":"answer"})
        } else {
            let status = decode_dial_status(&snapshot, &identity).unwrap();
            assert!(status.offer().is_none(), "test requires dialing disabled");
            serde_json::json!({"kind":"dial","generation":status.generation(),"offer":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","number":"5550100"})
        };
        let (mut ui, mut service_peer) = UnixStream::pair().unwrap();
        let session = service_session.identity_for(&service_peer).unwrap();
        let local = serde_json::json!({"v":1,"id":uuid::Uuid::new_v4().simple().to_string(),"epoch":ledger.epoch(),"session":session,"command":command});
        writeln!(ui, "{local}").unwrap();
        let request = service_session
            .receive(&mut service_peer, Duration::from_secs(1))
            .unwrap();
        drop(ui);
        drop(service_peer); // The command owner continues after local UI disconnect.
        let encoded = encode_command(&request);
        let outcome = execute(
            &mut ledger,
            client,
            address,
            &request,
            Duration::from_secs(5),
        )
        .unwrap();
        let expected = match expected {
            ReceiptOutcome::ControlRejected => Outcome::ControlRejected,
            ReceiptOutcome::DialNotInvoked => Outcome::DialNotInvoked,
            _ => unreachable!(),
        };
        assert_eq!(outcome, expected);
        drop(ledger);
        ledger = Ledger::open(&journal).unwrap();
        let recovered = ledger
            .lookup(request.epoch(), request.id())
            .unwrap()
            .unwrap();
        assert_eq!(recovered.outcome(), expected);
        assert!(!recovered.acknowledged());
        assert_eq!(
            execute(&mut ledger, client, address, &request, Duration::ZERO).unwrap(),
            expected
        );
        let command: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        for (key, value) in if command_op == "control" {
            vec![
                ("call_id", serde_json::json!(0)),
                ("call_id", serde_json::json!(2147483648u64)),
                ("expected_revision", serde_json::json!(9007199254740992u64)),
                ("expected_state", serde_json::json!(21)),
            ]
        } else {
            vec![
                ("number", serde_json::json!("555;0100")),
                ("number", serde_json::json!(5550100)),
            ]
        } {
            let mut invalid = command.clone();
            invalid[key] = value;
            assert!(
                client
                    .exchange(
                        address,
                        &serde_json::to_vec(&invalid).unwrap(),
                        Duration::from_secs(5)
                    )
                    .is_err()
            );
        }
    }
    drop(ledger);
    std::fs::remove_dir_all(journal).unwrap();
    let canonical = format!("{{\"v\":1,\"id\":\"{id}\",\"op\":\"status\"}}");
    let bad = vec![
        canonical.replace("\"v\":1", "\"v\":1,\"v\":1"),
        canonical.replace("\"v\":1", "\"v\":1,\"\\u0076\":1"),
        canonical.replace("\"v\":1", "\"v\":true"),
        canonical.replace("\"v\":1", "\"v\":1.0"),
        canonical.replace("\"v\":1", "\"v\":1e0"),
        canonical.replace("\"v\":1", "\"v\":\"1\""),
        canonical.replace("\"v\":1", "\"v\":1,\"extra\":false"),
        canonical.replace("\"status\"", "[]"),
        format!("{canonical}{{}}"),
        "[]".into(),
        "null".into(),
    ];
    for body in bad {
        assert!(
            client
                .exchange(address, body.as_bytes(), Duration::from_secs(5))
                .is_err(),
            "invalid request accepted"
        );
    }
    let malformed = b"{\"v\":1,\"id\":\"\xff\",\"op\":\"status\"}";
    assert!(
        client
            .exchange(address, malformed, Duration::from_secs(5))
            .is_err()
    );
}

#[test]
#[ignore = "requires the isolated Android emulator and disposable enrollment"]
fn service_owner_recovers_real_android_receipt_after_ui_disconnect() {
    use omarchy_phone_core::coordinator::Coordinator;
    use std::time::Instant;
    let path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT").unwrap();
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let query_id = Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap();
    let query =
        serde_json::to_vec(&serde_json::json!({"v":1,"id":query_id,"op":"dial_status"})).unwrap();
    let reply = enrollment
        .client()
        .exchange(enrollment.address(), &query, Duration::from_secs(5))
        .unwrap();
    let status = decode_dial_status(&reply, &query_id).unwrap();
    assert!(status.offer().is_none(), "requires dialing disabled");
    let journal =
        std::env::temp_dir().join(format!("emulator-owner-{}", uuid::Uuid::new_v4().simple()));
    let ledger = Ledger::open(&journal).unwrap();
    let (address, client) = enrollment.into_parts();
    let owner = Coordinator::new(ledger, client, address).unwrap();
    let (mut ui, mut service_peer) = UnixStream::pair().unwrap();
    let (epoch, session) = owner.hello(&service_peer).unwrap();
    let (admission, _) = owner.fence(&service_peer, &epoch, &session).unwrap();
    let id = Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap();
    let request = serde_json::json!({"op":"submit","v":1,"id":id,"epoch":epoch,"session":session,"admission":admission,"command":{"kind":"dial","generation":status.generation(),"offer":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","number":"5550100"}});
    writeln!(ui, "{request}").unwrap();
    drop(ui);
    // Reply delivery fails after the UI closes; admitted work must still finish.
    assert!(omarchy_phone_core::rpc::serve(&owner, &mut service_peer).is_err());
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let receipt = owner.lookup(&service_peer, &epoch, &id).unwrap().unwrap();
        if receipt.outcome() != Outcome::Pending {
            assert_eq!(receipt.outcome(), Outcome::DialNotInvoked);
            break;
        }
        assert!(Instant::now() < deadline, "worker completion deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(owner);
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let (address, client) = enrollment.into_parts();
    let owner = Coordinator::new(Ledger::open(&journal).unwrap(), client, address).unwrap();
    assert_ne!(owner.hello(&service_peer).unwrap().1, session);
    assert_eq!(
        owner
            .lookup(&service_peer, &epoch, &id)
            .unwrap()
            .unwrap()
            .outcome(),
        Outcome::DialNotInvoked
    );
    let (mut stale_ui, mut stale_peer) = UnixStream::pair().unwrap();
    writeln!(stale_ui, "{request}").unwrap();
    assert!(owner.submit(&mut stale_peer).is_err());
    drop(owner);
    std::fs::remove_dir_all(journal).unwrap();
}

#[test]
#[ignore = "requires the isolated Android emulator and disposable enrollment"]
fn local_rpc_observes_real_android_framework() {
    use omarchy_phone_core::{coordinator::Coordinator, local_protocol, local_reply, rpc};
    use std::io::Read;
    let enrollment_path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT")
        .expect("explicit emulator enrollment required");
    let enrollment = Enrollment::load(Path::new(&enrollment_path)).unwrap();
    let (address, client) = enrollment.into_parts();
    let journal = std::env::temp_dir().join(format!(
        "emulator-observations-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let owner = Coordinator::new(Ledger::open(&journal).unwrap(), client, address).unwrap();
    for op in ["status", "controls", "dial_status"] {
        let frame = format!(
            "{}\n",
            serde_json::json!({"v":1,"op":op,"id":uuid::Uuid::new_v4().simple().to_string()})
        );
        let (mut app, mut service) = UnixStream::pair().unwrap();
        app.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        app.write_all(frame.as_bytes()).unwrap();
        rpc::serve(&owner, &mut service).unwrap();
        drop(service);
        let mut bytes = Vec::new();
        app.read_to_end(&mut bytes).unwrap();
        let reply = local_reply::decode(&bytes, &local_protocol::decode(frame.as_bytes()).unwrap())
            .unwrap();
        assert!(
            matches!(
                reply.body(),
                local_reply::Body::Status { .. }
                    | local_reply::Body::Controls { .. }
                    | local_reply::Body::DialStatus { .. }
            ),
            "real Android observation unavailable"
        );
    }
    drop(owner);
    std::fs::remove_dir_all(journal).unwrap();
}
