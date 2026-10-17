//! Strict packet policy with authored bytes and controlled BOOTTIME samples.
//! These cases do not open sockets, execute a service or prove worker custody.
use pokelinux_brightness::{
    model::{Observation, View},
    protocol::{Reply, Request, decimal},
};
const NOW: u64 = 10_000_000_000;
fn observed() -> Reply {
    serde_json::from_slice(br#"{"schema":1,"id":9,"ok":true,"revision":3,"pending":false,"observation":{"max":400,"requested":300,"actual":120},"until_ns":"11000000000","requested_percent":75,"last_set_uncertain":false}"#).unwrap()
}
#[test]
fn canonical_decimal_is_bounded_and_exact() {
    for (text, value) in [("0", 0), ("42", 42), ("18446744073709551615", u64::MAX)] {
        assert_eq!(decimal(text), Some(value));
    }
    for text in [
        "",
        "00",
        "01",
        "+1",
        "-1",
        "1 ",
        " 1",
        "1\n",
        "18446744073709551616",
        "999999999999999999999",
    ] {
        assert_eq!(decimal(text), None);
    }
}
#[test]
fn request_original_deadline_and_revision_are_validated() {
    let r: Request =
        serde_json::from_slice(br#"{"op":"inspect","id":1,"until_ns":"10250000000"}"#).unwrap();
    assert_eq!(r.until(NOW), Some(NOW + 250_000_000));
    assert_eq!(r.until(NOW + 250_000_000), None);
    for r in [
        Request::Inspect {
            id: 0,
            until_ns: (NOW + 1).to_string(),
        },
        Request::Inspect {
            id: 1,
            until_ns: NOW.to_string(),
        },
        Request::Inspect {
            id: 1,
            until_ns: (NOW + 1_000_000_001).to_string(),
        },
        Request::Inspect {
            id: 1,
            until_ns: "010250000000".into(),
        },
        Request::Set {
            id: 1,
            percent: 101,
            expected_revision: 1,
            until_ns: (NOW + 1).to_string(),
        },
        Request::Set {
            id: 1,
            percent: 75,
            expected_revision: 0,
            until_ns: (NOW + 1).to_string(),
        },
    ] {
        assert_eq!(r.until(NOW), None);
    }
    let r = Request::Set {
        id: 7,
        percent: 0,
        expected_revision: 1,
        until_ns: (NOW + 1_000_000_000).to_string(),
    };
    assert_eq!(r.id(), 7);
    assert_eq!(r.until(NOW), Some(NOW + 1_000_000_000));
}
#[test]
fn unknown_duplicate_missing_and_wrong_type_packet_fields_are_refused() {
    for bytes in [
        br#"{"op":"inspect","id":1,"until_ns":"10250000000","device":"panel0"}"#.as_slice(),
        br#"{"op":"inspect","id":1,"id":2,"until_ns":"10250000000"}"#,
        br#"{"op":"inspect","id":1,"until_ns":10250000000}"#,
        br#"{"op":"set","id":1,"percent":25,"until_ns":"10250000000"}"#,
    ] {
        assert!(serde_json::from_slice::<Request>(bytes).is_err());
    }
    let good = serde_json::to_value(observed()).unwrap();
    for field in [
        "observation",
        "requested_percent",
        "last_set_uncertain",
        "until_ns",
    ] {
        let mut value = good.clone();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<Reply>(value).is_err(),
            "accepted missing {field}"
        );
    }
    let mut missing_actual = good.clone();
    missing_actual["observation"]
        .as_object_mut()
        .unwrap()
        .remove("actual");
    assert!(serde_json::from_value::<Reply>(missing_actual).is_err());
    let mut extra = good.clone();
    extra["path"] = serde_json::json!("/tmp/panel");
    assert!(serde_json::from_value::<Reply>(extra).is_err());
    assert!(serde_json::from_slice::<Reply>(br#"{"schema":1,"id":9,"ok":false,"revision":3,"pending":false,"observation":null,"until_ns":"0","requested_percent":null,"last_set_uncertain":true,"id":10}"#).is_err());
}
#[test]
fn independent_attribute_readback_expires_and_malformed_positive_is_refused() {
    let positive = observed();
    assert!(positive.valid(NOW));
    assert_eq!(positive.observation.unwrap().actual, Some(120));
    assert!(!positive.valid(NOW + 1_000_000_000));
    for mutate in 0..9 {
        let mut r = positive.clone();
        match mutate {
            0 => r.schema = 2,
            1 => r.id = 0,
            2 => r.revision = 0,
            3 => r.pending = true,
            4 => r.until_ns = "011000000000".into(),
            5 => r.until_ns = (NOW + 2_000_000_001).to_string(),
            6 => r.observation.as_mut().unwrap().max = 0,
            7 => r.observation.as_mut().unwrap().actual = Some(401),
            _ => r.last_set_uncertain = true,
        }
        assert!(!r.valid(NOW), "accepted malformed positive {mutate}");
    }
}
#[test]
fn unavailable_keeps_real_pending_without_positive_deadline_or_ack() {
    let mut r = Reply {
        schema: 1,
        id: 1,
        ok: false,
        revision: 4,
        pending: true,
        observation: None,
        until_ns: "0".into(),
        requested_percent: None,
        last_set_uncertain: true,
    };
    assert!(r.valid(NOW));
    assert!(r.pending);
    r.pending = false;
    assert!(r.valid(NOW));
    r.until_ns = (NOW + 1).to_string();
    assert!(!r.valid(NOW));
    r.until_ns = "0".into();
    r.requested_percent = Some(75);
    assert!(!r.valid(NOW));
    r.requested_percent = None;
    r.ok = true;
    assert!(!r.valid(NOW));
}
#[test]
fn exact_view_projection_preserves_requested_and_optional_actual_separately() {
    let view = View {
        revision: 8,
        observation: Some(Observation {
            max: 400,
            requested: 300,
            actual: None,
        }),
        until_ns: NOW + 2_000_000_000,
        pending: false,
        requested_percent: None,
        last_set_uncertain: false,
    };
    let reply = Reply::from_view(5, true, view, Some(75));
    assert!(reply.valid(NOW));
    let raw = serde_json::to_vec(&reply).unwrap();
    assert!(raw.len() <= pokelinux_brightness::protocol::LIMIT);
    let parsed: Reply = serde_json::from_slice(&raw).unwrap();
    assert_eq!(parsed, reply);
    assert_eq!(parsed.requested_percent, Some(75));
    assert_eq!(parsed.observation.unwrap().actual, None);
    let denied = Reply::from_view(5, false, view, Some(75));
    assert_eq!(denied.requested_percent, None);
    assert!(denied.valid(NOW));
}
