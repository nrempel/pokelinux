//! Root-installed fixed session selection. No permission or restoration ingress.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub runtime: PathBuf,
    pub policy: PathBuf,
    pub profiles: PathBuf,
    pub app_entry: PathBuf,
    pub display_parent: PathBuf,
    pub wayland: PathBuf,
    pub audio_socket: PathBuf,
    pub appearance_socket: PathBuf,
    pub messages: Messages,
    pub compositor_executable: PathBuf,
    pub compositor_config: PathBuf,
    pub environment: BTreeMap<String, String>,
}
/// Explicit installed target capability, never an incoming/backend selector.
#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Messages {
    Disabled,
    History {
        runtime: PathBuf,
        raw: PathBuf,
        inbox: PathBuf,
    },
}
/// Fixed installed principals. Display admission uses this same mapping.
pub(crate) fn app_uid(app: &pokelinux_lifecycle::AppProfile) -> io::Result<u32> {
    let uid = match app.app.as_str() {
        pokelinux_lifecycle::audio::APP => 11042,
        pokelinux_lifecycle::messages::APP => 11010,
        _ => return Err(io::ErrorKind::PermissionDenied.into()),
    };
    if app.uid != uid || app.args.len() != 6 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(uid)
}
fn absolute(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.components().any(|p| matches!(p, Component::ParentDir)) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
fn protected_directory(path: &Path) -> io::Result<()> {
    absolute(path)?;
    for parent in path.ancestors() {
        let m = fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}
fn installed_file(path: &Path) -> io::Result<()> {
    absolute(path)?;
    protected_directory(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.uid() != 0 || m.mode() & 0o6022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
fn private_directory(path: &Path, uid: u32) -> io::Result<()> {
    absolute(path)?;
    protected_directory(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.uid() != uid || m.mode() & 0o7777 != 0o700 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}
impl Config {
    /// Installer paths and policy remain stable while the session runs. This
    /// preflight does not establish hostile-filesystem syscall time bounds.
    pub fn load(path: &Path) -> io::Result<Self> {
        if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        installed_file(path)?;
        let fd = rustix::fs::open(
            path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::NONBLOCK,
            rustix::fs::Mode::empty(),
        )?;
        let file = fs::File::from(fd);
        let m = file.metadata()?;
        if !m.is_file() || m.uid() != 0 || m.mode() & 0o6022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let mut bytes = Vec::new();
        file.take(16385).read_to_end(&mut bytes)?;
        if bytes.len() > 16384 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let config: Self = serde_json::from_slice(&bytes)?;
        config.validate()?;
        Ok(config)
    }
    fn validate(&self) -> io::Result<()> {
        for path in [&self.runtime, &self.display_parent] {
            protected_directory(path)?;
        }
        for path in [&self.wayland, &self.audio_socket, &self.appearance_socket] {
            absolute(path)?;
        }
        private_directory(&self.policy, 0)?;
        protected_directory(
            self.appearance_socket
                .parent()
                .ok_or(io::ErrorKind::InvalidInput)?,
        )?;
        if let Messages::History {
            runtime,
            raw,
            inbox,
        } = &self.messages
        {
            let mut identities = std::collections::BTreeSet::new();
            for path in [
                &self.runtime,
                &self.policy,
                &self.display_parent,
                runtime,
                raw,
                inbox,
            ] {
                if [runtime, raw, inbox].contains(&path) {
                    private_directory(path, 0)?;
                }
                let m = fs::symlink_metadata(path)?;
                if !identities.insert((m.dev(), m.ino())) {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
            }
        }
        installed_file(&self.profiles)?;
        installed_file(&self.compositor_config)?;
        pokelinux_lifecycle::service::trusted_executable(&self.app_entry)?;
        pokelinux_lifecycle::service::trusted_executable(&self.compositor_executable)?;
        const ALLOWED_ENV: &[&str] = &[
            "HOME",
            "XDG_RUNTIME_DIR",
            "PATH",
            "LANG",
            "LC_ALL",
            "XDG_SESSION_TYPE",
            "XDG_CURRENT_DESKTOP",
            "DBUS_SESSION_BUS_ADDRESS",
            "DISPLAY",
            "XAUTHORITY",
            "AQ_BACKEND",
            "LIBGL_ALWAYS_SOFTWARE",
            "HYPRLAND_NO_RT",
            "WLR_RENDERER",
            "AQ_DRM_DEVICES",
            "SDL_VIDEODRIVER",
            "POKELINUX_DISPLAY_PROFILE",
            "POKELINUX_DISPLAY_PROFILE_RECEIPT",
        ];
        if self.environment.len() > 16
            || self.environment.iter().any(|(k, v)| {
                !ALLOWED_ENV.contains(&k.as_str()) || v.len() > 4096 || v.contains('\0')
            })
            || self.environment.get("PATH").map(String::as_str) != Some("/usr/bin")
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let home = Path::new(
            self.environment
                .get("HOME")
                .ok_or(io::ErrorKind::InvalidInput)?,
        );
        let desktop = Path::new(
            self.environment
                .get("XDG_RUNTIME_DIR")
                .ok_or(io::ErrorKind::InvalidInput)?,
        );
        private_directory(home, 1000)?;
        private_directory(desktop, 1000)?;
        if self.wayland.parent() != Some(desktop) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let profiles = pokelinux_lifecycle::service::profile(&self.profiles)?;
        let history = matches!(&self.messages, Messages::History { .. });
        if profiles.apps.len() != if history { 2 } else { 1 } {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let mut roles = std::collections::BTreeSet::new();
        for app in &profiles.apps {
            let uid = app_uid(app)?;
            if !roles.insert(app.app.as_str())
                || (!history && app.app != pokelinux_lifecycle::audio::APP)
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            for arg in &app.args {
                absolute(Path::new(arg))?;
            }
            if Path::new(&app.args[2]) != self.wayland
                || Path::new(&app.args[3]) != self.appearance_socket
                || Path::new(&app.args[4]) != self.runtime.join("app.sock")
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            private_directory(Path::new(&app.args[0]), uid)?;
            private_directory(Path::new(&app.args[1]), uid)?;
            protected_directory(Path::new(&app.args[5]))?;
        }
        Ok(())
    }
}
