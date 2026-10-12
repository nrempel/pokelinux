use omarchy_phone_core::local_protocol::{LocalRequest, decode};
fn message(op: &str) -> String {
    let mut value = serde_json::json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","op":op});
    if op != "hello" {
        value["epoch"] = serde_json::json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    }
    if op == "submit" {
        value["admission"] = serde_json::json!("ffffffffffffffffffffffffffffffff");
        value["session"] = serde_json::json!("cccccccccccccccccccccccccccccccc");
        value["command"] = serde_json::json!({"kind":"control","generation":"dddddddddddddddddddddddddddddddd","call_id":1,"revision":1,"state":"ringing","action":"answer"});
    } else if op != "hello" {
        value["target"] = serde_json::json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    }
    format!("{value}\n")
}
#[test]
fn decodes_one_current_protocol_for_commands_and_read_only_recovery() {
    assert!(matches!(
        decode(message("hello").as_bytes()).unwrap(),
        LocalRequest::Hello { .. }
    ));
    assert!(matches!(
        decode(message("submit").as_bytes()).unwrap(),
        LocalRequest::Submit(_, _)
    ));
    assert!(matches!(
        decode(message("lookup").as_bytes()).unwrap(),
        LocalRequest::Lookup { .. }
    ));
    assert!(matches!(
        decode(message("acknowledge").as_bytes()).unwrap(),
        LocalRequest::Acknowledge { .. }
    ));
}
#[test]
fn rejects_duplicate_tags_unknown_fields_wrong_types_and_invalid_commands() {
    for op in ["hello", "submit", "lookup", "acknowledge"] {
        let original = message(op);
        for invalid in [
            original.replace("\"v\":1", "\"v\":2"),
            original.replace("\"v\":1", "\"v\":1,\"v\":1"),
            original.replace("\"v\":1", "\"v\":1,\"\\u0076\":1"),
            original.replace("\"v\":1", "\"v\":true"),
            original.replace("\"v\":1", "\"v\":1,\"unknown\":0"),
            original.replace(
                &format!("\"op\":\"{op}\""),
                &format!("\"op\":\"{op}\",\"op\":\"{op}\""),
            ),
            original.trim_end().to_string(),
            format!("{original}{{}}\n"),
        ] {
            assert!(decode(invalid.as_bytes()).is_err(), "accepted {invalid}");
        }
    }
    let command = message("submit");
    for invalid in [
        command.replace("\"call_id\":1", "\"call_id\":0"),
        command.replace("ringing", "active"),
        command.replace("\"revision\":1", "\"revision\":9007199254740992"),
    ] {
        assert!(decode(invalid.as_bytes()).is_err());
    }
    assert!(decode(&vec![b' '; 16385]).is_err());
}

#[test]
fn admission_token_is_required_for_every_submission() {
    let mut value: serde_json::Value = serde_json::from_str(&message("submit")).unwrap();
    value.as_object_mut().unwrap().remove("admission");
    assert!(decode(format!("{value}\n").as_bytes()).is_err());
}
