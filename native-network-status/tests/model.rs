use pokelinux_network_status::{COLLECTION_NS, Counts, Model, VALIDITY_NS};
#[test]
fn healthy_empty_is_available_and_expires_on_getter() {
    let mut model = Model::default();
    let collection = model.begin(10).unwrap();
    assert!(model.complete(
        collection,
        11,
        Some(Counts {
            local_interfaces: 0,
            address_records: 0
        })
    ));
    let view = model.view(12);
    assert_eq!(
        view.counts,
        Some(Counts {
            local_interfaces: 0,
            address_records: 0
        })
    );
    assert_eq!(view.until_ns, 10 + VALIDITY_NS);
    let expired = model.view(10 + VALIDITY_NS);
    assert!(expired.counts.is_none() && expired.until_ns == 0);
}
#[test]
fn original_deadline_and_withdrawal_never_publish_late_or_old_tokens() {
    let mut model = Model::default();
    let old = model.begin(100).unwrap();
    let counts = Counts {
        local_interfaces: 1,
        address_records: 2,
    };
    assert!(model.complete(old, 101, Some(counts)));
    let late = model.begin(102).unwrap();
    assert!(!model.complete(late, 102 + COLLECTION_NS, Some(counts)));
    assert!(model.view(103 + COLLECTION_NS).counts.is_none());
    let retired = model.begin(104 + COLLECTION_NS).unwrap();
    model.withdraw();
    let successor = model.begin(105 + COLLECTION_NS).unwrap();
    assert!(!model.complete(retired, 106 + COLLECTION_NS, Some(counts)));
    assert!(model.complete(successor, 107 + COLLECTION_NS, Some(counts)));
    assert_eq!(model.view(108 + COLLECTION_NS).counts, Some(counts));
}
#[test]
fn invalid_counts_clear_prior_success() {
    let mut model = Model::default();
    let first = model.begin(10).unwrap();
    assert!(model.complete(
        first,
        11,
        Some(Counts {
            local_interfaces: 1,
            address_records: 1
        })
    ));
    let invalid = model.begin(12).unwrap();
    assert!(!model.complete(
        invalid,
        13,
        Some(Counts {
            local_interfaces: 0,
            address_records: 1
        })
    ));
    assert!(model.view(14).counts.is_none());
}
