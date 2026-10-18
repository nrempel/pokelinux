//! Controlled graph policy cases; these do not claim process wait or PCM.
use pokelinux_audio::Role;
use pokelinux_audio::pipewire::{Graph, MediaSource, Profile};
use serde_json::{Value, json};
fn profile() -> Profile {
    serde_json::from_value(json!({"sources":{"media":"source.media"},"routes":[{"id":"speaker","label":"Speaker","target":"route.speaker","roles":["media"]}]})).unwrap()
}
fn objects(source: bool) -> Value {
    let mut rows = vec![
        json!({"type":"PipeWire:Interface:Core","info":{"cookie":91}}),
        json!({"id":2,"type":"PipeWire:Interface:Node","info":{"props":{"node.name":"route.speaker"}}}),
        json!({"id":3,"type":"PipeWire:Interface:Port","info":{"props":{"node.id":2,"port.direction":"in","port.name":"input_MONO"}}}),
    ];
    if source {
        rows.extend([json!({"id":4,"type":"PipeWire:Interface:Client","info":{"props":{"pipewire.sec.pid":70,"pipewire.sec.uid":11040,"pipewire.sec.gid":11040,"object.serial":40}}}),json!({"id":5,"type":"PipeWire:Interface:Node","info":{"props":{"node.name":"source.media","client.id":4,"object.serial":50},"params":{"EnumFormat":[{"mediaType":"audio","mediaSubtype":"raw","format":"F32LE","rate":48000,"channels":1,"position":["MONO"]}]}}}),json!({"id":6,"type":"PipeWire:Interface:Port","info":{"props":{"node.id":5,"port.direction":"out","port.name":"output_MONO"}}})]);
    }
    json!(rows)
}
#[test]
fn idle_selection_is_settled_policy_not_an_actual_render() {
    let p = profile();
    let desired = vec![(1, Role::Media, "speaker".into())];
    let idle = Graph::from_value(objects(false))
        .unwrap()
        .render_plan(&p, &desired, &MediaSource::Automatic)
        .unwrap();
    assert!(idle.wanted.is_empty());
    assert!(idle.idle_media);
    assert!(!idle.media_present);
    let external = Graph::from_value(objects(true))
        .unwrap()
        .render_plan(&p, &desired, &MediaSource::Automatic)
        .unwrap();
    assert_eq!(external.wanted.len(), 1);
    assert!(!external.idle_media);
    assert!(external.media_present);
}
#[test]
fn owned_source_requires_server_credentials_cookie_and_format() {
    let p = profile();
    let d = vec![(1, Role::Media, "speaker".into())];
    let own = MediaSource::Owned {
        pid: 70,
        uid: 11040,
        gid: 11040,
        cookie: 91,
        identity: None,
    };
    let g = Graph::from_value(objects(true)).unwrap();
    let plan = g.render_plan(&p, &d, &own).unwrap();
    assert!(plan.preview_identity.is_some());
    for (key, value) in [
        ("pipewire.sec.pid", json!(71)),
        ("pipewire.sec.uid", json!(0)),
        ("pipewire.sec.gid", json!(0)),
    ] {
        let mut raw = objects(true);
        raw[3]["info"]["props"][key] = value;
        assert!(
            Graph::from_value(raw)
                .unwrap()
                .render_plan(&p, &d, &own)
                .is_err()
        );
    }
    let mut raw = objects(true);
    raw[4]["info"]["params"]["EnumFormat"][0]["rate"] = json!(44100);
    assert!(
        Graph::from_value(raw)
            .unwrap()
            .render_plan(&p, &d, &own)
            .is_err()
    );
    assert!(
        g.render_plan(
            &p,
            &d,
            &MediaSource::Owned {
                pid: 70,
                uid: 11040,
                gid: 11040,
                cookie: 92,
                identity: None
            }
        )
        .is_err()
    );
}
#[test]
fn retiring_source_is_never_reclassified_as_external() {
    let p = profile();
    let d = vec![(1, Role::Media, "speaker".into())];
    let g = Graph::from_value(objects(true)).unwrap();
    let retired = g.render_plan(&p, &d, &MediaSource::Retiring).unwrap();
    assert!(retired.media_present);
    assert!(retired.wanted.is_empty());
    assert!(retired.preview_identity.is_none());
    let mut raw = objects(false);
    raw[2]["info"]["props"]["port.direction"] = json!("out");
    assert!(
        Graph::from_value(raw)
            .unwrap()
            .render_plan(&p, &d, &MediaSource::Automatic)
            .is_err()
    );
}
