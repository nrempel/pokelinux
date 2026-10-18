//! Config::load/filesystem boundary only in a disposable protected root fixture.
//! No compositor, Source, grants, constructor/backend connection or installed boot.
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
        assert_eq!(rustix::process::geteuid().as_raw(), 0);
        let parent = PathBuf::from(
            std::env::var_os("POKELINUX_SESSION_CONFIG_TEST_ROOT")
                .expect("isolated protected fixture root"),
        );
        let root = parent.join(format!(
            "connectivity-{}-{}",
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
            ("connectivity", 0, 0o700),
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
        for leaf in ["brightness/actor.sock", "connectivity/control.sock"] {
            assert!(
                rustix::net::SocketAddrUnix::new(root.join(leaf)).is_ok(),
                "protected fixture root must be short enough for valid AF_UNIX paths"
            );
        }
        let data = json!({"runtime":root.join("runtime"),"policy":root.join("policy"),"profiles":root.join("apps.json"),
            "app_entry":root.join("entry"),"display_parent":root.join("display"),"wayland":root.join("desktop/wayland-1"),
            "audio_socket":root.join("audio.sock"),"appearance_socket":root.join("appearance.sock"),"messages":{"mode":"disabled"},
            "brightness":{"mode":"disabled"},"connectivity":{"mode":"disabled"},
            "compositor_executable":root.join("compositor"),"compositor_config":root.join("hyprland.conf"),
            "environment":{"HOME":root.join("home"),"XDG_RUNTIME_DIR":root.join("desktop"),"PATH":"/usr/bin"}});
        let site = Self { root, data };
        site.profile(false, false);
        site
    }
    fn app(&self, name: &str, uid: u32, leaf: &str) -> Value {
        json!({"app":name,"uid":uid,"executable":self.root.join(leaf),"args":[self.root.join(format!("{leaf}-home")),
            self.root.join(format!("{leaf}-run")),self.root.join("desktop/wayland-1"),self.root.join("appearance.sock"),
            self.root.join("runtime/app.sock"),self.root.join("appearance")]})
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
    fn save_profile(&self, value: Value) {
        fs::write(
            self.root.join("apps.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    fn profile_value(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("apps.json")).unwrap()).unwrap()
    }
    fn select(&mut self, brightness: bool, connectivity: bool, history: bool) {
        self.data["brightness"] = if brightness {
            json!({"mode":"owner","socket":self.root.join("brightness/actor.sock")})
        } else {
            json!({"mode":"disabled"})
        };
        self.data["connectivity"] = if connectivity {
            json!({"mode":"owner","socket":self.root.join("connectivity/control.sock")})
        } else {
            json!({"mode":"disabled"})
        };
        if history {
            self.data["messages"] = json!({"mode":"history","runtime":self.root.join("messages-owner"),"raw":self.root.join("raw"),"inbox":self.root.join("inbox")});
        }
        self.profile(brightness || connectivity, history);
    }
    fn load(&self) -> std::io::Result<Config> {
        let path = self.root.join("session.json");
        fs::write(&path, serde_json::to_vec(&self.data).unwrap()).unwrap();
        Config::load(&path)
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
#[ignore = "isolated root protected fixture + CHOWN/FOWNER/DAC_OVERRIDE; no runtime"]
fn explicit_selection_uses_one_settings_role_for_brightness_or_connectivity() {
    let mut admitted = Vec::new();
    for (brightness, connectivity, history) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (true, true, false),
        (true, true, true),
    ] {
        let accepted = {
            let mut s = Site::new();
            s.select(brightness, connectivity, history);
            let profile = s.profile_value();
            assert_eq!(
                profile["apps"].as_array().unwrap().len(),
                1 + usize::from(brightness || connectivity) + usize::from(history)
            );
            let result = s.load();
            println!(
                "CONFIG_ADMISSION brightness={brightness} connectivity={connectivity} history={history}: {result:?}"
            );
            result.as_ref().is_ok_and(|v| {
                v.runtime == s.root.join("runtime") && v.wayland == s.root.join("desktop/wayland-1")
            })
        };
        admitted.push(accepted);
    }
    assert!(
        admitted.iter().all(|v| *v),
        "explicit Disabled/Owner selections must admit the exact installed principal matrix: {admitted:?}"
    );
}
#[test]
#[ignore = "isolated root protected fixture + CHOWN/FOWNER/DAC_OVERRIDE; no runtime"]
fn connectivity_selection_is_required_and_accepts_no_authority_or_backend_fields() {
    // On the current pre-field implementation, this actually loads successfully;
    // the desired refusal is an executable assertion, never a missing enum API.
    let missing_refused = {
        let mut s = Site::new();
        s.data.as_object_mut().unwrap().remove("connectivity");
        let result = s.load();
        println!("CONFIG_MISSING_CONNECTIVITY: {result:?}");
        result.is_err()
    };
    assert!(
        missing_refused,
        "omitted connectivity must be refused, never defaulted"
    );
    for field in ["grant", "backend", "device"] {
        let mut s = Site::new();
        s.select(false, true, false);
        assert!(s.load().is_ok(), "valid owner before mutation");
        s.data["connectivity"][field] = json!("not-an-installed-selection-input");
        assert!(s.load().is_err(), "accepted {field}");
    }
    let mut s = Site::new();
    s.select(false, true, false);
    assert!(s.load().is_ok());
    s.data["connectivity"]["mode"] = json!("automatic");
    assert!(s.load().is_err());
}
#[test]
#[ignore = "isolated root protected fixture + CHOWN/FOWNER/DAC_OVERRIDE; no runtime"]
fn selected_settings_requires_exact_uid_role_and_six_bound_arguments() {
    for mutation in 0..9 {
        let mut s = Site::new();
        s.select(false, true, false);
        assert!(
            s.load().is_ok(),
            "valid connectivity-only owner before principal mutation"
        );
        let mut profile = s.profile_value();
        match mutation {
            0 => profile["apps"][1]["uid"] = json!(11042),
            1 => {
                profile["apps"][1]["args"].as_array_mut().unwrap().pop();
            }
            2 => profile["apps"][1]["args"]
                .as_array_mut()
                .unwrap()
                .push(json!(s.root.join("extra"))),
            3 => profile["apps"][1]["args"][4] = json!(s.root.join("other.sock")),
            4 => profile["apps"][1]["args"][2] = json!(s.root.join("other-wayland")),
            5 => profile["apps"][1]["args"][3] = json!(s.root.join("other-appearance")),
            6 => profile["apps"][1]["app"] = json!("org.omarchy.Other"),
            7 => {
                profile["apps"].as_array_mut().unwrap().pop();
            }
            _ => {
                let duplicate = profile["apps"][1].clone();
                profile["apps"].as_array_mut().unwrap().push(duplicate);
            }
        }
        s.save_profile(profile);
        assert!(s.load().is_err(), "accepted principal mutation {mutation}");
    }
    let s = Site::new();
    assert!(s.load().is_ok());
    s.profile(true, false);
    assert!(
        s.load().is_err(),
        "Settings must be absent when neither owner is selected"
    );
}
#[test]
#[ignore = "isolated root protected fixture + CHOWN/FOWNER/DAC_OVERRIDE; no runtime"]
fn connectivity_owner_endpoint_is_private_canonical_unix_and_distinct() {
    for mutation in 0..10 {
        let mut s = Site::new();
        s.select(true, true, false);
        assert!(
            s.load().is_ok(),
            "valid both-owner selection before endpoint mutation"
        );
        match mutation {
            0 => s.data["connectivity"]["socket"] = json!("relative/socket"),
            1 => {
                s.data["connectivity"]["socket"] =
                    json!(s.root.join("connectivity/../control.sock"))
            }
            2 => fs::set_permissions(
                s.root.join("connectivity"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap(),
            3 => rustix::fs::chown(
                s.root.join("connectivity"),
                None,
                Some(rustix::process::Gid::from_raw(1)),
            )
            .unwrap(),
            4 => {
                s.data["connectivity"]["socket"] =
                    json!(s.root.join("connectivity").join("x".repeat(108)))
            }
            5 => s.data["audio_socket"] = s.data["connectivity"]["socket"].clone(),
            6 => {
                let socket = s.data["connectivity"]["socket"].clone();
                s.data["appearance_socket"] = socket.clone();
                let mut p = s.profile_value();
                for app in p["apps"].as_array_mut().unwrap() {
                    app["args"][3] = socket.clone();
                }
                s.save_profile(p);
            }
            7 => s.data["connectivity"]["socket"] = s.data["wayland"].clone(),
            8 => {
                fs::set_permissions(s.root.join("runtime"), fs::Permissions::from_mode(0o700))
                    .unwrap();
                s.data["connectivity"]["socket"] = json!(s.root.join("runtime/app.sock"));
            }
            _ => s.data["connectivity"]["socket"] = s.data["brightness"]["socket"].clone(),
        }
        assert!(s.load().is_err(), "accepted endpoint mutation {mutation}");
    }
}
