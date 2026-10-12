use omarchy_phone_core::{MAX_FRAME, decode_request};
use serde_json::{Value, json};

fn request() -> Value {
    json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "session":"cccccccccccccccccccccccccccccccc",
        "command":{"kind":"dial","generation":"dddddddddddddddddddddddddddddddd",
                   "offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"+15550100"}})
}
fn frame(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn accepts_dial_and_normalizes_field_order_for_identity() {
    let bytes = frame(&request());
    let first = decode_request(&bytes).expect("valid dial request");
    let pretty = format!("{}\n", serde_json::to_string_pretty(&request()).unwrap());
    // Whitespace between tokens is JSON, but embedded newlines violate one-line framing.
    assert!(decode_request(pretty.as_bytes()).is_err());
    let reordered = String::from_utf8(bytes).unwrap().replace(
        "\"number\":\"+15550100\",\"offer\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\"",
        "\"offer\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\",\"number\":\"+15550100\"",
    );
    assert_eq!(
        first.command_digest(),
        decode_request(reordered.as_bytes())
            .unwrap()
            .command_digest()
    );
    let mut changed = request();
    changed["command"]["number"] = json!("5550101");
    assert_ne!(
        first.command_digest(),
        decode_request(&frame(&changed)).unwrap().command_digest()
    );
}

#[test]
fn accepts_valid_control_and_rejects_impossible_state_action_pairs() {
    for (state, action) in [
        ("ringing", "answer"),
        ("ringing", "reject"),
        ("active", "disconnect"),
        ("held", "disconnect"),
        ("dialing", "disconnect"),
        ("connecting", "disconnect"),
    ] {
        let mut value = request();
        value["command"] = json!({"kind":"control","generation":"dddddddddddddddddddddddddddddddd",
            "call_id":7,"revision":3,"state":state,"action":action});
        decode_request(&frame(&value)).expect("valid state/action");
        value["command"]["state"] = json!("disconnected");
        assert!(decode_request(&frame(&value)).is_err());
    }
}

#[test]
fn refuses_ambiguous_json_and_extra_fields() {
    let valid = String::from_utf8(frame(&request())).unwrap();
    for text in [
        valid.replace("\"v\":1", "\"v\":1,\"v\":1"),
        valid.replace(
            "\"number\":\"+15550100\"",
            "\"number\":\"+15550100\",\"number\":\"5550101\"",
        ),
        valid.replace("\"kind\":\"dial\"", "\"kind\":\"dial\",\"kind\":\"dial\""),
        valid.replace("\"v\":1", "\"v\":true"),
        valid.replace("\"v\":1", "\"v\":1.0"),
    ] {
        assert!(decode_request(text.as_bytes()).is_err(), "{text}");
    }
    for key in ["unexpected", "op"] {
        let mut value = request();
        value[key] = json!("dial");
        assert!(decode_request(&frame(&value)).is_err());
    }
    let mut value = request();
    value["command"]["unexpected"] = json!(true);
    assert!(decode_request(&frame(&value)).is_err());
}

#[test]
fn rejects_invalid_numbers_and_identities() {
    for number in [
        "",
        "12",
        "１２３",
        "555;0100",
        "555 0100",
        "++1555",
        "123456789012345678901234567890123",
    ] {
        let mut value = request();
        value["command"]["number"] = json!(number);
        assert!(decode_request(&frame(&value)).is_err(), "{number}");
    }
    for field in ["id", "epoch", "session"] {
        for bad in [
            json!(null),
            json!(3),
            json!("ABC"),
            json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ] {
            let mut value = request();
            value[field] = bad;
            assert!(decode_request(&frame(&value)).is_err());
        }
    }
    let mut value = request();
    value["v"] = json!(2);
    assert!(decode_request(&frame(&value)).is_err());
}

#[test]
fn enforces_framing_limits_before_parsing() {
    let bytes = frame(&request());
    assert!(decode_request(&bytes[..bytes.len() - 1]).is_err());
    let mut two = bytes.clone();
    two.extend(&bytes);
    assert!(decode_request(&two).is_err());
    assert!(decode_request(b"[]\n").is_err());
    assert!(decode_request(b"{\"v\":NaN}\n").is_err());
    assert!(decode_request(b"{\"v\":\xff}\n").is_err());
    let mut exact = bytes.clone();
    exact.pop();
    exact.resize(MAX_FRAME - 1, b' ');
    exact.push(b'\n');
    decode_request(&exact).expect("maximum valid frame");
    exact.insert(0, b' ');
    assert!(decode_request(&exact).is_err());
}
