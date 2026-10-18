use crate::{
    AdminRequest, AppRequest, Profile, Reply,
    authority::{Authority, CaptureStamp, Decision, ReadStamp, WriteIntent},
    boottime_ns, ipc,
    storage::{Event, Worker},
};
use rustix::{
    event::{PollFd, PollFlags},
    fd::OwnedFd,
    net::{self, SocketFlags},
};
use std::{
    fs,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

fn inspect_brightness(
    provider: &mut dyn crate::brightness::Provider,
    actor_pending: bool,
    stopping: bool,
    inspected_at: &mut u64,
) {
    if !stopping && !actor_pending && boottime_ns().saturating_sub(*inspected_at) >= 250_000_000 {
        *inspected_at = boottime_ns();
        let _ = provider.inspect(inspected_at.saturating_add(1_000_000_000));
    }
}

// Brightness uses the same actual request-half EOF/extra-packet boundary.
pub(crate) fn brightness_client_input(
    brightness: &mut crate::brightness_session::Session,
    connection: u64,
    fd: &OwnedFd,
    provider: &mut dyn crate::brightness::Provider,
) -> io::Result<()> {
    if !brightness.pending_on(connection) {
        return Ok(());
    }
    match ipc::receive(fd) {
        Ok(None) => Ok(()),
        Err(e) if e.raw_os_error() == Some(4) => Err(e),
        result => {
            brightness.retire(provider);
            match result {
                Err(e) => Err(e),
                _ => Err(io::ErrorKind::InvalidData.into()),
            }
        }
    }
}

// Check the exact pending request stream before dispatch/publication. A successful
// send alone cannot establish that the client kept its request half alive.
fn media_client_input(
    media: &mut crate::audio_session::Session,
    connection: u64,
    fd: &OwnedFd,
    provider: &mut dyn crate::audio::Provider,
) -> io::Result<()> {
    if !media.pending_on(connection) {
        return Ok(());
    }
    match ipc::receive(fd) {
        Ok(None) => Ok(()),
        Err(e) if e.raw_os_error() == Some(4) => Err(e),
        result => {
            media.retire(provider);
            match result {
                Err(e) => Err(e),
                _ => Err(io::ErrorKind::InvalidData.into()),
            }
        }
    }
}

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
    camera: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopEndpoint {
    Messages,
    Sounds,
    Settings,
}
#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum SoundsDesktopRequest {
    LaunchSounds {},
}
#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum SettingsDesktopRequest {
    LaunchSettings {},
}
struct Connection {
    id: u64,
    awaiting: Option<u64>,
    fd: OwnedFd,
    admin: bool,
    desktop: Option<DesktopEndpoint>,
    lock: bool,
    reading: Option<(u64, ReadStamp)>,
    read_stamp: Option<ReadStamp>,
    deadline: u64,
    pending: Option<Vec<u8>>,
    // Set only with typed Ok on the lock-only route. Retain storage admission
    // identity even if an intervening write finishes before the next send.
    lock_ack: Option<u64>,
    launching: Option<String>,
}
fn queue_media_reply(connections: &mut [Connection], id: u64, reply: Reply) -> io::Result<()> {
    if let Some(connection) = connections.iter_mut().find(|c| c.id == id) {
        connection.pending = Some(serde_json::to_vec(&reply)?);
        connection.deadline = boottime_ns().saturating_add(1_000_000_000);
    }
    Ok(())
}

fn revalidate_lock_ack(
    connection: &mut Connection,
    stopping: bool,
    authority: &Authority,
    operation: &Option<Operation>,
    storage: &Worker,
) {
    if let Some(admission) = connection.lock_ack
        && (stopping
            || !authority.ready()
            || operation.is_some()
            || storage.busy()
            || authority.required_lock_write_pending()
            || admission != storage.admission_sequence())
    {
        // Fence once before the send. A blocked Unavailable reply
        // must never turn back into Ok when storage later clears.
        connection.pending = serde_json::to_vec(&Reply::Unavailable).ok();
        connection.lock_ack = None;
    }
}

// One scheduler uses the sole Worker. A known superseded false commit gives
// priority to its required true successor before any other publication/ingress.
fn schedule_restoration(
    authority: &mut Authority,
    storage: &mut Worker,
    operation: &mut Option<Operation>,
    stopping: bool,
) -> bool {
    if stopping || !authority.ready() || operation.is_some() || storage.busy() {
        return true;
    }
    let now = boottime_ns();
    let intent = authority.take_required_lock_write(now).or_else(|| {
        match authority.plan_authenticated_clear(now) {
            Decision::Persist(intent) => Some(intent),
            Decision::Immediate(_) => None,
        }
    });
    if !authority.ready() {
        storage.stop();
        return false;
    }
    let Some(intent) = intent else {
        return true;
    };
    match storage.start_write(intent.next.clone(), intent.until) {
        Ok(token) => {
            *operation = Some(Operation {
                token,
                connection: None,
                until: intent.until,
                intent: Some(intent),
                retired: false,
                shutdown: false,
                camera: false,
            });
            true
        }
        Err(_) => {
            authority.invalidate();
            storage.stop();
            false
        }
    }
}

