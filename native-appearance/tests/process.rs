//! Actual Linux processes/socket credentials, filesystem locks and kernel filters.
//! These are component controls, not original Omarchy or physical phone evidence.
use pokelinux_appearance::{Snapshot, boottime_ns, ipc};
use rustix::{
    net,
    process::{Pid, Signal},
};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Realm {
    root: PathBuf,
    home: PathBuf,
    runtime: PathBuf,
    socket: PathBuf,
    child: Option<Child>,
    binary: PathBuf,
}
fn until(mut check: impl FnMut() -> bool, seconds: u64) {
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        if check() {
            return;
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    panic!("actual appearance prerequisite timed out");
}
fn call(path: &Path, request: Value) -> io::Result<Value> {
    let fd = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::SEQPACKET,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )?;
    net::sockopt::set_socket_passcred(&fd, true)?;
    net::connect(&fd, &net::SocketAddrUnix::new(path)?)?;
    let peer = net::sockopt::socket_peercred(&fd)?;
    let end = boottime_ns() + 200_000_000;
    ipc::send(&fd, &serde_json::to_vec(&request)?)?;
    while boottime_ns() < end {
        if let Some((bytes, cred)) = ipc::receive(&fd)? {
            assert_eq!(
                (cred.pid, cred.uid, cred.gid),
                (peer.pid, peer.uid, peer.gid)
            );
            return Ok(serde_json::from_slice(&bytes)?);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Err(io::ErrorKind::TimedOut.into())
}
impl Realm {
    fn new(name: &str) -> Self {
        assert_eq!(
            rustix::process::getuid().as_raw(),
            0,
            "tests require actual root in the isolated container"
        );
        let root = PathBuf::from(format!(
            "/run/appearance-component-{}-{}",
            std::process::id(),
            name
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let home = root.join("home");
        let runtime = root.join("runtime");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&runtime).unwrap();
        fs::create_dir_all(home.join(".local/state/omarchy/current/theme")).unwrap();
        let walk = |path: &Path| {
            let mut pending = vec![path.to_path_buf()];
            while let Some(p) = pending.pop() {
                rustix::fs::chown(
                    &p,
                    Some(rustix::process::Uid::from_raw(1000)),
                    Some(rustix::process::Gid::from_raw(1000)),
                )
                .unwrap();
                fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
                for entry in fs::read_dir(p).unwrap() {
                    let p = entry.unwrap().path();
                    if p.is_dir() {
                        pending.push(p);
                    }
                }
            }
        };
        walk(&home);
        walk(&runtime);
        let binary = root.join("installed-appearance");
        fs::copy(env!("CARGO_BIN_EXE_pokelinux-appearance"), &binary).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let config = root.join("config.json");
        fs::write(
            &config,
            serde_json::to_vec(&json!({"home":home,"runtime":runtime})).unwrap(),
        )
        .unwrap();
        let socket = root.join("appearance.sock");
        let mut r = Self {
            root,
            home,
            runtime,
            socket,
            child: None,
            binary,
        };
        r.theme("#1a1b26");
        r.start();
        r
    }
    fn colors(&self) -> PathBuf {
        self.home
            .join(".local/state/omarchy/current/theme/colors.toml")
    }
    fn theme(&self, background: &str) {
        fs::write(self.colors(),format!("background = '{background}'\nforeground = '#a9b1d6'\naccent = '#7aa2f7'\nred = '#ff0000'\ncolor1 = '#f7768e'\ngreen = '#9ece6a'\n")).unwrap();
        rustix::fs::chown(
            self.colors(),
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        fs::set_permissions(self.colors(), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn start(&mut self) {
        self.start_with_sigchld(false);
    }
    fn start_with_sigchld(&mut self, ignored: bool) {
        assert!(self.child.is_none());
        let mut command = Command::new(&self.binary);
        command
            .arg("--serve")
            .arg(&self.socket)
            .arg(self.root.join("config.json"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(File::create(self.root.join("service.log")).unwrap());
        if ignored {
            unsafe {
                command.pre_exec(|| {
                    if libc::signal(libc::SIGCHLD, libc::SIG_IGN) == libc::SIG_ERR {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        self.child = Some(command.spawn().unwrap());
        until(
            || {
                self.child.as_mut().unwrap().try_wait().unwrap().is_none()
                    && call(
                        &self.root.join("appearance-control.sock"),
                        json!({"op":"status"}),
                    )
                    .is_ok()
            },
            2,
        );
    }
    fn status(&self) -> Value {
        call(
            &self.root.join("appearance-control.sock"),
            json!({"op":"status"}),
        )
        .unwrap()
    }
    fn snapshot(&self) -> Snapshot {
        serde_json::from_value(call(&self.socket, json!({"op":"snapshot"})).unwrap()).unwrap()
    }
    fn healthy(&self, background: &str) {
        until(
            || {
                let s = self.snapshot();
                s.available && s.appearance.unwrap().palette.background == background
            },
            4,
        );
    }
    fn lock(&self) -> File {
        let path = self.runtime.join("omarchy-theme-set.lock");
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        rustix::fs::chown(
            path,
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        rustix::fs::flock(&f, rustix::fs::FlockOperation::LockExclusive).unwrap();
        f
    }
    fn held_worker(&self, lock: &File) -> u32 {
        let _ = lock;
        let mut pid = 0;
        until(
            || {
                let s = self.status();
                pid = s["worker_pid"].as_u64().unwrap_or(0) as u32;
                pid != 0
                    && fs::read_to_string(format!("/proc/{pid}/status")).is_ok_and(|s| {
                        s.lines().any(|l| {
                            l.starts_with("Uid:")
                                && l.split_whitespace().skip(1).all(|u| u == "1000")
                        })
                    })
            },
            3,
        );
        pid
    }
    fn stop(&mut self) {
        if self.child.is_none() {
            return;
        }
        assert_eq!(
            call(
                &self.root.join("appearance-control.sock"),
                json!({"op":"stop"})
            )
            .unwrap(),
            json!({"result":"stopping"})
        );
        until(
            || self.child.as_mut().unwrap().try_wait().unwrap().is_some(),
            3,
        );
        let status = self.child.take().unwrap().wait().unwrap();
        assert!(status.success());
        assert!(!self.socket.exists());
    }
}
impl Drop for Realm {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = rustix::process::kill_process(
                Pid::from_raw(child.id() as i32).unwrap(),
                Signal::TERM,
            );
            let end = Instant::now() + Duration::from_secs(3);
            while Instant::now() < end {
                if child.try_wait().unwrap().is_some() {
                    self.child = None;
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            if self.child.is_some() {
                std::process::abort();
            }
        }
        // Test sources/logs persist until the owning container exits.
    }
}
#[test]
fn actual_palette_private_home_and_live_changes() {
    let mut r = Realm::new("palette");
    r.healthy("#1a1b26");
    let old = r.snapshot();
    assert_eq!(old.appearance.unwrap().palette.urgent, "#f7768e");
    assert_eq!(r.home.metadata().unwrap().mode() & 0o777, 0o700);
    let guard = r.lock();
    r.theme("#eff1f5");
    drop(guard);
    r.healthy("#eff1f5");
    assert!(r.snapshot().revision > old.revision);
    r.stop();
}

#[test]
fn actual_user_style_replacement_deletion_and_revision() {
    let mut r = Realm::new("user-style");
    r.healthy("#1a1b26");
    let write = |path: &Path, text: &str| {
        let temporary = path.with_extension("new");
        fs::write(&temporary, text).unwrap();
        rustix::fs::chown(
            &temporary,
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).unwrap();
        fs::rename(temporary, path).unwrap();
    };
    let generated = r.colors().with_file_name("shell.toml");
    write(
        &generated,
        "[font]\nbase-size = 14\nbody = 20\n[spacing]\npanel-gap = 19\n",
    );
    for path in [r.home.join(".config"), r.home.join(".config/omarchy")] {
        fs::create_dir(&path).unwrap();
        rustix::fs::chown(
            &path,
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let user = r.home.join(".config/omarchy/shell.toml");
    write(
        &user,
        "[font]\nbase-size = 18\nbody = 23\ndisplay = 45\n[spacing]\ncontrol-gap = 17\npanel-padding = 31\n",
    );
    let expected_user = json!({
        "base_size":18,
        "font_tokens":{"caption":15,"bodySmall":17,"body":23,"subtitle":19,
            "title":21,"heading":24,"display":45,"displayLarge":42},
        "spacing_tokens":{"controlGap":17,"panelPadding":31,"panelGap":19,"rowGap":12,
            "controlPaddingX":15,"controlPaddingY":9}
    });
    let mut observed = Value::Null;
    let end = Instant::now() + Duration::from_secs(4);
    while Instant::now() < end {
        observed = call(&r.socket, json!({"op":"snapshot"})).unwrap();
        if observed["available"] == true && observed["appearance"]["style"] == expected_user {
            break;
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    fs::write(
        r.root.join("actual-user-style-snapshot.json"),
        serde_json::to_vec(&observed).unwrap(),
    )
    .unwrap();
    assert_eq!(
        observed["appearance"]["style"], expected_user,
        "actual service omitted the qualified user style: {observed}"
    );
    assert_eq!(observed["schema"], 2);
    let revision = observed["revision"].as_u64().unwrap();
    fs::remove_file(&user).unwrap();
    until(
        || {
            observed = call(&r.socket, json!({"op":"snapshot"})).unwrap();
            observed["available"] == true
                && observed["appearance"]["style"]["base_size"] == 14
                && observed["appearance"]["style"]["font_tokens"]["body"] == 20
                && observed["appearance"]["style"]["spacing_tokens"]["panelGap"] == 19
                && observed["revision"].as_u64().unwrap() > revision
        },
        4,
    );
    let revision = observed["revision"].as_u64().unwrap();
    fs::remove_file(generated).unwrap();
    until(
        || {
            observed = call(&r.socket, json!({"op":"snapshot"})).unwrap();
            observed["available"] == true
                && observed["appearance"]["style"]["base_size"] == 12
                && observed["appearance"]["style"]["font_tokens"]["body"] == 12
                && observed["appearance"]["style"]["spacing_tokens"]["panelGap"] == 14
                && observed["revision"].as_u64().unwrap() > revision
        },
        4,
    );
    assert_eq!(observed["appearance"]["palette"]["background"], "#1a1b26");
    r.stop();
}
#[test]
fn actual_unsafe_user_shell_refusal_and_absence_recovery() {
    let mut r = Realm::new("unsafe-user-shell");
    r.healthy("#1a1b26");
    for path in [r.home.join(".config"), r.home.join(".config/omarchy")] {
        fs::create_dir(&path).unwrap();
        rustix::fs::chown(
            &path,
            Some(rustix::process::Uid::from_raw(1000)),
            Some(rustix::process::Gid::from_raw(1000)),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let user = r.home.join(".config/omarchy/shell.toml");
    let auxiliary = r.home.join("unrelated-synthetic-settings");
    for kind in ["symlink", "hardlink", "fifo", "invalid-utf8", "wrong-owner"] {
        match kind {
            "symlink" => std::os::unix::fs::symlink(r.colors(), &user).unwrap(),
            "hardlink" => {
                fs::write(&auxiliary, "[font]\nbase-size=18\n").unwrap();
                fs::hard_link(&auxiliary, &user).unwrap();
            }
            "fifo" => rustix::fs::mknodat(
                rustix::fs::CWD,
                &user,
                rustix::fs::FileType::Fifo,
                rustix::fs::Mode::from_raw_mode(0o600),
                0,
            )
            .unwrap(),
            "invalid-utf8" => fs::write(&user, [0xff]).unwrap(),
            "wrong-owner" => fs::write(&user, "[font]\nbase-size=18\n").unwrap(),
            _ => unreachable!(),
        }
        if !matches!(kind, "symlink" | "wrong-owner") {
            rustix::fs::chown(
                &user,
                Some(rustix::process::Uid::from_raw(1000)),
                Some(rustix::process::Gid::from_raw(1000)),
            )
            .unwrap();
        }
        until(|| !r.snapshot().available, 3);
        assert!(
            r.status()["state"].is_string(),
            "owner stays responsive for {kind}"
        );
        fs::remove_file(&user).unwrap();
        if kind == "hardlink" {
            fs::remove_file(&auxiliary).unwrap();
        }
        r.healthy("#1a1b26");
    }
    r.stop();
}
#[test]
fn stopped_real_collector_keeps_snapshot_and_stop_responsive() {
    let mut r = Realm::new("held");
    r.healthy("#1a1b26");
    let guard = r.lock();
    let pid = r.held_worker(&guard);
    rustix::process::kill_process(Pid::from_raw(pid as i32).unwrap(), Signal::STOP).unwrap();
    for _ in 0..12 {
        let start = Instant::now();
        let _ = r.status();
        let _ = r.snapshot();
        assert!(start.elapsed() < Duration::from_millis(150));
    }
    r.stop();
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    drop(guard);
}
#[test]
fn original_deadline_expires_held_collection_and_recovers_only_after_reap() {
    let mut r = Realm::new("expiry");
    r.healthy("#1a1b26");
    let guard = r.lock();
    let pid = r.held_worker(&guard);
    rustix::process::kill_process(Pid::from_raw(pid as i32).unwrap(), Signal::STOP).unwrap();
    until(|| !Path::new(&format!("/proc/{pid}")).exists(), 3);
    until(|| !r.snapshot().available, 2);
    drop(guard);
    r.healthy("#1a1b26");
    r.stop();
}
#[test]
fn private_symlink_hardlink_and_unsupported_inputs_do_not_publish() {
    let mut r = Realm::new("input");
    r.healthy("#1a1b26");
    let guard = r.lock();
    fs::remove_file(r.colors()).unwrap();
    let secret = r.home.join("secret");
    fs::write(&secret, "AUTH_TOKEN=synthetic-not-real\n").unwrap();
    rustix::fs::chown(
        &secret,
        Some(rustix::process::Uid::from_raw(1000)),
        Some(rustix::process::Gid::from_raw(1000)),
    )
    .unwrap();
    std::os::unix::fs::symlink(&secret, r.colors()).unwrap();
    drop(guard);
    until(|| !r.snapshot().available, 3);
    let guard = r.lock();
    fs::remove_file(r.colors()).unwrap();
    fs::hard_link(&secret, r.colors()).unwrap();
    drop(guard);
    std::thread::sleep(Duration::from_millis(1400));
    assert!(!r.snapshot().available);
    let guard = r.lock();
    fs::remove_file(r.colors()).unwrap();
    r.theme("#1a1b26");
    drop(guard);
    r.healthy("#1a1b26");
    r.stop();
}
#[test]
fn publisher_restart_has_fresh_namespace() {
    let mut r = Realm::new("restart");
    r.healthy("#1a1b26");
    let owner = r.snapshot().owner;
    r.stop();
    r.start();
    r.healthy("#1a1b26");
    assert_ne!(owner, r.snapshot().owner);
    r.stop();
}

#[test]
fn killed_publisher_reaps_collector_and_restarts_same_paths() {
    // This test is the directly owned subreaper for the service's orphan child.
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) },
        0
    );
    let mut r = Realm::new("abrupt-restart");
    r.healthy("#1a1b26");
    let owner = r.snapshot().owner;
    let guard = r.lock();
    let collector = r.held_worker(&guard);
    rustix::process::kill_process(Pid::from_raw(collector as i32).unwrap(), Signal::STOP).unwrap();
    let mut publisher = r.child.take().unwrap();
    publisher.kill().unwrap();
    let publisher_status = publisher.wait().unwrap();
    assert!(!publisher_status.success());
    let mut collector_status = 0;
    until(|| unsafe { libc::waitpid(collector as i32, &mut collector_status, libc::WNOHANG) } == collector as i32, 3);
    assert!(libc::WIFSIGNALED(collector_status));
    assert_eq!(libc::WTERMSIG(collector_status), libc::SIGKILL);
    assert!(!Path::new(&format!("/proc/{collector}")).exists());
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 0, 0, 0, 0) },
        0
    );
    drop(guard);
    // Require actual restart on the existing paths, without fixture unlinking.
    r.start();
    r.healthy("#1a1b26");
    assert_ne!(owner, r.snapshot().owner);
    r.stop();
}

#[test]
fn second_live_publisher_cannot_replace_owned_endpoints() {
    let mut r = Realm::new("exclusive-owner");
    r.healthy("#1a1b26");
    let owner = r.snapshot().owner;
    let mut rejected = Command::new(&r.binary)
        .arg("--serve")
        .arg(&r.socket)
        .arg(r.root.join("config.json"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    until(|| rejected.try_wait().unwrap().is_some(), 2);
    assert!(!rejected.wait().unwrap().success());
    assert_eq!(r.snapshot().owner, owner);
    assert_eq!(r.status()["owner"], owner);
    r.stop();
}

#[test]
#[ignore = "entered only by the root-owned UID11010 access control"]
fn app_role_protocol_probe() {
    assert_eq!(rustix::process::getuid().as_raw(), 11010);
    assert_eq!(rustix::process::getgid().as_raw(), 11010);
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    let root = PathBuf::from(std::env::var_os("APPEARANCE_TEST_REALM").unwrap());
    let public = root.join("appearance.sock");
    let snapshot: Snapshot =
        serde_json::from_value(call(&public, json!({"op":"snapshot"})).unwrap()).unwrap();
    assert!(snapshot.available);
    assert_eq!(snapshot.appearance.unwrap().palette.background, "#1a1b26");
    let fd = net::socket_with(
        net::AddressFamily::UNIX,
        net::SocketType::SEQPACKET,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    let denied = net::connect(
        &fd,
        &net::SocketAddrUnix::new(root.join("appearance-control.sock")).unwrap(),
    )
    .unwrap_err();
    assert_eq!(denied, rustix::io::Errno::ACCESS);
    let denied = call(&public, json!({"op":"status"})).unwrap_err();
    assert_eq!(denied.kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn actual_app_uid_reads_public_palette_but_cannot_control_owner() {
    let mut r = Realm::new("app-access");
    r.healthy("#1a1b26");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "app_role_protocol_probe",
            "--nocapture",
        ])
        .env("APPEARANCE_TEST_REALM", &r.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(File::create(r.root.join("app-probe.log")).unwrap());
    // Only async-signal-safe credential operations occur before the child exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setgroups(0, std::ptr::null()) != 0
                || libc::setresgid(11010, 11010, 11010) != 0
                || libc::setresuid(11010, 11010, 11010) != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    until(|| child.try_wait().unwrap().is_some(), 3);
    assert!(child.wait().unwrap().success());
    assert!(r.snapshot().available);
    r.stop();
}

#[test]
fn ignored_sigchld_launcher_cannot_break_reap_and_recovery() {
    let mut r = Realm::new("ignored-sigchld");
    r.healthy("#1a1b26");
    r.stop();
    r.start_with_sigchld(true);
    let end = Instant::now() + Duration::from_secs(3);
    let mut progressed = false;
    let mut last = r.status();
    while Instant::now() < end {
        last = r.status();
        if last["completed"].as_u64().unwrap() >= 2 && r.snapshot().available {
            progressed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    fs::write(
        r.root.join("ignored-sigchld-status.json"),
        serde_json::to_vec(&last).unwrap(),
    )
    .unwrap();
    if progressed {
        r.stop();
    } else {
        // Preserve a failed assertion without abandoning the defective owner.
        let mut child = r.child.take().unwrap();
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
    }
    assert!(
        progressed,
        "actual inherited SIGCHLD left collection/reap permanently occupied: {last}"
    );
}
#[test]
fn public_idle_clients_cannot_block_owner_control() {
    let mut r = Realm::new("idle");
    r.healthy("#1a1b26");
    let mut held = Vec::new();
    for _ in 0..16 {
        let fd = net::socket_with(
            net::AddressFamily::UNIX,
            net::SocketType::SEQPACKET,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
            None,
        )
        .unwrap();
        net::connect(&fd, &net::SocketAddrUnix::new(&r.socket).unwrap()).unwrap();
        held.push(fd);
        std::thread::sleep(Duration::from_millis(8));
    }
    std::thread::sleep(Duration::from_millis(150));
    let control = call(
        &r.root.join("appearance-control.sock"),
        json!({"op":"status"}),
    );
    assert!(
        control.is_ok(),
        "public idle holders must not starve independent owner control: {control:?}"
    );
    r.stop();
    drop(held);
}

#[test]
fn executable_in_app_writable_ancestor_is_refused() {
    let mut r = Realm::new("executable-trust");
    r.healthy("#1a1b26");
    r.stop();
    let mutable = r.root.join("app-writable");
    fs::create_dir(&mutable).unwrap();
    rustix::fs::chown(
        &mutable,
        Some(rustix::process::Uid::from_raw(1000)),
        Some(rustix::process::Gid::from_raw(1000)),
    )
    .unwrap();
    let old = r.binary.clone();
    r.binary = mutable.join("root-owned-executable");
    fs::rename(old, &r.binary).unwrap();
    assert_eq!(r.binary.metadata().unwrap().uid(), 0);
    r.child = Some(
        Command::new(&r.binary)
            .arg("--serve")
            .arg(&r.socket)
            .arg(r.root.join("config.json"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    until(
        || r.socket.exists() || r.child.as_mut().unwrap().try_wait().unwrap().is_some(),
        2,
    );
    assert!(
        !r.socket.exists(),
        "root-owned leaf in an app-writable ancestor must not admit the service"
    );
    assert!(!r.child.take().unwrap().wait().unwrap().success());
}
