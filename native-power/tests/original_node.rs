mod support;
use pokelinux_power::{
    ApplyResult, Arbiter, Expiry, OwnerId, ResourcePolicy, RetryState, clock::BootTime,
};
use std::{collections::BTreeMap, fs, time::Duration};
use support::TemporaryNode;
fn now(ms: u64) -> BootTime {
    BootTime::from_elapsed(Duration::from_millis(ms))
}
fn arbiter() -> Arbiter {
    Arbiter::new(
        vec![ResourcePolicy {
            key: "performance".into(),
            values: vec!["high".into(), "low".into(), "idle".into()],
            default_index: 2,
        }],
        Duration::from_millis(500),
    )
    .unwrap()
}
fn apply(a: &mut Arbiter, node: &TemporaryNode, ms: u64) -> String {
    if let Some(plan) = a.begin_apply("performance", now(ms)).unwrap() {
        node.execute(&plan).unwrap();
        a.finish_apply(&plan, ApplyResult::Applied, now(ms))
            .unwrap();
    }
    node.read()
}
fn insert(out: &mut BTreeMap<String, String>, key: &str, value: impl ToString) {
    assert!(out.insert(key.to_owned(), value.to_string()).is_none());
}
fn bit(out: &mut BTreeMap<String, String>, key: &str, value: bool) {
    insert(out, key, u8::from(value));
}
#[test]
fn native_outcomes_match_actual_unmodified_public_node_execution() {
    let mut actual = BTreeMap::new();
    let mut a = arbiter();
    let lease = a
        .acquire(
            OwnerId(1),
            "performance",
            "high",
            Expiry::At(now(3_600_000)),
            now(0),
        )
        .unwrap();
    a.renew(OwnerId(1), &lease, Expiry::At(now(7_200_000)), now(0))
        .unwrap();
    bit(
        &mut actual,
        "renewal-extended",
        a.snapshot("performance", now(7_100_001)).unwrap().desired == "high",
    );
    // Fresh instance keeps both renewal operations at the same clock sample.
    let mut a = arbiter();
    let lease = a
        .acquire(
            OwnerId(1),
            "performance",
            "high",
            Expiry::At(now(3_600_000)),
            now(0),
        )
        .unwrap();
    a.renew(OwnerId(1), &lease, Expiry::At(now(7_200_000)), now(0))
        .unwrap();
    a.renew(OwnerId(1), &lease, Expiry::At(now(1_800_000)), now(0))
        .unwrap();
    bit(
        &mut actual,
        "renewal-never-shortens",
        a.snapshot("performance", now(7_100_001)).unwrap().desired == "high",
    );
    let mut a = arbiter();
    // Native acquisition rejects an already elapsed deadline; reference prunes it.
    bit(
        &mut actual,
        "expired-active",
        a.acquire(
            OwnerId(1),
            "performance",
            "high",
            Expiry::At(now(0)),
            now(1),
        )
        .is_ok(),
    );
    let forever = a
        .acquire(
            OwnerId(1),
            "performance",
            "high",
            Expiry::UntilCancelled,
            now(1),
        )
        .unwrap();
    bit(
        &mut actual,
        "forever-active",
        a.snapshot("performance", now(86_400_001)).unwrap().desired == "high",
    );
    bit(
        &mut actual,
        "owner-cancel",
        a.cancel(OwnerId(1), &forever, now(86_400_001)).is_ok(),
    );
    bit(
        &mut actual,
        "cancelled-active",
        a.snapshot("performance", now(86_400_001)).unwrap().desired != "idle",
    );
    let mut a = arbiter();
    let node = TemporaryNode::new();
    node.write("stale");
    insert(&mut actual, "initial-default", apply(&mut a, &node, 0));
    let low = a
        .acquire(
            OwnerId(1),
            "performance",
            "low",
            Expiry::UntilCancelled,
            now(0),
        )
        .unwrap();
    insert(&mut actual, "low-value", apply(&mut a, &node, 0));
    let high = a
        .acquire(
            OwnerId(2),
            "performance",
            "high",
            Expiry::UntilCancelled,
            now(0),
        )
        .unwrap();
    insert(&mut actual, "high-value", apply(&mut a, &node, 0));
    a.cancel(OwnerId(2), &high, now(0)).unwrap();
    insert(&mut actual, "cancel-high", apply(&mut a, &node, 0));
    a.cancel(OwnerId(1), &low, now(0)).unwrap();
    insert(&mut actual, "cancel-last", apply(&mut a, &node, 0));
    a.acquire(
        OwnerId(1),
        "performance",
        "high",
        Expiry::At(now(1)),
        now(0),
    )
    .unwrap();
    insert(&mut actual, "expired-high", apply(&mut a, &node, 1));
    bit(
        &mut actual,
        "invalid-value-accepted",
        a.acquire(
            OwnerId(1),
            "performance",
            "bad-index",
            Expiry::UntilCancelled,
            now(1),
        )
        .is_ok(),
    );
    a.acquire(
        OwnerId(1),
        "performance",
        "low",
        Expiry::UntilCancelled,
        now(1),
    )
    .unwrap();
    a.acquire(
        OwnerId(1),
        "performance",
        "high",
        Expiry::UntilCancelled,
        now(1),
    )
    .unwrap();
    a.cancel_owner(OwnerId(1), now(1)).unwrap();
    insert(
        &mut actual,
        "cancel-owner-all-values",
        apply(&mut a, &node, 1),
    );
    let mut a = arbiter();
    let node = TemporaryNode::new();
    let lease = a
        .acquire(
            OwnerId(1),
            "performance",
            "high",
            Expiry::UntilCancelled,
            now(0),
        )
        .unwrap();
    let plan = a.begin_apply("performance", now(0)).unwrap().unwrap();
    assert!(node.execute(&plan).is_err());
    a.finish_apply(&plan, ApplyResult::Failed, now(0)).unwrap();
    let RetryState::After(retry) = a.snapshot("performance", now(0)).unwrap().retry else {
        panic!("retry deadline required");
    };
    insert(&mut actual, "failed-retry-ms", retry.elapsed().as_millis());
    bit(&mut actual, "failed-file-absent", !node.path().exists());
    node.write("old");
    insert(
        &mut actual,
        "retry-current-value",
        apply(&mut a, &node, 500),
    );
    a.cancel(OwnerId(1), &lease, now(501)).unwrap();
    insert(&mut actual, "retry-default", apply(&mut a, &node, 501));
    let mut shared = 0;
    let mut only = BTreeMap::new();
    for line in include_str!("fixtures/public-node-reference.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3);
        match fields[0] {
            "shared" => {
                assert_eq!(
                    actual.remove(fields[1]).as_deref(),
                    Some(fields[2]),
                    "original-source case {}",
                    fields[1]
                );
                shared += 1;
            }
            "reference-only" => {
                assert!(only.insert(fields[1], fields[2]).is_none());
            }
            scope => panic!("unexpected fixture scope {scope}"),
        }
    }
    assert_eq!(shared, 18);
    assert!(actual.is_empty());
    assert_eq!(
        only,
        BTreeMap::from([("held-descriptor", "1"), ("released-descriptor", "0")])
    );
    // The component does not claim to implement the vendor file descriptor owner.
    assert!(fs::metadata(node.path()).unwrap().len() <= 5);
}
