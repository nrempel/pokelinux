//! UID11050 six-argument app fixture. No GUI or synthetic actor response.
use pokelinux_lifecycle::{Reply, boottime_ns, ipc};
use rustix::{
    fd::{AsRawFd, OwnedFd},
    net::sockopt::socket_peercred,
};
use serde_json::{Value, json};
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf, thread, time::Duration};
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
fn call(fd: &OwnedFd, bytes: &[u8], until: u64, root: u32) -> io::Result<(Value, u64, u64)> {
    let before = boottime_ns();
    if before >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    // A request is transmitted exactly once; no effect retry, pipelining or reconnect.
    ipc::send(fd, bytes)?;
    while boottime_ns() < until {
        if let Some((data, c)) = ipc::receive(fd)? {
            if c.pid.as_raw_nonzero().get() as u32 != root
                || c.uid.as_raw() != 0
                || c.gid.as_raw() != 0
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let value: Value = serde_json::from_slice(&data)?;
            let _: Reply = serde_json::from_value(value.clone())?;
            let after = boottime_ns();
            if after >= until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            return Ok((value, before, after));
        }
        pause();
    }
    Err(io::ErrorKind::TimedOut.into())
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
    let actor = ipc::connect(&a[4])?;
    let cred = socket_peercred(&actor)?;
    if cred.pid.as_raw_nonzero().get() as u32 != root
        || cred.uid.as_raw() != 0
        || cred.gid.as_raw() != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut sequence = 0_u64;
    let mut exiting = false;
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
        let (reply, sent, returned, literal) = match op {
            "status" => {
                let literal = br#"{"op":"connectivity_status"}"#.to_vec();
                let (r, s, e) = call(&actor, &literal, until, root)?;
                (r, s, e, literal)
            }
            "set" => {
                let connected = cmd["connected"]
                    .as_bool()
                    .ok_or(io::ErrorKind::InvalidInput)?;
                let literal = serde_json::to_vec(
                    &json!({"op":"connectivity_set","request":{"connected":connected,"until_ns":until.to_string()}}),
                )?;
                let (r, s, e) = call(&actor, &literal, until, root)?;
                (r, s, e, literal)
            }
            "exit" => {
                exiting = true;
                (Value::Null, t, boottime_ns(), vec![])
            }
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        };
        let response = json!({"operation":op,"sequence":sequence,"pid":std::process::id(),"uid":11050,
            "actual_caps":actual_caps()?,"actor_fd":actor.as_raw_fd(),"actor_peer_pid":root,"actor_peer_uid":0,"actor_peer_gid":0,
            "actual_packet":String::from_utf8(literal).map_err(|_|io::ErrorKind::InvalidData)?,"reply":reply,
            "send_before_ns":sent,"reply_ns":returned,"original_until_ns":until,"completed_ns":boottime_ns()});
        let data = serde_json::to_vec(&response)?;
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        ipc::send(&connection, &data)?;
    }
    drop(actor);
    drop(control);
    fs::remove_file(path)?;
    if !exiting {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}
