//! Protected installation-profile boundary, not app IPC or user-consent evidence.
use crate::{Profile, fresh_id, service};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};

fn app(name: &str, uid: u32, role: Value) -> Value {
    json!({"app":name,"uid":uid,"executable":"/usr/bin/true","args":[],"role":role})
}
fn ordinary() -> Value {
    app("org.omarchy.Sounds", 11042, json!({"kind":"application"}))
}
fn controller(target: &str, permission: &str) -> Value {
    app(
        "org.omarchy.Permissions",
        11045,
        json!({"kind":"permission_controller","target":target,"permission":permission}),
    )
}
fn load(apps: Vec<Value>, mode: u32) -> std::io::Result<Profile> {
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let root = std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT").expect("isolated test root");
    let path =
        std::path::PathBuf::from(root).join(format!("permission-profile-{}.json", fresh_id()?));
    fs::write(&path, serde_json::to_vec(&json!({"apps":apps}))?)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
    let result = service::profile(&path);
    fs::remove_file(path)?;
    result
}

#[test]
fn permission_profile_accepts_explicit_ordinary_and_scoped_controller() {
    let p = load(
        vec![ordinary(), controller("org.omarchy.Sounds", "media_route")],
        0o600,
    )
    .expect("root-installed scoped controller must load");
    assert_eq!(p.apps.len(), 2);
    let value = serde_json::to_value(p).unwrap();
    assert_eq!(value["apps"][0]["role"], json!({"kind":"application"}));
    assert_eq!(
        value["apps"][1]["role"],
        json!({"kind":"permission_controller","target":"org.omarchy.Sounds","permission":"media_route"})
    );
}

#[test]
fn permission_profile_requires_explicit_role_without_legacy_fallback() {
    let mut value = ordinary();
    value.as_object_mut().unwrap().remove("role");
    assert!(load(vec![value], 0o600).is_err());
}

#[test]
fn permission_profile_rejects_missing_self_or_unsupported_target() {
    for (target, permission) in [
        ("org.omarchy.Missing", "media_route"),
        ("org.omarchy.Permissions", "media_route"),
        ("org.omarchy.Sounds", "capture"),
    ] {
        assert!(load(vec![ordinary(), controller(target, permission)], 0o600).is_err());
    }
    assert!(load(vec![controller("org.omarchy.Sounds", "media_route")], 0o600).is_err());
}

#[test]
fn permission_profile_retains_protected_file_and_executable_checks() {
    assert!(
        load(
            vec![ordinary(), controller("org.omarchy.Sounds", "media_route")],
            0o666
        )
        .is_err()
    );
    let mut value = controller("org.omarchy.Sounds", "media_route");
    value["executable"] = json!("/tmp/app-owned-controller");
    assert!(load(vec![ordinary(), value], 0o600).is_err());
}

#[test]
fn permission_profile_rejects_unknown_role_fields() {
    let mut value = controller("org.omarchy.Sounds", "media_route");
    value["role"]["general_admin"] = json!(true);
    assert!(load(vec![ordinary(), value], 0o600).is_err());
}
