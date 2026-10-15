//! Typed historical status contract; include as a child test module of status.rs.
//! Authored wire values are opaque diagnostics, never IMS or availability state.
use super::*;
use crate::registration::RegistrationWire;

#[test]
fn unavailable_status_retains_typed_history_and_zero_versus_absent_options() {
    let identity = ProcessIdentity::from_hex("0123456789abcdef0123456789abcdef").unwrap();
    let wire = RegistrationWire::new(omarchy_ims::registration::RegistrationWire {
        registration_state_wire: 255,
        feature_wire: 128,
        feature_mask_wire: i32::MIN,
        status_code_wire: u16::MAX,
        uri_bytes: &[0, 255, 192, 0],
        reason_bytes: &[128, 10],
        feature_mask_ext_wire: Some(0),
        sip_delegate_mask_wire: None,
        feature_ext_wire: Some(0),
    })
    .unwrap();
    let history = Frame {
        identity,
        attempt: 1,
        revision: 7,
        kind: Kind::RegistrationWire {
            slot_id: 1,
            message_id: 3012,
            wire,
        },
    };
    let status = Status {
        phase: Phase::Unavailable,
        identity: Some(identity),
        revision: 11,
        failure: Some(ReceiverFailure::ServiceDied),
        last_observation: Some(history.clone()),
    };
    let reply = Reply::Status(status.clone());
    let packet = reply.encode().unwrap();
    let decoded = decode_reply(&packet).unwrap();
    assert_eq!(decoded, reply);
    let Reply::Status(decoded) = decoded else {
        panic!("typed status reply became another reply kind");
    };
    assert_eq!(decoded.last_observation.as_ref(), Some(&history));
    assert_eq!(decoded.phase, Phase::Unavailable);
    assert_eq!(decoded.failure, Some(ReceiverFailure::ServiceDied));
    assert_eq!(
        decoded.json(42).unwrap(),
        concat!(
            "{\"server_pid\":42,\"phase\":\"unavailable\",\"failure\":\"service-died\",",
            "\"incarnation\":\"0123456789abcdef0123456789abcdef\",\"revision\":11,",
            "\"last_observation\":{\"kind\":\"registration-wire\",\"revision\":7,\"attempt\":1,",
            "\"slot_id\":1,\"message_id\":3012,\"wire\":{",
            "\"registration_state_wire\":255,\"feature_wire\":128,",
            "\"feature_mask_wire\":-2147483648,\"status_code_wire\":65535,",
            "\"uri_hex\":\"00ffc000\",\"reason_hex\":\"800a\",",
            "\"feature_mask_ext_wire\":0,\"sip_delegate_mask_wire\":null,\"feature_ext_wire\":0}}}\n"
        )
    );
}

#[test]
fn maximum_typed_status_preserves_opaque_bytes_extremes_and_packet_json_bounds() {
    let identity = ProcessIdentity::from_hex("ffffffffffffffffffffffffffffffff").unwrap();
    let uri = [0, 255, 192, 128].repeat(250);
    let reason = [255, 0, 128, 254].repeat(64);
    assert_eq!(uri.len(), 1000);
    assert_eq!(reason.len(), 256);
    let wire = RegistrationWire::new(omarchy_ims::registration::RegistrationWire {
        registration_state_wire: u8::MAX,
        feature_wire: 128,
        feature_mask_wire: i32::MIN,
        status_code_wire: u16::MAX,
        uri_bytes: &uri,
        reason_bytes: &reason,
        feature_mask_ext_wire: Some(i32::MAX),
        sip_delegate_mask_wire: Some(i64::MIN),
        feature_ext_wire: Some(u16::MAX),
    })
    .unwrap();
    let history = Frame {
        identity,
        attempt: 1,
        revision: u64::MAX - 1,
        kind: Kind::RegistrationWire {
            slot_id: 1,
            message_id: 3012,
            wire,
        },
    };
    let status = Status {
        phase: Phase::Unavailable,
        identity: Some(identity),
        revision: u64::MAX,
        failure: Some(ReceiverFailure::ServiceDied),
        last_observation: Some(history.clone()),
    };
    let reply = Reply::Status(status);
    let packet = reply.encode().unwrap();
    assert_eq!(MAX_PACKET, 2160);
    assert_eq!(packet.len(), 1384);
    assert!(packet.len() <= MAX_PACKET);
    let decoded = decode_reply(&packet).unwrap();
    assert_eq!(decoded, reply);
    let Reply::Status(decoded) = decoded else {
        panic!("maximum typed status reply became another reply kind");
    };
    assert_eq!(decoded.phase, Phase::Unavailable);
    assert_eq!(decoded.failure, Some(ReceiverFailure::ServiceDied));
    assert_eq!(decoded.last_observation.as_ref(), Some(&history));
    let uri_hex = "00ffc080".repeat(250);
    let reason_hex = "ff0080fe".repeat(64);
    assert_eq!(uri_hex.len(), 2000);
    assert_eq!(reason_hex.len(), 512);
    let json = decoded.json(u32::MAX).unwrap();
    assert!(json.len() <= 5120);
    let expected = format!(
        concat!(
            "{{\"server_pid\":4294967295,\"phase\":\"unavailable\",\"failure\":\"service-died\",",
            "\"incarnation\":\"ffffffffffffffffffffffffffffffff\",\"revision\":18446744073709551615,",
            "\"last_observation\":{{\"kind\":\"registration-wire\",\"revision\":18446744073709551614,\"attempt\":1,",
            "\"slot_id\":1,\"message_id\":3012,\"wire\":{{",
            "\"registration_state_wire\":255,\"feature_wire\":128,",
            "\"feature_mask_wire\":-2147483648,\"status_code_wire\":65535,",
            "\"uri_hex\":\"{}\",\"reason_hex\":\"{}\",",
            "\"feature_mask_ext_wire\":2147483647,\"sip_delegate_mask_wire\":-9223372036854775808,\"feature_ext_wire\":65535}}}}}}\n"
        ),
        uri_hex, reason_hex
    );
    assert_eq!(json, expected);
}
