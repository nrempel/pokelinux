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
    cell::Cell, fs, io, os::unix::fs::PermissionsExt, path::PathBuf, thread, time::Duration,
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
// The SDK owns its socket and worker. This fixture observes only public getters
// and the SDK's actual stop/join result; it never exports invented wire facts.
struct OwnedSdk {
    engine: Engine,
    stop_started: Cell<u64>,
    stop_cutoff: Cell<u64>,
}
impl OwnedSdk {
    fn new(endpoint: PathBuf) -> io::Result<Self> {
        Ok(Self {
            engine: Engine::new(endpoint)?,
            stop_started: Cell::new(0),
            stop_cutoff: Cell::new(0),
        })
    }
    fn close(&self, outer_until: u64) -> io::Result<Value> {
        if self.stop_cutoff.get() == 0 {
            let before = boottime_ns();
            let end = before
                .checked_add(200_000_000)
                .ok_or(io::ErrorKind::Other)?
                .min(outer_until);
            if before >= end {
                return Err(io::ErrorKind::TimedOut.into());
            }
            self.stop_started.set(before);
            self.stop_cutoff.set(end);
            if !self.engine.stop() {
                return Err(io::ErrorKind::Other.into());
            }
        }
        let end = self.stop_cutoff.get();
        while boottime_ns() < end {
            let status = self.engine.join();
            if self.engine.joined() {
                let actual_task_count_after_join = fs::read_dir("/proc/self/task")?
                    .collect::<io::Result<Vec<_>>>()?
                    .len();
                if actual_task_count_after_join != 1 {
                    return Err(io::ErrorKind::Other.into());
                }
                let at = boottime_ns();
                if status != OK || at >= end {
                    return Err(io::ErrorKind::Other.into());
                }
                return Ok(
                    json!({"stop_requested":true,"join_status":status,"joined":true,"actual_task_count_after_join":actual_task_count_after_join,
                    "stop_call_before_ns":self.stop_started.get(),"fixture_stop_cutoff_ns":end,"joined_at_ns":at}),
                );
            }
            if status != WAIT {
                return Err(io::ErrorKind::Other.into());
            }
            pause();
        }
        // Engine's retained-thread Drop guard remains authoritative on failure;
        // a timed-out/aborted join cannot be reported as normal app completion.
        Err(io::ErrorKind::TimedOut.into())
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
    let control = ipc::listener(&path, 0o666)?;
    let sdk = OwnedSdk::new(a[4].clone())?;
    let mut sequence = 0_u64;
    let mut exiting = false;
    let outcome = (|| -> io::Result<bool> {
        while boottime_ns() < case && !exiting {
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
            rustix::net::sockopt::set_socket_passcred(&connection, true)?;
            let peer = socket_peercred(&connection)?;
            if peer.pid.as_raw_nonzero().get() as u32 != root
                || peer.uid.as_raw() != 0
                || peer.gid.as_raw() != 0
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let mut received = None;
            // One request on each private test-control connection, no new action after case.
            let ingress = case.min(
                boottime_ns()
                    .checked_add(1_000_000_000)
                    .ok_or(io::ErrorKind::Other)?,
            );
            while boottime_ns() < ingress {
                if let Some((data, c)) = ipc::receive(&connection)? {
                    if c.pid != peer.pid || c.uid != peer.uid || c.gid != peer.gid {
                        return Err(io::ErrorKind::PermissionDenied.into());
                    }
                    received = Some(data);
                    break;
                }
                pause();
            }
            let bytes = received.ok_or(io::ErrorKind::TimedOut)?;
            let cmd: Value = serde_json::from_slice(&bytes)?;
            let op = cmd["op"].as_str().ok_or(io::ErrorKind::InvalidData)?;
            let until = decimal(&cmd["until_ns"])?;
            let t = boottime_ns();
            if t >= until || until > case || until - t > 1_000_000_000 {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            let expected = if op == "set" { 3 } else { 2 };
            if cmd.as_object().map(|o| o.len()) != Some(expected) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            sequence = sequence.checked_add(1).ok_or(io::ErrorKind::Other)?;
            let before = boottime_ns();
            let mut accepted: Option<bool> = None;
            let mut choice: Option<bool> = None;
            let mut sdk_stop = Value::Null;
            match op {
                "status" => {}
                "set" => {
                    let connected = cmd["connected"]
                        .as_bool()
                        .ok_or(io::ErrorKind::InvalidInput)?;
                    // Exactly one ordinary SDK call for this explicit control action.
                    choice = Some(connected);
                    accepted = Some(sdk.engine.set(connected, until));
                }
                "exit" => {
                    sdk_stop = sdk.close(until)?;
                    exiting = true;
                }
                _ => return Err(io::ErrorKind::InvalidInput.into()),
            }
            let sdk_view = sdk.engine.view();
            let returned = boottime_ns();
            if returned >= until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let response = json!({"operation":op,"sequence":sequence,"pid":std::process::id(),"uid":11050,
            "actual_caps":actual_caps()?,"sdk_view":sdk_view,"choice_accepted":accepted,"choice_connected":choice,
            "sdk_call_before_ns":before,"sdk_call_returned_ns":returned,"sdk_stop":sdk_stop,
            "original_until_ns":until,"completed_ns":boottime_ns()});
            let data = serde_json::to_vec(&response)?;
            if boottime_ns() >= until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            ipc::send(&connection, &data)?;
        }
        Ok(exiting)
    })();
    // Preserve the first Stop deadline even on a setup/control error. No thread
    // is detached; an unsuccessful owned join leaves the SDK Drop guard intact.
    let closed = if sdk.engine.joined() {
        Ok(Value::Null)
    } else {
        sdk.close(case)
    };
    drop(control);
    fs::remove_file(path)?;
    let exited = outcome?;
    closed?;
    if !exited {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}
