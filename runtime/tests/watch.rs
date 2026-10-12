use omarchy_phone_core::{
    Identity,
    watch::{SourceState, WatchQuery, decode_snapshot},
};
use serde_json::{Value, json};
fn id(c: char) -> Identity {
    c.to_string().repeat(32).try_into().unwrap()
}
fn snapshot() -> Value {
    json!({"v":1,"id":id('a'),"generation":id('b'),"revision":4,
        "source":"android.telecom.InCallService","source_state":"observed",
        "ringing":[{"id":7,"episode":id('c')}]})
}
#[test]
fn watch_queries_have_no_command_authority_and_bounded_wait() {
    let first = WatchQuery::new(id('a'), None, 25_000).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&first.encode()).unwrap(),
        json!({"v":1,"op":"watch","id":id('a'),"wait_ms":25000})
    );
    let reply = decode_snapshot(&serde_json::to_vec(&snapshot()).unwrap(), &id('a')).unwrap();
    let next = WatchQuery::new(id('d'), Some(reply.cursor()), 1000).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&next.encode()).unwrap(),
        json!({"v":1,"op":"watch","id":id('d'),"wait_ms":1000,"after_generation":id('b'),"after_revision":4})
    );
    assert!(WatchQuery::new(id('a'), None, 0).is_ok());
    assert!(WatchQuery::new(id('a'), None, 25_001).is_err());
}
#[test]
fn complete_snapshot_distinguishes_detached_and_unavailable_from_observed() {
    for state in ["observed", "detached", "unavailable"] {
        let mut value = snapshot();
        value["source_state"] = json!(state);
        value["ringing"] = json!([]);
        let reply = decode_snapshot(&serde_json::to_vec(&value).unwrap(), &id('a')).unwrap();
        assert!(reply.ringing().is_empty());
        assert_eq!(
            reply.source_state() == SourceState::Observed,
            state == "observed"
        );
        if state != "observed" {
            value["ringing"] = snapshot()["ringing"].clone();
            assert!(decode_snapshot(&serde_json::to_vec(&value).unwrap(), &id('a')).is_err());
        }
    }
}
#[test]
fn snapshot_rejects_invalid_schema_identity_duplicates_and_bounds() {
    let mut invalid = Vec::new();
    for (key, value) in [
        ("v", json!(2)),
        ("id", json!(id('d'))),
        ("revision", json!(0)),
        ("revision", json!(9_007_199_254_740_992_u64)),
        ("revision", json!(4.0)),
        ("source_state", json!("idle")),
        ("source", json!("other")),
        ("generation", json!("")),
    ] {
        let mut v = snapshot();
        v[key] = value;
        invalid.push(v);
    }
    let mut v = snapshot();
    v["number"] = json!("5550100");
    invalid.push(v);
    for calls in [
        json!([{"id":0,"episode":id('c')}]),
        json!([{"id":2147483648_u64,"episode":id('c')}]),
        json!([{"id":1,"episode":id('c')},{"id":1,"episode":id('d')}]),
        json!([{"id":1,"episode":id('c')},{"id":2,"episode":id('c')}]),
        json!([{"id":1,"episode":id('c'),"name":"Caller"}]),
        json!([{"id":1,"episode":true}]),
    ] {
        let mut v = snapshot();
        v["ringing"] = calls;
        invalid.push(v);
    }
    let mut v = snapshot();
    v["ringing"] = json!(
        (1..=65)
            .map(|n| json!({"id":n,"episode":format!("{n:032x}")}))
            .collect::<Vec<_>>()
    );
    invalid.push(v);
    for value in invalid {
        assert!(
            decode_snapshot(&serde_json::to_vec(&value).unwrap(), &id('a')).is_err(),
            "{value}"
        );
    }
    let raw = serde_json::to_string(&snapshot()).unwrap();
    let duplicate = raw.replacen("{", "{\"revision\":3,", 1);
    assert!(decode_snapshot(duplicate.as_bytes(), &id('a')).is_err());
    assert!(decode_snapshot(&[b' '; 65_537], &id('a')).is_err());
}
