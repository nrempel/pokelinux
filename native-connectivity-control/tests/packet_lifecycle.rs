//! Actual Linux fixed-profile packet behavior. Literal owner authority below is a
//! trusted fixture input, not Settings UI, OS permission or service integration.
use pokelinux_connectivity_control::{
    kernel::{Fact, VirtualLink},
    owner::{Action, Authority, Finish, Owner, Principal},
};
use pokelinux_network_state::{KernelObserver, ObserveError, boot_time_ns};
use serde_json::{Value, json};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
const PEER: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 2);
const PORT: u16 = 36042;
fn until(case: u64) -> u64 {
    case.min(boot_time_ns().checked_add(1_000_000_000).unwrap())
}
fn publish(root: &Path, name: &str, value: Value) {
    let temporary = root.join(format!("{name}.tmp"));
    fs::write(&temporary, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::rename(temporary, root.join(name)).unwrap();
}
fn gate(root: &Path, name: &str, end: u64) -> Value {
    while boot_time_ns() < end {
        match fs::read(root.join(name)) {
            Ok(bytes) => return serde_json::from_slice(&bytes).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(e) => panic!("gate {name}: {e}"),
        }
    }
    panic!("original case cutoff waiting for {name}");
}
fn fact(value: Fact) -> Value {
    json!({"namespace":{"dev":value.namespace.dev,"ino":value.namespace.ino},
        "ifindex":value.ifindex,"lifetime":value.lifetime,"connected":value.connected})
}
fn auth(instance: u64) -> Authority {
    Authority {
        principal: Principal {
            instance,
            generation: 7,
        },
        permission_epoch: 11,
        allowed: true,
    }
}
fn same(first: Fact, later: Fact, connected: bool) {
    assert_eq!(
        (first.namespace, first.ifindex, first.lifetime),
        (later.namespace, later.ifindex, later.lifetime)
    );
    assert_eq!(later.connected, connected);
}
fn inventory(first: Fact, connected: bool, case: u64) -> Value {
    let end = until(case);
    let mut observer = KernelObserver::subscribe_current_namespace(end).unwrap();
    let snapshot = loop {
        match observer.refresh(end) {
            Ok(value) => break value,
            Err(ObserveError::ConcurrentChange) if boot_time_ns() < end => {}
            other => panic!("independent kernel readback: {other:?}"),
        }
    };
    assert_eq!(snapshot.namespace, first.namespace);
    assert_eq!(snapshot.links.len(), 2);
    let link = snapshot
        .links
        .iter()
        .find(|link| link.ifindex == first.ifindex)
        .unwrap();
    assert_eq!(link.name, "pklown0");
    assert_ne!(link.flags & 1, 0);
    let ipv4: Vec<_> = link
        .addresses
        .iter()
        .filter(|a| a.local.is_ipv4())
        .collect();
    if connected {
        assert_eq!(ipv4.len(), 1);
        assert_eq!(ipv4[0].local, IpAddr::V4(LOCAL));
        assert_eq!(ipv4[0].prefix_len, 30);
    } else {
        assert!(ipv4.is_empty());
    }
    let routes = fs::read_to_string("/proc/net/route").unwrap();
    assert!(
        !routes
            .lines()
            .skip(1)
            .any(|line| line.split_whitespace().nth(1) == Some("00000000"))
    );
    json!({"observed_at_ns":snapshot.observed_at_boottime_ns,"local_ipv4_records":ipv4.len(),"default_route_absent":true})
}
fn nonce(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0_u8; 37];
    bytes[..4].copy_from_slice(b"PKL1");
    bytes[4] = tag;
    File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut bytes[5..])
        .unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn roundtrip(bytes: &[u8], case: u64) -> Value {
    let end = until(case);
    let socket = UdpSocket::bind((LOCAL, 0)).unwrap();
    let sampled = boot_time_ns();
    assert!(sampled < end, "original UDP cutoff before timeout setup");
    socket
        .set_read_timeout(Some(Duration::from_nanos(end - sampled)))
        .unwrap();
    let source = socket.local_addr().unwrap();
    let target = SocketAddr::from((PEER, PORT));
    assert_eq!(
        socket.send_to(bytes, target).unwrap(),
        bytes.len(),
        "one real nonce send, no retry"
    );
    let mut answer = [0_u8; 128];
    let (length, from) = socket.recv_from(&mut answer).unwrap();
    assert_eq!(from, target);
    assert_eq!(&answer[..4], b"ACK1");
    assert_eq!(&answer[4..length], bytes);
    assert!(boot_time_ns() < end);
    json!({"nonce_hex":hex(bytes),"source":source.to_string(),"peer":target.to_string(),"received_bytes":length,"original_until_ns":end,"observed_at_ns":boot_time_ns()})
}
#[test]
#[ignore = "requires owned fresh namespaces, serialized peer provisioner and cap0 packet peer"]
fn actual_fixed_ipv4_packet_lifecycle() {
    assert_eq!(
        env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),
        Ok("1")
    );
    let root = PathBuf::from(env::var_os("POKELINUX_CONNECTIVITY_GATES").unwrap());
    let case: u64 = env::var("POKELINUX_CONNECTIVITY_CASE_UNTIL_NS")
        .unwrap()
        .parse()
        .unwrap();
    let started = boot_time_ns();
    assert!(started < case && case - started <= 6_000_000_000);
    let status = fs::read_to_string("/proc/self/status").unwrap();
    for name in ["CapEff:", "CapPrm:", "CapBnd:"] {
        let raw = status
            .lines()
            .find(|line| line.starts_with(name))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        assert_eq!(u64::from_str_radix(raw, 16).unwrap(), 1 << 12);
    }
    for name in ["CapInh:", "CapAmb:"] {
        let raw = status
            .lines()
            .find(|line| line.starts_with(name))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        assert_eq!(u64::from_str_radix(raw, 16).unwrap(), 0);
    }
    assert!(
        status
            .lines()
            .any(|line| line.starts_with("NoNewPrivs:")
                && line.split_whitespace().nth(1) == Some("1"))
    );
    let mut link = VirtualLink::create(
        File::open(env::var_os("POKELINUX_CONNECTIVITY_PEER_NS").unwrap()).unwrap(),
        until(case),
    )
    .unwrap();
    let original = link.fact().unwrap();
    assert!(!original.connected);
    publish(
        &root,
        "created.json",
        json!({"pid":std::process::id(),"fact":fact(original),"created_at_ns":boot_time_ns()}),
    );
    let ready = gate(&root, "ready.json", case);
    assert_eq!(ready["provisioner_reaped_before_packet_peer"], true);
    assert_eq!(ready["supervisor_all_caps_zero"], true);
    assert_eq!(ready["packet_peer_all_caps_zero"], true);
    let first = auth(101);
    let mut owner = Owner::new();
    owner.observe(false, boot_time_ns());
    let connect_until = until(case);
    let id = owner
        .stage(first, Action::Connect, connect_until, boot_time_ns())
        .unwrap();
    let command = owner.dispatch(id, first, boot_time_ns()).unwrap();
    publish(
        &root,
        "connect-attempt.json",
        json!({"original_until_ns":command.original_until_ns,"attempted_at_ns":boot_time_ns(),"fixture_authority_only":true}),
    );
    let connected = link.set_connected(true, command.original_until_ns);
    publish(
        &root,
        "connect-return.json",
        json!({"result_debug":format!("{connected:?}"),"returned_at_ns":boot_time_ns()}),
    );
    // Genuine behavioral RED is this desired success assertion on current
    // Error::Unsupported, AFTER actual peer provisioning and capability removal.
    let connected = connected.expect("CONNECT_REQUIRES_ACTUAL_FIXED_IPV4_SUCCESS");
    same(original, connected, true);
    assert!(boot_time_ns() < connect_until);
    assert!(owner.finish(id, boot_time_ns(), Finish::Observed(connected.connected)));
    assert_eq!(
        owner.take_reply(id, first, boot_time_ns()).unwrap().outcome,
        Finish::Observed(true)
    );
    let connected_inventory = inventory(original, true, case);
    let n1 = nonce(1);
    let first_packet = roundtrip(&n1, case);
    publish(
        &root,
        "connected-proof.json",
        json!({"fact":fact(connected),"first_packet":first_packet}),
    );
    let independent = gate(&root, "connected-readback.json", case);
    assert_eq!(
        independent["ipv4"],
        json!([{"ifindex":original.ifindex,"prefix":30,"local":"192.0.2.1"}])
    );
    assert_eq!(independent["default_route_absent"], true);
    owner.withdraw(first.principal, boot_time_ns());
    assert!(owner.view(boot_time_ns()).pending.is_none());
    assert!(owner.view(boot_time_ns()).observed.unwrap().connected);
    let n2 = nonce(2);
    assert_ne!(n1, n2);
    let background_packet = roundtrip(&n2, case);
    same(original, link.fact().unwrap(), true);
    let fresh = auth(202);
    assert_ne!(first.principal, fresh.principal);
    let disconnect_until = until(case);
    let id = owner
        .stage(fresh, Action::Disconnect, disconnect_until, boot_time_ns())
        .unwrap();
    let command = owner.dispatch(id, fresh, boot_time_ns()).unwrap();
    let disconnected = link
        .set_connected(false, command.original_until_ns)
        .expect("DISCONNECT_REQUIRES_ACTUAL_FIXED_IPV4_REMOVAL");
    same(original, disconnected, false);
    assert!(boot_time_ns() < disconnect_until);
    assert!(owner.finish(id, boot_time_ns(), Finish::Observed(disconnected.connected)));
    assert_eq!(
        owner.take_reply(id, fresh, boot_time_ns()).unwrap().outcome,
        Finish::Observed(false)
    );
    let disconnected_inventory = inventory(original, false, case);
    publish(
        &root,
        "disconnected-proof.json",
        json!({"fact":fact(disconnected)}),
    );
    let independent = gate(&root, "disconnected-readback.json", case);
    assert_eq!(independent["ipv4"], json!([]));
    assert_eq!(independent["default_route_absent"], true);
    let n3 = nonce(3);
    assert_ne!(n2, n3);
    let quiet_until = boot_time_ns() + 800_000_000;
    assert!(
        quiet_until < case,
        "full original800ms quiet window fits original case"
    );
    publish(
        &root,
        "quiet-request.json",
        json!({"original_until_ns":quiet_until,"probe_hex":hex(&n3),"requested_at_ns":boot_time_ns()}),
    );
    let armed = gate(&root, "peer-quiet-armed.json", quiet_until);
    assert_eq!(armed["original_until_ns"], quiet_until);
    // A new socket cannot receive an ACK buffered by either successful socket.
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let sent_at = boot_time_ns();
    assert!(sent_at < quiet_until);
    let send = socket.send_to(&n3, (PEER, PORT));
    let send_debug = format!("{send:?}");
    if let Err(ref error) = send {
        assert!(matches!(error.raw_os_error(), Some(99 | 101 | 113)));
    }
    socket.set_nonblocking(true).unwrap();
    let mut response = [0_u8; 128];
    while boot_time_ns() < quiet_until {
        match socket.recv_from(&mut response) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            other => {
                panic!("fresh disconnected UDP socket received an unexpected result: {other:?}")
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    let quiet = gate(&root, "peer-quiet.json", case);
    assert_eq!(quiet["received_datagrams"], 0);
    assert_eq!(quiet["original_until_ns"], quiet_until);
    assert!(quiet["armed_at_ns"].as_u64().unwrap() <= sent_at);
    assert!(quiet["completed_at_ns"].as_u64().unwrap() >= quiet_until);
    assert_eq!(quiet["quiet_end_actual_EAGAIN"], true);
    assert!(quiet["quiet_end_EAGAIN_at_ns"].as_u64().unwrap() >= quiet_until);
    assert!(boot_time_ns() < case);
    publish(
        &root,
        "passed.json",
        json!({"passed":true,"fixture_authority_only":true,"Settings_window_executed":false,
        "original_fact":fact(original),"connected_fact":fact(connected),"disconnected_fact":fact(disconnected),
        "connected_inventory":connected_inventory,"disconnected_inventory":disconnected_inventory,
        "first_packet":first_packet,"background_packet_after_withdraw":background_packet,
        "disconnect_fresh_principal":202,"fresh_loss_probe":{"nonce_hex":hex(&n3),"send_result":send_debug,"sent_at_ns":sent_at},
        "peer_quiet":quiet,"original_case_until_ns":case,"completed_at_ns":boot_time_ns()}),
    );
}
