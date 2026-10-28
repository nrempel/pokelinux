use pokelinux_session_owner::config::Config;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};
static NEXT: AtomicU32 = AtomicU32::new(0);
struct Case {
    root: PathBuf,
    path: PathBuf,
    data: Value,
}
impl Case {
    fn new() -> Self {
        assert_eq!(
            rustix::process::getuid().as_raw(),
            0,
            "isolated root required"
        );
        let parent = PathBuf::from(
            std::env::var_os("POKELINUX_SESSION_CONFIG_TEST_ROOT").expect("protected test parent"),
        );
        let root = parent.join(format!(
            "case-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        for name in [
            "runtime",
            "policy",
            "display",
            "desktop",
            "home",
            "app-home",
            "app-run",
            "appearance",
        ] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::set_permissions(root.join("policy"), fs::Permissions::from_mode(0o700)).unwrap();
        for (name, uid) in [
            ("desktop", 1000),
            ("home", 1000),
            ("app-home", 11042),
            ("app-run", 11042),
        ] {
            rustix::fs::chown(
                root.join(name),
                Some(rustix::process::Uid::from_raw(uid)),
                Some(rustix::process::Gid::from_raw(uid)),
            )
            .unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        for name in ["compositor", "entry", "sounds"] {
            fs::write(root.join(name), b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(
            root.join("hyprland.conf"),
            b"# Root-installed test configuration\n",
        )
        .unwrap();
        let args = json!([
            root.join("app-home"),
            root.join("app-run"),
            root.join("desktop/wayland-1"),
            root.join("appearance.sock"),
            root.join("runtime/app.sock"),
            root.join("appearance")
        ]);
        fs::write(root.join("apps.json"),serde_json::to_vec(&json!({"apps":[{"app":"org.omarchy.Sounds","uid":11042,"executable":root.join("sounds"),"args":args}]})).unwrap()).unwrap();
        let data = json!({"runtime":root.join("runtime"),"policy":root.join("policy"),"profiles":root.join("apps.json"),"app_entry":root.join("entry"),"display_parent":root.join("display"),"wayland":root.join("desktop/wayland-1"),"audio_socket":root.join("audio.sock"),"appearance_socket":root.join("appearance.sock"),"messages":{"mode":"disabled"},"brightness":{"mode":"disabled"},"connectivity":{"mode":"disabled"},"compositor_executable":root.join("compositor"),"compositor_config":root.join("hyprland.conf"),"environment":{"HOME":root.join("home"),"XDG_RUNTIME_DIR":root.join("desktop"),"PATH":"/usr/bin"}});
        let path = root.join("session.json");
        let c = Self { root, path, data };
        c.write();
        c
    }
    fn write(&self) {
        fs::write(&self.path, serde_json::to_vec(&self.data).unwrap()).unwrap();
    }
    fn profile(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("apps.json")).unwrap()).unwrap()
    }
    fn set_profile(&self, p: &Value) {
        fs::write(self.root.join("apps.json"), serde_json::to_vec(p).unwrap()).unwrap();
    }
    fn denied(&self) {
        assert!(Config::load(&self.path).is_err());
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        // Restore fixture directory ownership after assertions. No DAC override
        // is needed, and fixture symlinks are not followed during this pass.
        for entry in fs::read_dir(&self.root).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                rustix::fs::chown(
                    entry.path(),
                    Some(rustix::process::Uid::from_raw(0)),
                    Some(rustix::process::Gid::from_raw(0)),
                )
                .unwrap();
            }
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn valid_installed_sounds_configuration_is_selected() {
    let c = Case::new();
    let value = Config::load(&c.path).expect("valid fixed session composition must be selected");
    assert_eq!(value.runtime, c.root.join("runtime"));
    assert_eq!(value.wayland, c.root.join("desktop/wayland-1"));
    assert_eq!(value.environment["PATH"], "/usr/bin");
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn unknown_permission_or_raw_descriptor_fields_are_refused() {
    for field in ["grant", "unlock", "source_fd"] {
        let mut c = Case::new();
        c.data[field] = json!(true);
        c.write();
        c.denied();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn wrong_app_uid_and_argument_shape_are_refused() {
    for mutation in [0, 1, 2, 3] {
        let c = Case::new();
        let mut p = c.profile();
        match mutation {
            0 => p["apps"][0]["app"] = json!("org.omarchy.Messages"),
            1 => p["apps"][0]["uid"] = json!(1000),
            2 => {
                p["apps"][0]["args"].as_array_mut().unwrap().pop();
            }
            _ => {
                let extra = p["apps"][0].clone();
                p["apps"].as_array_mut().unwrap().push(extra);
            }
        };
        c.set_profile(&p);
        c.denied();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn actor_endpoint_wayland_and_private_uid_directories_are_bound() {
    for mutation in [0, 1, 2, 3] {
        let c = Case::new();
        let mut p = c.profile();
        match mutation {
            0 => p["apps"][0]["args"][4] = json!(c.root.join("other.sock")),
            1 => p["apps"][0]["args"][2] = json!(c.root.join("other-wayland")),
            2 => p["apps"][0]["args"][0] = json!(c.root.join("home")),
            _ => p["apps"][0]["args"][1] = json!(c.root.join("runtime")),
        };
        c.set_profile(&p);
        c.denied();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn writable_and_symlinked_installer_inputs_are_refused() {
    for name in [
        "session.json",
        "apps.json",
        "hyprland.conf",
        "compositor",
        "entry",
    ] {
        let c = Case::new();
        fs::set_permissions(c.root.join(name), fs::Permissions::from_mode(0o777)).unwrap();
        c.denied();
    }
    let c = Case::new();
    let real = c.root.join("real.json");
    fs::rename(&c.path, &real).unwrap();
    symlink(&real, &c.path).unwrap();
    c.denied();
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn relative_paths_and_reserved_source_environment_are_refused() {
    for field in ["runtime", "policy", "wayland", "audio_socket", "profiles"] {
        let mut c = Case::new();
        c.data[field] = json!("relative");
        c.write();
        c.denied();
    }
    for key in [
        "POKELINUX_SESSION_FD",
        "POKELINUX_SESSION_SOURCE",
        "LD_PRELOAD",
    ] {
        let mut c = Case::new();
        c.data["environment"][key] = json!("bad");
        c.write();
        c.denied();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn bounded_document_refuses_oversize_before_role_selection() {
    let c = Case::new();
    fs::write(&c.path, vec![b' '; 16385]).unwrap();
    c.denied();
}

#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn root_installed_display_profile_environment_is_selected() {
    let mut c = Case::new();
    c.data["environment"]["POKELINUX_DISPLAY_PROFILE"] = json!(c.root.join("pixel10.json"));
    c.data["environment"]["POKELINUX_DISPLAY_PROFILE_RECEIPT"] =
        json!(c.root.join("desktop/display-receipt.json"));
    c.write();
    let config = Config::load(&c.path).expect(
        "Root-selected native display profile must remain available to the installed compositor",
    );
    assert_eq!(
        config.environment["POKELINUX_DISPLAY_PROFILE"],
        c.root.join("pixel10.json").to_str().unwrap()
    );
    assert_eq!(
        config.environment["POKELINUX_DISPLAY_PROFILE_RECEIPT"],
        c.root
            .join("desktop/display-receipt.json")
            .to_str()
            .unwrap()
    );
}

#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_existing_root_private_control_is_selected() {
    let c = Case::new();
    let policy = c.root.join("policy");
    let metadata = fs::symlink_metadata(&policy).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert!(metadata.is_dir());
    assert_eq!(metadata.uid(), 0);
    assert_eq!(metadata.mode() & 0o7777, 0o700);
    let config = Config::load(&c.path).expect("existing root-private policy directory");
    assert_eq!(config.policy, policy);
    assert_eq!(fs::read_dir(policy).unwrap().count(), 0);
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_missing_filename_is_refused() {
    let mut c = Case::new();
    Config::load(&c.path).expect("valid common directory fixture before mutation");
    let missing = c.root.join("policy/state.json");
    assert!(!missing.exists());
    c.data["policy"] = json!(missing);
    c.write();
    c.denied();
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_regular_file_is_refused() {
    let mut c = Case::new();
    Config::load(&c.path).expect("valid common directory fixture before mutation");
    let file = c.root.join("policy.json");
    fs::write(&file, b"{}").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    c.data["policy"] = json!(file);
    c.write();
    c.denied();
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_nonprivate_modes_are_refused() {
    for mode in [0o755, 0o750, 0o770, 0o1700] {
        let c = Case::new();
        Config::load(&c.path).expect("valid common directory fixture before mutation");
        fs::set_permissions(c.root.join("policy"), fs::Permissions::from_mode(mode)).unwrap();
        c.denied();
    }
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_symlink_is_refused() {
    let mut c = Case::new();
    Config::load(&c.path).expect("valid common directory fixture before mutation");
    let link = c.root.join("policy-link");
    symlink(c.root.join("policy"), &link).unwrap();
    c.data["policy"] = json!(link);
    c.write();
    c.denied();
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_wrong_owner_is_refused() {
    let c = Case::new();
    Config::load(&c.path).expect("valid common directory fixture before mutation");
    rustix::fs::chown(
        c.root.join("policy"),
        Some(rustix::process::Uid::from_raw(11042)),
        Some(rustix::process::Gid::from_raw(11042)),
    )
    .unwrap();
    c.denied();
}
#[test]
#[ignore = "requires isolated root and protected fixture directory"]
fn policy_directory_writable_ancestor_is_refused() {
    let mut c = Case::new();
    Config::load(&c.path).expect("valid common directory fixture before mutation");
    let parent = c.root.join("unprotected");
    let policy = parent.join("policy");
    fs::create_dir(&parent).unwrap();
    fs::create_dir(&policy).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o777)).unwrap();
    c.data["policy"] = json!(policy);
    c.write();
    c.denied();
}
