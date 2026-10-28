//! Normal initial-home provisioning; no palette or shell implementation lives here.
use pokelinux_appearance::{Configuration, service, style};
use rustix::process::{Gid, Uid};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Component, Path},
    process::Command,
};
fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}
fn canonical(path: &Path) -> io::Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|v| matches!(v, Component::ParentDir | Component::CurDir))
    {
        return Err(denied());
    }
    Ok(())
}
fn user_dir(path: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.uid() != 1000 || m.gid() != 1000 || m.mode() & 0o022 != 0 {
        return Err(denied());
    }
    Ok(())
}
fn optional_user_file(home: &Path, parts: &[&str]) -> io::Result<Option<String>> {
    let mut p = home.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        p.push(part);
        let m = match fs::symlink_metadata(&p) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        if index + 1 < parts.len() {
            user_dir(&p)?;
            continue;
        }
        if !m.is_file()
            || m.uid() != 1000
            || m.gid() != 1000
            || m.nlink() != 1
            || m.mode() & 0o6022 != 0
            || m.len() > 65536
        {
            return Err(denied());
        }
        let f = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&p)?;
        let opened = f.metadata()?;
        if opened.dev() != m.dev() || opened.ino() != m.ino() {
            return Err(denied());
        }
        let mut text = String::new();
        f.take(65537).read_to_string(&mut text)?;
        if text.len() > 65536 {
            return Err(denied());
        }
        return Ok(Some(text));
    }
    Err(io::ErrorKind::InvalidInput.into())
}
fn selected(home: &Path) -> io::Result<bool> {
    let name = optional_user_file(
        home,
        &[".local", "state", "omarchy", "current", "theme.name"],
    )?;
    let colors = optional_user_file(
        home,
        &[
            ".local",
            "state",
            "omarchy",
            "current",
            "theme",
            "colors.toml",
        ],
    )?;
    let theme = home.join(".local/state/omarchy/current/theme");
    if name.is_none() && colors.is_none() && !theme.try_exists()? {
        return Ok(false);
    }
    let name = name.ok_or(io::ErrorKind::InvalidData)?;
    if name.trim().is_empty() || name.len() > 256 || name.contains('\0') {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let colors = colors.ok_or(io::ErrorKind::InvalidData)?;
    let generated = optional_user_file(
        home,
        &[
            ".local",
            "state",
            "omarchy",
            "current",
            "theme",
            "shell.toml",
        ],
    )?;
    let user = optional_user_file(home, &[".config", "omarchy", "shell.toml"])?;
    style::project(
        &colors,
        generated.as_deref().unwrap_or(""),
        user.as_deref().unwrap_or(""),
    )?;
    Ok(true)
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3
        || rustix::process::getuid().as_raw() != 0
        || rustix::process::getgid().as_raw() != 0
    {
        return Err(denied());
    }
    let config_path = Path::new(&args[1]);
    let assets = Path::new(&args[2]);
    canonical(assets)?;
    if args[2].contains(':') {
        return Err(denied());
    }
    service::trusted_path(config_path, false)?;
    let config_file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(config_path)?;
    if config_file.metadata()?.len() > 4096 {
        return Err(denied());
    }
    let mut bytes = Vec::new();
    config_file.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(denied());
    }
    let config: Configuration = serde_json::from_slice(&bytes)?;
    let home = Path::new(&config.home);
    let runtime = Path::new(&config.runtime);
    canonical(home)?;
    canonical(runtime)?;
    for path in [
        "bin/omarchy-theme-set",
        "bin/omarchy-theme-set-templates",
        "bin/omarchy-theme-color",
    ] {
        service::trusted_path(&assets.join(path), true)?;
    }
    for path in [
        "themes/tokyo-night/colors.toml",
        "default/themed/shell.toml.tpl",
    ] {
        service::trusted_path(&assets.join(path), false)?;
    }
    service::trusted_path(Path::new("/usr/bin/bash"), true)?;
    rustix::thread::set_thread_groups(&[])?;
    let gid = Gid::from_raw(1000);
    rustix::thread::set_thread_res_gid(gid, gid, gid)?;
    let uid = Uid::from_raw(1000);
    rustix::thread::set_thread_res_uid(uid, uid, uid)?;
    user_dir(home)?;
    user_dir(runtime)?;
    if selected(home)? {
        println!("existing_complete_selection_preserved");
        return Ok(());
    }
    rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o077));
    fs::create_dir_all(home.join(".config/omarchy/themes"))?;
    // Same default/preservation rule as pinned install/user/theme.sh. The
    // original setter's headless provisioning branch skips live IPC/app hooks.
    let error = Command::new("/usr/bin/bash")
        .arg(assets.join("bin/omarchy-theme-set"))
        .arg("Tokyo Night")
        .env_clear()
        .env("HOME", home)
        .env("XDG_RUNTIME_DIR", runtime)
        .env("OMARCHY_PATH", assets)
        .env("PATH", format!("{}:/usr/bin", assets.join("bin").display()))
        .env("LANG", "C.UTF-8")
        .env("OMARCHY_THEME_HEADLESS", "1")
        .current_dir(home)
        .exec();
    Err(error)
}
