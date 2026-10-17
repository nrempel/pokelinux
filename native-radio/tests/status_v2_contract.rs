//! Independent current-v2 frontend acceptance: owned wire bytes and whole typed history.
use pokelinux_process_owner::supervisor::ProcessIdentity;
use pokelinux_radio::{
    operator::{AdmitReply, HistoryResponse, RequestHistory, RequestOutcome},
    status::{MAX_PACKET, Phase, Reply, Status, decode_reply},
};
fn value() -> Status {
    let identity = ProcessIdentity::from_bytes([0x12; 16]);
    Status {
        phase: Phase::Stopping,
        identity: Some(identity),
        revision: 7,
        failure: None,
        last_observation: None,
        request_history: Some(RequestHistory {
            request_revision: 2,
            intent: "34".repeat(16),
            incarnation: identity.hex(),
            attempt: 1,
            serial: 1,
            client_id: 42,
            request: 105,
            slot: 1,
            payload_sha256: "56".repeat(32),
            deadline_ns: 9_000_000,
            outcome: RequestOutcome::OutcomeUnknown,
            response: Some(HistoryResponse {
                vendor_error: -27,
                payload_hex: "00ff".repeat(2048),
                ingress_sequence: 1,
                received_at_ns: 8_000_000,
                publication_revision: 2,
            }),
        }),
    }
}
fn body(mut wire: Vec<u8>, json: &[u8]) -> Vec<u8> {
    wire.truncate(12);
    wire.extend_from_slice(json);
    let count = wire.len() as u32;
    wire[8..12].copy_from_slice(&count.to_le_bytes());
    wire
}
#[test]
fn whole_current_stop_snapshot_preserves_max_response_and_independent_revisions() {
    assert_eq!(MAX_PACKET, 16384);
    let status = value();
    let original = status.clone();
    let wire = Reply::StopAck(status.clone()).encode().unwrap();
    assert_eq!(&wire[..8], b"RADS\x02\x02\0\0");
    assert!(wire.len() < MAX_PACKET);
    assert_eq!(
        &wire[12..],
        status.json(std::process::id()).unwrap().as_bytes()
    );
    assert_eq!(
        decode_reply(&wire).unwrap(),
        Reply::StopAck(original.clone())
    );
    assert_eq!(
        decode_reply(&Reply::Status(status).encode().unwrap()).unwrap(),
        Reply::Status(original)
    );
}
#[test]
fn current_version_exact_shape_and_complete_packet_are_required() {
    let mut status = value();
    status.request_history = None;
    let wire = Reply::StopAck(status).encode().unwrap();
    for end in 0..12 {
        assert!(decode_reply(&wire[..end]).is_err());
    }
    let mut legacy = wire.clone();
    legacy[4] = 1;
    assert!(decode_reply(&legacy).is_err());
    let mut reserved = wire.clone();
    reserved[6] = 1;
    assert!(decode_reply(&reserved).is_err());
    let text = std::str::from_utf8(&wire[12..]).unwrap();
    let unknown = text.replacen('{', "{\"extra\":1,", 1);
    assert!(decode_reply(&body(wire.clone(), unknown.as_bytes())).is_err());
    let missing = text.replace(",\"request_history\":null", "");
    assert_ne!(missing, text);
    assert!(decode_reply(&body(wire.clone(), missing.as_bytes())).is_err());
    let duplicate = text.replacen('{', "{\"server_pid\":1,", 1);
    assert!(decode_reply(&body(wire.clone(), duplicate.as_bytes())).is_err());
    assert!(decode_reply(&body(wire, &[b' '; MAX_PACKET])).is_err());
}
#[test]
fn request_history_keeps_exact_incarnation_and_bounded_response() {
    let mut status = value();
    status.request_history.as_mut().unwrap().incarnation = "ff".repeat(16);
    assert!(Reply::Status(status).encode().is_err());
    let mut status = value();
    status
        .request_history
        .as_mut()
        .unwrap()
        .response
        .as_mut()
        .unwrap()
        .payload_hex
        .push_str("00");
    assert!(Reply::StopAck(status).encode().is_err());
}
#[test]
fn operator_reply_is_exact_bare_json_without_diagnostic_authority() {
    let reply = AdmitReply {
        schema: 1,
        admitted: true,
        reason: "admitted-pending-durability".into(),
        incarnation: "12".repeat(16),
        attempt: 1,
        request_revision: 1,
    };
    let wire = Reply::Admit(reply.clone()).encode().unwrap();
    assert_eq!(wire.first(), Some(&b'{'));
    assert!(wire.len() <= 1024);
    assert_eq!(decode_reply(&wire).unwrap(), Reply::Admit(reply));
}
