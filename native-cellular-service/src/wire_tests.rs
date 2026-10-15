use super::*;
use serde_json::{Value, json};

use crate::expected;

fn fixture() -> Frame {
    Frame {
        schema: 1,
        incarnation: "12121212121212121212121212121212".into(),
        attempt: 1,
        revision: 7,
        serial: 713,
        until_ns: 20_000,
        sampled_at_ns: 10_000,
        disposition: Disposition::ProviderAccepted { cid: 9 },
        provider_report: Some(Report::from_generated(&expected::result()).unwrap()),
        association: Some(Association {
            namespace: Namespace { dev: 5, ino: 123 },
            ifindex: 2,
            lifetime: 1,
            revision: 5,
            observed_at_boottime_ns: 9_000,
        }),
        observer_initial_sync_completed: true,
        fault: None,
    }
}
fn line(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn expected_report() -> Value {
    json!({
        "cause":0,"suggestedRetryTime":-1,"cid":9,"active":2,"type":2,"ifname":"pklabi0",
        "addresses":[
            {"address":"192.0.2.2/32","addressProperties":0,"deprecationTime":i64::MAX,"expirationTime":i64::MAX},
            {"address":"2001:db8:1::2/64","addressProperties":0,"deprecationTime":i64::MAX,"expirationTime":i64::MAX}
        ],
        "dnses":["192.0.2.53","2001:db8::53"],"gateways":["192.0.2.1"],
        "pcscf":["192.0.2.20","2001:db8::20"],"mtuV4":1400,"mtuV6":1400,
        "defaultQos":{"noinit":false},"qosSessions":[],"handoverFailureMode":0,"pduSessionId":7,
        "sliceInfo":{"sliceServiceType":1,"sliceDifferentiator":0x010203,
            "mappedHplmnSst":2,"mappedHplmnSd":0x040506,"status":2},
        "trafficDescriptors":[{"dnn":"interop.invalid","osAppId":{"osAppId":[1,2,0,255]}}]
    })
}

#[test]
fn full_encoded_report_then_complete_decode() {
    let frame = fixture();
    let bytes = frame.encode().unwrap();
    assert!(bytes.len() <= MAX_RECORD_BYTES);
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 1);
    // This independent generic parser checks the complete encoder mapping before
    // exercising the intentionally missing production decoder API.
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(actual.as_object().unwrap().len(), 12);
    assert_eq!(actual["schema"], 1);
    assert_eq!(actual["incarnation"], "12121212121212121212121212121212");
    assert_eq!(actual["attempt"], 1);
    assert_eq!(actual["revision"], 7);
    assert_eq!(actual["serial"], 713);
    assert_eq!(actual["until_ns"], 20_000);
    assert_eq!(actual["sampled_at_ns"], 10_000);
    assert_eq!(
        actual["disposition"],
        json!({"kind":"ProviderAccepted","cid":9})
    );
    assert_eq!(actual["provider_report"].as_object().unwrap().len(), 18);
    assert_eq!(actual["provider_report"], expected_report());
    assert_eq!(
        actual["association"],
        json!({"namespace":{"dev":5,"ino":123},
        "ifindex":2,"lifetime":1,"revision":5,"observed_at_boottime_ns":9_000})
    );
    assert_eq!(actual["observer_initial_sync_completed"], true);
    assert!(actual["fault"].is_null());
    println!("DATA-WIRE-CONTROL exact18/nested/complete-bounded-line ready");
    let decoded = decode_exact(&bytes)
        .unwrap_or_else(|error| panic!("MISSING_NATIVE_DATA_FRAME_DECODE: {error:?}"));
    assert_eq!(decoded, frame);
}

#[test]
fn malformed_or_multiple_records_are_refused() {
    let bytes = fixture().encode().unwrap();
    let mut no_newline = bytes.clone();
    no_newline.pop();
    let mut two_records = bytes.clone();
    two_records.extend_from_slice(&bytes);
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(b"null\n");
    let mut invalid_utf8 = bytes.clone();
    invalid_utf8[0] = 255;
    for malformed in [
        no_newline,
        two_records,
        trailing,
        invalid_utf8,
        b"{\n".to_vec(),
    ] {
        assert!(decode_exact(&malformed).is_err());
    }
}

