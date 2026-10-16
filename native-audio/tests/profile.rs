use pokelinux_audio::pipewire::Profile;
use serde_json::{Value, json};

fn route() -> Value {
    json!({"id":"media-a7","label":"Speaker","target":"private.output.21","roles":["media"]})
}
fn parse(routes: Value) -> Result<Profile, serde_json::Error> {
    serde_json::from_value(json!({"sources":{"media":"private.input.9"},"routes":routes}))
}

#[test]
fn duplicate_selectors_cannot_choose_between_private_targets() {
    let first = route();
    let mut second = route();
    second["target"] = json!("different.private.output");
    assert!(parse(json!([first, second])).is_err());
}

#[test]
fn selectors_are_bounded_logical_ids() {
    for invalid in [
        "".to_string(),
        "x".repeat(65),
        "node:port".into(),
        "/dev/audio".into(),
        "two words".into(),
        "line\nbreak".into(),
    ] {
        let mut item = route();
        item["id"] = json!(invalid);
        assert!(parse(json!([item])).is_err());
    }
    let mut item = route();
    item["id"] = json!("A".repeat(64));
    assert!(parse(json!([item])).is_ok());
}

#[test]
fn display_labels_have_utf8_byte_and_control_bounds() {
    for invalid in [
        "".to_string(),
        "   ".into(),
        "x".repeat(97),
        "é".repeat(49),
        "hidden\u{0}text".into(),
    ] {
        let mut item = route();
        item["label"] = json!(invalid);
        assert!(parse(json!([item])).is_err());
    }
    let mut item = route();
    item["label"] = json!("é".repeat(48));
    assert!(parse(json!([item])).is_ok());
}

#[test]
fn configured_route_count_is_bounded() {
    assert!(parse(json!([])).is_err());
    let items: Vec<_> = (0..32)
        .map(|i| {
            let mut item = route();
            item["id"] = json!(format!("choice-{i}"));
            item
        })
        .collect();
    assert!(parse(json!(items)).is_ok());
    let mut excessive = items;
    let mut last = route();
    last["id"] = json!("choice-32");
    excessive.push(last);
    assert!(parse(json!(excessive)).is_err());
}

#[test]
fn route_roles_are_nonempty_unique_and_known() {
    for roles in [
        json!([]),
        json!(["media", "media"]),
        json!(["arbitrary"]),
        json!(["media", "call", "capture", "media"]),
    ] {
        let mut item = route();
        item["roles"] = roles;
        assert!(parse(json!([item])).is_err());
    }
}

#[test]
fn private_target_has_finite_text_bounds() {
    for invalid in ["".to_string(), "x".repeat(257), "node\u{0}name".into()] {
        let mut item = route();
        item["target"] = json!(invalid);
        assert!(parse(json!([item])).is_err());
    }
}

#[test]
fn old_node_keyed_schema_is_refused() {
    assert!(parse(json!({"private.output.21":["media"]})).is_err());
}
