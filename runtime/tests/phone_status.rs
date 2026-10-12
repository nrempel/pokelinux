use omarchy_phone_core::{Identity, android_protocol::decode_status};
use serde_json::{Value, json};
fn id() -> Identity {
    Identity::try_from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()).unwrap()
}
fn valid() -> Value {
    json!({"v":1,"id":id(),"generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","observed_ms":123,"dialer_role":true,"availability":"observed","source":"android.telecom.InCallService","calls":[{"id":1,"state":2}],"phone_state":{"availability":"observed","state":1,"source":"android.telephony.TelephonyCallback","scope":"default_subscription"}})
}
#[test]
fn distinguishes_telecom_observations_from_default_subscription_phone_state() {
    let value = valid();
    let status = decode_status(&serde_json::to_vec(&value).unwrap(), &id()).unwrap();
    assert_eq!(status.calls().len(), 1);
    assert_eq!(status.phone_state(), Some(1));
    assert!(status.dialer_role());
    let mut absent = value;
    absent["dialer_role"] = json!(false);
    absent["availability"] = json!("unavailable");
    absent["calls"] = json!([]);
    // Phone-state permission and dialer role are independent observations.
    assert_eq!(
        decode_status(&serde_json::to_vec(&absent).unwrap(), &id())
            .unwrap()
            .phone_state(),
        Some(1)
    );
    absent["phone_state"]["availability"] = json!("unavailable");
    absent["phone_state"]["state"] = Value::Null;
    assert_eq!(
        decode_status(&serde_json::to_vec(&absent).unwrap(), &id())
            .unwrap()
            .phone_state(),
        None
    );
}
#[test]
fn rejects_missing_nullable_state_and_incoherent_observations() {
    for (field, bad) in [
        ("state", json!(null)),
        ("state", json!(3)),
        ("state", json!("1")),
        ("availability", json!("unavailable")),
        ("scope", json!("all_subscriptions")),
        ("source", json!("other")),
    ] {
        let mut value = valid();
        value["phone_state"][field] = bad;
        assert!(decode_status(&serde_json::to_vec(&value).unwrap(), &id()).is_err());
    }
    let mut missing = valid();
    missing["phone_state"]["availability"] = json!("unavailable");
    missing["phone_state"]
        .as_object_mut()
        .unwrap()
        .remove("state");
    assert!(decode_status(&serde_json::to_vec(&missing).unwrap(), &id()).is_err());
    for (field, bad) in [
        ("dialer_role", json!(false)),
        ("availability", json!("unavailable")),
        ("observed_ms", json!(9007199254740992u64)),
        ("observed_ms", json!(-1)),
        ("source", json!("other")),
        ("id", json!("cccccccccccccccccccccccccccccccc")),
        ("calls", json!([])),
        ("calls", json!([{"id":1,"state":2},{"id":1,"state":4}])),
        ("calls", json!([{"id":0,"state":2}])),
        ("calls", json!([{"id":1,"state":21}])),
    ] {
        let mut value = valid();
        value[field] = bad;
        assert!(
            decode_status(&serde_json::to_vec(&value).unwrap(), &id()).is_err(),
            "accepted {field}"
        );
    }
}
#[test]
fn rejects_duplicate_fields_extra_data_and_unknown_fields() {
    let valid = valid().to_string();
    for bad in [
        valid.replace("\"state\":1", "\"state\":1,\"state\":1"),
        valid.replace("\"v\":1", "\"v\":1,\"\\u0076\":1"),
        valid.replace("\"state\":1", "\"state\":1,\"extra\":false"),
        format!("{valid}{{}}"),
        " ".repeat(65537),
    ] {
        assert!(decode_status(bad.as_bytes(), &id()).is_err());
    }
}

#[test]
fn local_status_reply_validates_nested_schema_and_identity() {
    use omarchy_phone_core::{local_protocol, local_reply};
    let request = local_protocol::decode(
        format!(
            "{{\"v\":1,\"id\":{},\"op\":\"status\"}}\n",
            serde_json::to_string(&id()).unwrap()
        )
        .as_bytes(),
    )
    .unwrap();
    let response = |snapshot: Value| {
        format!(
            "{}\n",
            json!({"v":1,"id":id(),"result":{"kind":"status","snapshot":snapshot}})
        )
    };
    assert!(local_reply::decode(response(valid()).as_bytes(), &request).is_ok());
    let mut wrong_id = valid();
    wrong_id["id"] = json!("cccccccccccccccccccccccccccccccc");
    assert!(local_reply::decode(response(wrong_id).as_bytes(), &request).is_err());
    let mut invalid_call = valid();
    invalid_call["calls"] = json!([{"id":0,"state":2}]);
    assert!(local_reply::decode(response(invalid_call).as_bytes(), &request).is_err());
    let duplicate = response(valid()).replace("\"state\":1", "\"state\":1,\"state\":1");
    assert!(local_reply::decode(duplicate.as_bytes(), &request).is_err());
}
