//! Installed selection/filesystem validation only, in a disposable root container.
//! No compositor, Source, backend connection, permission grant or phone is opened.
use pokelinux_session_owner::config::Config;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Site {
    root: PathBuf,
    data: Value,
}
impl Site {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let parent = PathBuf::from(
            std::env::var_os("POKELINUX_SESSION_CONFIG_TEST_ROOT")
                .expect("isolated protected fixture root"),
        );
        let root = parent.join(format!(
            "brightness-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        for (name, uid, mode) in [
            ("runtime", 0, 0o755),
            ("policy", 0, 0o700),
            ("display", 0, 0o755),
            ("appearance", 0, 0o755),
            ("brightness", 0, 0o700),
            ("home", 1000, 0o700),
            ("desktop", 1000, 0o700),
            ("sounds-home", 11042, 0o700),
            ("sounds-run", 11042, 0o700),
            ("settings-home", 11050, 0o700),
            ("settings-run", 11050, 0o700),
            ("messages-home", 11010, 0o700),
            ("messages-run", 11010, 0o700),
            ("messages-owner", 0, 0o700),
            ("raw", 0, 0o700),
            ("inbox", 0, 0o700),
        ] {
            let p = root.join(name);
            fs::create_dir(&p).unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
            rustix::fs::chown(
                &p,
                Some(rustix::process::Uid::from_raw(uid)),
                Some(rustix::process::Gid::from_raw(uid)),
            )
            .unwrap();
        }
        for name in ["compositor", "entry", "sounds", "settings", "messages"] {
            fs::write(root.join(name), b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(
            root.join("hyprland.conf"),
            b"# authored installed configuration fixture\n",
        )
        .unwrap();
        let data = json!({"runtime":root.join("runtime"),"policy":root.join("policy"),"profiles":root.join("apps.json"),"app_entry":root.join("entry"),"display_parent":root.join("display"),"wayland":root.join("desktop/wayland-1"),"audio_socket":root.join("audio.sock"),"appearance_socket":root.join("appearance.sock"),"messages":{"mode":"disabled"},"compositor_executable":root.join("compositor"),"compositor_config":root.join("hyprland.conf"),"environment":{"HOME":root.join("home"),"XDG_RUNTIME_DIR":root.join("desktop"),"PATH":"/usr/bin"}});
        let s = Self { root, data };
        s.profile(false, false);
        s
    }
    fn app(&self, name: &str, uid: u32, leaf: &str) -> Value {
        json!({"app":name,"uid":uid,"executable":self.root.join(leaf),"args":[self.root.join(format!("{leaf}-home")),self.root.join(format!("{leaf}-run")),self.root.join("desktop/wayland-1"),self.root.join("appearance.sock"),self.root.join("runtime/app.sock"),self.root.join("appearance")]})
    }
    fn profile(&self, settings: bool, history: bool) {
        let mut apps = vec![self.app("org.omarchy.Sounds", 11042, "sounds")];
        if settings {
            apps.push(self.app("org.omarchy.Settings", 11050, "settings"));
        }
        if history {
            apps.push(self.app("org.omarchy.Messages", 11010, "messages"));
        }
        self.save_profile(json!({"apps":apps}));
    }
    fn save_profile(&self, p: Value) {
        fs::write(self.root.join("apps.json"), serde_json::to_vec(&p).unwrap()).unwrap();
    }
    fn profile_value(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("apps.json")).unwrap()).unwrap()
    }
    fn owner(&mut self, history: bool) {
        self.data["brightness"] =
            json!({"mode":"owner","socket":self.root.join("brightness/actor.sock")});
        if history {
            self.data["messages"] = json!({"mode":"history","runtime":self.root.join("messages-owner"),"raw":self.root.join("raw"),"inbox":self.root.join("inbox")});
        }
        self.profile(true, history);
    }
    fn load(&self) -> std::io::Result<Config> {
        let p = self.root.join("session.json");
        fs::write(&p, serde_json::to_vec(&self.data).unwrap()).unwrap();
        Config::load(&p)
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
#[ignore = "requires isolated root, temporary protected directories and CHOWN/FOWNER/DAC_OVERRIDE"]
fn explicit_brightness_selection_supports_the_exact_installed_app_matrix() {
    let mut disabled = Site::new();
    disabled.data["brightness"] = json!({"mode":"disabled"});
    assert!(
        disabled.load().is_ok(),
        "explicit disabled selection must retain Sounds without Settings"
    );
    let mut owner = Site::new();
    owner.owner(false);
    assert!(
        owner.load().is_ok(),
        "selected owner must admit Sounds plus fixed Settings principal"
    );
    let mut history = Site::new();
    history.owner(true);
    assert!(
        history.load().is_ok(),
        "selected owner and history must admit exactly all three roles"
    );
}
#[test]
#[ignore = "requires isolated root, temporary protected directories and CHOWN/FOWNER/DAC_OVERRIDE"]
fn selection_is_required_and_never_grants_app_authority() {
    let missing = Site::new();
    assert!(
        missing.load().is_err(),
        "omitted selection must not silently select compatibility behavior"
    );
    for extra in ["grant", "unlock", "source_fd"] {
        let mut s = Site::new();
        s.owner(false);
        assert!(s.load().is_ok());
        s.data[extra] = json!(true);
        assert!(s.load().is_err());
    }
    let mut s = Site::new();
    s.owner(false);
    assert!(s.load().is_ok());
    let mut p = s.profile_value();
    p["apps"][1]["grant"] = json!("display_brightness");
    s.save_profile(p);
    assert!(s.load().is_err());
}
#[test]
#[ignore = "requires isolated root, temporary protected directories and CHOWN/FOWNER/DAC_OVERRIDE"]
fn settings_requires_selected_owner_exact_uid_role_and_six_bound_arguments() {
    for mutation in 0..7 {
        let mut s = Site::new();
        s.owner(false);
        assert!(s.load().is_ok());
        let mut p = s.profile_value();
        match mutation {
            0 => p["apps"][1]["uid"] = json!(11042),
            1 => {
                p["apps"][1]["args"].as_array_mut().unwrap().pop();
            }
            2 => p["apps"][1]["args"][4] = json!(s.root.join("other.sock")),
            3 => p["apps"][1]["args"][2] = json!(s.root.join("other-wayland")),
            4 => p["apps"][1]["app"] = json!("org.omarchy.Other"),
            5 => {
                p["apps"].as_array_mut().unwrap().pop();
            }
            _ => {
                let duplicate = p["apps"][1].clone();
                p["apps"].as_array_mut().unwrap().push(duplicate);
            }
        }
        s.save_profile(p);
        assert!(s.load().is_err(), "accepted mutation {mutation}");
    }
    let mut s = Site::new();
    s.owner(false);
    s.data["brightness"] = json!({"mode":"disabled"});
    assert!(s.load().is_err(), "unselected Settings must be refused");
}
#[test]
#[ignore = "requires isolated root, temporary protected directories and CHOWN/FOWNER/DAC_OVERRIDE"]
fn owner_endpoint_requires_private_root_deployment_and_distinct_socket_selection() {
    for mutation in 0..6 {
        let mut s = Site::new();
        s.owner(false);
        assert!(s.load().is_ok());
        match mutation {
            0 => s.data["brightness"]["socket"] = json!("relative/socket"),
            1 => s.data["brightness"]["socket"] = json!(s.root.join("brightness/../actor.sock")),
            2 => fs::set_permissions(s.root.join("brightness"), fs::Permissions::from_mode(0o755))
                .unwrap(),
            3 => s.data["brightness"]["mode"] = json!("automatic"),
            4 => s.data["audio_socket"] = s.data["brightness"]["socket"].clone(),
            _ => s.data["brightness"]["device"] = json!("panel0"),
        };
        assert!(s.load().is_err(), "accepted endpoint mutation {mutation}");
    }
}
