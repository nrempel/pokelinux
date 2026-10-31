//! Actual normal Settings Rust SDK in a launched UID11050 six-argument fixture.
//! No normal Qt window, C ABI invocation or synthetic SDK/actor response.
use pokelinux_lifecycle::{boottime_ns, ipc};
use pokelinux_settings_client::{
    connectivity::Engine,
    ffi::{OK, WAIT},
};
use rustix::net::sockopt::socket_peercred;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    fs,
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    thread,
    time::Duration,
};
fn pause() {
    thread::sleep(Duration::from_millis(2));
}
fn decimal(v: &Value) -> io::Result<u64> {
    let s = v.as_str().ok_or(io::ErrorKind::InvalidData)?;
    let n = s.parse::<u64>().map_err(|_| io::ErrorKind::InvalidData)?;
    if n.to_string() != s {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(n)
}
#[derive(Clone, Copy)]
struct CommandContext {
    operation: Option<&'static str>,
    sequence: u64,
    original_until_ns: u64,
}
struct Diagnostic {
    path: PathBuf,
    context: Cell<CommandContext>,
    first: RefCell<Option<Value>>,
    attempted: Cell<bool>,
}
impl Diagnostic {
    fn new(directory: &std::path::Path, case: u64) -> Self {
        Self {
            path: directory.join(format!("fixture-error-{}.json", std::process::id())),
            context: Cell::new(CommandContext {
                operation: None,
                sequence: 0,
                original_until_ns: case,
            }),
            first: RefCell::new(None),
            attempted: Cell::new(false),
        }
    }
    fn context(&self, operation: Option<&'static str>, sequence: u64, original_until_ns: u64) {
        self.context.set(CommandContext {
            operation,
            sequence,
            original_until_ns,
        });
    }
    fn note(&self, stage: &'static str, error: &io::Error) {
        let mut first = self.first.borrow_mut();
        if first.is_none() {
            let context = self.context.get();
            *first = Some(json!({"schema":1,"pid":std::process::id(),
                "operation":context.operation,"sequence":context.sequence,
                "original_until_ns":context.original_until_ns,"first_failing_stage":stage,
                "io_kind":format!("{:?}",error.kind()),"raw_os_error":error.raw_os_error()}));
        }
    }
    fn emit(&self, sdk: Option<&OwnedSdk>) {
        let first = self.first.borrow();
        let Some(first) = first.as_ref() else {
            return;
        };
        if self.attempted.replace(true) {
            return;
        }
        let mut value = first.clone();
        value["sdk_close"] = match sdk {
            Some(sdk) => json!({"stop_started_ns":sdk.stop_started.get(),
                "latched_cutoff_ns":sdk.stop_cutoff.get(),"stop_returned":sdk.stop_returned.get(),
                "last_join_status":sdk.last_join_status.get(),"joined":sdk.joined.get(),
                "task_count":sdk.task_count.get()}),
            None => Value::Null,
        };
        let Ok(mut body) = serde_json::to_vec(&value) else {
            return;
        };
        body.push(b'\n');
        if body.len() > 4096 {
            return;
        }
        let flags = rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC;
        let Ok(mut file) = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .custom_flags(flags.bits() as i32)
            .open(&self.path)
        else {
            return;
        };
        if file
            .set_permissions(fs::Permissions::from_mode(0o644))
            .is_err()
        {
            return;
        }
        // One bounded private write attempt; failure never replaces the actual
        // fixture error. No SDK call, new cutoff, wait, clobber or retry here.
        let _ = file.write(&body);
    }
}
// The SDK owns its socket and worker. This fixture observes only public getters
// and the SDK's actual stop/join result; it never exports invented wire facts.
struct OwnedSdk {
    engine: Engine,
    stop_started: Cell<u64>,
    stop_cutoff: Cell<u64>,
    stop_returned: Cell<Option<bool>>,
    last_join_status: Cell<Option<i32>>,
    joined: Cell<Option<bool>>,
    task_count: Cell<Option<usize>>,
}
impl OwnedSdk {
    fn new(endpoint: PathBuf) -> io::Result<Self> {
        Ok(Self {
            engine: Engine::new(endpoint)?,
            stop_started: Cell::new(0),
            stop_cutoff: Cell::new(0),
            stop_returned: Cell::new(None),
            last_join_status: Cell::new(None),
            joined: Cell::new(None),
            task_count: Cell::new(None),
        })
    }
    fn close(&self, outer_until: u64, diagnostic: &Diagnostic) -> io::Result<Value> {
        let stage = Cell::new("sdk_close_deadline_overflow");
        let outcome = (|| -> io::Result<Value> {
            if self.stop_cutoff.get() == 0 {
                let before = boottime_ns();
                let end = before
                    .checked_add(200_000_000)
                    .ok_or(io::ErrorKind::Other)?
                    .min(outer_until);
                if before >= end {
                    stage.set("sdk_close_outer_cutoff");
                    return Err(io::ErrorKind::TimedOut.into());
                }
                self.stop_started.set(before);
                self.stop_cutoff.set(end);
                let stopped = self.engine.stop();
                self.stop_returned.set(Some(stopped));
                if !stopped {
                    stage.set("sdk_stop_refused");
                    return Err(io::ErrorKind::Other.into());
                }
            }
            let end = self.stop_cutoff.get();
            while boottime_ns() < end {
                let status = self.engine.join();
                self.last_join_status.set(Some(status));
                let joined = self.engine.joined();
                self.joined.set(Some(joined));
                if joined {
                    stage.set("sdk_close_task_directory");
                    let entries = fs::read_dir("/proc/self/task")?;
                    stage.set("sdk_close_task_entries");
                    let actual_task_count_after_join =
                        entries.collect::<io::Result<Vec<_>>>()?.len();
                    self.task_count.set(Some(actual_task_count_after_join));
                    if actual_task_count_after_join != 1 {
                        stage.set("sdk_close_task_count");
                        return Err(io::ErrorKind::Other.into());
                    }
                    let at = boottime_ns();
                    if status != OK || at >= end {
                        stage.set(if status != OK {
                            "sdk_close_joined_status"
                        } else {
                            "sdk_close_joined_cutoff"
                        });
                        return Err(io::ErrorKind::Other.into());
                    }
                    return Ok(
                        json!({"stop_requested":true,"join_status":status,"joined":true,"actual_task_count_after_join":actual_task_count_after_join,
                        "stop_call_before_ns":self.stop_started.get(),"fixture_stop_cutoff_ns":end,"joined_at_ns":at}),
                    );
                }
                if status != WAIT {
                    stage.set("sdk_close_nonwaiting_status");
                    return Err(io::ErrorKind::Other.into());
                }
                pause();
            }
            // Engine's retained-thread Drop guard remains authoritative on failure;
            // a timed-out/aborted join cannot be reported as normal app completion.
            stage.set("sdk_close_cutoff");
            Err(io::ErrorKind::TimedOut.into())
        })();
        if let Err(error) = &outcome {
            diagnostic.note(stage.get(), error);
        }
        outcome
    }
}
fn actual_caps() -> io::Result<Value> {
    let text = fs::read_to_string("/proc/self/status")?;
    let mut map = serde_json::Map::new();
    for key in [
        "Uid",
        "Gid",
        "Groups",
        "CapEff",
        "CapPrm",
        "CapBnd",
        "CapInh",
        "CapAmb",
        "NoNewPrivs",
    ] {
        let line = text
            .lines()
            .find(|l| l.starts_with(&format!("{key}:")))
            .ok_or(io::ErrorKind::InvalidData)?;
        map.insert(key.into(), json!(line.split_once(':').unwrap().1.trim()));
    }
    Ok(Value::Object(map))
}
fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if a.len() != 6 || a.iter().any(|p| !p.is_absolute()) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    if rustix::process::getuid().as_raw() != 11050
        || rustix::process::geteuid().as_raw() != 11050
        || rustix::process::getgid().as_raw() != 11050
        || rustix::process::getegid().as_raw() != 11050
        || !rustix::process::getgroups()?.is_empty()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let config: Value = serde_json::from_slice(&fs::read(a[0].join("fixture.json"))?)?;
    let root = config["actor_pid"]
        .as_u64()
        .ok_or(io::ErrorKind::InvalidData)? as u32;
    let case = decimal(&config["original_case_until_ns"])?;
    if boottime_ns() >= case {
        return Err(io::ErrorKind::TimedOut.into());
    }
    fs::create_dir_all(&a[1])?;
    fs::set_permissions(&a[1], fs::Permissions::from_mode(0o755))?;
    let path = a[1].join("control.sock");
    if path.exists() {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    // Test-only control: no secret input; access is exact root PID and credentials,
    // not discretionary root override (the fixture has no DAC_OVERRIDE).
    let diagnostic = Diagnostic::new(&a[1], case);
    let control = ipc::listener(&path, 0o666).inspect_err(|error| {
        diagnostic.note("control_listener", error);
        diagnostic.emit(None);
    })?;
    let sdk = OwnedSdk::new(a[4].clone()).inspect_err(|error| {
        diagnostic.note("sdk_new", error);
        diagnostic.emit(None);
    })?;
    let mut sequence = 0_u64;
    let mut exiting = false;
    let stage = Cell::new("control_accept");
    let outcome = (|| -> io::Result<bool> {
        while boottime_ns() < case && !exiting {
            diagnostic.context(None, sequence, case);
            stage.set("control_accept");
            let connection = match rustix::net::accept_with(
                &control,
                rustix::net::SocketFlags::CLOEXEC | rustix::net::SocketFlags::NONBLOCK,
            ) {
                Ok(fd) => fd,
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                    pause();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let exchange = (|| -> io::Result<()> {
                stage.set("control_passcred");
                rustix::net::sockopt::set_socket_passcred(&connection, true)?;
                stage.set("control_peercred");
                let peer = socket_peercred(&connection)?;
                if peer.pid.as_raw_nonzero().get() as u32 != root
                    || peer.uid.as_raw() != 0
                    || peer.gid.as_raw() != 0
                {
                    stage.set("control_peer_identity");
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                let mut received = None;
                // One request on each private test-control connection, no new action after case.
                stage.set("control_ingress_deadline_overflow");
                let ingress = case.min(
                    boottime_ns()
                        .checked_add(1_000_000_000)
                        .ok_or(io::ErrorKind::Other)?,
                );
                diagnostic.context(None, sequence, ingress);
                while boottime_ns() < ingress {
                    stage.set("control_receive");
                    if let Some((data, c)) = ipc::receive(&connection)? {
                        if c.pid != peer.pid || c.uid != peer.uid || c.gid != peer.gid {
                            stage.set("control_packet_identity");
                            return Err(io::ErrorKind::PermissionDenied.into());
                        }
                        received = Some(data);
                        break;
                    }
                    pause();
                }
                stage.set("control_receive_cutoff");
                let bytes = received.ok_or(io::ErrorKind::TimedOut)?;
                stage.set("command_json");
                let cmd: Value = serde_json::from_slice(&bytes)?;
                stage.set("command_operation");
                let op = cmd["op"].as_str().ok_or(io::ErrorKind::InvalidData)?;
                let operation = Some(match op {
                    "status" => "status",
                    "set" => "set",
                    "exit" => "exit",
                    _ => "unsupported",
                });
                stage.set("command_cutoff_parse");
                let until = decimal(&cmd["until_ns"])?;
                diagnostic.context(operation, sequence, until);
                let t = boottime_ns();
                if t >= until || until > case || until - t > 1_000_000_000 {
                    stage.set("command_cutoff_validation");
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                let expected = if op == "set" { 3 } else { 2 };
                if cmd.as_object().map(|o| o.len()) != Some(expected) {
                    stage.set("command_fields");
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                stage.set("command_sequence_overflow");
                sequence = sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
                diagnostic.context(operation, sequence, until);
                let before = boottime_ns();
                let mut accepted: Option<bool> = None;
                let mut choice: Option<bool> = None;
                let mut sdk_stop = Value::Null;
                match op {
                    "status" => {}
                    "set" => {
                        stage.set("command_choice");
                        let connected = cmd["connected"]
                            .as_bool()
                            .ok_or(io::ErrorKind::InvalidInput)?;
                        // Exactly one ordinary SDK call for this explicit control action.
                        choice = Some(connected);
                        accepted = Some(sdk.engine.set(connected, until));
                    }
                    "exit" => {
                        sdk_stop = sdk.close(until, &diagnostic)?;
                        exiting = true;
                    }
                    _ => {
                        stage.set("command_unsupported");
                        return Err(io::ErrorKind::InvalidInput.into());
                    }
                }
                let sdk_view = sdk.engine.view();
                let returned = boottime_ns();
                if returned >= until {
                    stage.set("reply_after_getter_cutoff");
                    return Err(io::ErrorKind::TimedOut.into());
                }
                stage.set("reply_actual_caps");
                let response = json!({"operation":op,"sequence":sequence,"pid":std::process::id(),"uid":11050,
                "actual_caps":actual_caps()?,"sdk_view":sdk_view,"choice_accepted":accepted,"choice_connected":choice,
                "sdk_call_before_ns":before,"sdk_call_returned_ns":returned,"sdk_stop":sdk_stop,
                "original_until_ns":until,"completed_ns":boottime_ns()});
                stage.set("reply_serialize");
                let data = serde_json::to_vec(&response)?;
                if boottime_ns() >= until {
                    stage.set("reply_before_send_cutoff");
                    return Err(io::ErrorKind::TimedOut.into());
                }
                stage.set("reply_send");
                ipc::send(&connection, &data)?;
                Ok(())
            })();
            if let Err(error) = &exchange {
                diagnostic.note(stage.get(), error);
                diagnostic.emit(Some(&sdk));
            }
            exchange?;
        }
        Ok(exiting)
    })();
    if let Err(error) = &outcome {
        diagnostic.note(stage.get(), error);
        diagnostic.emit(Some(&sdk));
    }
    // Preserve the first Stop deadline even on a setup/control error. No thread
    // is detached; an unsuccessful owned join leaves the SDK Drop guard intact.
    diagnostic.context(Some("cleanup"), sequence, case);
    let joined = sdk.engine.joined();
    sdk.joined.set(Some(joined));
    let closed = if joined {
        Ok(Value::Null)
    } else {
        sdk.close(case, &diagnostic)
    };
    diagnostic.emit(Some(&sdk));
    drop(control);
    fs::remove_file(path).inspect_err(|error| {
        diagnostic.note("cleanup_remove_control", error);
        diagnostic.emit(Some(&sdk));
    })?;
    let exited = outcome?;
    closed?;
    if !exited {
        let error = io::Error::from(io::ErrorKind::TimedOut);
        diagnostic.note("case_without_exit", &error);
        diagnostic.emit(Some(&sdk));
        return Err(error);
    }
    Ok(())
}
