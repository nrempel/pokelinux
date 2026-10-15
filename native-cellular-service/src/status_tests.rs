use super::*;
use serde_json::Value;
fn full() -> Status {
    Status {
        schema: 1,
        server_pid: 100,
        worker_pid: 101,
        phase: Phase::Initializing,
        until_ns: 1000,
        sampled_at_ns: 200,
        fault: None,
        observation: Projection::new("11111111111111111111111111111111".into()),
    }
}
fn omitted(field: &str, nested: bool) {
    let expected = full();
    let bytes = expected.encode().unwrap();
    assert_eq!(serde_json::from_slice::<Status>(&bytes).unwrap(), expected);
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    let object = if nested {
        value["observation"].as_object_mut().unwrap()
    } else {
        value.as_object_mut().unwrap()
    };
    assert!(object.remove(field).unwrap().is_null());
    let altered = serde_json::to_vec(&value).unwrap();
    println!(
        "STATUS-SHAPE-CONTROL valid complete status then exactly one omitted nullable {field}"
    );
    assert!(
        serde_json::from_slice::<Status>(&altered).is_err(),
        "MISSING_REQUIRED_NULLABLE_STATUS_FIELD:{field}"
    );
}
#[test]
fn missing_local_fault_is_refused() {
    omitted("fault", false);
}
#[test]
fn missing_provider_history_is_refused() {
    omitted("provider_report", true);
}
#[test]
fn missing_association_is_refused() {
    omitted("association", true);
}
