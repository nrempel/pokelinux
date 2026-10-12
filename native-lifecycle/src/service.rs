use crate::{
    AdminRequest, AppProfile, AppRequest, Profile, Reply, authority::Authority, boottime_ns, ipc,
};
use rustix::{
    event::{PollFd, PollFlags},
    fd::OwnedFd,
    net::{self, SocketFlags},
    process::{Pid, PidfdFlags},
};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub fn trusted_executable(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file()
        || meta.uid() != 0
        || meta.mode() & 0o6022 != 0
        || meta.mode() & 0o111 == 0
        || !path.is_absolute()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // Every containing directory belongs to the installer and cannot be app-mutated.
    for directory in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(directory)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}
pub fn profile(path: &Path) -> io::Result<Profile> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut data = Vec::new();
    fs::File::open(path)?.take(16385).read_to_end(&mut data)?;
    if data.len() > 16384 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let profile: Profile = serde_json::from_slice(&data)?;
    if profile.apps.is_empty() || profile.apps.len() > crate::MAX_APPS {
        return Err(io::ErrorKind::InvalidData.into());
    }
    for (i, p) in profile.apps.iter().enumerate() {
        if !(10000..=60000).contains(&p.uid)
            || p.app.is_empty()
            || p.app.len() > 64
            || !p
                .app
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            || p.args.len() > 16
            || p.args.iter().any(|a| a.len() > 256)
            || profile.apps[..i]
                .iter()
                .any(|q| q.app == p.app || q.uid == p.uid)
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        trusted_executable(Path::new(&p.executable))?;
    }
    Ok(profile)
}
struct Connection {
    fd: OwnedFd,
    admin: bool,
    deadline: u64,
    pending: Option<Vec<u8>>,
}
fn launch(entry: &Path, p: &AppProfile) -> io::Result<Child> {
    trusted_executable(entry)?;
    trusted_executable(Path::new(&p.executable))?;
    Command::new(entry)
        .arg("app-entry")
        .arg(p.uid.to_string())
        .arg(std::process::id().to_string())
        .arg(&p.executable)
        .args(&p.args)
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
pub fn run(runtime: &Path, policy: &Path, profile_path: &Path, entry: &Path) -> io::Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let m = fs::symlink_metadata(runtime)?;
    if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let profiles = profile(profile_path)?.apps;
    let mut authority = Authority::open(profiles.clone(), policy)?;
    let admin_path = runtime.join("admin.sock");
    let app_path = runtime.join("app.sock");
    let admin = ipc::listener(&admin_path, 0o600)?;
    let app = ipc::listener(&app_path, 0o666)?;
    let socket_ids: Vec<_> = [&admin_path, &app_path]
        .iter()
        .map(|p| fs::symlink_metadata(p).map(|m| (m.dev(), m.ino())))
        .collect::<io::Result<_>>()?;
    let mut children = BTreeMap::<String, Child>::new();
    let mut connections: Vec<Connection> = Vec::new();
    let mut stopping = false;
    let outcome = (|| {
        loop {
            // Never block on child wait, request IO, or an idle client.
            let dead: Vec<_> = children
                .iter_mut()
                .filter_map(|(app, c)| match c.try_wait() {
                    Ok(Some(_)) => Some(Ok(app.clone())),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                })
                .collect::<io::Result<_>>()?;
            for name in dead {
                authority.revoke(&name);
                children.remove(&name);
            }
            authority.reap_dead()?;
            if stopping && children.is_empty() {
                break;
            }
            let now = boottime_ns();
            let mut fds = vec![
                PollFd::new(&admin, PollFlags::IN),
                PollFd::new(&app, PollFlags::IN),
            ];
            for c in &connections {
                fds.push(PollFd::new(
                    &c.fd,
                    if c.pending.is_some() {
                        PollFlags::OUT
                    } else {
                        PollFlags::IN
                    },
                ));
            }
            match rustix::event::poll(
                &mut fds,
                Some(&rustix::event::Timespec {
                    tv_sec: 0,
                    tv_nsec: 20_000_000,
                }),
            ) {
                Ok(_) | Err(rustix::io::Errno::INTR) => {}
                Err(e) => return Err(e.into()),
            }
            drop(fds);
            for (listener, is_admin, limit) in [(&admin, true, 4), (&app, false, 12)] {
                if connections.iter().filter(|c| c.admin == is_admin).count() >= limit {
                    continue;
                }
                match net::accept_with(listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                    Ok(fd) => {
                        net::sockopt::set_socket_passcred(&fd, true)?;
                        connections.push(Connection {
                            fd,
                            admin: is_admin,
                            deadline: now.saturating_add(30_000_000_000),
                            pending: None,
                        });
                    }
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            connections.retain_mut(|connection| {
                if boottime_ns() >= connection.deadline {
                    return false;
                }
                if let Some(bytes) = &connection.pending {
                    return match ipc::send(&connection.fd, bytes) {
                        Ok(()) => {
                            connection.pending = None;
                            connection.deadline = boottime_ns().saturating_add(30_000_000_000);
                            true
                        }
                        Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => true,
                        Err(_) => false,
                    };
                }
                let (bytes, cred) = match ipc::receive(&connection.fd) {
                    Ok(Some(v)) => v,
                    Ok(None) => return true,
                    Err(e) if e.raw_os_error() == Some(4) => return true,
                    Err(_) => return false,
                };
                let reply = if connection.admin {
                    if cred.uid.as_raw() != 0 {
                        Reply::Denied
                    } else {
                        match serde_json::from_slice::<AdminRequest>(&bytes) {
                            Err(_) => Reply::Denied,
                            Ok(AdminRequest::Snapshot {}) => authority.snapshot(),
                            Ok(AdminRequest::Stop {}) => {
                                stopping = true;
                                for (app, c) in &mut children {
                                    authority.revoke(app);
                                    let _ = c.kill();
                                }
                                Reply::Ok
                            }
                            Ok(_) if stopping => Reply::Unavailable,
                            Ok(AdminRequest::Launch { app }) => {
                                if authority.is_degraded() {
                                    Reply::Unavailable
                                } else if let std::collections::btree_map::Entry::Vacant(slot) =
                                    children.entry(app.clone())
                                {
                                    if let Some(p) = profiles.iter().find(|p| p.app == app) {
                                        match launch(entry, p) {
                                            Err(_) => Reply::Unavailable,
                                            Ok(mut child) => {
                                                let pid = child.id();
                                                let registration = Pid::from_raw(pid as i32)
                                                    .ok_or_else(|| {
                                                        io::Error::from(io::ErrorKind::InvalidInput)
                                                    })
                                                    .and_then(|p| {
                                                        rustix::process::pidfd_open(
                                                            p,
                                                            PidfdFlags::NONBLOCK,
                                                        )
                                                        .map_err(Into::into)
                                                    })
                                                    .and_then(|fd| {
                                                        authority.register(&app, pid, fd)
                                                    });
                                                if registration.is_err() {
                                                    authority.revoke(&app);
                                                    let _ = child.kill();
                                                }
                                                slot.insert(child);
                                                if registration.is_ok() {
                                                    Reply::Ok
                                                } else {
                                                    Reply::Unavailable
                                                }
                                            }
                                        }
                                    } else {
                                        Reply::Denied
                                    }
                                } else {
                                    Reply::Denied
                                }
                            }
                            Ok(AdminRequest::Terminate { app }) => {
                                authority.revoke(&app);
                                match children.get_mut(&app) {
                                    Some(c) => {
                                        if c.kill().is_ok() {
                                            Reply::Ok
                                        } else {
                                            Reply::Unavailable
                                        }
                                    }
                                    None => Reply::Denied,
                                }
                            }
                            Ok(AdminRequest::Grant {
                                app,
                                permission,
                                allowed,
                            }) => {
                                if authority.grant(&app, permission, allowed).is_ok() {
                                    Reply::Ok
                                } else {
                                    Reply::Unavailable
                                }
                            }
                            Ok(AdminRequest::Background { app, background }) => {
                                if authority
                                    .lifecycle(None, None, Some((&app, background)))
                                    .is_ok()
                                {
                                    Reply::Ok
                                } else {
                                    Reply::Denied
                                }
                            }
                            Ok(AdminRequest::Locked { locked }) => {
                                if authority.lifecycle(Some(locked), None, None).is_ok() {
                                    Reply::Ok
                                } else {
                                    Reply::Unavailable
                                }
                            }
                            Ok(AdminRequest::Suspended { suspended }) => {
                                if authority.lifecycle(None, Some(suspended), None).is_ok() {
                                    Reply::Ok
                                } else {
                                    Reply::Unavailable
                                }
                            }
                        }
                    }
                } else {
                    match serde_json::from_slice::<AppRequest>(&bytes) {
                        Ok(r) if !stopping => authority.request(
                            cred.pid.as_raw_nonzero().get() as u32,
                            cred.uid.as_raw(),
                            r,
                            boottime_ns,
                        ),
                        _ => Reply::Denied,
                    }
                };
                connection.pending = serde_json::to_vec(&reply).ok();
                connection.deadline = boottime_ns().saturating_add(1_000_000_000);
                connection.pending.is_some()
            });
        }
        Ok(())
    })();
    if !children.is_empty() {
        // Fail closed without relinquishing live children. Fatal IO cannot detach them.
        for (name, child) in &mut children {
            authority.revoke(name);
            let _ = child.kill();
        }
        loop {
            children.retain(|_, child| !matches!(child.try_wait(), Ok(Some(_))));
            if children.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    for (path, identity) in [&admin_path, &app_path].into_iter().zip(socket_ids) {
        if fs::symlink_metadata(path).is_ok_and(|m| (m.dev(), m.ino()) == identity) {
            fs::remove_file(path)?;
        }
    }
    outcome
}

pub fn entry(args: &[String]) -> io::Result<()> {
    use std::os::unix::process::CommandExt;
    let uid: u32 = args
        .first()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let parent: u32 = args
        .get(1)
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let executable = PathBuf::from(args.get(2).ok_or(io::ErrorKind::InvalidInput)?);
    if !(10000..=60000).contains(&uid) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    trusted_executable(&executable)?;
    rustix::thread::set_no_new_privs(true)?;
    rustix::thread::set_thread_groups(&[])?;
    rustix::thread::set_thread_res_gid(
        rustix::process::Gid::from_raw(uid),
        rustix::process::Gid::from_raw(uid),
        rustix::process::Gid::from_raw(uid),
    )?;
    rustix::thread::set_thread_res_uid(
        rustix::process::Uid::from_raw(uid),
        rustix::process::Uid::from_raw(uid),
        rustix::process::Uid::from_raw(uid),
    )?;
    // Credential changes clear PDEATHSIG; arm it after all changes, then check race.
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))?;
    if rustix::process::getppid().map(|p| p.as_raw_nonzero().get() as u32) != Some(parent) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Err(Command::new(executable)
        .args(&args[3..])
        .env_clear()
        .current_dir("/")
        .exec())
}
