use pokelinux_messages::{
    projection::Store,
    protocol::{Page, Query, StorageStatus},
};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use std::{fs, path::PathBuf};
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Directory, pokelinux_messaging::receipt::ReceiptId) {
    let p = std::env::temp_dir().join(format!(
        "messages-service-projection-{}",
        pokelinux_lifecycle::fresh_id().unwrap()
    ));
    fs::create_dir(&p).unwrap();
    let mut raw = ReceiptOwner::open(&p.join("raw")).unwrap();
    let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let hex = "000008915155100000006201213141510005e8329bfd06";
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|n| u8::from_str_radix(&hex[n..n + 2], 16).unwrap())
        .collect();
    let id = raw.admit(source, IncomingKind::Deliver, &bytes).unwrap();
    drop(raw);
    (Directory(p), id)
}
#[test]
fn service_owned_projection_runs_with_no_window_and_preserves_local_receipt_id() {
    let (d, id) = fixture();
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    assert_eq!(
        store.reconcile().ok(),
        Some(StorageStatus::Ready),
        "background projection must operate without an app read grant/window"
    );
    let Page::Conversations { items } = store
        .query(&Query::Conversations {
            after: None,
            limit: 8,
        })
        .unwrap()
    else {
        panic!("wrong page")
    };
    assert_eq!(items.len(), 1);
    let Page::Messages { items } = store
        .query(&Query::Messages {
            conversation: items[0].id.clone(),
            after_ordinal: 0,
            limit: 2,
        })
        .unwrap()
    else {
        panic!("wrong page")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].receipt.as_str(), id.to_string());
    assert_eq!(items[0].text, "hello");
}

#[test]
fn explicit_local_reopen_preserves_equal_payload_occurrences_and_exclusions() {
    let (d, id) = fixture();
    let mut raw = ReceiptOwner::open(&d.0.join("raw")).unwrap();
    let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let payload = raw.lookup(id).unwrap().raw().to_vec();
    let second = raw.admit(source, IncomingKind::Deliver, &payload).unwrap();
    raw.admit(source, IncomingKind::Deliver, &[0, 0]).unwrap();
    raw.admit(source, IncomingKind::StatusReport, &payload)
        .unwrap();
    drop(raw);
    for _ in 0..2 {
        let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
        store.reconcile().unwrap();
        let Page::Conversations { items } = store
            .query(&Query::Conversations {
                after: None,
                limit: 8,
            })
            .unwrap()
        else {
            panic!("page")
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].messages, 2);
        let Page::Messages { items } = store
            .query(&Query::Messages {
                conversation: items[0].id.clone(),
                after_ordinal: 0,
                limit: 2,
            })
            .unwrap()
        else {
            panic!("page")
        };
        assert_eq!(items[0].receipt.as_str(), id.to_string());
        assert_eq!(items[1].receipt.as_str(), second.to_string());
    }
}
#[test]
fn unavailable_backlog_preserves_every_raw_occurrence_and_releases_no_text() {
    let (d, id) = fixture();
    let mut raw = ReceiptOwner::open(&d.0.join("raw")).unwrap();
    let s = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let bytes = raw.lookup(id).unwrap().raw().to_vec();
    for _ in 0..32 {
        raw.admit(s, IncomingKind::Deliver, &bytes).unwrap();
    }
    assert_eq!(raw.snapshot(32).unwrap().total, 33);
    drop(raw);
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    assert_eq!(
        store.reconcile().unwrap(),
        StorageStatus::ProjectionBacklogUnavailable
    );
    assert!(
        store
            .query(&Query::Conversations {
                after: None,
                limit: 8
            })
            .is_err()
    );
    drop(store);
    let raw = ReceiptOwner::open(&d.0.join("raw")).unwrap();
    assert_eq!(raw.snapshot(32).unwrap().total, 33);
}
#[test]
fn identifiers_and_queries_have_one_bounded_wire_format() {
    use pokelinux_messages::protocol::DataId;
    for key in [
        "",
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ] {
        assert!(DataId::parse(key).is_err());
    }
    for value in [
        serde_json::json!({"query":"conversations","after":null,"limit":9}),
        serde_json::json!({"query":"conversations","after":null,"limit":0}),
    ] {
        let query: Query = serde_json::from_value(value).unwrap();
        assert!(query.validate().is_err());
    }
    assert!(
        serde_json::from_value::<Query>(
            serde_json::json!({"query":"conversations","after":null,"limit":8,"uid":11010})
        )
        .is_err()
    );
}