#[derive(PartialEq)]
enum CapturePhase {
    Persisting,
    Dispatching,
    Submitted,
    ReplyReady,
    Published,
}
struct Capture {
    connection: u64,
    stamp: CaptureStamp,
    phase: CapturePhase,
}
fn revoke_capture(
    capture: &mut Option<Capture>,
    camera: &mut Option<&mut dyn crate::camera::Provider>,
) {
    if let Some(capture) = capture.take()
        && let Some(provider) = camera.as_deref_mut()
    {
        // Fence the actor's publication first. Actual child occupancy belongs
        // to the provider until its parent wait; no reset/cleanup follows.
        match capture.phase {
            // The backend has no accepted capture ID yet. Retire the owned
            // ready child even if persistence or the actual send is blocked.
            CapturePhase::Persisting | CapturePhase::Dispatching => provider.stop(),
            _ => provider.revoke(capture.stamp.intent().capture),
        }
    }
}
fn current_capture(
    capture: &Capture,
    authority: &Authority,
    camera: &Option<&mut dyn crate::camera::Provider>,
) -> bool {
    camera.as_deref().is_some_and(|provider| {
        authority.current_capture(
            &capture.stamp,
            provider.incarnation(),
            provider.generation(),
            boottime_ns(),
        ) && match capture.phase {
            CapturePhase::Persisting | CapturePhase::Dispatching => {
                provider.available() && !provider.busy()
            }
            _ => provider.current(capture.stamp.intent().capture),
        }
    })
}
fn complete_launches(
    completed: Vec<crate::apps::Completion>,
    connections: &mut [Connection],
    recovery: &mut Option<LocalRecovery>,
    messages: &Option<&mut dyn crate::messages::Provider>,
) -> io::Result<()> {
    for completion in completed {
        if let Some(connection) = connections.iter_mut().find(|c| {
            c.id == completion.connection
                && c.launching.as_deref() == Some(completion.instance.as_str())
        }) {
            connection.pending = Some(serde_json::to_vec(&completion.reply)?);
            if completion.reply == Reply::Ok
                && completion.app == crate::messages::APP
                && let Some(provider) = messages.as_deref()
            {
                *recovery = Some(LocalRecovery {
                    source: provider.incarnation().into(),
                    until: boottime_ns().saturating_add(2_000_000_000),
                });
            }
        }
    }
    Ok(())
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
// Mark intentional retirement before the existing Source stop; no new failure cause.
fn stop_session(authority: &mut Authority, provider: &mut dyn crate::session::Provider) {
    authority.source_stopping();
    provider.stop();
}
#[allow(clippy::too_many_arguments)]
fn begin_stop(
    authority: &mut Authority,
    apps: &mut crate::apps::Apps,
    storage: &mut Worker,
    operation: &mut Option<Operation>,
    messages: &mut Option<&mut dyn crate::messages::Provider>,
    session: &mut Option<&mut dyn crate::session::Provider>,
    startup_failed: bool,
    stopping: &mut bool,
    stop_reply: &mut Option<Reply>,
    cleanup_until: &mut Option<u64>,
) -> Reply {
    if let Some(reply) = stop_reply.as_ref() {
        // The first Stop owns the operation, deadline and acknowledgement truth.
        return reply.clone();
    }
    *stopping = true;
    let cleanup = *cleanup_until.get_or_insert_with(|| boottime_ns().saturating_add(2_000_000_000));
    if let Some(provider) = messages.as_deref_mut() {
        provider.stop();
    }
    let clean = !startup_failed
        && authority.ready()
        && operation.is_none()
        && !storage.busy()
        && !authority.required_lock_write_pending();
    authority.invalidate();
    let app_stop = if startup_failed {
        apps.stop(authority)
    } else {
        apps.normal_stop(authority, cleanup)
    };
    if let Some(provider) = session.as_deref_mut() {
        stop_session(authority, provider);
    }
    if clean {
        let until = boottime_ns().saturating_add(1_000_000_000).min(cleanup);
        if let Ok(token) = storage.start_shutdown(until) {
            *operation = Some(Operation {
                token,
                connection: None,
                intent: None,
                until,
                retired: false,
                shutdown: true,
                camera: false,
            });
        } else {
            storage.stop();
        }
    } else {
        if let Some(operation) = operation.as_mut() {
            operation.retired = true;
        }
        storage.stop();
    }
    let reply = if app_stop.is_ok() {
        Reply::Ok
    } else {
        Reply::Unavailable
    };
    *stop_reply = Some(reply.clone());
    reply
}
fn refresh_session(
    authority: &mut Authority,
    session: &mut Option<&mut dyn crate::session::Provider>,
) -> io::Result<()> {
    if let Some(provider) = session.as_deref_mut() {
        match provider.poll() {
            Ok(Some(s)) => authority.observe_session(s, boottime_ns())?,
            Ok(None) => authority.missing_session()?,
            Err(error) => {
                authority.source_provider_failed(error.kind(), provider.first_rejection().cloned());
                authority.fail_session()?;
                provider.stop();
            }
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
    messages: Option<&mut dyn crate::messages::Provider>,
    session: Option<&mut dyn crate::session::Provider>,
) -> io::Result<()> {
    // Headless service/test entrypoints use the same actor slot machinery;
    // installed GUI provisioning supplies a display Factory explicitly.
    run_with_services(
        runtime,
        policy,
        profile_path,
        entry,
        test,
        messages,
        session,
        None,
        None,
        None,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn run_with_services(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    messages: Option<&mut dyn crate::messages::Provider>,
    session: Option<&mut dyn crate::session::Provider>,
    display: Option<&mut dyn crate::display::Factory>,
    camera: Option<&mut dyn crate::camera::Provider>,
    audio: Option<&mut dyn crate::audio::Provider>,
    startup: Option<&mut dyn crate::startup::Observer>,
) -> io::Result<()> {
    run_with_services_inner(
        runtime,
        policy,
        profile_path,
        entry,
        test,
        messages,
        session,
        display,
        camera,
        audio,
        startup,
        #[cfg(test)]
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn run_with_services_inner(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    messages: Option<&mut dyn crate::messages::Provider>,
    session: Option<&mut dyn crate::session::Provider>,
    display: Option<&mut dyn crate::display::Factory>,
    camera: Option<&mut dyn crate::camera::Provider>,
    audio: Option<&mut dyn crate::audio::Provider>,
    startup: Option<&mut dyn crate::startup::Observer>,
    #[cfg(test)] restore: Option<&mut restore_tests::ActorControl>,
) -> io::Result<()> {
    run_with_brightness_services_inner(
        runtime,
        policy,
        profile_path,
        entry,
        test,
        messages,
        session,
        display,
        camera,
        audio,
        None,
        startup,
        #[cfg(test)]
        restore,
    )
}
/// Explicit provider configuration; ordinary entrypoints supply no brightness authority.
#[allow(clippy::too_many_arguments)]
pub fn run_with_brightness_services(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    messages: Option<&mut dyn crate::messages::Provider>,
    session: Option<&mut dyn crate::session::Provider>,
    display: Option<&mut dyn crate::display::Factory>,
    camera: Option<&mut dyn crate::camera::Provider>,
    audio: Option<&mut dyn crate::audio::Provider>,
    brightness: Option<&mut dyn crate::brightness::Provider>,
    startup: Option<&mut dyn crate::startup::Observer>,
) -> io::Result<()> {
    run_with_brightness_services_inner(
        runtime,
        policy,
        profile_path,
        entry,
        test,
        messages,
        session,
        display,
        camera,
        audio,
        brightness,
        startup,
        #[cfg(test)]
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn run_with_brightness_services_inner(
    runtime: &Path,
    policy: &Path,
    profile_path: &Path,
    entry: &Path,
    test: Option<&str>,
    mut messages: Option<&mut dyn crate::messages::Provider>,
    mut session: Option<&mut dyn crate::session::Provider>,
    mut display: Option<&mut dyn crate::display::Factory>,
    mut camera: Option<&mut dyn crate::camera::Provider>,
    mut audio: Option<&mut dyn crate::audio::Provider>,
    mut brightness_provider: Option<&mut dyn crate::brightness::Provider>,
    mut startup: Option<&mut dyn crate::startup::Observer>,
    #[cfg(test)] mut restore: Option<&mut restore_tests::ActorControl>,
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
    let lock_path = runtime.join("lock.sock");
    let admin = ipc::listener(&admin_path, 0o600)?;
    let app = ipc::listener(&app_path, 0o666)?;
    // This route belongs to the lifecycle actor, independently of Messages.
    let lock = ipc::listener(&lock_path, 0o666)?;
    // Fixed Sounds ingress exists independently of the optional Messages provider.
    let sounds_path = runtime.join("launch-sounds.sock");
    let sounds = ipc::listener(&sounds_path, 0o666)?;
    let sounds_identity = {
        let metadata = fs::symlink_metadata(&sounds_path)?;
        (metadata.dev(), metadata.ino())
    };
    // Fixed Settings ingress does not depend on a selected brightness backend.
    let settings_path = runtime.join("launch-settings.sock");
    let settings = ipc::listener(&settings_path, 0o666)?;
    let settings_identity = {
        let metadata = fs::symlink_metadata(&settings_path)?;
        (metadata.dev(), metadata.ino())
    };
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
    let socket_ids: Vec<_> = [&admin_path, &app_path, &lock_path]
        .iter()
        .map(|p| fs::symlink_metadata(p).map(|m| (m.dev(), m.ino())))
        .collect::<io::Result<_>>()?;
    let storage_executable = std::env::current_exe()?;
    #[cfg(test)]
    let storage_executable = restore.as_ref().map_or(storage_executable, |control| {
        control.storage_executable.clone()
    });
    let mut storage = Worker::spawn(runtime, policy, &storage_executable, test)?;
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
        camera: false,
    });
    let mut connection_id = 0u64;
    let mut startup_failed = false;
    let mut apps = crate::apps::Apps::default();
    #[cfg(test)]
    if let Some(control) = restore.as_ref() {
        apps.wait_observer = Some(control.app_wait_observer.clone());
    }
    let mut connections: Vec<Connection> = Vec::new();
    let mut stopping = false;
    let mut stop_reply: Option<Reply> = None;
    let mut cleanup_until = None;
    let mut read_backpressure = 0u64;
    let mut local_recovery: Option<LocalRecovery> = None;
    let mut capture: Option<Capture> = None;
    let mut audio_inspected_at = 0u64;
    let mut media = crate::audio_session::Session::default();
    let mut brightness = crate::brightness_session::Session::default();
    let mut brightness_inspected_at = 0u64;
    let mut outcome = (|| {
        loop {
            // The signal handler only latches intent. The actor owns the same
            // Stop/fence/drain operation as authenticated admin Stop, including
            // startup with an outstanding policy load and no accepted Source.
            if pokelinux_kernel_identity::termination_requested() {
                begin_stop(
                    &mut authority,
                    &mut apps,
                    &mut storage,
                    &mut operation,
                    &mut messages,
                    &mut session,
                    startup_failed,
                    &mut stopping,
                    &mut stop_reply,
                    &mut cleanup_until,
                );
            }
            // Never block on child wait, request IO, or an idle client.
            authority.reap_dead()?;
            refresh_session(&mut authority, &mut session)?;
            if let Some(observer) = startup.as_deref_mut() {
                let until = if stopping {
                    None
                } else {
                    authority.startup_source_until(boottime_ns())
                };
                observer.poll(until, stopping)?;
            }
            #[cfg(test)]
            if let Some(control) = restore.as_deref_mut() {
                control.poll(
                    &mut authority,
                    operation.is_some() || storage.busy(),
                    stopping,
                )?;
            }
            complete_launches(
                apps.poll(&mut authority)?,
                &mut connections,
                &mut local_recovery,
                &messages,
            )?;
            if let Some(provider) = audio.as_deref_mut() {
                if let Some((id, reply)) = media.fence(&authority, provider, stopping, |id| {
                    connections.iter().any(|c| c.id == id)
                }) {
                    queue_media_reply(&mut connections, id, reply)?;
                }
                if let Some(event) = provider.poll()
                    && let Some((id, reply)) = media.observe(event, &authority, provider)
                {
                    queue_media_reply(&mut connections, id, reply)?;
                }
                // Inspection never dispatches an effect. Do not starve an
                // existing unsent selection by starting another read request.
                if !stopping
                    && !media.pending()
                    && provider.state().connected
                    && boottime_ns().saturating_sub(audio_inspected_at) >= 250_000_000
                {
                    audio_inspected_at = boottime_ns();
                    let _ = provider.inspect(audio_inspected_at.saturating_add(1_000_000_000));
                }
            }
            if let Some(provider) = brightness_provider.as_deref_mut() {
                if let Some((id, reply)) = brightness.fence(&authority, provider, stopping, |id| {
                    connections.iter().any(|c| c.id == id)
                }) {
                    queue_media_reply(&mut connections, id, reply)?;
                }
                if let Some(event) = provider.poll()
                    && let Some((id, reply)) = brightness.observe(event, &authority, provider)
                {
                    queue_media_reply(&mut connections, id, reply)?;
                }
                // Readonly reconciliation may establish a fresh connection, but
                // never dispatches or replays a Set, including after revoke.
                inspect_brightness(
                    provider,
                    brightness.pending(),
                    stopping,
                    &mut brightness_inspected_at,
                );
            }
            if capture.as_ref().is_some_and(|active| {
                stopping
                    || !current_capture(active, &authority, &camera)
                    || !connections.iter().any(|c| c.id == active.connection)
            }) {
                let connection = capture.as_ref().unwrap().connection;
                revoke_capture(&mut capture, &mut camera);
                if let Some(c) = connections.iter_mut().find(|c| c.id == connection)
                    && c.awaiting.is_none()
                {
                    c.pending = Some(serde_json::to_vec(&Reply::Denied)?);
                }
            }
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
                                        if let Some(active) = &mut capture
                                            && op.camera
                                            && active.connection == c.id
                                            && active.phase == CapturePhase::Persisting
                                            && matches!(reply, Reply::AdmittedUncertain { .. })
                                        {
                                            // Durable history is retained, but only the
                                            // still-owned stamp can authorize dispatch.
                                            active.phase = CapturePhase::Dispatching;
                                            c.deadline = active
                                                .stamp
                                                .intent()
                                                .until_ns
                                                .saturating_add(1_000_000_000);
                                        } else {
                                            c.pending = Some(serde_json::to_vec(&if op.camera {
                                                Reply::Unavailable
                                            } else {
                                                reply.clone()
                                            })?);
                                            c.lock_ack = (c.lock && reply == Reply::Ok)
                                                .then(|| storage.admission_sequence());
                                            c.deadline =
                                                boottime_ns().saturating_add(1_000_000_000);
                                            if capture
                                                .as_ref()
                                                .is_some_and(|s| s.connection == c.id)
                                            {
                                                revoke_capture(&mut capture, &mut camera);
                                            }
                                        }
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
            if !schedule_restoration(
                &mut authority,
                &mut storage,
                &mut operation,
                stopping || startup_failed,
            ) {
                startup_failed = true;
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
            if let Some(provider) = camera.as_deref_mut() {
                let event = match provider.poll() {
                    Ok(event) => event,
                    Err(_) => {
                        provider.stop();
                        None
                    }
                };
                if let Some(event) = event {
                    let (id, state) = match event {
                        crate::camera::Event::Complete { capture, state } => (capture, Some(state)),
                        crate::camera::Event::Failed { capture } => (capture, None),
                    };
                    if let Some(active) = &mut capture
                        && active.phase == CapturePhase::Submitted
                        && active.stamp.intent().capture == id
                    {
                        if let Some(c) = connections.iter_mut().find(|c| c.id == active.connection)
                        {
                            c.pending = Some(serde_json::to_vec(&match state {
                                Some(state) => Reply::CameraState { state },
                                None => Reply::Unavailable,
                            })?);
                            active.phase = CapturePhase::ReplyReady;
                        }
                    } else {
                        // Old/unsolicited callbacks never establish a new stamp.
                        provider.stop();
                    }
                }
            }
            if !storage.occupied() && operation.as_ref().is_some_and(|o| o.retired) {
                operation = None;
            }
            if stopping || startup_failed {
                authority.invalidate();
                let until = *cleanup_until
                    .get_or_insert_with(|| boottime_ns().saturating_add(2_000_000_000));
                if startup_failed {
                    apps.stop(&mut authority)?;
                } else {
                    apps.normal_stop(&mut authority, until)?;
                }
                if let Some(provider) = session.as_deref_mut() {
                    provider.stop();
                }
                revoke_capture(&mut capture, &mut camera);
                if let Some(provider) = camera.as_deref_mut() {
                    provider.stop();
                }
                if boottime_ns() >= until {
                    break;
                }
            }
            if (stopping || startup_failed)
                && !apps.occupied()
                && !storage.occupied()
                && messages.as_deref().is_none_or(|p| !p.occupied())
                && session.as_deref().is_none_or(|p| !p.occupied())
                && camera.as_deref().is_none_or(|p| !p.occupied())
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
                PollFd::new(&lock, PollFlags::IN),
            ];
            if let Some(fd) = &desktop {
                fds.push(PollFd::new(fd, PollFlags::IN));
            }
            fds.push(PollFd::new(&sounds, PollFlags::IN));
            fds.push(PollFd::new(&settings, PollFlags::IN));
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
            let mut listeners = vec![
                (&admin, true, None, false, 4),
                (&app, false, None, false, 12),
                (&lock, false, None, true, 2),
                (&sounds, false, Some(DesktopEndpoint::Sounds), false, 2),
                (&settings, false, Some(DesktopEndpoint::Settings), false, 2),
            ];
            if let Some(fd) = &desktop {
                listeners.push((fd, false, Some(DesktopEndpoint::Messages), false, 2));
            }
            for (listener, is_admin, is_desktop, is_lock, limit) in listeners {
                if connections
                    .iter()
                    .filter(|c| c.admin == is_admin && c.desktop == is_desktop && c.lock == is_lock)
                    .count()
                    >= limit
                {
                    continue;
                }
                match net::accept_with(listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                    Ok(fd) => {
                        net::sockopt::set_socket_passcred(&fd, true)?;
                        if messages.is_some() && !is_admin && is_desktop.is_none() && !is_lock {
                            net::sockopt::set_socket_send_buffer_size(&fd, 8192)?;
                        }
                        connection_id = connection_id.checked_add(1).ok_or(io::ErrorKind::Other)?;
                        connections.push(Connection {
                            id: connection_id,
                            awaiting: None,
                            fd,
                            admin: is_admin,
                            desktop: is_desktop,
                            lock: is_lock,
                            reading: None,
                            read_stamp: None,
                            deadline: now.saturating_add(30_000_000_000),
                            pending: None,
                            lock_ack: None,
                            launching: None,
                        });
                    }
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            let mut launch_completions = Vec::new();
            connections.retain_mut(|connection| {
                // Drain facts before processing each request/publication, including
                // every retry after EAGAIN. This is ordered ingress, not a barrier
                // against an external compositor change that is not yet observed.
                if refresh_session(&mut authority, &mut session).is_err() {
                    authority.invalidate();
                    return false;
                }
                match apps.poll(&mut authority) {
                    Ok(completed) => launch_completions.extend(completed),
                    Err(_) => {
                        authority.invalidate();
                        startup_failed = true;
                        return false;
                    }
                }
                if boottime_ns() >= connection.deadline {
                    if let Some(instance) = connection.launching.take()
                        && apps.cancel(&instance, &mut authority).is_err()
                    {
                        authority.invalidate();
                        startup_failed = true;
                    }
                    if connection.reading.is_some()
                        && let Some(p) = messages.as_deref_mut()
                    {
                        p.cancel_read(connection.reading.as_ref().unwrap().0);
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
                if let Some((token, _)) = &connection.reading {
                    match ipc::receive(&connection.fd) {
                        Ok(None) => return true,
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        _ => {
                            if let Some(p) = messages.as_deref_mut() {
                                p.cancel_read(*token);
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
                if let Some(provider) = audio.as_deref_mut() {
                    match media_client_input(&mut media, connection.id, &connection.fd, provider) {
                        Ok(()) => {}
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        Err(_) => return false,
                    }
                }
                if let Some(provider) = brightness_provider.as_deref_mut() {
                    match brightness_client_input(
                        &mut brightness,
                        connection.id,
                        &connection.fd,
                        provider,
                    ) {
                        Ok(()) => {}
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        Err(_) => return false,
                    }
                }
                if brightness.needs_drive(connection.id) {
                    if !stopping
                        && operation.is_none()
                        && let Some(provider) = brightness_provider.as_deref_mut()
                        && let Some(reply) = brightness.drive(connection.id, &authority, provider)
                    {
                        connection.pending = serde_json::to_vec(&reply).ok();
                    }
                    return true;
                }
                if media.needs_drive(connection.id) {
                    if !stopping
                        && operation.is_none()
                        && let Some(provider) = audio.as_deref_mut()
                        && let Some(reply) = media.drive(connection.id, &authority, provider)
                    {
                        connection.pending = serde_json::to_vec(&reply).ok();
                    }
                    return true;
                }
                if capture.as_ref().is_some_and(|active| {
                    active.connection == connection.id
                        && matches!(
                            active.phase,
                            CapturePhase::Dispatching | CapturePhase::Submitted
                        )
                }) {
                    // Exact owning-client EOF/extra input is observed before any
                    // effect send; a general provider poll never dispatches.
                    match ipc::receive(&connection.fd) {
                        Ok(None) => {}
                        Err(e) if e.raw_os_error() == Some(4) => return true,
                        _ => {
                            revoke_capture(&mut capture, &mut camera);
                            return false;
                        }
                    }
                    let active = capture.as_ref().unwrap();
                    if stopping || !current_capture(active, &authority, &camera) {
                        revoke_capture(&mut capture, &mut camera);
                        connection.pending = serde_json::to_vec(&Reply::Denied).ok();
                        return true;
                    }
                    if active.phase == CapturePhase::Dispatching {
                        let intent = active.stamp.intent();
                        match camera.as_deref_mut().unwrap().submit(&intent) {
                            Ok(true) => capture.as_mut().unwrap().phase = CapturePhase::Submitted,
                            Ok(false) => {} // Recheck the same stamp/deadline on every retry.
                            Err(_) => {
                                revoke_capture(&mut capture, &mut camera);
                                connection.pending = serde_json::to_vec(&Reply::Unavailable).ok();
                            }
                        }
                    }
                    return true;
                }
                if connection.pending.is_none() && connection.launching.is_some() {
                    return match ipc::receive(&connection.fd) {
                        Ok(None) => {
                            let instance = connection.launching.as_deref().unwrap();
                            match apps.launch(instance, connection.id, entry, &mut authority) {
                                Ok(Some(completed)) => launch_completions.push(completed),
                                Ok(None) => {}
                                Err(_) => {
                                    authority.invalidate();
                                    startup_failed = true;
                                    return false;
                                }
                            }
                            true
                        }
                        Err(e) if e.raw_os_error() == Some(4) => true,
                        _ => {
                            if let Some(instance) = connection.launching.take()
                                && apps.cancel(&instance, &mut authority).is_err()
                            {
                                authority.invalidate();
                                startup_failed = true;
                            }
                            false
                        }
                    };
                }
                if connection
                    .launching
                    .as_ref()
                    .is_some_and(|id| !apps.current(id))
                {
                    connection.pending = serde_json::to_vec(&Reply::Unavailable).ok();
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
                if capture.as_ref().is_some_and(|active| {
                    active.connection == connection.id && active.phase == CapturePhase::ReplyReady
                }) && (stopping
                    || !current_capture(capture.as_ref().unwrap(), &authority, &camera))
                {
                    revoke_capture(&mut capture, &mut camera);
                    connection.pending = serde_json::to_vec(&Reply::Denied).ok();
                }
                if let Some(provider) = audio.as_deref_mut()
                    && let Some(reply) =
                        media.revalidate_reply(connection.id, &authority, provider, stopping)
                {
                    connection.pending = serde_json::to_vec(&reply).ok();
                }
                if let Some(provider) = brightness_provider.as_deref_mut()
                    && let Some(reply) =
                        brightness.revalidate_reply(connection.id, &authority, provider, stopping)
                {
                    connection.pending = serde_json::to_vec(&reply).ok();
                }
                revalidate_lock_ack(connection, stopping, &authority, &operation, &storage);
                if let Some(bytes) = &connection.pending {
                    return match ipc::send(&connection.fd, bytes) {
                        Ok(()) => {
                            #[cfg(test)]
                            if let Some(control) = restore.as_deref_mut() {
                                control.observe_gui_publication(
                                    connection.id,
                                    &connection.fd,
                                    bytes,
                                );
                            }
                            if let Some(provider) = audio.as_deref_mut() {
                                media.published(connection.id, &authority, provider);
                            }
                            brightness.published(connection.id);
                            connection.pending = None;
                            connection.lock_ack = None;
                            connection.read_stamp = None;
                            if let Some(active) = &mut capture
                                && active.connection == connection.id
                                && active.phase == CapturePhase::ReplyReady
                            {
                                active.phase = CapturePhase::Published;
                            }
                            if let Some(instance) = connection.launching.take() {
                                apps.acknowledge(&instance);
                            }
                            connection.deadline = boottime_ns().saturating_add(30_000_000_000);
                            true
                        }
                        Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => {
                            if e.raw_os_error() == Some(11) && connection.read_stamp.is_some() {
                                read_backpressure = read_backpressure.saturating_add(1);
                            }
                            true
                        }
                        Err(_) => {
                            if let Some(instance) = connection.launching.take()
                                && apps.cancel(&instance, &mut authority).is_err()
                            {
                                authority.invalidate();
                                startup_failed = true;
                            }
                            false
                        }
                    };
                }
                let (bytes, cred) = match ipc::receive(&connection.fd) {
                    Ok(Some(v)) => v,
                    Ok(None) => return true,
                    Err(e) if e.raw_os_error() == Some(4) => return true,
                    Err(_) => return false,
                };
                let reply = if connection.lock {
                    if cred.uid.as_raw() != crate::lock::SESSION_UID
                        || serde_json::from_slice::<crate::lock::Request>(&bytes).is_err()
                    {
                        Reply::Denied
                    } else if stopping {
                        Reply::Unavailable
                    } else {
                        match authority.plan_lock(boottime_ns()) {
                            // Fence memory/attempts first, but do not acknowledge
                            // cached pending intent across an unresolved write.
                            // A later clear may already be changing disk state.
                            Decision::Immediate(Reply::Ok)
                                if operation.is_some() || storage.busy() =>
                            {
                                Reply::Unavailable
                            }
                            Decision::Immediate(reply) => reply,
                            Decision::Persist(_) if operation.is_some() || storage.busy() => {
                                Reply::Unavailable
                            }
                            Decision::Persist(intent) => {
                                match storage.start_write(intent.next.clone(), intent.until) {
                                    Ok(token) => {
                                        connection.awaiting = Some(token);
                                        connection.deadline = intent.until;
                                        operation = Some(Operation {
                                            token,
                                            connection: Some(connection.id),
                                            until: intent.until,
                                            intent: Some(intent),
                                            retired: false,
                                            shutdown: false,
                                            camera: false,
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
                        }
                    }
                } else if connection.desktop == Some(DesktopEndpoint::Sounds) {
                    if cred.uid.as_raw() != crate::messages::DESKTOP_UID
                        || stopping
                        || serde_json::from_slice::<SoundsDesktopRequest>(&bytes).is_err()
                    {
                        Reply::Denied
                    } else if let Some(p) = profiles.iter().find(|p| p.app == crate::audio::APP) {
                        match apps.start(p, connection.id, &authority, &mut display) {
                            crate::apps::Start::Rejected(reply) => reply,
                            crate::apps::Start::Reserved { instance, until } => {
                                connection.launching = Some(instance);
                                connection.deadline = until.saturating_add(1_000_000_000);
                                return true;
                            }
                        }
                    } else {
                        Reply::Unavailable
                    }
                } else if connection.desktop == Some(DesktopEndpoint::Settings) {
                    if cred.uid.as_raw() != crate::messages::DESKTOP_UID
                        || stopping
                        || serde_json::from_slice::<SettingsDesktopRequest>(&bytes).is_err()
                    {
                        Reply::Denied
                    } else if let Some(p) =
                        profiles.iter().find(|p| p.app == crate::brightness::APP)
                    {
                        match apps.start(p, connection.id, &authority, &mut display) {
                            crate::apps::Start::Rejected(reply) => reply,
                            crate::apps::Start::Reserved { instance, until } => {
                                connection.launching = Some(instance);
                                connection.deadline = until.saturating_add(1_000_000_000);
                                return true;
                            }
                        }
                    } else {
                        Reply::Unavailable
                    }
                } else if connection.desktop == Some(DesktopEndpoint::Messages) {
                    if cred.uid.as_raw() != crate::messages::DESKTOP_UID
                        || stopping
                        || serde_json::from_slice::<crate::messages::DesktopRequest>(&bytes)
                            .is_err()
                    {
                        Reply::Denied
                    } else if let Some(p) = profiles.iter().find(|p| p.app == crate::messages::APP)
                    {
                        match apps.start(p, connection.id, &authority, &mut display) {
                            crate::apps::Start::Rejected(reply) => reply,
                            crate::apps::Start::Reserved { instance, until } => {
                                connection.launching = Some(instance);
                                connection.deadline = until.saturating_add(1_000_000_000);
                                return true;
                            }
                        }
                    } else {
                        Reply::Unavailable
                    }
                } else if connection.admin {
                    if cred.uid.as_raw() != 0 {
                        Reply::Denied
                    } else {
                        match serde_json::from_slice::<AdminRequest>(&bytes) {
                            Err(_) => Reply::Denied,
                            Ok(AdminRequest::CameraStatus {}) => match camera.as_deref() {
                                Some(p) => Reply::CameraStatus {
                                    occupied: p.occupied(),
                                    busy: p.busy(),
                                    available: p.available(),
                                    pending: usize::from(capture.is_some()),
                                },
                                None => Reply::Unavailable,
                            },
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
                            Ok(AdminRequest::Stop {}) => begin_stop(
                                &mut authority,
                                &mut apps,
                                &mut storage,
                                &mut operation,
                                &mut messages,
                                &mut session,
                                startup_failed,
                                &mut stopping,
                                &mut stop_reply,
                                &mut cleanup_until,
                            ),
                            Ok(_) if stopping => Reply::Unavailable,
                            Ok(AdminRequest::Launch { app }) => {
                                match profiles.iter().find(|p| p.app == app) {
                                    Some(p) => {
                                        match apps.start(p, connection.id, &authority, &mut display)
                                        {
                                            crate::apps::Start::Rejected(reply) => reply,
                                            crate::apps::Start::Reserved { instance, until } => {
                                                connection.launching = Some(instance);
                                                connection.deadline =
                                                    until.saturating_add(1_000_000_000);
                                                return true;
                                            }
                                        }
                                    }
                                    None => Reply::Denied,
                                }
                            }
                            Ok(AdminRequest::Terminate { app }) => {
                                apps.terminate(&app, &mut authority)
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
                                                        camera: false,
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
                        Ok(AppRequest::BrightnessSet { request }) if !stopping => {
                            if operation.is_some() {
                                Reply::Unavailable
                            } else if let Some(provider) = brightness_provider.as_deref() {
                                match brightness.begin(
                                    connection.id,
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    request,
                                    &mut authority,
                                    provider,
                                ) {
                                    Ok(until) => {
                                        connection.deadline = until.saturating_add(1_000_000_000);
                                        return true;
                                    }
                                    Err(reply) => reply,
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(AppRequest::BrightnessStatus {}) if !stopping => {
                            let authenticated = matches!(
                                authority.plan_request(
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    AppRequest::Query {},
                                    boottime_ns()
                                ),
                                Decision::Immediate(Reply::Ok)
                            );
                            if !authenticated {
                                Reply::Denied
                            } else if let Some(provider) = brightness_provider.as_deref() {
                                let state = brightness.state(connection.id, provider);
                                let can_set = operation.is_none()
                                    && brightness.available_to(connection.id)
                                    && state.available
                                    && !state.pending
                                    && authority.brightness_allowed(
                                        cred.pid.as_raw_nonzero().get() as u32,
                                        cred.uid.as_raw(),
                                        provider.incarnation(),
                                    );
                                Reply::BrightnessState { state, can_set }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(AppRequest::MediaRoute { request }) if !stopping => {
                            if operation.is_some() {
                                Reply::Unavailable
                            } else if let Some(provider) = audio.as_deref() {
                                match media.begin(
                                    connection.id,
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    request,
                                    &mut authority,
                                    provider,
                                ) {
                                    Ok(until) => {
                                        connection.deadline = until.saturating_add(1_000_000_000);
                                        return true;
                                    }
                                    Err(reply) => reply,
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(AppRequest::MediaPreviewPlay { request }) if !stopping => {
                            if operation.is_some() {
                                Reply::Unavailable
                            } else if let Some(provider) = audio.as_deref() {
                                match media.begin_preview(
                                    connection.id,
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    request,
                                    &mut authority,
                                    provider,
                                ) {
                                    Ok(until) => {
                                        connection.deadline = until.saturating_add(1_000_000_000);
                                        return true;
                                    }
                                    Err(reply) => reply,
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(AppRequest::MediaPreviewStop {}) if !stopping => {
                            let authenticated = matches!(
                                authority.plan_request(
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    AppRequest::Query {},
                                    boottime_ns()
                                ),
                                Decision::Immediate(Reply::Ok)
                            );
                            if !authenticated || !media.owns(connection.id) {
                                Reply::Denied
                            } else {
                                match media.stop_preview(connection.id) {
                                    Ok(until) => {
                                        connection.deadline = until.saturating_add(1_000_000_000);
                                        return true;
                                    }
                                    Err(reply) => reply,
                                }
                            }
                        }
                        Ok(AppRequest::MediaStatus { offset }) if !stopping => {
                            let authenticated = matches!(
                                authority.plan_request(
                                    cred.pid.as_raw_nonzero().get() as u32,
                                    cred.uid.as_raw(),
                                    AppRequest::Query {},
                                    boottime_ns()
                                ),
                                Decision::Immediate(Reply::Ok)
                            );
                            if !authenticated {
                                Reply::Denied
                            } else if let Some(provider) = audio.as_deref() {
                                let catalog = provider.routes();
                                if offset > catalog.len()
                                    || offset > 32
                                    || offset % crate::audio::ROUTES_PER_PAGE != 0
                                {
                                    Reply::Denied
                                } else {
                                    let state = media.state(connection.id, provider);
                                    let authorized = operation.is_none()
                                        && media.available_to(connection.id)
                                        && authority.media_allowed(
                                            cred.pid.as_raw_nonzero().get() as u32,
                                            cred.uid.as_raw(),
                                            provider.incarnation(),
                                        );
                                    let end =
                                        (offset + crate::audio::ROUTES_PER_PAGE).min(catalog.len());
                                    let routes = catalog[offset..end].to_vec();
                                    let next_offset = (end < catalog.len()).then_some(end);
                                    Reply::MediaStatus {
                                        can_choose: authorized && state.available,
                                        can_reconnect: authorized && !state.connected,
                                        state,
                                        routes,
                                        next_offset,
                                    }
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }
                        Ok(AppRequest::MediaReconnect {}) if !stopping => {
                            if operation.is_some() || !media.available_to(connection.id) {
                                Reply::Unavailable
                            } else if let Some(provider) = audio.as_deref_mut() {
                                if provider.state().connected
                                    || !authority.media_allowed(
                                        cred.pid.as_raw_nonzero().get() as u32,
                                        cred.uid.as_raw(),
                                        provider.incarnation(),
                                    )
                                {
                                    Reply::Denied
                                } else {
                                    media.retire(provider);
                                    match provider.reconnect() {
                                        Ok(()) => Reply::Ok,
                                        Err(_) => Reply::Unavailable,
                                    }
                                }
                            } else {
                                Reply::Unavailable
                            }
                        }

                        Ok(AppRequest::CameraCapture { request }) if !stopping => {
                            if operation.is_some() || capture.is_some() {
                                Reply::Unavailable
                            } else if let Some(p) = camera.as_deref() {
                                if !p.available() || p.busy() {
                                    Reply::Unavailable
                                } else {
                                    match authority.begin_capture(
                                        cred.pid.as_raw_nonzero().get() as u32,
                                        cred.uid.as_raw(),
                                        p.incarnation(),
                                        p.generation(),
                                        request.ttl_ms,
                                        boottime_ns(),
                                    ) {
                                        Err(reply) => reply,
                                        Ok((stamp, intent)) => match storage
                                            .start_write(intent.next.clone(), intent.until)
                                        {
                                            Ok(token) => {
                                                connection.awaiting = Some(token);
                                                connection.deadline =
                                                    intent.until.saturating_add(1_000_000_000);
                                                capture = Some(Capture {
                                                    connection: connection.id,
                                                    stamp,
                                                    phase: CapturePhase::Persisting,
                                                });
                                                operation = Some(Operation {
                                                    token,
                                                    connection: Some(connection.id),
                                                    until: intent.until,
                                                    intent: Some(intent),
                                                    retired: false,
                                                    shutdown: false,
                                                    camera: true,
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
                            } else {
                                Reply::Unavailable
                            }
                        }
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
                                                camera: false,
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
                connection.lock_ack =
                    (connection.lock && reply == Reply::Ok && connection.pending.is_some())
                        .then(|| storage.admission_sequence());
                connection.deadline = boottime_ns().saturating_add(1_000_000_000);
                connection.pending.is_some()
            });
            if !schedule_restoration(
                &mut authority,
                &mut storage,
                &mut operation,
                stopping || startup_failed,
            ) {
                startup_failed = true;
            }
            complete_launches(
                launch_completions,
                &mut connections,
                &mut local_recovery,
                &messages,
            )?;
            let active_launches: Vec<_> = connections
                .iter()
                .filter_map(|c| c.launching.as_deref().map(|instance| (c.id, instance)))
                .collect();
            apps.cancel_disconnected(&active_launches, &mut authority)?;
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
                    } else if !provider.read_occupied() && !provider.busy() {
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
    if let Some(provider) = session.as_deref_mut() {
        stop_session(&mut authority, provider);
    }
    #[allow(clippy::needless_option_as_deref)]
    if let Some(provider) = audio.as_deref_mut() {
        media.retire(provider);
    }
    #[allow(clippy::needless_option_as_deref)]
    if let Some(provider) = brightness_provider.as_deref_mut() {
        brightness.retire(provider);
    }
    revoke_capture(&mut capture, &mut camera);
    if let Some(provider) = camera.as_deref_mut() {
        provider.stop();
    }
    // Normal grace completes/escalates inside the main loop. On any fallback,
    // force every retained app before another owner's drain can spend the budget.
    if let Err(e) = apps.stop(&mut authority) {
        outcome = Err(e);
    }
    if messages.is_some_and(|p| !p.drain_until(until)) {
        std::process::abort();
    }
    if !storage.drain_until(until) {
        std::process::abort();
    }
    if camera.is_some_and(|p| !p.drain_until(until)) {
        std::process::abort();
    }
    if let Some(provider) = session {
        while provider.occupied() {
            if boottime_ns() >= until {
                std::process::abort();
            }
            // Terminal facts still drive resource reaping. An error never
            // clears occupancy or substitutes for actual helper completion.
            let _ = provider.poll();
            if provider.occupied() {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }
    while apps.occupied() {
        if boottime_ns() >= until {
            std::process::abort();
        }
        // Errors retain the slot; only actual app/helper parent waits clear it.
        let _ = apps.poll(&mut authority);
        if apps.occupied() {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    for (path, identity) in [&admin_path, &app_path, &lock_path]
        .into_iter()
        .zip(socket_ids)
    {
        if fs::symlink_metadata(path).is_ok_and(|m| (m.dev(), m.ino()) == identity) {
            fs::remove_file(path)?;
        }
    }
    if fs::symlink_metadata(&sounds_path)
        .is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == sounds_identity)
    {
        fs::remove_file(&sounds_path)?;
    }
    if fs::symlink_metadata(&settings_path)
        .is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == settings_identity)
    {
        fs::remove_file(&settings_path)?;
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

#[cfg(test)]
#[path = "service_auth_attempt_tests.rs"]
mod auth_attempt_tests;

#[cfg(test)]
#[path = "service_restore_tests.rs"]
mod restore_tests;

#[cfg(test)]
#[path = "service_lock_ack_tests.rs"]
mod lock_ack_tests;

#[cfg(test)]
#[path = "service_media_input_tests.rs"]
mod media_input_tests;

#[cfg(test)]
#[path = "service_source_observation_tests.rs"]
mod source_observation_tests;

#[cfg(test)]
#[path = "brightness_tests.rs"]
mod brightness_tests;

#[cfg(test)]
mod brightness_inspection_tests {
    //! Controlled Provider scheduling only; no transport, actual remote custody,
    //! remote reap, full actor, backend or hardware qualification.
    use super::inspect_brightness;
    use crate::{
        boottime_ns,
        brightness::{Event, Intent, Provider, State},
    };
    use std::io;
    struct PendingProvider {
        remote_pending: bool,
        transport_busy: bool,
        attempts: usize,
        reads: usize,
    }
    impl Provider for PendingProvider {
        fn incarnation(&self) -> &str {
            "controlled-inspection"
        }
        fn state(&self) -> State {
            State {
                pending: self.remote_pending,
                ..State::default()
            }
        }
        fn readonly_in_flight(&self) -> bool {
            false
        }
        fn submit(&mut self, _: &Intent) -> io::Result<bool> {
            panic!("inspection must never submit Set")
        }
        fn inspect(&mut self, until: u64) -> io::Result<bool> {
            assert!(boottime_ns() < until && until <= boottime_ns() + 1_000_000_000);
            self.attempts += 1;
            if self.transport_busy {
                return Ok(false);
            }
            self.reads += 1;
            Ok(true)
        }
        fn poll(&mut self) -> Option<Event> {
            panic!("inspection helper must not poll or replay")
        }
        fn revoke(&mut self) {
            panic!("inspection helper must not revoke/reconnect for Set")
        }
    }
    #[test]
    fn brightness_remote_pending_still_allows_readonly_reconciliation() {
        let mut idle = PendingProvider {
            remote_pending: false,
            transport_busy: false,
            attempts: 0,
            reads: 0,
        };
        let mut last = 0;
        inspect_brightness(&mut idle, false, false, &mut last);
        assert_eq!(
            (idle.attempts, idle.reads),
            (1, 1),
            "existing idle inspection control"
        );
        inspect_brightness(&mut idle, false, false, &mut last);
        assert_eq!(
            idle.attempts, 1,
            "same cadence must not issue a second read"
        );
        for (actor_pending, stopping) in [(true, false), (false, true)] {
            let mut last = 0;
            inspect_brightness(&mut idle, actor_pending, stopping, &mut last);
            assert_eq!(last, 0, "local pending and Stop still suppress inspection");
            assert_eq!(idle.attempts, 1);
        }
        let mut remote = PendingProvider {
            remote_pending: true,
            transport_busy: false,
            attempts: 0,
            reads: 0,
        };
        inspect_brightness(&mut remote, false, false, &mut 0);
        let mut busy = PendingProvider {
            remote_pending: true,
            transport_busy: true,
            attempts: 0,
            reads: 0,
        };
        inspect_brightness(&mut busy, false, false, &mut 0);
        println!(
            "idle_reads={} remote_pending_reads={} busy_attempts={} busy_reads={}",
            idle.reads, remote.reads, busy.attempts, busy.reads
        );
        assert_eq!(
            (remote.reads, busy.attempts, busy.reads),
            (1, 1, 0),
            "MISSING_READONLY_RECONCILIATION_WHILE_REMOTE_PENDING"
        );
        assert!(
            remote.state().pending,
            "a status attempt is not remote cleanup proof"
        );
    }
}

#[cfg(test)]
#[path = "brightness_readonly_reservation_tests.rs"]
mod brightness_readonly_reservation_tests;
