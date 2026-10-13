use crate::{
    AdminRequest, AppProfile, AppRequest, Profile, Reply,
    authority::{Authority, Decision, ReadStamp, WriteIntent},
    boottime_ns, ipc,
    storage::{Event, Worker},
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
struct LocalRecovery {
    source: String,
    until: u64,
}
struct Operation {
    token: u64,
    connection: Option<u64>,
    intent: Option<WriteIntent>,
    until: u64,
    retired: bool,
    shutdown: bool,
}
struct Connection {
    id: u64,
    awaiting: Option<u64>,
    fd: OwnedFd,
    admin: bool,
    desktop: bool,
    reading: Option<(u64, ReadStamp)>,
    read_stamp: Option<ReadStamp>,
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
fn launch_registered(
    entry: &Path,
    p: &AppProfile,
    children: &mut BTreeMap<String, Child>,
    authority: &mut Authority,
) -> Reply {
    if !authority.ready() {
        return Reply::Unavailable;
    }
    let std::collections::btree_map::Entry::Vacant(slot) = children.entry(p.app.clone()) else {
        return Reply::Denied;
    };
    match launch(entry, p) {
        Err(_) => Reply::Unavailable,
        Ok(mut child) => {
            let registration = Pid::from_raw(child.id() as i32)
                .ok_or(io::ErrorKind::InvalidInput)
                .and_then(|pid| {
                    rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK)
                        .map_err(|_| io::ErrorKind::Other)
                })
                .and_then(|fd| {
                    authority
                        .register(&p.app, child.id(), fd)
                        .map_err(|_| io::ErrorKind::Other)
                });
            if registration.is_err() {
                authority.revoke(&p.app);
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
}
pub fn run(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
) -> io::Result<()> {
    run_with_messages(runtime, policy, profile_path, entry, test, None)
}
pub fn run_with_messages(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    messages: Option<&mut dyn crate::messages::Provider>,
) -> io::Result<()> {
    run_with_messages_and_session(runtime, policy, profile_path, entry, test, messages, None)
}
fn refresh_session(
    authority: &mut Authority,
    session: &mut Option<&mut dyn crate::session::Provider>,
) -> io::Result<()> {
    if let Some(provider) = session.as_deref_mut() {
        match provider.poll() {
            Ok(Some(s)) => authority.observe_session(s, boottime_ns())?,
            Ok(None) => authority.missing_session()?,
            Err(_) => authority.fail_session()?,
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn run_with_messages_and_session(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    mut messages: Option<&mut dyn crate::messages::Provider>,
    mut session: Option<&mut dyn crate::session::Provider>,
) -> io::Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let m = fs::symlink_metadata(runtime)?;
    if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let profiles = profile(profile_path)?.apps;
    let mut authority = Authority::loading(profiles.clone())?;
    if session.is_some() {
        authority.require_session()?;
    }
    let admin_path = runtime.join("admin.sock");
    let app_path = runtime.join("app.sock");
    let admin = ipc::listener(&admin_path, 0o600)?;
    let app = ipc::listener(&app_path, 0o666)?;
    let desktop_path = runtime.join("launch-messages.sock");
    let desktop = if messages.is_some() {
        Some(ipc::listener(&desktop_path, 0o666)?)
    } else {
        None
    };
    let desktop_identity = desktop
        .as_ref()
        .map(|_| fs::symlink_metadata(&desktop_path).map(|m| (m.dev(), m.ino())))
        .transpose()?;
    let socket_ids: Vec<_> = [&admin_path, &app_path]
        .iter()
        .map(|p| fs::symlink_metadata(p).map(|m| (m.dev(), m.ino())))
        .collect::<io::Result<_>>()?;
    let mut storage = Worker::spawn(runtime, policy, &std::env::current_exe()?, test)?;
    let initial_until = boottime_ns().saturating_add(1_000_000_000);
    let initial_token = storage.start_load(
        profiles.iter().map(|p| p.app.clone()).collect(),
        initial_until,
    )?;
    let mut operation = Some(Operation {
        token: initial_token,
        connection: None,
        intent: None,
        until: initial_until,
        retired: false,
        shutdown: false,
    });
    let mut connection_id = 0u64;
    let mut startup_failed = false;
    let mut children = BTreeMap::<String, Child>::new();
    let mut connections: Vec<Connection> = Vec::new();
    let mut stopping = false;
    let mut cleanup_until = None;
    let mut read_backpressure = 0u64;
    let mut local_recovery: Option<LocalRecovery> = None;
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
            refresh_session(&mut authority, &mut session)?;
            if let Some(op) = &mut operation
                && !op.retired
                && boottime_ns() >= op.until
            {
                authority.invalidate();
                op.retired = true;
                storage.stop();
                for c in &mut connections {
                    if c.awaiting == Some(op.token) {
                        c.awaiting = None;
                        c.pending = Some(serde_json::to_vec(&Reply::Unavailable)?);
                        c.deadline = boottime_ns().saturating_add(1_000_000_000);
                    }
                }
                if op.connection.is_none() && !stopping {
                    startup_failed = true;
                }
            }
            match storage.poll() {
                Ok(Some(event)) => match event {
                    Event::Complete {
                        token,
                        success,
                        state,
                    } => {
                        if let Some(op) = &mut operation {
                            if token != op.token || op.retired {
                                authority.invalidate();
                                storage.stop();
                                op.retired = true;
                            } else if op.shutdown {
                                op.retired = true;
                                storage.stop();
                            } else {
                                let reply = if let Some(intent) = op.intent.take() {
                                    authority.finish(
                                        intent,
                                        success && state.is_none(),
                                        boottime_ns(),
                                    )
                                } else if success && boottime_ns() < op.until {
                                    if let Some(state) = state {
                                        authority.activate(state);
                                        Reply::Ok
                                    } else {
                                        authority.invalidate();
                                        startup_failed = true;
                                        Reply::Unavailable
                                    }
                                } else {
                                    authority.invalidate();
                                    startup_failed = true;
                                    Reply::Unavailable
                                };
                                for c in &mut connections {
                                    if c.id == op.connection.unwrap_or(0)
                                        && c.awaiting == Some(token)
                                    {
                                        c.awaiting = None;
                                        c.pending = Some(serde_json::to_vec(&reply)?);
                                        c.deadline = boottime_ns().saturating_add(1_000_000_000);
                                    }
                                }
                                if authority.is_degraded() {
                                    op.retired = true;
                                    storage.stop();
                                } else {
                                    operation = None;
                                }
                            }
                        } else {
                            authority.invalidate();
                            storage.stop();
                        }
                    }
                    Event::Failed { token } => {
                        authority.invalidate();
                        storage.stop();
                        if let Some(op) = &mut operation {
                            op.retired = true;
                            if op.connection.is_none() && !stopping {
                                startup_failed = true;
                            }
                            for c in &mut connections {
                                if c.awaiting == Some(token) {
                                    c.awaiting = None;
                                    c.pending = Some(serde_json::to_vec(&Reply::Unavailable)?);
                                    c.deadline = boottime_ns().saturating_add(1_000_000_000);
                                }
                            }
                        }
                    }
                    Event::Reaped => {
                        authority.invalidate();
                    }
                },
                Ok(None) => {}
                Err(_) => {
                    authority.invalidate();
                    storage.stop();
                    if let Some(op) = &mut operation {
                        op.retired = true;
                    }
                }
            }
            if let Some(provider) = messages.as_deref_mut() {
                let event = match provider.poll() {
                    Ok(v) => v,
                    Err(_) => {
                        provider.stop();
                        None
                    }
                };
                if let Some(event) = event {
                    let (token, mut packet) = match event {
                        crate::messages::Event::Complete { token, packet } => (token, Some(packet)),
                        crate::messages::Event::Failed { token } => (token, None),
                    };
                    for c in &mut connections {
                        if c.reading.as_ref().is_some_and(|(t, _)| *t == token) {
                            let (_, stamp) = c.reading.take().unwrap();
                            c.read_stamp = packet.as_ref().map(|_| stamp);
                            c.pending = Some(
                                packet
                                    .take()
                                    .filter(|p| !p.is_empty() && p.len() <= crate::MAX_PACKET)
                                    .unwrap_or(serde_json::to_vec(&Reply::Unavailable)?),
                            );
                        }
                    }
                }
            }
            if !storage.occupied() && operation.as_ref().is_some_and(|o| o.retired) {
                operation = None;
            }
            if stopping || startup_failed {
                let until = *cleanup_until
                    .get_or_insert_with(|| boottime_ns().saturating_add(2_000_000_000));
                if boottime_ns() >= until {
                    break;
                }
            }
            if (stopping || startup_failed)
                && children.is_empty()
                && !storage.occupied()
                && messages.as_deref().is_none_or(|p| !p.occupied())
                && connections
                    .iter()
                    .all(|c| c.pending.is_none() && c.awaiting.is_none())
            {
                break;
            }
            let now = boottime_ns();
            let pending_publications = connections
                .iter()
                .filter(|c| c.read_stamp.is_some())
                .count();
            let mut fds = vec![
                PollFd::new(&admin, PollFlags::IN),
                PollFd::new(&app, PollFlags::IN),
            ];
            if let Some(fd) = &desktop {
                fds.push(PollFd::new(fd, PollFlags::IN));
            }
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
            let mut listeners = vec![(&admin, true, false, 4), (&app, false, false, 12)];
            if let Some(fd) = &desktop {
                listeners.push((fd, false, true, 2));
            }
            for (listener, is_admin, is_desktop, limit) in listeners {
                if connections
                    .iter()
                    .filter(|c| c.admin == is_admin && c.desktop == is_desktop)
                    .count()
                    >= limit
                {
                    continue;
                }
                match net::accept_with(listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                    Ok(fd) => {
                        net::sockopt::set_socket_passcred(&fd, true)?;
                        if messages.is_some() && !is_admin && !is_desktop {
                            net::sockopt::set_socket_send_buffer_size(&fd, 8192)?;
                        }
                        connection_id = connection_id.checked_add(1).ok_or(io::ErrorKind::Other)?;
                        connections.push(Connection {
                            id: connection_id,
                            awaiting: None,
                            fd,
                            admin: is_admin,
                            desktop: is_desktop,
                            reading: None,
                            read_stamp: None,
                            deadline: now.saturating_add(30_000_000_000),
                            pending: None,
                        });
                    }
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            connections.retain_mut(|connection| {
                // Drain facts before processing each request/publication, including
                // every retry after EAGAIN. This is ordered ingress, not a barrier
                // against an external compositor change that is not yet observed.
                if refresh_session(&mut authority, &mut session).is_err() {
                    authority.invalidate();
                    return false;
                }
                if boottime_ns() >= connection.deadline {
                    if connection.reading.is_some()
                        && let Some(p) = messages.as_deref_mut()
                    {
                        p.stop();
                    }
                    if connection.awaiting.is_some() {
                        authority.invalidate();
                        storage.stop();
                        if let Some(op) = &mut operation {
                            op.retired = true;
                        }
                    }
                    return false;
                }
                if connection.reading.is_some() {
                    match ipc::receive(&connection.fd) {
                        Ok(None) => return true,
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        _ => {
                            if let Some(p) = messages.as_deref_mut() {
                                p.stop();
                            }
                            return false;
                        }
                    }
                }
                if connection.awaiting.is_some() {
                    match ipc::receive(&connection.fd) {
                        Ok(None) => return true,
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        _ => {
                            authority.invalidate();
                            if let Some(op) = &mut operation {
                                op.retired = true;
                            }
                            storage.stop();
                            return false;
                        }
                    }
                }
                // This check runs immediately before every nonblocking send attempt,
                // including every retry after EAGAIN. Storage owns no read permission.
                if let Some(stamp) = &connection.read_stamp {
                    let current = operation.is_none()
                        && !stopping
                        && messages.as_deref().is_some_and(|p| {
                            p.available()
                                && authority.current_read(stamp, p.incarnation(), boottime_ns())
                        });
                    if !current {
                        connection.pending = serde_json::to_vec(&Reply::Denied).ok();
                        connection.read_stamp = None;
                    }
                }
                if let Some(bytes) = &connection.pending {
                    return match ipc::send(&connection.fd, bytes) {
                        Ok(()) => {
                            connection.pending = None;
                            connection.read_stamp = None;
                            connection.deadline = boottime_ns().saturating_add(30_000_000_000);
                            true
                        }
                        Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                            if e.raw_os_error() == Some(11) && connection.read_stamp.is_some() {
                                read_backpressure = read_backpressure.saturating_add(1);
                            }
                            true
                        }
                        Err(_) => false,
                    };
                }
                let (bytes, cred) = match ipc::receive(&connection.fd) {
                    Ok(Some(v)) => v,
                    Ok(None) => return true,
                    Err(e) if e.raw_os_error() == Some(4) => return true,
                    Err(_) => return false,
                };
                let reply = if connection.desktop {
                    if cred.uid.as_raw() != crate::messages::DESKTOP_UID
                        || stopping
                        || serde_json::from_slice::<crate::messages::DesktopRequest>(&bytes)
                            .is_err()
                    {
                        Reply::Denied
                    } else if let Some(p) = profiles.iter().find(|p| p.app == crate::messages::APP)
                    {
                        let reply = launch_registered(entry, p, &mut children, &mut authority);
                        if reply == Reply::Ok
                            && let Some(provider) = messages.as_deref()
                        {
                            local_recovery = Some(LocalRecovery {
                                source: provider.incarnation().into(),
                                until: boottime_ns().saturating_add(2_000_000_000),
                            });
                        }
                        reply
                    } else {
                        Reply::Unavailable
                    }
                } else if connection.admin {
                    if cred.uid.as_raw() != 0 {
                        Reply::Denied
                    } else {
                        match serde_json::from_slice::<AdminRequest>(&bytes) {
                            Err(_) => Reply::Denied,
                            Ok(AdminRequest::MessagesStatus {}) => match messages.as_deref() {
                                Some(p) => Reply::MessagesStorage {
                                    incoming_started: p.incoming_progress().0,
                                    incoming_raw_committed: p.incoming_progress().1,
                                    incoming_completed: p.incoming_progress().2,
                                    receiving: p.receiving(),
                                    incoming_failure: p.incoming_failure().map(str::to_owned),
                                    occupied: p.occupied(),
                                    busy: p.busy(),
                                    available: p.available(),
                                    pending_publications,
                                    backpressure: read_backpressure,
                                },
                                None => Reply::Unavailable,
                            },
                            Ok(AdminRequest::MessagesRestart {}) => {
                                if stopping {
                                    Reply::Unavailable
                                } else {
                                    match messages.as_deref_mut() {
                                        Some(p) => {
                                            if p.restart().is_ok() {
                                                Reply::Ok
                                            } else {
                                                Reply::Unavailable
                                            }
                                        }
                                        _ => Reply::Unavailable,
                                    }
                                }
                            }
                            Ok(AdminRequest::Snapshot {}) => {
                                authority.snapshot(storage.occupied(), storage.busy())
                            }
                            Ok(AdminRequest::Stop {}) => {
                                stopping = true;
                                if let Some(p) = messages.as_deref_mut() {
                                    p.stop();
                                }
                                cleanup_until.get_or_insert_with(|| {
                                    boottime_ns().saturating_add(2_000_000_000)
                                });
                                let clean =
                                    authority.ready() && operation.is_none() && !storage.busy();
                                authority.invalidate();
                                if clean {
                                    let until = boottime_ns().saturating_add(1_000_000_000);
                                    if let Ok(token) = storage.start_shutdown(until) {
                                        operation = Some(Operation {
                                            token,
                                            connection: None,
                                            intent: None,
                                            until,
                                            retired: false,
                                            shutdown: true,
                                        });
                                    } else {
                                        storage.stop();
                                    }
                                } else {
                                    if let Some(op) = &mut operation {
                                        op.retired = true;
                                    }
                                    storage.stop();
                                }
                                for (app, c) in &mut children {
                                    authority.revoke(app);
                                    let _ = c.kill();
                                }
                                Reply::Ok
                            }
                            Ok(_) if stopping => Reply::Unavailable,
                            Ok(AdminRequest::Launch { app }) => {
                                match profiles.iter().find(|p| p.app == app) {
                                    Some(p) => {
                                        launch_registered(entry, p, &mut children, &mut authority)
                                    }
                                    None => Reply::Denied,
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
                                if operation.is_some() {
                                    authority.invalidate();
                                    if let Some(op) = &mut operation {
                                        op.retired = true;
                                    }
                                    storage.stop();
                                    Reply::Unavailable
                                } else {
                                    match authority.plan_grant(
                                        &app,
                                        permission,
                                        allowed,
                                        boottime_ns(),
                                    ) {
                                        Ok(intent) => {
                                            match storage
                                                .start_write(intent.next.clone(), intent.until)
                                            {
                                                Ok(token) => {
                                                    connection.awaiting = Some(token);
                                                    connection.deadline =
                                                        intent.until.saturating_add(1_000_000_000);
                                                    operation = Some(Operation {
                                                        token,
                                                        connection: Some(connection.id),
                                                        until: intent.until,
                                                        intent: Some(intent),
                                                        retired: false,
                                                        shutdown: false,
                                                    });
                                                    return true;
                                                }
                                                Err(_) => {
                                                    authority.invalidate();
                                                    storage.stop();
                                                    Reply::Unavailable
                                                }
                                            }
                                        }
                                        Err(_) => Reply::Unavailable,
                                    }
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
                        Ok(AppRequest::MessagesRead { request }) if !stopping => {
                            if operation.is_some() {
                                Reply::Unavailable
                            } else if let Some(p) = messages.as_deref_mut() {
                                if !p.available() || p.busy() {
                                    Reply::Unavailable
                                } else {
                                    let now = boottime_ns();
                                    match authority.begin_read(
                                        cred.pid.as_raw_nonzero().get() as u32,
                                        cred.uid.as_raw(),
                                        p.incarnation(),
                                        request.ttl_ms,
                                        now,
                                    ) {
                                        Err(r) => r,
                                        Ok(stamp) => match p.start(
                                            request.query,
                                            now.saturating_add(request.ttl_ms * 1_000_000),
                                        ) {
                                            Ok(token) => {
                                                connection.reading = Some((token, stamp));
                                                connection.deadline =
                                                    now.saturating_add(request.ttl_ms * 1_000_000);
                                                return true;
                                            }
                                            Err(_) => Reply::Unavailable,
                                        },
                                    }
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(r) if !stopping => {
                            if operation.is_some() && !matches!(r, AppRequest::Query {}) {
                                authority.refuse_busy(
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    r,
                                )
                            } else {
                                match authority.plan_request(
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    r,
                                    boottime_ns(),
                                ) {
                                    Decision::Immediate(r) => r,
                                    Decision::Persist(intent) => match storage
                                        .start_write(intent.next.clone(), intent.until)
                                    {
                                        Ok(token) => {
                                            connection.awaiting = Some(token);
                                            connection.deadline =
                                                intent.until.saturating_add(1_000_000_000);
                                            operation = Some(Operation {
                                                token,
                                                connection: Some(connection.id),
                                                until: intent.until,
                                                intent: Some(intent),
                                                retired: false,
                                                shutdown: false,
                                            });
                                            return true;
                                        }
                                        Err(_) => {
                                            authority.invalidate();
                                            storage.stop();
                                            Reply::Unavailable
                                        }
                                    },
                                }
                            }
                        }
                        _ => Reply::Denied,
                    }
                };
                connection.pending = serde_json::to_vec(&reply).ok();
                connection.deadline = boottime_ns().saturating_add(1_000_000_000);
                connection.pending.is_some()
            });
            // A successful fixed launch records one source-bound local intent.
            // Decide only after every connection retirement in this pass: an
            // older desktop connection can precede the dead app's read socket.
            if let Some(intent) = &local_recovery {
                if let Some(provider) = messages.as_deref_mut() {
                    let now = boottime_ns();
                    let invalid_read = connections.iter().any(|c| {
                        c.reading.as_ref().is_some_and(|(_, stamp)| {
                            !authority.current_read(stamp, provider.incarnation(), now)
                        })
                    });
                    if stopping
                        || !authority.ready()
                        || now >= intent.until
                        || provider.incarnation() != intent.source
                        || (provider.available() && !invalid_read)
                    {
                        local_recovery = None;
                    } else if !provider.occupied() {
                        // Consume before the single attempt, and never replace
                        // an occupied child or replay the old read request.
                        local_recovery = None;
                        let _ = provider.restart();
                    }
                } else {
                    local_recovery = None;
                }
            }
        }
        if startup_failed {
            Err(io::ErrorKind::InvalidData.into())
        } else {
            Ok(())
        }
    })();
    authority.invalidate();
    let until = cleanup_until.unwrap_or_else(|| boottime_ns().saturating_add(2_000_000_000));
    // Fence before every signal, and retain both child sets through actual reaping.
    for (name, child) in &mut children {
        authority.revoke(name);
        let _ = child.kill();
    }
    if messages.is_some_and(|p| !p.drain_until(until)) {
        std::process::abort();
    }
    if !storage.drain_until(until) {
        std::process::abort();
    }
    while !children.is_empty() {
        if boottime_ns() >= until {
            std::process::abort();
        }
        let mut failed = false;
        children.retain(|_, child| match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => {
                failed = true;
                true
            }
        });
        if failed {
            std::process::abort();
        }
        if !children.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    for (path, identity) in [&admin_path, &app_path].into_iter().zip(socket_ids) {
        if fs::symlink_metadata(path).is_ok_and(|m| (m.dev(), m.ino()) == identity) {
            fs::remove_file(path)?;
        }
    }
    if desktop_identity.is_some_and(|identity| {
        fs::symlink_metadata(&desktop_path).is_ok_and(|m| (m.dev(), m.ino()) == identity)
    }) {
        fs::remove_file(desktop_path)?;
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