#[test]
fn unknown_and_duplicate_fields_are_refused() {
    let bytes = fixture().encode().unwrap();
    let original = String::from_utf8(bytes.clone()).unwrap();
    let duplicate_top = original.replacen("\"revision\":7", "\"revision\":7,\"revision\":7", 1);
    let duplicate_nested = original.replacen("\"cause\":0", "\"cause\":0,\"cause\":0", 1);
    assert_ne!(duplicate_top, original);
    assert_ne!(duplicate_nested, original);
    assert!(decode_exact(duplicate_top.as_bytes()).is_err());
    assert!(decode_exact(duplicate_nested.as_bytes()).is_err());
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    let mut top_unknown = actual.clone();
    top_unknown["unknown"] = json!(0);
    let mut nested_unknown = actual.clone();
    nested_unknown["provider_report"]["addresses"][0]["unknown"] = json!(0);
    let mut variant_unknown = actual;
    variant_unknown["disposition"]["unknown"] = json!(0);
    for unknown in [top_unknown, nested_unknown, variant_unknown] {
        assert!(decode_exact(&line(&unknown)).is_err());
    }
}

#[test]
fn omitted_nullable_fields_are_refused() {
    let original: Value = serde_json::from_slice(&fixture().encode().unwrap()).unwrap();
    for field in ["provider_report", "association", "fault"] {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(field).unwrap();
        assert!(decode_exact(&line(&value)).is_err());
    }
    let mut missing_slice = original.clone();
    missing_slice["provider_report"]
        .as_object_mut()
        .unwrap()
        .remove("sliceInfo")
        .unwrap();
    assert!(decode_exact(&line(&missing_slice)).is_err());
    for field in ["dnn", "osAppId"] {
        let mut value = original.clone();
        value["provider_report"]["trafficDescriptors"][0]
            .as_object_mut()
            .unwrap()
            .remove(field)
            .unwrap();
        assert!(decode_exact(&line(&value)).is_err());
    }
}

#[test]
fn oversized_valid_json_is_refused() {
    let bytes = fixture().encode().unwrap();
    let expected: Value = serde_json::from_slice(&bytes).unwrap();
    let mut oversized = bytes[..bytes.len() - 1].to_vec();
    oversized.resize(MAX_RECORD_BYTES, b' ');
    oversized.push(b'\n');
    assert_eq!(oversized.len(), MAX_RECORD_BYTES + 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&oversized).unwrap(),
        expected
    );
    assert!(decode_exact(&oversized).is_err());
}

#[test]
fn final_escaped_record_bound_is_exact() {
    let mut frame = fixture();
    frame.association = None;
    frame.fault = Some(String::new());
    frame.provider_report.as_mut().unwrap().dnses = vec![String::new(); 4];
    let base = serde_json::to_vec(&frame).unwrap().len() + 1;
    let needed = MAX_RECORD_BYTES.checked_sub(base).unwrap();
    let mut escaped = needed / 6; // U+0001 has six encoded JSON bytes.
    assert!(escaped <= 4 * 128);
    for value in &mut frame.provider_report.as_mut().unwrap().dnses {
        let count = escaped.min(128);
        *value = "\u{1}".repeat(count);
        escaped -= count;
    }
    assert_eq!(escaped, 0);
    frame.fault = Some("x".repeat(needed % 6));
    let bytes = frame.encode().unwrap();
    assert_eq!(bytes.len(), MAX_RECORD_BYTES);
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        serde_json::to_value(&frame).unwrap()
    );
    frame.fault.as_mut().unwrap().push('x');
    assert_eq!(
        serde_json::to_vec(&frame).unwrap().len() + 1,
        MAX_RECORD_BYTES + 1
    );
    assert!(frame.encode().is_err());
}

#[test]
fn unsupported_subset_and_impossible_association_are_refused() {
    let mut frame = fixture();
    frame
        .provider_report
        .as_mut()
        .unwrap()
        .qos_sessions
        .push(());
    assert!(frame.encode().is_err());
    let mut frame = fixture();
    frame.provider_report.as_mut().unwrap().traffic_descriptors[0]
        .os_app_id
        .as_mut()
        .unwrap()
        .os_app_id
        .resize(257, 0);
    assert!(frame.encode().is_err());
    let mut frame = fixture();
    frame.association.as_mut().unwrap().observed_at_boottime_ns = frame.sampled_at_ns + 1;
    assert!(frame.encode().is_err());
    let mut frame = fixture();
    frame.sampled_at_ns = frame.until_ns;
    assert!(frame.encode().is_err());
    frame.association = None; // Terminal withdrawal must remain encodable after expiry.
    assert!(frame.encode().is_ok());
}
