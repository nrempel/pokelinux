//! Current-authority components with real pidfds and synced policy files.
//! App PID/UID and lifecycle facts are injected here, not kernel IPC evidence.
use super::*;
use crate::{boottime_ns, storage::Disk};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
const APP: &str = "org.omarchy.Sounds";
const UID: u32 = 11042;
const BACKEND: &str = "media-connection-one";
struct Case {
    authority: Authority,
    disk: Disk,
    path: PathBuf,
}
impl Case {
    fn new(granted: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let path = PathBuf::from(
            std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").expect("isolated test root"),
        )
        .join(format!("media-{}", fresh_id().unwrap()));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let (disk, durable) = Disk::open(&path, &[APP.into()]).unwrap();
        let mut authority = Authority::loading(vec![AppProfile {
            app: APP.into(),
            uid: UID,
            executable: "/unused".into(),
            args: vec![],
        }])
        .unwrap();
        authority.activate(durable);
        authority
            .register(
                APP,
                std::process::id(),
                rustix::process::pidfd_open(
                    rustix::process::getpid(),
                    rustix::process::PidfdFlags::NONBLOCK,
                )
                .unwrap(),
            )
            .unwrap();
        authority.lifecycle(Some(false), None, None).unwrap();
        let mut case = Self {
            authority,
            disk,
            path,
        };
        if granted {
            case.grant(Permission::MediaRoute, true);
        }
        case
    }
    fn grant(&mut self, permission: Permission, allowed: bool) {
        let now = boottime_ns();
        let intent = self
            .authority
            .plan_grant(APP, permission, allowed, now)
            .unwrap();
        self.disk.replace(&intent.next).unwrap();
        assert_eq!(
            self.authority.finish(intent, true, boottime_ns()),
            Reply::Ok
        );
    }
    fn begin(&mut self) -> MediaStamp {
        self.authority
            .begin_media(
                std::process::id(),
                UID,
                BACKEND,
                "speaker",
                1000,
                boottime_ns(),
            )
            .unwrap_or_else(|r| panic!("current granted media route was refused: {r:?}"))
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}

#[test]
fn media_requires_grant_exact_principal_and_bounded_request() {
    let mut c = Case::new(false);
    assert!(
        c.authority
            .begin_media(
                std::process::id(),
                UID,
                BACKEND,
                "speaker",
                1000,
                boottime_ns()
            )
            .is_err()
    );
    c.grant(Permission::MediaRoute, true);
    for (pid, uid, route, ttl) in [
        (std::process::id() + 1, UID, "speaker", 1000),
        (std::process::id(), UID + 1, "speaker", 1000),
        (std::process::id(), UID, "private/source", 1000),
        (std::process::id(), UID, "", 1000),
        (std::process::id(), UID, "speaker", 0),
        (std::process::id(), UID, "speaker", 1001),
    ] {
        assert!(
            c.authority
                .begin_media(pid, uid, BACKEND, route, ttl, boottime_ns())
                .is_err()
        );
    }
}
#[test]
fn media_admits_current_route_and_keeps_original_deadline() {
    let mut c = Case::new(true);
    let before = boottime_ns();
    let stamp = c.begin();
    assert_eq!(stamp.intent().route, "speaker");
    assert!(stamp.intent().id > 0);
    assert!(
        (before + 1_000_000_000..=boottime_ns() + 1_000_000_000).contains(&stamp.intent().until_ns)
    );
    assert!(c.authority.current_media(&stamp, BACKEND, boottime_ns()));
    assert!(
        !c.authority
            .current_media(&stamp, BACKEND, stamp.intent().until_ns)
    );
    assert!(
        c.authority
            .approve_media(&stamp, BACKEND, stamp.intent().until_ns)
            .is_none()
    );
    assert!(
        !c.authority
            .current_media(&stamp, "successor", boottime_ns())
    );
}
#[test]
fn media_pending_focus_fences_but_approved_lease_continues() {
    let mut c = Case::new(true);
    let stamp = c.begin();
    let lease = c
        .authority
        .approve_media(&stamp, BACKEND, boottime_ns())
        .expect("current route approval");
    c.authority
        .lifecycle(None, None, Some((APP, true)))
        .unwrap();
    assert!(!c.authority.current_media(&stamp, BACKEND, boottime_ns()));
    assert!(c.authority.current_media_lease(&lease, BACKEND));
    c.authority.lifecycle(Some(true), Some(true), None).unwrap();
    assert!(c.authority.current_media_lease(&lease, BACKEND));
    assert!(!c.authority.current_media_lease(&lease, "successor"));
    c.authority
        .lifecycle(Some(false), Some(false), Some((APP, false)))
        .unwrap();
    assert!(!c.authority.current_media(&stamp, BACKEND, boottime_ns()));
}
#[test]
fn media_revoke_fences_before_storage_and_regrant_never_revives_lease() {
    let mut c = Case::new(true);
    let stamp = c.begin();
    let lease = c
        .authority
        .approve_media(&stamp, BACKEND, boottime_ns())
        .unwrap();
    let intent = c
        .authority
        .plan_grant(APP, Permission::MediaRoute, false, boottime_ns())
        .unwrap();
    assert!(!c.authority.current_media(&stamp, BACKEND, boottime_ns()));
    assert!(
        !c.authority.current_media_lease(&lease, BACKEND),
        "old route survived in-memory revocation while storage has not committed"
    );
    c.disk.replace(&intent.next).unwrap();
    assert_eq!(c.authority.finish(intent, true, boottime_ns()), Reply::Ok);
    c.grant(Permission::MediaRoute, true);
    assert!(!c.authority.current_media_lease(&lease, BACKEND));
    let fresh = c.begin();
    assert!(c.authority.current_media(&fresh, BACKEND, boottime_ns()));
}
#[test]
fn media_unrelated_grant_preserves_approved_lease_and_replacement_cancels_it() {
    let mut c = Case::new(true);
    let stamp = c.begin();
    let lease = c
        .authority
        .approve_media(&stamp, BACKEND, boottime_ns())
        .unwrap();
    c.grant(Permission::Capture, true);
    assert!(!c.authority.current_media(&stamp, BACKEND, boottime_ns()));
    assert!(c.authority.current_media_lease(&lease, BACKEND));
    c.authority
        .register(
            APP,
            std::process::id(),
            rustix::process::pidfd_open(
                rustix::process::getpid(),
                rustix::process::PidfdFlags::NONBLOCK,
            )
            .unwrap(),
        )
        .unwrap();
    assert!(!c.authority.current_media_lease(&lease, BACKEND));
}
#[test]
fn media_invalidation_revokes_approved_lease() {
    let mut c = Case::new(true);
    let stamp = c.begin();
    let lease = c
        .authority
        .approve_media(&stamp, BACKEND, boottime_ns())
        .unwrap();
    c.authority.invalidate();
    assert!(!c.authority.current_media_lease(&lease, BACKEND));
}

#[test]
fn media_generic_historical_permit_is_not_route_admission() {
    let mut c = Case::new(true);
    assert!(
        matches!(
            c.authority.plan_request(
                std::process::id(),
                UID,
                AppRequest::Prepare {
                    permission: Permission::MediaRoute,
                    ttl_ms: 1000
                },
                boottime_ns()
            ),
            Decision::Immediate(Reply::Denied)
        ),
        "generic historical permit accepted media effect permission"
    );
}

#[test]
fn preview_preserves_the_app_absolute_start_deadline_after_transport_delay() {
    let mut c = Case::new(true);
    let original = boottime_ns() + 80_000_000;
    let (stamp, level) = c
        .authority
        .begin_media_preview(
            std::process::id(),
            UID,
            BACKEND,
            &crate::audio::PreviewRequest {
                route: "speaker".into(),
                ttl_ms: 1000,
                until_boottime_ns: original,
                level_percent: 50,
                muted: false,
            },
            boottime_ns(),
        )
        .unwrap();
    assert_eq!(level, crate::audio::PreviewLevel::new(50, false).unwrap());
    assert!(stamp.intent().until_ns <= original);
    while boottime_ns() < original {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!c.authority.current_media(&stamp, BACKEND, boottime_ns()));
    assert!(
        c.authority
            .approve_media(&stamp, BACKEND, boottime_ns())
            .is_none()
    );
}

#[test]
fn preview_rejects_expired_or_unbounded_absolute_deadlines() {
    let mut c = Case::new(true);
    for absolute in [0, boottime_ns(), boottime_ns() + 2_000_000_000, u64::MAX] {
        assert!(
            c.authority
                .begin_media_preview(
                    std::process::id(),
                    UID,
                    BACKEND,
                    &crate::audio::PreviewRequest {
                        route: "speaker".into(),
                        ttl_ms: 1000,
                        until_boottime_ns: absolute,
                        level_percent: 100,
                        muted: false,
                    },
                    boottime_ns(),
                )
                .is_err()
        );
    }
}

#[test]
fn preview_level_bounds_are_checked_before_authority_admission() {
    let mut c = Case::new(true);
    for percent in [101, u64::MAX] {
        assert!(
            c.authority
                .begin_media_preview(
                    std::process::id(),
                    UID,
                    BACKEND,
                    &crate::audio::PreviewRequest {
                        route: "speaker".into(),
                        ttl_ms: 1000,
                        until_boottime_ns: boottime_ns() + 900_000_000,
                        level_percent: percent,
                        muted: false,
                    },
                    boottime_ns(),
                )
                .is_err()
        );
    }
}

#[test]
fn preview_wire_requires_explicit_integer_level_and_boolean_mute() {
    use serde_json::json;
    let valid = json!({"op":"media_preview_play","request":{
        "route":"speaker","ttl_ms":1000,"until_boottime_ns":123,
        "level_percent":50,"muted":false}});
    assert!(serde_json::from_value::<crate::AppRequest>(valid.clone()).is_ok());
    for field in ["level_percent", "muted"] {
        let mut missing = valid.clone();
        missing["request"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<crate::AppRequest>(missing).is_err());
    }
    for value in [
        json!(null),
        json!(-1),
        json!(50.5),
        json!("50"),
        json!(true),
    ] {
        let mut malformed = valid.clone();
        malformed["request"]["level_percent"] = value;
        assert!(serde_json::from_value::<crate::AppRequest>(malformed).is_err());
    }
    for value in [json!(null), json!(0), json!("false"), json!([])] {
        let mut malformed = valid.clone();
        malformed["request"]["muted"] = value;
        assert!(serde_json::from_value::<crate::AppRequest>(malformed).is_err());
    }
    let mut extra = valid;
    extra["request"]["gain"] = json!(0.5);
    assert!(serde_json::from_value::<crate::AppRequest>(extra).is_err());
}
