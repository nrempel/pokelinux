//! Explicit fixture record transport, not a device-node adapter or phone service.
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WORKER_SELECTOR, WorkerCommand, WorkerState},
    deadline::{BootClock, Clock, Deadline},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use pokelinux_sit_attempt::{Attempt, Error as AttemptError, Setup};
use pokelinux_sit_codec::{InitialLteSetupV3, Protocol, modern_report::ModernSetupReport22};
use rustix::net::{
    self, AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType,
};
use serde_json::json;
use std::{
    fmt::Debug,
    io::{self, Write},
    num::NonZeroU8,
    os::unix::process::ExitStatusExt,
    path::Path,
    sync::Arc,
    time::Duration,
};
fn checked<T, E: Debug>(value: Result<T, E>) -> io::Result<T> {
    value.map_err(|e| io::Error::other(format!("{e:?}")))
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn now() -> u64 {
    BootClock
        .now()
        .as_nanos()
        .try_into()
        .expect("bounded boot time")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str) -> io::Result<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || text.len() > 2048
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| invalid()))
        .collect()
}
fn configure_output() -> io::Result<()> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let out = io::stdout();
    if FileType::from_raw_mode(fstat(&out)?.st_mode) != FileType::Fifo {
        return Err(invalid());
    }
    fcntl_setfl(&out, fcntl_getfl(&out)? | OFlags::NONBLOCK)?;
    Ok(())
}
fn event(value: serde_json::Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    if bytes.len() > 4096 || bytes.len() > rustix::pipe::PIPE_BUF {
        return Err(invalid());
    }
    match rustix::io::write(io::stdout(), &bytes) {
        Ok(n) if n == bytes.len() => Ok(()),
        Ok(_) => Err(io::ErrorKind::WriteZero.into()),
        Err(e) => Err(e.into()),
    }
}
fn wait_release(until: u64) -> io::Result<()> {
    use rustix::fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat};
    let input = io::stdin();
    if FileType::from_raw_mode(fstat(&input)?.st_mode) != FileType::Fifo {
        return Err(invalid());
    }
    fcntl_setfl(&input, fcntl_getfl(&input)? | OFlags::NONBLOCK)?;
    let mut released = false;
    loop {
        if now() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let mut bytes = [0u8; 2];
        match rustix::io::read(&input, &mut bytes) {
            Ok(0) if released => return Ok(()),
            Ok(1) if !released && bytes[0] == b'G' => released = true,
            Ok(_) => return Err(invalid()),
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
            Err(e) => return Err(e.into()),
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn report(value: &ModernSetupReport22) -> serde_json::Value {
    json!({"cause":value.cause,"opaque_i64_at_8":value.opaque_i64_at_8,"cid":value.cid,
        "active":value.active,"protocol_code":value.protocol_code,"mtu_v4":value.mtu_v4,
        "mtu_v6":value.mtu_v6,"ifname":value.ifname,"address_text":value.address_text,
        "dnses":value.dnses,"gateways":value.gateways,"pcscf":value.pcscf,
        "structured_address_vector_empty":value.structured_addresses.is_empty(),
        "QoS_TD_vectors_empty":value.qos_sessions.is_empty() && value.traffic_descriptors.is_empty()})
}
fn worker(args: &[String]) -> io::Result<()> {
    if args.len() != 9
        || args[1] != "--peer"
        || args[3] != "--packet"
        || args[5] != "--peer-pid"
        || args[7] != "--until-ns"
    {
        return Err(invalid());
    }
    let request = unhex(&args[4])?;
    if request.len() != 983 {
        return Err(invalid());
    }
    // Main thread/final credentials, before connecting the explicit fixture peer.
    let until: u64 = args[8].parse().map_err(|_| invalid())?;
    let parent = BoundParent::from_stdin()?;
    if now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    let socket = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC,
        None,
    )?;
    net::connect(&socket, &SocketAddrUnix::new(Path::new(&args[2]))?)?;
    let expected_peer: u32 = args[6].parse().map_err(|_| invalid())?;
    let peer = net::sockopt::socket_peercred(&socket)?;
    let peer_pid = peer.pid.as_raw_pid() as u32;
    if peer_pid != expected_peer
        || peer.uid != rustix::process::getuid()
        || peer.gid != rustix::process::getgid()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    if now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    if net::send(&socket, &request, SendFlags::NOSIGNAL)? != 983 {
        return Err(invalid());
    }
    let mut input = [0u8; 145];
    let (count, total) = net::recv(&socket, &mut input, RecvFlags::TRUNC)?;
    if count != 144 || total != 144 {
        return Err(invalid());
    }
    writeln!(
        io::stdout().lock(),
        "{}:{}:{}",
        parent.identity().hex(),
        peer_pid,
        hex(&input[..count])
    )?;
    io::stdout().flush()?;
    // Deliberately block only this supervised fixture worker until Stop kills it.
    // Unix record/EOF semantics here make no CPIF read-zero or retry claim.
    let _ = net::recv(&socket, &mut input, RecvFlags::empty())?;
    Err(invalid())
}
fn receive(owner: &mut Supervisor, until: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        if now() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if owner.poll_worker()? == WorkerState::Terminal {
            return Err(invalid());
        }
        let mut part = [0u8; 512];
        match owner.read_output(&mut part) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => {
                bytes.extend_from_slice(&part[..n]);
                if bytes.len() > 1024 {
                    return Err(invalid());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
        if let Some(end) = bytes.iter().position(|b| *b == b'\n') {
            if end + 1 != bytes.len() {
                return Err(io::Error::other(format!(
                    "frame trailing data: newline {end}, bytes {}",
                    bytes.len()
                )));
            }
            bytes.truncate(end);
            return Ok(bytes);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn dispose(owner: &mut Supervisor, identity: ProcessIdentity) -> io::Result<()> {
    let start = now();
    let cutoff = start.checked_add(2_000_000_000).ok_or_else(invalid)?;
    owner.stop()?;
    if owner.admits(identity) {
        return Err(invalid());
    }
    loop {
        if let Some(status) = owner.poll_reap()? {
            event(
                json!({"event":"worker-wait","actual_wait":true,"worker_signal":status.signal(),
                "worker_code":status.code(),"occupied":owner.occupied(),"admitted":owner.admits(identity),
                "stop_started_ns":start,"waited_at_ns":now(),"original_cleanup_until_ns":cutoff}),
            )?;
            if owner.occupied() || status.signal() != Some(9) || now() >= cutoff {
                return Err(invalid());
            }
            return Ok(());
        }
        if now() >= cutoff {
            return Err(io::ErrorKind::TimedOut.into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn parent(args: &[String]) -> io::Result<i32> {
    if args.len() != 8
        || args[0] != "--runtime"
        || args[2] != "--peer"
        || args[4] != "--case"
        || args[6] != "--peer-pid"
    {
        return Err(invalid());
    }
    let runtime = Path::new(&args[1]);
    let peer = Path::new(&args[3]);
    let mode = args[5].as_str();
    if !runtime.is_absolute()
        || !peer.is_absolute()
        || !["completion", "expiry", "cancel"].contains(&mode)
    {
        return Err(invalid());
    }
    let expected_peer: u32 = args[7].parse().map_err(|_| invalid())?;
    let journal = runtime.join("admission");
    let exe = std::env::current_exe()?;
    let context = NonZeroU8::new(1).unwrap();
    let request = checked(InitialLteSetupV3::new(
        context,
        "pokelinux.invalid",
        Protocol::Ipv4,
    ))?;
    let bytes = checked(request.encode())?;
    let until = now().checked_add(5_000_000_000).ok_or_else(invalid)?;
    let mut command = WorkerCommand::new(&exe)?;
    command
        .arg("--peer")
        .arg(peer)
        .arg("--packet")
        .arg(hex(&bytes))
        .arg("--peer-pid")
        .arg(expected_peer.to_string())
        .arg("--until-ns")
        .arg(until.to_string());
    let mut owner = Supervisor::new(journal.clone());
    let identity = match owner.start_worker(&mut command) {
        Ok(id) => id,
        Err(e) => {
            if owner.occupied() {
                let _ = owner.stop();
                let cleanup_until = now() + 2_000_000_000;
                loop {
                    if owner.poll_reap()?.is_some() {
                        break;
                    }
                    if now() >= cleanup_until {
                        std::process::abort();
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            return Err(io::Error::other(format!("start: {e:?}")));
        }
    };
    let worker_pid = owner.child_pid().ok_or_else(invalid)?;
    let result = (|| -> io::Result<bool> {
        let setup = checked(Setup::new(context, "pokelinux.invalid", Protocol::Ipv4, 0))?;
        let attempt = checked(Attempt::new(
            setup,
            identity,
            Arc::new(BootClock),
            Deadline::at(Duration::from_nanos(until)),
        ))?;
        // This precedes the first poll that may send GO to the owned worker.
        let permit = checked(attempt.take_dispatch())?;
        if permit.bytes() != &bytes {
            return Err(invalid());
        }
        event(
            json!({"event":"dispatch-unknown","worker_pid":worker_pid,"incarnation":identity.hex(),
            "request_bytes":permit.bytes().len(),"until_ns":until,"sampled_ns":now()}),
        )?;
        let frame = receive(&mut owner, until)?;
        let frame = std::str::from_utf8(&frame).map_err(|_| io::Error::other("frame UTF8"))?;
        let (source_text, rest) = frame
            .split_once(':')
            .ok_or_else(|| io::Error::other("frame source delimiter"))?;
        let (peer_text, record_hex) = rest
            .split_once(':')
            .ok_or_else(|| io::Error::other("frame peer delimiter"))?;
        let observed_peer: u32 = peer_text
            .parse()
            .map_err(|_| io::Error::other("frame peer integer"))?;
        if observed_peer != expected_peer {
            return Err(io::Error::other("frame peer mismatch"));
        }
        let source = ProcessIdentity::from_hex(source_text)
            .ok_or_else(|| io::Error::other("frame incarnation"))?;
        if !owner.admits_frame(identity, source, 1) {
            return Err(io::Error::other("frame source not admitted"));
        }
        let record =
            unhex(record_hex).map_err(|e| io::Error::other(format!("frame hex: {e:?}")))?;
        let candidate = checked(attempt.prepare_response(identity, &record))?;
        event(
            json!({"event":"prepared","actual_source":identity.hex(),"record_hex":hex(&record),
            "peer_pid_observed":observed_peer,"report":report(candidate.report()),"prepared_at_ns":now(),"original_until_ns":until}),
        )?;
        let start = now();
        for _ in 0..100 {
            let _ = checked(attempt.snapshot())?;
        }
        let snapshot_elapsed = now() - start;
        event(
            json!({"event":"snapshots","count":100,"elapsed_ns":snapshot_elapsed,"worker_still_owned":owner.occupied()}),
        )?;
        if snapshot_elapsed >= 200_000_000 || !owner.occupied() {
            return Err(invalid());
        }
        wait_release(until)?;
        if mode == "expiry" {
            while now() < until + 10_000_000 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if mode == "cancel" {
            checked(attempt.cancel())?;
        }
        if owner.poll_reap()?.is_some() || !owner.admits(identity) {
            checked(attempt.source_lost(identity))?;
        }
        let expected_source = candidate.source();
        let expected_record = *candidate.record();
        let expected_report = candidate.report().clone();
        let committed = attempt.commit(candidate);
        let expected = match mode {
            "completion" => match committed {
                Ok(snapshot) => match (&snapshot.current, &snapshot.history) {
                    (Some(current), Some(history)) => {
                        Arc::ptr_eq(current, history)
                            && current.source() == expected_source
                            && current.record() == &expected_record
                            && current.report() == &expected_report
                    }
                    _ => false,
                },
                Err(AttemptError::Unsupported) => false,
                Err(e) => return Err(io::Error::other(format!("commit: {e:?}"))),
            },
            "expiry" => {
                if !matches!(committed, Err(AttemptError::Deadline)) {
                    return Err(invalid());
                }
                true
            }
            "cancel" => {
                if !matches!(committed, Err(AttemptError::Cancelled)) {
                    return Err(invalid());
                }
                true
            }
            _ => unreachable!(),
        };
        let snapshot = checked(attempt.snapshot())?;
        event(
            json!({"event":"commit","case":mode,"completion_present":snapshot.current.is_some(),
            "history_present":snapshot.history.is_some(),"disposition":format!("{:?}",snapshot.disposition),
            "withdrawal":format!("{:?}",snapshot.withdrawal),"sampled_ns":now(),"original_until_ns":until,
            "capability_present":expected}),
        )?;
        checked(attempt.cancel())?;
        let withdrawn = checked(attempt.snapshot())?;
        if withdrawn.current.is_some()
            || withdrawn.history.is_some() != (mode == "completion" && expected)
        {
            return Err(invalid());
        }
        Ok(expected)
    })();
    // Cleanup and durable refusal occur before a deliberately missing-capability RED.
    let cleanup = dispose(&mut owner, identity);
    cleanup?;
    let mut restart = Supervisor::new(journal);
    let refused = restart.start_worker(&mut command);
    if refused != Err(StartError::AdmissionUncertain) || restart.occupied() {
        return Err(invalid());
    }
    event(
        json!({"event":"successor-refused","reason":"AdmissionUncertain","occupied":restart.occupied()}),
    )?;
    let capability = match result {
        Ok(value) => value,
        Err(error) => {
            event(json!({"event":"fixture-failure","error":format!("{error:?}")}))?;
            return Err(error);
        }
    };
    match capability {
        true => Ok(0),
        false => {
            event(json!({"event":"MISSING_OWNED_SIT_COMPLETION","actual_exit":101}))?;
            Ok(101)
        }
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let code = if args.first().is_some_and(|s| s == WORKER_SELECTOR) {
        match worker(&args) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("fixture worker: {e}");
                70
            }
        }
    } else {
        configure_output()
            .and_then(|()| parent(&args))
            .unwrap_or(70)
    };
    std::process::exit(code);
}
