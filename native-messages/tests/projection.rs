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

#[test]
fn complete33_projection_uses_bounded_turns_and_never_releases_partial_ready() {
    let (d, first) = fixture();
    let mut raw = ReceiptOwner::open(&d.0.join("raw")).unwrap();
    let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let bytes = raw.lookup(first).unwrap().raw().to_vec();
    let mut ids = vec![first.to_string()];
    for _ in 0..32 {
        ids.push(
            raw.admit(source, IncomingKind::Deliver, &bytes)
                .unwrap()
                .to_string(),
        );
    }
    drop(raw);
    for _ in 0..2 {
        let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
        let q = Query::Conversations {
            after: None,
            limit: 8,
        };
        for _ in 0..4 {
            assert_eq!(
                store.reconcile().unwrap(),
                StorageStatus::ProjectionBacklogUnavailable
            );
            assert!(store.query(&q).is_err());
        }
        assert_eq!(store.reconcile().unwrap(), StorageStatus::Ready);
        let Page::Conversations { items } = store.query(&q).unwrap() else {
            panic!("page")
        };
        assert_eq!(items[0].messages, 33);
        let conversation = items[0].id.clone();
        let mut seen = Vec::new();
        let mut after = 0;
        loop {
            let Page::Messages { items } = store
                .query(&Query::Messages {
                    conversation: conversation.clone(),
                    after_ordinal: after,
                    limit: 2,
                })
                .unwrap()
            else {
                panic!("page")
            };
            if items.is_empty() {
                break;
            }
            after = items.last().unwrap().ordinal;
            seen.extend(items.into_iter().map(|r| r.receipt.as_str().to_owned()));
        }
        assert_eq!(seen, ids);
    }
}

#[test]
fn paged_reconstruction_keeps_deleted_retained_and_excluded_receipt_keys() {
    let (d, first) = fixture();
    let mut raw = ReceiptOwner::open(&d.0.join("raw")).unwrap();
    let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let bytes = raw.lookup(first).unwrap().raw().to_vec();
    for _ in 0..32 {
        raw.admit(source, IncomingKind::Deliver, &bytes).unwrap();
    }
    raw.admit(source, IncomingKind::Deliver, &[1]).unwrap();
    raw.admit(source, IncomingKind::StatusReport, &[0]).unwrap();
    drop(raw);
    let finish = |store: &mut Store| {
        for _ in 0..513 {
            if store.reconcile().unwrap() == StorageStatus::Ready {
                return;
            }
        }
        panic!("bounded reconstruction did not complete");
    };
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    finish(&mut store);
    drop(store);
    let mut inbox = pokelinux_inbox::Inbox::open(&d.0.join("inbox")).unwrap();
    let control = inbox.control();
    control.set_import_allowed(true).unwrap();
    control.set_read_allowed(true).unwrap();
    let import = control.import_lease().unwrap();
    inbox.delete(&import, first.into()).unwrap();
    assert_eq!(inbox.retain_latest(&import, 30, 2).unwrap(), 2);
    assert_eq!(
        inbox.stats(&control.read_lease().unwrap()).unwrap().keys,
        35
    );
    drop(inbox);
    for _ in 0..2 {
        let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
        finish(&mut store);
        let Page::Conversations { items } = store
            .query(&Query::Conversations {
                after: None,
                limit: 8,
            })
            .unwrap()
        else {
            panic!("page")
        };
        assert_eq!(items[0].messages, 30);
        drop(store);
    }
    let inbox = pokelinux_inbox::Inbox::open(&d.0.join("inbox")).unwrap();
    let control = inbox.control();
    control.set_read_allowed(true).unwrap();
    let stats = inbox.stats(&control.read_lease().unwrap()).unwrap();
    assert_eq!(stats.keys, 35);
    assert_eq!(stats.text_messages, 30);
}

