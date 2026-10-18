//! Actor authority and real app-socket cancellation components only.
//! No PipeWire, delivered PCM, Qt, ARM or physical-audio qualification.
use super::*;

const GENERATION: u64 = 37;
fn playing() -> Ready {
    let mut c = Ready::new();
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    c.provider.observed = audio::State {
        connected: true,
        available: true,
        requested: Some("speaker".into()),
        applied: Some("speaker".into()),
        policy_settled: true,
        preview: audio::PreviewState::Playing,
        preview_generation: Some(GENERATION),
        preview_muted: Some(false),
        ..Default::default()
    };
    c
}
fn request(muted: bool) -> audio::PreviewMuteRequest {
    audio::PreviewMuteRequest {
        generation: GENERATION,
        muted,
        ttl_ms: 1000,
        until_boottime_ns: boottime_ns() + 900_000_000,
    }
}
fn begin(c: &mut Ready, request: audio::PreviewMuteRequest) -> Result<u64, Reply> {
    c.media.begin_preview_mute(
        CONNECTION,
        std::process::id(),
        11042,
        request,
        &mut c.authority,
        &c.provider,
    )
}
fn dispatch(c: &mut Ready, muted: bool) -> audio::MuteIntent {
    begin(c, request(muted)).unwrap();
    c.check().unwrap();
    assert!(
        c.media
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    c.provider.mute_intent.clone().expect("one explicit mute")
}
fn acknowledged(c: &mut Ready, muted: bool) -> Reply {
    let intent = dispatch(c, muted);
    c.provider.observed.preview_muted = Some(muted);
    let (connection, reply) = c
        .media
        .observe(
            audio::Event::Muted {
                id: intent.id,
                state: c.provider.state(),
            },
            &c.authority,
            &mut c.provider,
        )
        .unwrap();
    assert_eq!(connection, CONNECTION);
    reply
}

#[test]
fn live_mute_request_is_strict_and_has_no_route_or_sample_input() {
    let good = serde_json::json!({"op":"media_preview_mute","request":{
        "generation":GENERATION,"muted":true,"ttl_ms":1000,"until_boottime_ns":123
    }});
    assert!(matches!(
        serde_json::from_value::<crate::AppRequest>(good.clone()),
        Ok(crate::AppRequest::MediaPreviewMute { .. })
    ));
    for extra in ["route", "level_percent", "pid", "source"] {
        let mut value = good.clone();
        value["request"][extra] = serde_json::json!("forbidden");
        assert!(serde_json::from_value::<crate::AppRequest>(value).is_err());
    }
    for missing in ["generation", "muted", "ttl_ms", "until_boottime_ns"] {
        let mut value = good.clone();
        value["request"].as_object_mut().unwrap().remove(missing);
        assert!(serde_json::from_value::<crate::AppRequest>(value).is_err());
    }
}

#[test]
fn live_mute_dispatch_freezes_generation_bool_and_original_deadline_without_replay() {
    for muted in [true, false] {
        let mut c = playing();
        c.provider.observed.preview_muted = Some(!muted);
        let request = request(muted);
        let original = request.until_boottime_ns;
        assert_eq!(begin(&mut c, request), Ok(original));
        assert!(
            c.media
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        let intent = c.provider.mute_intent.as_ref().unwrap();
        assert_eq!(intent.generation, GENERATION);
        assert_eq!(intent.muted, muted);
        assert_eq!(intent.until_ns, original);
        assert!(intent.id > 0);
        assert_eq!(c.provider.mute_calls, 1);
        assert!(
            c.media
                .drive(CONNECTION, &c.authority, &mut c.provider)
                .is_none()
        );
        assert_eq!(c.provider.mute_calls, 1);
        assert!(c.provider.played.is_none() && c.provider.stopped.is_none());
    }
}

#[test]
fn live_mute_needs_exact_approved_owner_current_generation_and_observed_route() {
    let mut c = playing();
    assert_eq!(
        c.media.begin_preview_mute(
            CONNECTION + 1,
            std::process::id(),
            11042,
            request(true),
            &mut c.authority,
            &c.provider,
        ),
        Err(Reply::Denied)
    );
    for generation in [0, GENERATION + 1] {
        let mut r = request(true);
        r.generation = generation;
        assert_eq!(begin(&mut c, r), Err(Reply::Denied));
    }
    c.provider.observed.applied = None;
    assert_eq!(begin(&mut c, request(true)), Err(Reply::Denied));
    c.provider.observed.applied = Some("speaker".into());
    for phase in [
        audio::PreviewState::Idle,
        audio::PreviewState::Starting,
        audio::PreviewState::Retiring,
        audio::PreviewState::Completed,
    ] {
        c.provider.observed.preview = phase;
        assert_eq!(begin(&mut c, request(true)), Err(Reply::Denied));
    }
    let mut unapproved = Ready::new();
    assert_eq!(begin(&mut unapproved, request(true)), Err(Reply::Denied));
    assert_eq!(c.provider.mute_calls + unapproved.provider.mute_calls, 0);
}

#[test]
fn live_mute_and_unmute_need_fresh_foreground_authority_but_stop_remains_cleanup() {
    for muted in [false, true] {
        for locked in [false, true] {
            let mut c = playing();
            if locked {
                c.authority.lifecycle(Some(true), None, None).unwrap();
            } else {
                c.authority
                    .lifecycle(None, None, Some((audio::APP, true)))
                    .unwrap();
            }
            assert!(begin(&mut c, request(muted)).is_err());
            assert!(c.media.owns(CONNECTION));
            assert_eq!(c.provider.mute_calls, 0);
            c.media
                .stop_preview(CONNECTION)
                .expect("existing background cleanup");
        }
    }
}

#[test]
fn live_mute_unsent_generation_change_keeps_prior_approval_and_never_retargets() {
    let mut c = playing();
    begin(&mut c, request(true)).unwrap();
    c.provider.observed.preview_generation = Some(GENERATION + 1);
    assert_eq!(
        c.media.drive(CONNECTION, &c.authority, &mut c.provider),
        Some(Reply::Denied)
    );
    assert!(c.media.owns(CONNECTION) && !c.media.pending());
    assert_eq!(c.provider.revoked, 0);
    assert_eq!(c.provider.mute_calls, 0);
    assert!(
        c.media
            .drive(CONNECTION, &c.authority, &mut c.provider)
            .is_none()
    );
    assert_eq!(c.provider.mute_calls, 0);
    c.media.stop_preview(CONNECTION).unwrap();
}

#[test]
fn live_mute_sent_generation_change_retires_before_late_reply() {
    let mut c = playing();
    let intent = dispatch(&mut c, true);
    c.provider.observed.preview_generation = Some(GENERATION + 1);
    assert_eq!(
        c.media
            .fence(&c.authority, &mut c.provider, false, |_| true),
        Some((CONNECTION, Reply::Denied))
    );
    assert!(!c.media.owns(CONNECTION));
    assert_eq!(c.provider.revoked, 1);
    assert!(
        c.media
            .observe(
                audio::Event::Muted {
                    id: intent.id,
                    state: c.provider.state()
                },
                &c.authority,
                &mut c.provider
            )
            .is_none()
    );
    assert_eq!(c.provider.mute_calls, 1);
}

#[test]
fn live_mute_ack_requires_matching_generation_value_and_playing_state() {
    for wrong in 0..3 {
        let mut c = playing();
        let intent = dispatch(&mut c, true);
        let mut state = c.provider.state();
        state.preview_muted = Some(true);
        match wrong {
            0 => state.preview_generation = Some(GENERATION + 1),
            1 => state.preview_muted = Some(false),
            _ => state.preview = audio::PreviewState::Retiring,
        }
        assert_eq!(
            c.media.observe(
                audio::Event::Muted {
                    id: intent.id,
                    state
                },
                &c.authority,
                &mut c.provider
            ),
            Some((CONNECTION, Reply::Unavailable))
        );
        assert!(!c.media.owns(CONNECTION));
        assert_eq!(c.provider.revoked, 1);
    }
}

#[test]
fn live_mute_positive_publication_rechecks_generation_and_stop() {
    for stop in [false, true] {
        let mut c = playing();
        let reply = acknowledged(&mut c, true);
        assert!(matches!(reply, Reply::MediaPreview { state }
            if state.preview_generation == Some(GENERATION) && state.preview_muted == Some(true)));
        if !stop {
            c.provider.observed.preview_generation = Some(GENERATION + 1);
        }
        assert_eq!(
            c.media
                .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, stop),
            Some(Reply::Denied)
        );
        assert!(!c.media.owns(CONNECTION));
        assert_eq!(c.provider.revoked, 1);
    }
}

#[test]
fn live_mute_publication_keeps_existing_approved_lease_without_another_play() {
    let mut c = playing();
    let reply = acknowledged(&mut c, true);
    assert!(
        c.media
            .revalidate_reply(CONNECTION, &c.authority, &mut c.provider, false)
            .is_none()
    );
    ipc::send(&c.server, &serde_json::to_vec(&reply).unwrap()).unwrap();
    c.media.published(CONNECTION, &c.authority, &mut c.provider);
    assert!(c.media.owns(CONNECTION) && !c.media.pending());
    c.authority
        .lifecycle(None, None, Some((audio::APP, true)))
        .unwrap();
    assert!(
        c.media
            .fence(&c.authority, &mut c.provider, false, |_| true)
            .is_none()
    );
    assert_eq!(c.provider.mute_calls, 1);
    assert!(c.provider.played.is_none());
    c.media.stop_preview(CONNECTION).unwrap();
}

#[test]
fn live_mute_real_request_eof_withdraws_before_dispatch() {
    let mut c = playing();
    begin(&mut c, request(true)).unwrap();
    net::shutdown(&c.client, Shutdown::Write).unwrap();
    assert_eq!(c.check().unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(c.provider.mute_calls, 0);
    c.assert_retired();
}

#[test]
fn live_mute_backend_replacement_and_revocation_withdraw_ownership() {
    for revoke in [false, true] {
        let mut c = playing();
        begin(&mut c, request(true)).unwrap();
        if revoke {
            c.authority
                .plan_grant(audio::APP, Permission::MediaRoute, false, boottime_ns())
                .unwrap();
        } else {
            c.provider.incarnation = "media-successor";
        }
        assert_eq!(
            c.media.drive(CONNECTION, &c.authority, &mut c.provider),
            Some(Reply::Denied)
        );
        assert!(!c.media.owns(CONNECTION));
        assert_eq!(c.provider.revoked, 1);
        assert_eq!(c.provider.mute_calls, 0);
    }
}

#[test]
fn live_mute_state_is_hidden_from_other_app_connections() {
    let c = playing();
    let state = c.media.state(CONNECTION + 1, &c.provider);
    assert_eq!(state.preview, audio::PreviewState::Idle);
    assert_eq!(state.preview_generation, None);
    assert_eq!(state.preview_muted, None);
}
