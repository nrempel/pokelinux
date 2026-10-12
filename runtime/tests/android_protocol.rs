use omarchy_phone_core::{
    android_protocol::{ReceiptOutcome, decode_receipt, encode_command},
    decode_request,
};
use serde_json::{Value, json};
fn request(dial: bool) -> omarchy_phone_core::Request {
    let command = if dial {
        json!({"kind":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"})
    } else {
        json!({"kind":"control","generation":"dddddddddddddddddddddddddddddddd","call_id":7,"revision":3,"state":"ringing","action":"answer"})
    };
    decode_request(format!("{}\n",json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","session":"cccccccccccccccccccccccccccccccc","command":command})).as_bytes()).unwrap()
}
fn reply(outcome: &str, reason: &str) -> Value {
    json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","generation":"dddddddddddddddddddddddddddddddd","outcome":outcome,"reason":reason})
}
#[test]
fn command_encoding_matches_android_fields_without_local_authority_fields() {
    let encoded: Value = serde_json::from_slice(&encode_command(&request(false))).unwrap();
    assert_eq!(
        encoded,
        json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","op":"control","generation":"dddddddddddddddddddddddddddddddd","call_id":7,"expected_revision":3,"expected_state":2,"action":"answer"})
    );
    let encoded: Value = serde_json::from_slice(&encode_command(&request(true))).unwrap();
    assert_eq!(
        encoded,
        json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","op":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"})
    );
}
#[test]
fn receipt_identity_operation_and_reason_are_bound_together() {
    for (dial, outcome, reason, expected) in [
        (
            false,
            "submitted",
            "accepted",
            ReceiptOutcome::ControlSubmitted,
        ),
        (false, "unknown", "dispatch_error", ReceiptOutcome::Unknown),
        (
            false,
            "rejected",
            "disabled",
            ReceiptOutcome::ControlRejected,
        ),
        (
            true,
            "invocation_attempted",
            "returned",
            ReceiptOutcome::DialInvoked,
        ),
        (
            true,
            "not_invoked",
            "emergency",
            ReceiptOutcome::DialNotInvoked,
        ),
        (
            true,
            "outcome_unknown",
            "dispatch_error",
            ReceiptOutcome::Unknown,
        ),
    ] {
        let bytes = serde_json::to_vec(&reply(outcome, reason)).unwrap();
        assert_eq!(
            decode_receipt(&bytes, &request(dial)).unwrap().outcome(),
            expected
        );
        assert!(decode_receipt(&bytes, &request(!dial)).is_err());
        let mut wrong = reply(outcome, reason);
        wrong["id"] = json!("ffffffffffffffffffffffffffffffff");
        assert!(decode_receipt(&serde_json::to_vec(&wrong).unwrap(), &request(dial)).is_err());
        wrong = reply(outcome, reason);
        wrong["generation"] = json!("ffffffffffffffffffffffffffffffff");
        assert!(decode_receipt(&serde_json::to_vec(&wrong).unwrap(), &request(dial)).is_err());
    }
    for (dial, outcome, reason) in [
        (false, "submitted", "disabled"),
        (false, "rejected", "accepted"),
        (true, "invocation_attempted", "dispatch_error"),
        (true, "not_invoked", "returned"),
        (true, "outcome_unknown", "disabled"),
        (false, "unknown", "accepted"),
        (true, "not_invoked", "missing_call"),
    ] {
        assert!(
            decode_receipt(
                &serde_json::to_vec(&reply(outcome, reason)).unwrap(),
                &request(dial)
            )
            .is_err()
        );
    }
}
#[test]
fn duplicate_escaped_unknown_and_trailing_reply_data_are_rejected() {
    let valid = serde_json::to_string(&reply("submitted", "accepted")).unwrap();
    for invalid in [
        valid.replace("\"v\":1", "\"v\":1,\"v\":1"),
        valid.replace("\"v\":1", "\"v\":1,\"\\u0076\":1"),
        valid.replace("\"v\":1", "\"v\":true"),
        valid.replace("\"v\":1", "\"v\":1.0"),
        valid.replace("\"v\":1", "\"v\":1,\"extra\":null"),
        format!("{valid}{{}}"),
        "[]".into(),
    ] {
        assert!(decode_receipt(invalid.as_bytes(), &request(false)).is_err());
    }
    assert!(decode_receipt(&[0xff], &request(false)).is_err());
    let padded = format!("{valid}{}", " ".repeat(65536));
    assert!(decode_receipt(padded.as_bytes(), &request(false)).is_err());
}

#[test]
fn controls_require_coherent_availability_and_actions() {
    use omarchy_phone_core::android_protocol::decode_controls;
    let req = request(false);
    let value = json!({"v":1,"id":req.id(),"generation":"dddddddddddddddddddddddddddddddd","revision":3,"controls_allowed":true,"availability":"observed","source":"android.telecom.InCallService","calls":[{"id":7,"state":2,"actions":["answer","reject"]}]});
    let decode = |v: &Value| decode_controls(&serde_json::to_vec(v).unwrap(), req.id());
    let parsed = decode(&value).unwrap();
    assert_eq!(parsed.calls().len(), 1);
    assert_eq!(parsed.revision(), 3);
    for (key, bad) in [
        ("controls_allowed", json!(false)),
        ("availability", json!("unavailable")),
        ("revision", json!(0)),
        ("source", json!("other")),
    ] {
        let mut v = value.clone();
        v[key] = bad;
        assert!(decode(&v).is_err());
    }
    for (key, bad) in [
        ("state", json!(4)),
        ("id", json!(0)),
        ("id", json!(2147483648u64)),
        ("actions", json!(["answer", "answer"])),
    ] {
        let mut v = value.clone();
        v["calls"][0][key] = bad;
        assert!(decode(&v).is_err());
    }
    let mut v = value.clone();
    let repeated = v["calls"][0].clone();
    v["calls"].as_array_mut().unwrap().push(repeated);
    assert!(decode(&v).is_err());
    let duplicate = serde_json::to_string(&value)
        .unwrap()
        .replace("\"state\":2", "\"state\":2,\"\\u0073tate\":2");
    assert!(decode_controls(duplicate.as_bytes(), req.id()).is_err());
    let mut empty = value.clone();
    empty["calls"] = json!([]);
    empty["availability"] = json!("unavailable");
    decode(&empty).unwrap();
}
#[test]
fn dial_availability_cannot_invent_an_offer_or_hide_pending() {
    use omarchy_phone_core::android_protocol::decode_dial_status;
    let req = request(true);
    let value = json!({"v":1,"id":req.id(),"generation":"dddddddddddddddddddddddddddddddd","can_dial":true,"offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","pending":false,"reason":"ready"});
    let decode = |v: &Value| decode_dial_status(&serde_json::to_vec(v).unwrap(), req.id());
    assert!(decode(&value).unwrap().offer().is_some());
    for (key, bad) in [
        ("offer", json!("")),
        ("pending", json!(true)),
        ("can_dial", json!(false)),
        ("reason", json!("disabled")),
    ] {
        let mut v = value.clone();
        v[key] = bad;
        assert!(decode(&v).is_err());
    }
    let pending = json!({"v":1,"id":req.id(),"generation":"dddddddddddddddddddddddddddddddd","can_dial":false,"offer":"","pending":true,"reason":"pending"});
    assert!(decode(&pending).unwrap().offer().is_none());
    let mut wrong = pending.clone();
    wrong["pending"] = json!(false);
    assert!(decode(&wrong).is_err());
}
