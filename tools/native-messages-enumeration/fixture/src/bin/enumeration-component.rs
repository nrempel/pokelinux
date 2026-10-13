//! Desired complete33 behavior against actual private SQLite, before implementation.
use pokelinux_messages::{projection::Store, protocol::StorageStatus};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert!(args.len() == 2 || (args.len() == 3 && args[2] == "--raw-only"));
    let root = std::path::Path::new(&args[0]);
    std::fs::create_dir(root).unwrap();
    let mut raw = ReceiptOwner::open(&root.join("raw")).unwrap();
    let source = raw.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let hex = "000008915155100000006201213141510005e8329bfd06";
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|n| u8::from_str_radix(&hex[n..n + 2], 16).unwrap())
        .collect();
    let mut ids = Vec::new();
    for _ in 0..33 {
        ids.push(
            raw.admit(source, IncomingKind::Deliver, &bytes)
                .unwrap()
                .to_string(),
        );
    }
    assert_eq!(raw.snapshot(32).unwrap().total, 33);
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        33
    );
    drop(raw);
    let mut store = Store::open(&root.join("raw"), &root.join("inbox")).unwrap();
    if args.len() == 3 {
        std::fs::write(&args[1], b"{\"raw_seed_only\":true,\"raw_occurrences\":33}").unwrap();
        return;
    }
    let mut status = store.reconcile().unwrap();
    let mut turns = 1;
    while status != StorageStatus::Ready && turns <= 4096 / 8 {
        assert!(
            store
                .query(&pokelinux_messages::protocol::Query::Conversations {
                    after: None,
                    limit: 8
                })
                .is_err()
        );
        status = store.reconcile().unwrap();
        turns += 1;
    }
    let report = serde_json::json!({"passed":status==StorageStatus::Ready,"expected_red":status!=StorageStatus::Ready,"signature":"raw33_complete_projection_unavailable","actual_sqlite_raw_occurrences":33,"distinct_equal_receipts":true,"expected_ready":true,"actual_status":format!("{status:?}"),"bounded_turns":turns,"source":"current bounded enumeration implementation"});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
    assert_eq!(
        status,
        StorageStatus::Ready,
        "all33 committed occurrences must be completely projected before Ready"
    );
}
