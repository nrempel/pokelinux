//! Private, unexecuted desired consumer: normal Main + actual verified Rust client.
//! Root-private ingress only; no app grants, Settings, Source or hardware claim.
use pokelinux_connectivity_control::{
    client::Client,
    protocol::{Choice, Reply, Status},
};
use pokelinux_network_state::boot_time_ns;
use serde_json::{Value, json};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
const PEER: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 2);
const PORT: u16 = 36042;
const MARKER: &str = "CONNECT_PRIVATE_INGRESS_REQUIRES_REAL_OBSERVED_TRUE";
fn pause() { thread::sleep(Duration::from_millis(2)); }
fn until(case: u64) -> u64 { case.min(boot_time_ns().checked_add(1_000_000_000).unwrap()) }
fn publish(root: &Path, name: &str, value: &Value) {
    let temporary = root.join(format!("{name}.tmp"));
    fs::write(&temporary, serde_json::to_vec(value).unwrap()).unwrap();
    fs::rename(temporary, root.join(name)).unwrap();
}
fn gate(root: &Path, name: &str, end: u64) -> Value {
    while boot_time_ns() < end {
        match fs::read(root.join(name)) {
            Ok(bytes) => return serde_json::from_slice(&bytes).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => pause(),
            Err(e) => panic!("gate {name}: {e}"),
        }
    }
    panic!("original cutoff waiting for {name}");
}
fn caps() -> Value {
    let text = fs::read_to_string("/proc/self/status").unwrap();
    let mut values = serde_json::Map::new();
    for name in ["Uid", "Gid", "CapEff", "CapPrm", "CapBnd", "CapInh", "CapAmb", "NoNewPrivs"] {
        let prefix = format!("{name}:");
        let raw = text.lines().find(|line| line.starts_with(&prefix)).unwrap();
        let value = raw.split_once(':').unwrap().1.trim();
        match name {
            "Uid" | "Gid" => assert!(value.split_whitespace().all(|v| v == "0")),
            "NoNewPrivs" => assert_eq!(value, "1"),
            _ => assert_eq!(u64::from_str_radix(value, 16).unwrap(), 0, "driver must enter cap0"),
        }
        values.insert(name.into(), json!(value));
    }
    Value::Object(values)
}
fn status(value: &Status) -> &'static str {
    match value {
        Status::Observed => "Observed",
        Status::Completed => "Completed",
        Status::Refused => "Refused",
        Status::Unavailable => "Unavailable",
    }
}
fn view(reply: &Reply) -> Value {
    let s = &reply.state;
    json!({"id":reply.id,"status":status(&reply.status),"state":{
        "available":s.available,"configured":s.configured,"observed_at_ns":s.observed_at_ns,
        "until_ns":s.until_ns,"pending":s.pending,"terminal_uncertain":s.terminal_uncertain,"worker":s.worker}})
}
fn decimal(value: &str) -> u64 {
    let parsed = value.parse::<u64>().unwrap();
    assert_eq!(parsed.to_string(), value, "canonical absolute BOOTTIME field required");
    parsed
}
fn healthy(reply: &Reply, configured: bool, worker: Option<&str>) -> u64 {
    let s = &reply.state;
    let now = boot_time_ns();
    let observed = decimal(&s.observed_at_ns);
    let expires = decimal(&s.until_ns);
    assert!(s.available && !s.pending && !s.terminal_uncertain);
    assert_eq!(s.configured, Some(configured));
    assert!(observed <= now && now < expires && expires - observed <= 250_000_000);
    assert_eq!(s.worker.len(), 32);
    assert!(s.worker.bytes().all(|v| v.is_ascii_hexdigit()));
    assert_ne!(u128::from_str_radix(&s.worker, 16).unwrap(), 0);
    if let Some(worker) = worker { assert_eq!(s.worker, worker, "same persistent worker epoch required"); }
    observed
}
fn receive(client: &mut Client, id: u64, end: u64) -> Reply {
    while boot_time_ns() < end {
        match client.poll() {
            Ok(Some(reply)) => {
                assert_eq!(reply.id, id, "one original request correlation");
                assert!(boot_time_ns() < end);
                return reply;
            }
            Ok(None) => pause(),
            Err(error) => panic!("actual verified Rust client poll: {error}"),
        }
    }
    panic!("original request deadline expired without typed reply");
}
fn inspect(client: &mut Client, next: &mut u64, case: u64) -> Reply {
    *next = next.checked_add(1).unwrap();
    let end = until(case);
    client.inspect(*next, end).expect("actual private Inspect send");
    let reply = receive(client, *next, end);
    assert!(matches!(&reply.status, Status::Observed), "healthy setup needs actual Inspect observation");
    reply
}
fn nonce(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0_u8; 37];
    bytes[..4].copy_from_slice(b"PKL1");
    bytes[4] = tag;
    File::open("/dev/urandom").unwrap().read_exact(&mut bytes[5..]).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String { bytes.iter().map(|v| format!("{v:02x}")).collect() }
