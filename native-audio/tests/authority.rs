use pokelinux_audio::{Authority, Identity, Role};
const APP: Identity = Identity {
    uid: 1002,
    connection: 1,
};
const CALL: Identity = Identity {
    uid: 1001,
    connection: 2,
};

#[test]
fn authenticated_call_interrupts_media_then_restores_it() {
    let mut authority = Authority::new(1001, 1002, true);
    let media = authority
        .acquire(APP, Role::Media, "route.speaker")
        .unwrap();
    assert!(authority.acquire(APP, Role::Call, "route.speaker").is_err());
    let call = authority
        .acquire(CALL, Role::Call, "route.earpiece")
        .unwrap();
    assert_eq!(
        authority.desired(),
        vec![(call, Role::Call, "route.earpiece".into())]
    );
    assert!(authority.release(APP, call).is_err());
    authority.release(CALL, call).unwrap();
    assert_eq!(
        authority.desired(),
        vec![(media, Role::Media, "route.speaker".into())]
    );
}

#[test]
fn revocation_removes_capture_intent_and_denies_new_capture() {
    let mut authority = Authority::new(1001, 1002, true);
    let capture = authority
        .acquire(APP, Role::Capture, "route.capture")
        .unwrap();
    assert!(authority.revoke_capture(APP).is_err());
    authority.revoke_capture(CALL).unwrap();
    assert!(!authority.desired().iter().any(|(id, _, _)| *id == capture));
    assert!(
        authority
            .acquire(APP, Role::Capture, "route.capture")
            .is_err()
    );
}

#[test]
fn leases_belong_to_the_authenticated_connection_and_end_on_disconnect() {
    let mut authority = Authority::new(1001, 1002, true);
    let lease = authority
        .acquire(APP, Role::Media, "route.speaker")
        .unwrap();
    let another = Identity {
        uid: APP.uid,
        connection: 99,
    };
    assert!(authority.release(another, lease).is_err());
    authority.disconnect(APP);
    assert!(authority.desired().is_empty());
}

#[test]
fn service_uids_cannot_collapse_the_permission_boundary() {
    use pokelinux_audio::validate_service_uids;
    assert!(validate_service_uids(1000, 1001, 1002));
    assert!(!validate_service_uids(1000, 1002, 1002));
    assert!(!validate_service_uids(1000, 1001, 1000));
    assert!(!validate_service_uids(1000, 1000, 1002));
}

#[test]
fn application_lease_exhaustion_preserves_privileged_call_admission() {
    let mut authority = Authority::new(1001, 1002, true);
    let mut admitted = 0;
    while authority.acquire(APP, Role::Media, "route.speaker").is_ok() {
        admitted += 1;
        assert!(admitted <= 64);
    }
    assert!(admitted > 0);
    let call = authority
        .acquire(CALL, Role::Call, "route.earpiece")
        .unwrap();
    assert_eq!(
        authority.desired(),
        vec![(call, Role::Call, "route.earpiece".into())]
    );
}
