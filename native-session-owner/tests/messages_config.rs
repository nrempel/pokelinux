//! Protected target configuration; no GUI, Source health or receiving claim.
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
    config: Value,
    profiles: Value,
}
impl Case {
    fn new(history: bool) -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from(
            std::env::var_os("POKELINUX_SESSION_CONFIG_TEST_ROOT")
                .expect("protected isolated parent"),
        )
        .join(format!(
            "messages-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        for name in [
            "runtime",
            "policy",
            "display",
            "home",
            "desktop",
            "sounds-home",
            "sounds-run",
            "messages-home",
            "messages-run",
            "appearance",
            "projector",
            "raw",
            "inbox",
        ] {
            let path = root.join(name);
            fs::create_dir(&path).unwrap();
            let uid = match name {
                "home" | "desktop" => 1000,
                "sounds-home" | "sounds-run" => 11042,
                "messages-home" | "messages-run" => 11010,
                _ => 0,
            };
            rustix::fs::chown(
                &path,
                Some(rustix::process::Uid::from_raw(uid)),
                Some(rustix::process::Gid::from_raw(uid)),
            )
            .unwrap();
            let private = uid != 0 || ["policy", "projector", "raw", "inbox"].contains(&name);
            fs::set_permissions(
                path,
                fs::Permissions::from_mode(if private { 0o700 } else { 0o755 }),
            )
            .unwrap();
        }
        for name in ["entry", "compositor", "sounds", "messages"] {
            fs::write(root.join(name), b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(root.join("hyprland.conf"), b"# trusted fixture\n").unwrap();
        let profile = |role: &str, uid: u32, prefix: &str| json!({"app":role,"uid":uid,"executable":root.join(prefix),"args":[root.join(format!("{prefix}-home")),root.join(format!("{prefix}-run")),root.join("desktop/wayland-1"),root.join("appearance.sock"),root.join("runtime/app.sock"),root.join("appearance")]});
        let mut apps = vec![profile("org.omarchy.Sounds", 11042, "sounds")];
        if history {
            apps.push(profile("org.omarchy.Messages", 11010, "messages"));
        }
        let messages = if history {
            json!({"mode":"owner","socket":root.join("projector/read-provider.sock")})
        } else {
            json!({"mode":"disabled"})
        };
        let config = json!({"runtime":root.join("runtime"),"policy":root.join("policy"),"profiles":root.join("apps.json"),"app_entry":root.join("entry"),"display_parent":root.join("display"),"wayland":root.join("desktop/wayland-1"),"audio_socket":root.join("audio.sock"),"appearance_socket":root.join("appearance.sock"),"messages":messages,"brightness":{"mode":"disabled"},"connectivity":{"mode":"disabled"},"compositor_executable":root.join("compositor"),"compositor_config":root.join("hyprland.conf"),"environment":{"HOME":root.join("home"),"XDG_RUNTIME_DIR":root.join("desktop"),"PATH":"/usr/bin"}});
        Self {
            root,
            config,
            profiles: json!({"apps":apps}),
        }
    }
    fn load(&self) -> std::io::Result<Config> {
        fs::write(
            self.root.join("apps.json"),
            serde_json::to_vec(&self.profiles).unwrap(),
        )
        .unwrap();
        let path = self.root.join("session.json");
        fs::write(&path, serde_json::to_vec(&self.config).unwrap()).unwrap();
        Config::load(&path)
    }
    fn denied(&self) {
        assert!(self.load().is_err(), "unsafe target configuration admitted");
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
#[ignore = "isolated Linux root/filesystem roles"]
fn owner_selects_both_fixed_app_roles() {
    let c = Case::new(true);
    let selected = c
        .load()
        .expect("installed two-role owner target must be selectable");
    assert_eq!(selected.runtime, c.root.join("runtime"));
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn disabled_is_an_explicit_sounds_only_target() {
    let c = Case::new(false);
    c.load().expect("explicit disabled target");
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn missing_mode_and_undeclared_backends_are_refused() {
    for mutation in 0..4 {
        let mut c = Case::new(true);
        match mutation {
            0 => {
                c.config.as_object_mut().unwrap().remove("messages");
            }
            1 => c.config["messages"] = json!(null),
            2 => c.config["messages"]["mode"] = json!("incoming"),
            _ => c.config["messages"]["source_fd"] = json!(3),
        }
        c.denied();
    }
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn owner_requires_exact_roles_and_uids() {
    for mutation in 0..6 {
        let mut c = Case::new(true);
        match mutation {
            0 => c.profiles["apps"][0]["uid"] = json!(11010),
            1 => c.profiles["apps"][1]["uid"] = json!(11042),
            2 => c.profiles["apps"][1]["app"] = json!("org.omarchy.Other"),
            3 => {
                c.profiles["apps"].as_array_mut().unwrap().pop();
            }
            4 => {
                let extra = c.profiles["apps"][1].clone();
                c.profiles["apps"].as_array_mut().unwrap().push(extra);
            }
            _ => {
                c.profiles["apps"][1]["args"].as_array_mut().unwrap().pop();
            }
        }
        c.denied();
    }
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn mode_and_profile_set_cannot_disagree() {
    let mut c = Case::new(true);
    c.config["messages"] = json!({"mode":"disabled"});
    c.denied();
    let mut c = Case::new(false);
    c.config["messages"] =
        json!({"mode":"owner","socket":c.root.join("projector/read-provider.sock")});
    c.denied();
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn both_apps_bind_canonical_actor_wayland_and_appearance_socket() {
    for app in 0..2 {
        for index in [2, 3, 4] {
            let mut c = Case::new(true);
            c.profiles["apps"][app]["args"][index] = json!(if index == 3 {
                c.root.join("home")
            } else {
                c.root.join("wrong.sock")
            });
            c.denied();
        }
    }
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn app_private_directories_cannot_cross_principals() {
    for index in [0, 1] {
        let mut c = Case::new(true);
        c.profiles["apps"][1]["args"][index] = c.profiles["apps"][0]["args"][index].clone();
        c.denied();
    }
}
#[test]
#[ignore = "isolated Linux root/filesystem roles"]
fn owner_endpoint_parent_is_existing_root_private_and_separate() {
    for mutation in 0..5 {
        let c = Case::new(true);
        let path = c.root.join("projector");
        match mutation {
            0 => fs::remove_dir(&path).unwrap(),
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap(),
            2 => {
                rustix::fs::chown(&path, Some(rustix::process::Uid::from_raw(11010)), None).unwrap()
            }
            3 => {
                rustix::fs::chown(&path, None, Some(rustix::process::Gid::from_raw(11010))).unwrap()
            }
            _ => {
                fs::remove_dir(&path).unwrap();
                symlink(c.root.join("policy"), &path).unwrap();
            }
        }
        c.denied();
    }
    for parent in ["runtime", "policy", "display"] {
        let mut c = Case::new(true);
        c.config["messages"]["socket"] = json!(c.root.join(parent).join("read-provider.sock"));
        c.denied();
    }
    let mut c = Case::new(true);
    c.config["messages"]["raw"] = json!(c.root.join("raw"));
    c.denied();
}

#[test]
#[ignore = "isolated root; actual authored Source child and missing private provider"]
fn missing_private_provider_setup_failure_drains_source() {
    use pokelinux_lifecycle::boottime_ns;
    use std::process::{Child, Command, ExitStatus};
    use std::time::Duration;
    struct Guard {
        child: Child,
        until: u64,
        status: Option<ExitStatus>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if self.status.is_none() {
                let _ = self.child.kill();
                while boottime_ns() < self.until {
                    if let Some(s) = self.child.try_wait().unwrap() {
                        self.status = Some(s);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                if self.status.is_none() {
                    std::process::abort();
                }
            }
        }
    }
    let mut c = Case::new(true);
    let helper =
        std::env::var("SESSION_RUNTIME_SETUP_HELPER").expect("explicit root-installed fixture");
    c.config["compositor_executable"] = json!(helper);
    c.config["app_entry"] = json!(helper);
    c.load()
        .expect("valid current target before actual setup failure");
    let start = boottime_ns();
    let until = start + 4_000_000_000;
    let child = Command::new(&helper)
        .arg(c.root.join("session.json"))
        .spawn()
        .unwrap();
    let mut g = Guard {
        child,
        until: start + 6_000_000_000,
        status: None,
    };
    while boottime_ns() < until {
        if let Some(status) = g.child.try_wait().unwrap() {
            g.status = Some(status);
            println!(
                "RUNTIME_FIXTURE_ACTUAL_WAIT pid={} code={:?} observed_at_ns={} original_until_ns={} stdout=INHERITED stderr=INHERITED",
                g.child.id(),
                status.code(),
                boottime_ns(),
                until
            );
            assert!(status.success());
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("original runtime setup-error test cutoff");
}

#[test]
#[ignore = "isolated Linux root/filesystem roles; actual Config::load"]
fn owner_selects_private_provider_without_database_paths() {
    let mut c = Case::new(true);
    // The session may know only the daemon endpoint. No Projector directories
    // or raw/inbox database paths exist in this selected target.
    for name in ["projector", "raw", "inbox"] {
        fs::remove_dir(c.root.join(name)).unwrap();
    }
    let provider = c.root.join("provider");
    fs::create_dir(&provider).unwrap();
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
    c.config["messages"] = json!({
        "mode": "owner",
        "socket": provider.join("read-provider.sock")
    });
    let observed = c.load();
    // Release the fixture's foreign-owned empty directories before asserting.
    // CAP_CHOWN releases fixture ownership; no DAC override is used.
    for name in [
        "home",
        "desktop",
        "sounds-home",
        "sounds-run",
        "messages-home",
        "messages-run",
    ] {
        rustix::fs::chown(
            c.root.join(name),
            Some(rustix::process::Uid::from_raw(0)),
            Some(rustix::process::Gid::from_raw(0)),
        )
        .unwrap();
    }
    let selected = observed.expect(
        "socket-only Messages owner must select both fixed apps without session database ownership",
    );
    assert_eq!(selected.runtime, c.root.join("runtime"));
}