fn roundtrip(bytes: &[u8], case: u64) -> Value {
    let end = until(case);
    let udp = UdpSocket::bind((LOCAL, 0)).unwrap();
    let sampled = boot_time_ns();
    assert!(sampled < end);
    udp.set_read_timeout(Some(Duration::from_nanos(end - sampled))).unwrap();
    let source = udp.local_addr().unwrap();
    let target = SocketAddr::from((PEER, PORT));
    let before = boot_time_ns();
    assert_eq!(udp.send_to(bytes, target).unwrap(), bytes.len(), "one nonce, no retry");
    let mut answer = [0_u8; 128];
    let (length, from) = udp.recv_from(&mut answer).unwrap();
    assert_eq!(from, target);
    assert_eq!(&answer[..4], b"ACK1");
    assert_eq!(&answer[4..length], bytes);
    assert!(boot_time_ns() < end);
    json!({"nonce_hex":hex(bytes),"source":source.to_string(),"peer":target.to_string(),
        "send_before_ns":before,"received_at_ns":boot_time_ns(),"received_bytes":length,"original_until_ns":end})
}
fn body(socket: &Path, root: &Path, main_pid: u32, case: u64, ready_until: u64) -> Value {
    let driver_caps = caps();
    let mut next = 0_u64;
    let mut client = Client::new(socket.into()).unwrap();
    // Each attempt is an explicit readonly request, bounded by the single
    // original startup cutoff; Unavailable is setup state, never behavioral RED.
    let first = loop {
        assert!(boot_time_ns() < ready_until, "actual Main/worker readiness before original startup cutoff");
        next += 1;
        let end = until(ready_until);
        client.inspect(next, end).expect("actual initial private Inspect send");
        let reply = receive(&mut client, next, end);
        if matches!(&reply.status, Status::Observed) && reply.state.available { break reply; }
        assert!(matches!(&reply.status, Status::Unavailable));
        pause();
    };
    healthy(&first, false, None);
    let worker = first.state.worker.clone();
    publish(root, "created.json", &json!({"driver_pid":std::process::id(),"main_pid":main_pid,
        "driver_caps":driver_caps,"initial":view(&first),"worker":worker,"created_at_ns":boot_time_ns()}));
    let ready = gate(root, "ready.json", case);
    for key in ["provisioner_reaped_before_packet_peer", "supervisor_all_caps_zero", "packet_peer_all_caps_zero",
        "Main_all_caps_zero", "driver_all_caps_zero", "worker_NET_ADMIN_only", "actual_UID11050_private_access_denied"] {
        assert_eq!(ready[key], true, "healthy prerequisite {key}");
    }
    let baseline = inspect(&mut client, &mut next, case);
    healthy(&baseline, false, Some(&worker));
    publish(root, "initial-proof.json", &json!({"state":view(&baseline),"requested_at_ns":boot_time_ns()}));
    let initial = gate(root, "initial-readback.json", case);
    assert_eq!(initial["ipv4"], json!([]));
    assert_eq!(initial["default_route_absent"], true);
    next += 1;
    let connect_until = until(case);
    publish(root, "connect-attempt.json", &json!({"id":next,"attempted_at_ns":boot_time_ns(),
        "original_until_ns":connect_until,"root_private_role_only":true}));
    client.apply(next, Choice::Connect, connect_until).expect("valid typed Connect must actually be sent once");
    let connected = receive(&mut client, next, connect_until);
    publish(root, "connect-return.json", &json!({"reply":view(&connected),"returned_at_ns":boot_time_ns()}));
    if !matches!(&connected.status, Status::Completed) || connected.state.configured != Some(true) {
        assert!(matches!(&connected.status, Status::Refused), "only explicit typed refusal is this frozen RED; other outcomes remain failure");
        healthy(&connected, false, Some(&worker));
        let after = inspect(&mut client, &mut next, case);
        healthy(&after, false, Some(&worker));
        publish(root, "refused-proof.json", &json!({"state":view(&after),"requested_at_ns":boot_time_ns()}));
        let independent = gate(root, "refused-readback.json", case);
        assert_eq!(independent["ipv4"], json!([]));
        client.withdraw();
        return json!({"functional_green":false,"genuine_refused_candidate":true,"marker":MARKER,
            "configured_positive":false,"root_private_role_only":true,"Settings_or_app_permission_executed":false,
            "worker":worker,"returned":view(&connected),"refused_readback":independent,"functional_finished_at_ns":boot_time_ns()});
    }
    healthy(&connected, true, Some(&worker));
    publish(root, "connected-proof.json", &json!({"state":view(&connected),"requested_at_ns":boot_time_ns()}));
    let independent_connected = gate(root, "connected-readback.json", case);
    assert_eq!(independent_connected["ipv4"].as_array().unwrap().len(), 1);
    let first_nonce = nonce(1);
    let first_packet = roundtrip(&first_nonce, case);
    let withdrawn_at = boot_time_ns();
    client.withdraw();
    drop(client);
    publish(root, "requester-withdrawn.json", &json!({"withdrawn_at_ns":withdrawn_at,"worker":worker}));
    // A distinct Rust client can only Inspect before the next explicit Apply.
    let mut fresh = Client::new(socket.into()).unwrap();
    let idle_until = case.min(boot_time_ns() + 250_000_000);
    let mut idle = Vec::new();
    let mut last_observed = withdrawn_at;
    while boot_time_ns() < idle_until {
        let observation = inspect(&mut fresh, &mut next, idle_until);
        let at = healthy(&observation, true, Some(&worker));
        if at > last_observed {
            idle.push(view(&observation));
            last_observed = at;
            if idle.len() == 2 { break; }
        }
        pause();
    }
    assert_eq!(idle.len(), 2, "two actual fresh idle observations after private requester close");
    let second_nonce = nonce(2);
    assert_ne!(first_nonce, second_nonce);
    let background_packet = roundtrip(&second_nonce, case);
    next += 1;
    let disconnect_until = until(case);
    fresh.apply(next, Choice::Disconnect, disconnect_until).expect("one explicit fresh-client Disconnect");
    let disconnected = receive(&mut fresh, next, disconnect_until);
    assert!(matches!(&disconnected.status, Status::Completed));
    healthy(&disconnected, false, Some(&worker));
    publish(root, "disconnected-proof.json", &json!({"state":view(&disconnected),"requested_at_ns":boot_time_ns()}));
    let independent_disconnected = gate(root, "disconnected-readback.json", case);
    assert_eq!(independent_disconnected["ipv4"], json!([]));
    let loss_nonce = nonce(3);
    assert_ne!(second_nonce, loss_nonce);
    let quiet_until = boot_time_ns() + 800_000_000;
    assert!(quiet_until < case, "full original800ms quiet measurement must fit original six-second aggregate");
    publish(root, "quiet-request.json", &json!({"original_until_ns":quiet_until,
        "probe_hex":hex(&loss_nonce),"requested_at_ns":boot_time_ns()}));
    let armed = gate(root, "peer-quiet-armed.json", quiet_until);
    assert_eq!(armed["original_until_ns"], quiet_until);
    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let sent_at = boot_time_ns();
    assert!(sent_at < quiet_until);
    let sent = udp.send_to(&loss_nonce, (PEER, PORT));
    if let Err(ref error) = sent { assert!(matches!(error.raw_os_error(), Some(99 | 101 | 113))); }
    let sent_debug = format!("{sent:?}");
    udp.set_nonblocking(true).unwrap();
    let mut answer = [0_u8; 128];
    while boot_time_ns() < quiet_until {
        match udp.recv_from(&mut answer) {
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {}
            other => panic!("disconnected UDP socket must remain quiet: {other:?}"),
        }
        pause();
    }
    let quiet = gate(root, "peer-quiet.json", case);
    assert_eq!(quiet["received_datagrams"], 0);
    assert_eq!(quiet["original_until_ns"], quiet_until);
    assert!(quiet["armed_at_ns"].as_u64().unwrap() <= sent_at);
    assert_eq!(quiet["quiet_end_actual_EAGAIN"], true);
    assert!(quiet["quiet_end_EAGAIN_at_ns"].as_u64().unwrap() >= quiet_until);
    fresh.withdraw();
    json!({"functional_green":true,"genuine_refused_candidate":false,"configured_positive":true,
        "root_private_role_only":true,"Settings_or_app_permission_executed":false,"worker":worker,
        "connected":view(&connected),"disconnected":view(&disconnected),"first_packet":first_packet,
        "withdrawn_at_ns":withdrawn_at,"fresh_idle_observations":idle,"background_packet_after_withdraw":background_packet,
        "connected_independent":independent_connected,"disconnected_independent":independent_disconnected,
        "fresh_loss_probe":{"nonce_hex":hex(&loss_nonce),"send_result":sent_debug,"sent_at_ns":sent_at},
        "peer_quiet":quiet,"original_case_until_ns":case,"functional_finished_at_ns":boot_time_ns()})
}
fn main() {
    assert_eq!(env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(), Ok("1"));
    let args: Vec<_> = env::args().skip(1).collect();
    assert_eq!(args.len(), 6, "socket gates main-pid original-case-until original-close-until original-start-until");
    let socket = PathBuf::from(&args[0]);
    let root = PathBuf::from(&args[1]);
    let main_pid: u32 = args[2].parse().unwrap();
    let case: u64 = args[3].parse().unwrap();
    let close: u64 = args[4].parse().unwrap();
    let ready_until: u64 = args[5].parse().unwrap();
    assert!(socket.is_absolute() && root.is_absolute() && main_pid > 0);
    assert!(boot_time_ns() < case && case < close && case - boot_time_ns() <= 6_000_000_000);
    // No target assertion is thrown until the controller's actual Main/worker
    // wait+EOF closure has completed. A caught setup panic is never a RED.
    let result = panic::catch_unwind(AssertUnwindSafe(|| body(&socket, &root, main_pid, case, ready_until)));
    let result = match result {
        Ok(value) => value,
        Err(error) => {
            let message = error.downcast_ref::<String>().cloned()
                .or_else(|| error.downcast_ref::<&str>().map(|v| v.to_string()))
                .unwrap_or_else(|| "non-string panic".to_owned());
            json!({"functional_green":false,"genuine_refused_candidate":false,"setup_error":message,
                "functional_finished_at_ns":boot_time_ns()})
        }
    };
    publish(&root, "functional-finished.json", &result);
    let cleanup = gate(&root, "cleanup-complete.json", close);
    assert_eq!(cleanup["main_worker_packet_keeper_actual_wait_EOF_closed"], true, "owned backend closure before desired assertion");
    assert_eq!(cleanup["all_namespace_handles_retained"], true);
    publish(&root, "desired-assertion.json", &json!({"asserted_after_cleanup_at_ns":boot_time_ns(),
        "cleanup_completed_at_ns":cleanup["completed_at_ns"],"genuine_refused_candidate":result["genuine_refused_candidate"],
        "functional_green":result["functional_green"],"original_close_until_ns":close}));
    if result["genuine_refused_candidate"] == true {
        assert!(result["configured_positive"] == true, "CONNECT_PRIVATE_INGRESS_REQUIRES_REAL_OBSERVED_TRUE");
    }
    assert!(result["functional_green"] == true, "actual consumer failed outside the frozen Connect assertion: {result}");
    println!("{}", json!({"passed":true,"normal_Main_and_Rust_client":true,"Settings_or_app_permission_executed":false}));
}
