use omarchy_phone_core::{
    local_protocol::{self, LocalRequest},
    local_reply,
};
use serde_json::{Value, json};
fn request(op: &str) -> LocalRequest {
    let mut value = json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","op":op});
    if matches!(op, "submit" | "lookup" | "acknowledge") {
        value["epoch"] = json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    }
    if op == "submit" {
        value["admission"] = serde_json::json!("ffffffffffffffffffffffffffffffff");
        value["session"] = json!("cccccccccccccccccccccccccccccccc");
        value["command"] = json!({"kind":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"});
    } else if matches!(op, "submit" | "lookup" | "acknowledge") {
        value["target"] = json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    }
    local_protocol::decode(format!("{value}\n").as_bytes()).unwrap()
}
fn reply(body: Value) -> Value {
    json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","result":body})
}
fn accepted(value: &Value, op: &str) -> bool {
    local_reply::decode(format!("{value}\n").as_bytes(), &request(op)).is_ok()
}
#[test]
fn accepts_only_the_expected_operation_and_correlated_identifiers() {
    for (op, body) in [
        (
            "hello",
            json!({"kind":"hello","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","session":"cccccccccccccccccccccccccccccccc"}),
        ),
        ("submit", json!({"kind":"submission","outcome":"pending"})),
        (
            "lookup",
            json!({"kind":"receipt","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","target":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","receipt":null}),
        ),
        (
            "acknowledge",
            json!({"kind":"acknowledged","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","target":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"}),
        ),
    ] {
        let value = reply(body);
        assert!(accepted(&value, op));
        for wrong in ["hello", "submit", "lookup", "acknowledge"] {
            if wrong != op {
                assert!(!accepted(&value, wrong));
            }
        }
        let mut wrong = value.clone();
        wrong["id"] = json!("ffffffffffffffffffffffffffffffff");
        assert!(!accepted(&wrong, op));
        if op == "lookup" || op == "acknowledge" {
            let mut wrong = value.clone();
            wrong["result"]["target"] = json!("ffffffffffffffffffffffffffffffff");
            assert!(!accepted(&wrong, op));
            let mut wrong = value;
            wrong["result"]["epoch"] = json!("ffffffffffffffffffffffffffffffff");
            assert!(!accepted(&wrong, op));
        }
    }
}
#[test]
fn submission_outcome_must_match_the_command_kind() {
    for outcome in [
        "pending",
        "unknown",
        "not_sent",
        "dial_invoked",
        "dial_not_invoked",
    ] {
        assert!(accepted(
            &reply(json!({"kind":"submission","outcome":outcome})),
            "submit"
        ));
    }
    for outcome in [
        "control_submitted",
        "control_rejected",
        "success",
        "connected",
    ] {
        assert!(!accepted(
            &reply(json!({"kind":"submission","outcome":outcome})),
            "submit"
        ));
    }
}
#[test]
fn explicit_absence_and_acknowledgement_are_not_invented() {
    let base = reply(
        json!({"kind":"receipt","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","target":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","receipt":null}),
    );
    assert!(accepted(&base, "lookup"));
    let mut missing = base.clone();
    missing["result"].as_object_mut().unwrap().remove("receipt");
    assert!(!accepted(&missing, "lookup"));
    for outcome in [
        "pending",
        "unknown",
        "not_sent",
        "dial_invoked",
        "dial_not_invoked",
        "control_submitted",
        "control_rejected",
    ] {
        let mut value = base.clone();
        value["result"]["receipt"] = json!({"outcome":outcome,"acknowledged":false});
        assert!(accepted(&value, "lookup"));
        value["result"]["receipt"]["acknowledged"] = json!(true);
        assert_eq!(accepted(&value, "lookup"), outcome == "unknown");
    }
}
#[test]
fn rejects_ambiguous_frames_and_unknown_errors() {
    let value = reply(json!({"kind":"error","code":"busy"}));
    for op in ["hello", "submit", "lookup", "acknowledge"] {
        assert!(accepted(&value, op));
    }
    let text = value.to_string();
    for bad in [
        text.replace("\"v\":1", "\"v\":1,\"v\":1"),
        text.replace("\"v\":1", "\"v\":1,\"\\u0076\":1"),
        text.replace("\"busy\"", "\"success\""),
        text.replace("\"code\":", "\"extra\":0,\"code\":"),
        format!("{text}{{}}"),
        " ".repeat(16384),
    ] {
        assert!(local_reply::decode(format!("{bad}\n").as_bytes(), &request("hello")).is_err());
    }
    assert!(local_reply::decode(text.as_bytes(), &request("hello")).is_err());
}

#[test]
fn control_submission_rejects_dial_outcomes() {
    let command=local_protocol::decode(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"submit\",\"epoch\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"session\":\"cccccccccccccccccccccccccccccccc\",\"admission\":\"ffffffffffffffffffffffffffffffff\",\"command\":{\"kind\":\"control\",\"generation\":\"dddddddddddddddddddddddddddddddd\",\"call_id\":1,\"revision\":1,\"state\":\"ringing\",\"action\":\"answer\"}}\n").unwrap();
    for (outcome, expected) in [
        ("control_submitted", true),
        ("control_rejected", true),
        ("pending", true),
        ("unknown", true),
        ("not_sent", true),
        ("dial_invoked", false),
        ("dial_not_invoked", false),
    ] {
        let value = reply(json!({"kind":"submission","outcome":outcome}));
        assert_eq!(
            local_reply::decode(format!("{value}\n").as_bytes(), &command).is_ok(),
            expected
        );
    }
}

#[test]
fn observation_snapshots_preserve_authority_and_reject_incoherent_data() {
    let controls = reply(
        json!({"kind":"controls","snapshot":{"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","revision":7,"controls_allowed":true,"availability":"observed","source":"android.telecom.InCallService","calls":[{"id":1,"state":2,"actions":["answer","reject"]}]}}),
    );
    let dial = reply(
        json!({"kind":"dial_status","snapshot":{"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","can_dial":true,"offer":"dddddddddddddddddddddddddddddddd","pending":false,"reason":"ready"}}),
    );
    for (op, value) in [
        ("controls", controls.clone()),
        ("dial_status", dial.clone()),
    ] {
        assert!(accepted(&value, op));
        assert!(!accepted(&value, "status"));
        let mut wrong = value.clone();
        wrong["result"]["snapshot"]["id"] = json!("cccccccccccccccccccccccccccccccc");
        assert!(!accepted(&wrong, op));
        let encoded = format!("{value}\n").replace("\"v\":1", "\"v\":1,\"v\":1");
        assert!(local_reply::decode(encoded.as_bytes(), &request(op)).is_err());
    }
    let mut invalid = controls;
    invalid["result"]["snapshot"]["calls"][0]["state"] = json!(4);
    assert!(!accepted(&invalid, "controls"));
    let mut invalid = dial.clone();
    invalid["result"]["snapshot"]["pending"] = json!(true);
    assert!(!accepted(&invalid, "dial_status"));
    let mut disabled = dial;
    disabled["result"]["snapshot"]["can_dial"] = json!(false);
    disabled["result"]["snapshot"]["offer"] = json!("");
    disabled["result"]["snapshot"]["reason"] = json!("disabled");
    assert!(accepted(&disabled, "dial_status"));
}

#[test]
fn recovery_accepts_only_one_unresolved_receipt_for_the_current_epoch() {
    let request=local_protocol::decode(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"recovery\",\"epoch\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\n").unwrap();
    let value = reply(
        json!({"kind":"recovery","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","items":[{"id":"cccccccccccccccccccccccccccccccc","outcome":"unknown"}]}),
    );
    let accepts = |v: &Value| local_reply::decode(format!("{v}\n").as_bytes(), &request).is_ok();
    assert!(accepts(&value));
    let mut pending = value.clone();
    pending["result"]["items"][0]["outcome"] = json!("pending");
    assert!(accepts(&pending));
    let mut empty = value.clone();
    empty["result"]["items"] = json!([]);
    assert!(accepts(&empty));
    let mut wrong = value.clone();
    wrong["result"]["epoch"] = json!("dddddddddddddddddddddddddddddddd");
    assert!(!accepts(&wrong));
    let mut terminal = value.clone();
    terminal["result"]["items"][0]["outcome"] = json!("dial_invoked");
    assert!(!accepts(&terminal));
    let mut duplicate = value.clone();
    duplicate["result"]["items"]
        .as_array_mut()
        .unwrap()
        .push(value["result"]["items"][0].clone());
    assert!(!accepts(&duplicate));
    let mut missing = value;
    missing["result"].as_object_mut().unwrap().remove("items");
    assert!(!accepts(&missing));
}

#[test]
fn fence_reply_binds_epoch_and_session_and_requires_admission_token() {
    let request=local_protocol::decode(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"fence\",\"epoch\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"session\":\"cccccccccccccccccccccccccccccccc\"}\n").unwrap();
    let value = reply(
        json!({"kind":"fence","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","session":"cccccccccccccccccccccccccccccccc","admission":"dddddddddddddddddddddddddddddddd","items":[]}),
    );
    let accepts = |v: &Value| local_reply::decode(format!("{v}\n").as_bytes(), &request).is_ok();
    assert!(accepts(&value));
    for field in ["epoch", "session", "admission", "items"] {
        let mut missing = value.clone();
        missing["result"].as_object_mut().unwrap().remove(field);
        assert!(!accepts(&missing));
    }
    for field in ["epoch", "session"] {
        let mut wrong = value.clone();
        wrong["result"][field] = json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert!(!accepts(&wrong));
    }
}
