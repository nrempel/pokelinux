//! Parent projection component contract; no process admission or Binder evidence.
use super::*;
use serde_json::{Value, json};
fn frame(revision: u64, accepted: bool) -> Frame {
    let expected: Value =
        serde_json::from_str(include_str!("../tests/fixtures/expected.json")).unwrap();
    let mut report = expected["response"].clone();
    assert_eq!(report.as_object().unwrap().len(), 18);
    for descriptor in report["trafficDescriptors"].as_array_mut().unwrap() {
        let bytes = descriptor["osAppId"].take();
        if !bytes.is_null() {
            descriptor["osAppId"] = json!({"osAppId":bytes});
        }
    }
    serde_json::from_value(json!({"schema":1,"incarnation":"11111111111111111111111111111111","attempt":1,
        "revision":revision,"serial":713,"until_ns":1000,"sampled_at_ns":100+revision,
        "disposition":if accepted {json!({"kind":"ProviderAccepted","cid":9})} else {json!({"kind":"OutcomeUnknown"})},
        "provider_report":if accepted {report} else {Value::Null},"association":null,
        "observer_initial_sync_completed":true,"fault":null})).unwrap()
}
#[test]
fn accepted_parent_projection_retains_complete_history_after_local_withdrawal() {
    let mut projection = Projection::new("11111111111111111111111111111111".into());
    let pending = frame(1, false);
    projection
        .accept(&pending, 713, 1000, 200, (10, 20))
        .unwrap();
    assert_eq!(projection.source_revision, 1);
    assert_eq!(projection.provider_disposition, Disposition::OutcomeUnknown);
    assert_eq!(projection.provider_report, None);
    let mut accepted = frame(2, true);
    accepted.association = Some(
        serde_json::from_value(json!({"namespace":{"dev":10,"ino":20},"ifindex":2,
        "lifetime":1,"revision":3,"observed_at_boottime_ns":101}))
        .unwrap(),
    );
    let encoded = accepted.encode().unwrap();
    let independent: Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        independent["provider_report"].as_object().unwrap().len(),
        18
    );
    assert_eq!(independent["provider_report"]["cid"], 9);
    assert_eq!(
        independent["provider_report"]["trafficDescriptors"][0]["osAppId"]["osAppId"],
        json!([1, 2, 0, 255])
    );
    println!(
        "PROJECTION-CONTROL typed complete18-field accepted frame and namespace association validated"
    );
    projection
        .accept(&accepted, 713, 1000, 200, (10, 20))
        .unwrap();
    assert_eq!(
        projection.provider_report, accepted.provider_report,
        "MISSING_PARENT_PROVIDER_HISTORY_PROJECTION"
    );
    assert_eq!(projection.association, accepted.association);
    projection.withdraw();
    assert_eq!(projection.association, None);
    assert_eq!(
        projection.provider_disposition,
        Disposition::ProviderAccepted { cid: 9 }
    );
    assert_eq!(projection.provider_report, accepted.provider_report);
}
#[test]
fn stale_foreign_and_expired_projection_inputs_are_refused() {
    let good = frame(1, false);
    let mut p = Projection::new(good.incarnation.clone());
    p.accept(&good, 713, 1000, 200, (10, 20)).unwrap();
    let before = p.clone();
    assert!(p.accept(&good, 713, 1000, 201, (10, 20)).is_err());
    assert_eq!(p, before);
    let mut foreign = frame(2, false);
    foreign.incarnation = "22222222222222222222222222222222".into();
    assert!(p.accept(&foreign, 713, 1000, 201, (10, 20)).is_err());
    assert_eq!(p, before);
    assert!(
        p.accept(&frame(2, false), 713, 1000, 1000, (10, 20))
            .is_err()
    );
    assert_eq!(p, before);
    let mut foreign = frame(2, true);
    foreign.association = Some(
        serde_json::from_value(json!({"namespace":{"dev":10,"ino":21},"ifindex":2,
        "lifetime":1,"revision":3,"observed_at_boottime_ns":101}))
        .unwrap(),
    );
    assert!(p.accept(&foreign, 713, 1000, 201, (10, 20)).is_err());
    assert_eq!(p, before);
}