#[test]
fn readonly_completed_prefix_excludes_partial_tail_and_derived_counts() {
    use pokelinux_messages::projection::Reader;
    let (d, first) = fixture();
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    assert_eq!(store.reconcile().unwrap(), StorageStatus::Ready);
    let upper = store.completed_upper().unwrap();
    assert_eq!(upper, 1);
    let mut reader = Reader::open(&d.0.join("inbox")).unwrap();
    let source = store.attach_test_incoming(Slot(0)).unwrap();
    let bytes: Vec<_> = (0.."000008915155100000006201213141510005e8329bfd06".len())
        .step_by(2)
        .map(|n| {
            u8::from_str_radix(
                &"000008915155100000006201213141510005e8329bfd06"[n..n + 2],
                16,
            )
            .unwrap()
        })
        .collect();
    for _ in 0..32 {
        store
            .admit_incoming(source, IncomingKind::Deliver, &bytes)
            .unwrap();
    }
    assert_eq!(
        store.reconcile().unwrap(),
        StorageStatus::ProjectionBacklogUnavailable
    );
    assert!(store.completed_upper().is_err());
    let q = Query::Conversations {
        after: None,
        limit: 8,
    };
    let Page::Conversations { items } = reader.query(&q, upper).unwrap() else {
        panic!("page")
    };
    assert_eq!(
        items[0].messages, 1,
        "summary counts must derive only from qualified prefix"
    );
    let Page::Messages { items } = reader
        .query(
            &Query::Messages {
                conversation: items[0].id.clone(),
                after_ordinal: 0,
                limit: 2,
            },
            upper,
        )
        .unwrap()
    else {
        panic!("page")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].receipt.as_str(), first.to_string());
    for _ in 0..5 {
        if store.reconcile().unwrap() == StorageStatus::Ready {
            break;
        }
    }
    let next = store.completed_upper().unwrap();
    assert_eq!(next, 33);
    let Page::Conversations { items } = reader.query(&q, upper).unwrap() else {
        panic!("page")
    };
    assert_eq!(items[0].messages, 1);
    let Page::Conversations { items } = reader.query(&q, next).unwrap() else {
        panic!("page")
    };
    assert_eq!(items[0].messages, 33);
    assert!(reader.query(&q, next + 1).is_err());
}
#[test]
fn readonly_open_preserves_writer_lock_and_creates_no_missing_storage() {
    use pokelinux_messages::projection::Reader;
    let (d, _) = fixture();
    assert!(Reader::open(&d.0.join("missing")).is_err());
    assert!(!d.0.join("missing").exists());
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    store.reconcile().unwrap();
    let mut reader = Reader::open(&d.0.join("inbox")).unwrap();
    assert!(
        pokelinux_inbox::Inbox::open(&d.0.join("inbox")).is_err(),
        "reader must not take or release writer ownership"
    );
    assert!(
        reader
            .query(
                &Query::Conversations {
                    after: None,
                    limit: 8
                },
                store.completed_upper().unwrap()
            )
            .is_ok()
    );
    drop(reader);
    assert!(pokelinux_inbox::Inbox::open(&d.0.join("inbox")).is_err());
}

#[test]
fn fixed_read_prefix_never_exposes_a_new_partial_conversation() {
    use pokelinux_messages::projection::Reader;
    let (d, _) = fixture();
    let mut store = Store::open(&d.0.join("raw"), &d.0.join("inbox")).unwrap();
    assert_eq!(store.reconcile().unwrap(), StorageStatus::Ready);
    let upper = store.completed_upper().unwrap();
    let mut reader = Reader::open(&d.0.join("inbox")).unwrap();
    let q = Query::Conversations {
        after: None,
        limit: 8,
    };
    let Page::Conversations { items: original } = reader.query(&q, upper).unwrap() else {
        panic!("page")
    };
    let source = store.attach_test_incoming(Slot(1)).unwrap();
    let bytes: Vec<_> = (0.."000008915155100000006201213141510005e8329bfd06".len())
        .step_by(2)
        .map(|n| {
            u8::from_str_radix(
                &"000008915155100000006201213141510005e8329bfd06"[n..n + 2],
                16,
            )
            .unwrap()
        })
        .collect();
    for _ in 0..32 {
        store
            .admit_incoming(source, IncomingKind::Deliver, &bytes)
            .unwrap();
    }
    assert_eq!(
        store.reconcile().unwrap(),
        StorageStatus::ProjectionBacklogUnavailable
    );
    let Page::Conversations { items } = reader.query(&q, upper).unwrap() else {
        panic!("page")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id.as_str(), original[0].id.as_str());
    assert_eq!(items[0].messages, 1);
    for _ in 0..5 {
        if store.reconcile().unwrap() == StorageStatus::Ready {
            break;
        }
    }
    let Page::Conversations { items } = reader.query(&q, store.completed_upper().unwrap()).unwrap()
    else {
        panic!("page")
    };
    let tail = items.iter().find(|c| c.slot == 1).unwrap();
    assert_eq!(tail.messages, 32);
    assert!(
        reader
            .query(
                &Query::Messages {
                    conversation: tail.id.clone(),
                    after_ordinal: 0,
                    limit: 2
                },
                upper
            )
            .is_err()
    );
    assert!(
        reader
            .query(
                &Query::Conversations {
                    after: Some(tail.id.clone()),
                    limit: 8
                },
                upper
            )
            .is_err()
    );
}
