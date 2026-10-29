//! Actual lifecycle actor/desktop credentials/child custody. The fixed native
//! six-argument child is a fixture, with no UI, Source, consent or grant effect.
use super::settings_desktop_tests::Case;
use crate::Reply;
const REQUEST: &[u8] = br#"{"op":"launch_permissions"}"#;

#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn permissions_desktop_launch_duplicate_and_stop() {
    let mut c = Case::permissions(true, false, crate::permission::UID);
    assert!(!c.runtime.join("launch-messages.sock").exists());
    assert_eq!(
        c.request(1000, "launch-permissions.sock", REQUEST),
        Reply::Ok
    );
    let (pid, alive) = c.app(crate::permission::UID);
    assert_eq!(
        c.request(1000, "launch-permissions.sock", REQUEST),
        Reply::Denied
    );
    assert_eq!(c.app(crate::permission::UID).0, pid);
    c.close().unwrap();
    assert_eq!(c.reaped.len(), 1);
    assert_eq!(c.reaped[0].0, pid);
    assert!(!crate::authority::living(&alive).unwrap());
}

#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn permissions_desktop_wrong_uids_and_strict_packets_are_denied() {
    let c = Case::permissions(true, false, crate::permission::UID);
    for uid in [0, 1001, 11042, crate::permission::UID] {
        assert_eq!(
            c.request(uid, "launch-permissions.sock", REQUEST),
            Reply::Denied
        );
    }
    for packet in [
        br#"{"op":"launch_settings"}"#.as_slice(),
        br#"{"op":"launch_permissions","app":"org.omarchy.Sounds"}"#,
        br#"{"op":"launch_permissions","permission":"media_route"}"#,
        br#"{"op":"grant","app":"org.omarchy.Sounds","permission":"media_route","allowed":true}"#,
        b"{",
    ] {
        assert_eq!(
            c.request(1000, "launch-permissions.sock", packet),
            Reply::Denied
        );
    }
    assert_eq!(
        c.request(1000, "launch-settings.sock", REQUEST),
        Reply::Denied
    );
    assert_eq!(c.request(1000, "app.sock", REQUEST), Reply::Denied);
}

#[test]
#[ignore = "requires isolated root with built lifecycle helpers"]
fn permissions_desktop_requires_fixed_installed_controller_role_and_uid() {
    for (profile, ordinary, uid) in [
        (false, false, 11045),
        (true, true, 11045),
        (true, false, 11046),
    ] {
        let c = Case::permissions(profile, ordinary, uid);
        assert_eq!(
            c.request(1000, "launch-permissions.sock", REQUEST),
            Reply::Unavailable
        );
    }
}
