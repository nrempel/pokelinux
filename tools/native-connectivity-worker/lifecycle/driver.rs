//! Private standalone desired behavior. Runs Service on the actual main thread.
//! Literal Authority is a trusted fixture input, never Settings/OS permission.
use pokelinux_connectivity_control::{
    kernel::Fact,
    owner::{Action, Authority, Finish, Id, Principal, Reply},
    service::{Service, Snapshot},
};
use pokelinux_network_state::{KernelObserver, ObserveError, boot_time_ns};
use serde_json::{Value, json};
use std::{
    env, fs::{self, File}, io::{self, Read},
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::{fs::DirBuilderExt, process::ExitStatusExt},
    panic::{self, AssertUnwindSafe}, path::{Path, PathBuf},
    thread, time::Duration,
};
const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
const PEER: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 2);
const PORT: u16 = 36042;
fn caps() -> Value {
    let text=fs::read_to_string("/proc/self/status").unwrap();
    let mut out=serde_json::Map::new();
    for name in ["CapEff", "CapPrm", "CapBnd", "CapInh", "CapAmb", "NoNewPrivs"] {
        let prefix=format!("{name}:");
        let value=text.lines().find(|line|line.starts_with(&prefix)).unwrap().split_whitespace().nth(1).unwrap();
        if name=="NoNewPrivs" {assert_eq!(value,"1");} else {assert_eq!(u64::from_str_radix(value,16).unwrap(),0,"Main must be irreversibly cap0 before clients");}
        out.insert(name.to_owned(),json!(value));
    }
    Value::Object(out)
}
fn snapshot(value: Snapshot) -> Value {
    json!({"worker":value.worker.map(|id|id.hex()),"ready":value.ready,
        "fact":value.fact.map(fact),"observed_at_ns":value.observed_at_ns,
        "owner_pending":value.owner.pending.map(|p|json!({"id":p.command.id.0,"phase":format!("{:?}",p.phase),"original_until_ns":p.command.original_until_ns})),
        "owner_observed":value.owner.observed.map(|o|json!({"connected":o.connected,"observed_at_ns":o.observed_at_ns})),
        "owner_terminal_uncertain":value.owner.terminal_uncertain,
        "terminal_uncertain":value.terminal_uncertain,"stopping":value.stopping,
        "output_eof":value.output_eof,"reaped":value.reaped,"occupied":value.occupied,
        "namespace_handles_open":value.namespace_handles_open,"main_caps_dropped":value.main_caps_dropped,
        "exit_status":value.exit_status.map(|status|json!({"code":status.code(),"signal":status.signal(),"success":status.success()}))})
}
fn poll(service: &mut Service) -> Snapshot {
    let began=boot_time_ns();
    service.poll().unwrap();
    let result=service.snapshot();
    assert!(boot_time_ns()-began<100_000_000,"Main poll+snapshot exceeded original100ms bound");
    result
}
fn reply(service:&mut Service,id:Id,authority:Authority,end:u64)->Reply {
    while boot_time_ns()<end {
        poll(service);
        if let Some(value)=service.take_reply(id,authority) {return value;}
        thread::sleep(Duration::from_millis(2));
    }
    panic!("original command deadline expired without reply");
}
fn idle(service:&mut Service,first:Fact,worker:&str,after:u64,case:u64)->Value {
    let end=case.min(boot_time_ns()+250_000_000);let mut samples=Vec::new();let mut last=after;
    while boot_time_ns()<end {
        let observed=poll(service);
        assert_eq!(observed.worker.map(|id|id.hex()).as_deref(),Some(worker));
        assert!(observed.owner.pending.is_none());
        if let (Some(value),Some(at))=(observed.fact,observed.observed_at_ns) {
            same(first,value,true);
            if observed.ready && at>last {samples.push(snapshot(observed));last=at;}
            if samples.len()==2 {return json!({"after_ns":after,"samples":samples,"original_until_ns":end});}
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("same persistent worker requires two actual fresh idle observations");
}
fn until(case: u64) -> u64 {
    case.min(boot_time_ns().checked_add(1_000_000_000).unwrap())
}
fn publish(root: &Path, name: &str, value: Value) {
    let temporary = root.join(format!("{name}.tmp"));
    fs::write(&temporary, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::rename(temporary, root.join(name)).unwrap();
}
fn gate(service: &mut Service, root: &Path, name: &str, end: u64) -> Value {
    while boot_time_ns() < end {
        poll(service);
        match fs::read(root.join(name)) {
            Ok(bytes) => return serde_json::from_slice(&bytes).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => thread::sleep(Duration::from_millis(2)),
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
fn lifecycle(service:&mut Service,root:&Path,case:u64)->Value {
    let start_end=until(case);
    let initial=loop {
        let value=poll(service);
        if value.ready {break value;}
        assert!(boot_time_ns()<start_end && !value.stopping,"actual worker startup must become Ready");
        thread::sleep(Duration::from_millis(2));
    };
    let original=initial.fact.unwrap();assert!(!original.connected);
    let worker=initial.worker.unwrap().hex();
    assert!(initial.main_caps_dropped);
    let main_caps=caps();
    publish(root,"created.json",json!({"pid":std::process::id(),"worker_pid":service.child_pid().unwrap(),"worker":worker,"fact":fact(original),"main_caps":main_caps,"created_at_ns":boot_time_ns(),"snapshot":snapshot(initial)}));
    let ready=gate(service,root,"ready.json",case);
    assert_eq!(ready["provisioner_reaped_before_packet_peer"],true);
    assert_eq!(ready["supervisor_all_caps_zero"],true);
    assert_eq!(ready["packet_peer_all_caps_zero"],true);
    assert_eq!(ready["Main_all_caps_zero"],true);
    assert_eq!(ready["worker_NET_ADMIN_only"],true);
    let first=auth(101);let connect_until=until(case);
    publish(root,"connect-attempt.json",json!({"original_until_ns":connect_until,"attempted_at_ns":boot_time_ns(),"fixture_authority_only":true}));
    let submitted=service.submit(first,Action::Connect,connect_until);
    if let Err(error)=submitted {
        publish(root,"connect-return.json",json!({"submitted_debug":format!("Err({error:?})"),"returned_at_ns":boot_time_ns()}));
        panic!("CONNECT_REQUIRES_REAL_WORKER_OBSERVED_TRUE: submit refused {error:?}");
    }
    let id=submitted.unwrap();let result=reply(service,id,first,connect_until);
    publish(root,"connect-return.json",json!({"id":id.0,"outcome_debug":format!("{:?}",result.outcome),"returned_at_ns":boot_time_ns(),"snapshot":snapshot(service.snapshot())}));
    assert_eq!(result.outcome,Finish::Observed(true),"CONNECT_REQUIRES_REAL_WORKER_OBSERVED_TRUE");
    let connected=service.snapshot().fact.unwrap();same(original,connected,true);
    assert_eq!(service.snapshot().worker.unwrap().hex(),worker);
    let connected_inventory=inventory(original,true,case);
    publish(root,"connected-proof.json",json!({"fact":fact(connected)}));
    let independent=gate(service,root,"connected-readback.json",case);assert_eq!(independent["ipv4"].as_array().unwrap().len(),1);assert_eq!(independent["default_route_absent"],true);
    let n1=nonce(1);let first_packet=roundtrip(&n1,case);poll(service);
    let withdrawn_at=boot_time_ns();service.withdraw(first.principal);
    assert!(service.take_reply(id,first).is_none());
    let idle_proof=idle(service,original,&worker,withdrawn_at,case);
    let n2=nonce(2);assert_ne!(n1,n2);let background_packet=roundtrip(&n2,case);poll(service);
    let fresh=auth(202);let disconnect_until=until(case);
    let disconnect_id=service.submit(fresh,Action::Disconnect,disconnect_until).unwrap();
    assert_eq!(reply(service,disconnect_id,fresh,disconnect_until).outcome,Finish::Observed(false));
    let disconnected=service.snapshot().fact.unwrap();same(original,disconnected,false);
    assert_eq!(service.snapshot().worker.unwrap().hex(),worker);
    let disconnected_inventory=inventory(original,false,case);
    publish(root,"disconnected-proof.json",json!({"fact":fact(disconnected)}));
    let independent=gate(service,root,"disconnected-readback.json",case);assert_eq!(independent["ipv4"],json!([]));assert_eq!(independent["default_route_absent"],true);
    let n3=nonce(3);assert_ne!(n2,n3);
    let quiet_until=boot_time_ns()+800_000_000;assert!(quiet_until<case,"full original800ms quiet window fits original case");
    publish(root,"quiet-request.json",json!({"original_until_ns":quiet_until,"probe_hex":hex(&n3),"requested_at_ns":boot_time_ns()}));
    let armed=gate(service,root,"peer-quiet-armed.json",quiet_until);assert_eq!(armed["original_until_ns"],quiet_until);
    let socket=UdpSocket::bind((Ipv4Addr::UNSPECIFIED,0)).unwrap();let sent_at=boot_time_ns();assert!(sent_at<quiet_until);
    let send=socket.send_to(&n3,(PEER,PORT));let send_debug=format!("{send:?}");
    if let Err(ref error)=send {assert!(matches!(error.raw_os_error(),Some(99|101|113)));}
    socket.set_nonblocking(true).unwrap();let mut response=[0_u8;128];
    while boot_time_ns()<quiet_until {
        poll(service);
        match socket.recv_from(&mut response) {Err(error)if matches!(error.kind(),io::ErrorKind::WouldBlock|io::ErrorKind::Interrupted)=>{},other=>panic!("fresh disconnected UDP socket unexpected result: {other:?}")}
        thread::sleep(Duration::from_millis(2));
    }
    let quiet=gate(service,root,"peer-quiet.json",case);
    assert_eq!(quiet["received_datagrams"],0);assert_eq!(quiet["original_until_ns"],quiet_until);assert!(quiet["armed_at_ns"].as_u64().unwrap()<=sent_at);assert_eq!(quiet["quiet_end_actual_EAGAIN"],true);assert!(quiet["quiet_end_EAGAIN_at_ns"].as_u64().unwrap()>=quiet_until);
    assert!(boot_time_ns()<case);
    json!({"passed":true,"case":"lifecycle","fixture_authority_only":true,"Settings_window_executed":false,"worker":worker,"original_fact":fact(original),"connected_fact":fact(connected),"disconnected_fact":fact(disconnected),"connected_inventory":connected_inventory,"disconnected_inventory":disconnected_inventory,"first_packet":first_packet,"withdrawn_at_ns":withdrawn_at,"fresh_idle_observations":idle_proof,"background_packet_after_withdraw":background_packet,"disconnect_fresh_principal":202,"fresh_loss_probe":{"nonce_hex":hex(&n3),"send_result":send_debug,"sent_at_ns":sent_at},"peer_quiet":quiet,"original_case_until_ns":case,"completed_at_ns":boot_time_ns()})
}
fn main() {
    assert_eq!(env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),Ok("1"));
    let args:Vec<_>=env::args().skip(1).collect();assert_eq!(args.len(),6,"case worker-elf gates peer-ns case-until close-until");assert_eq!(args[0],"lifecycle","only frozen lifecycle executable admitted; expiry/stop are contract-only");
    let worker=PathBuf::from(&args[1]);let root=PathBuf::from(&args[2]);let case:u64=args[4].parse().unwrap();let close:u64=args[5].parse().unwrap();let started=boot_time_ns();assert!(started<case && case-started<=6_000_000_000 && case<close);
    let journal=root.join("journal");fs::DirBuilder::new().mode(0o700).create(&journal).unwrap();
    let mut service=Service::start(&worker,File::open(&args[3]).unwrap(),journal.join("admitted"),until(case)).unwrap();
    let desired=panic::catch_unwind(AssertUnwindSafe(||lifecycle(&mut service,&root,case)));
    let before_stop=snapshot(service.snapshot());let stop_result=service.stop();let after_stop=snapshot(service.snapshot());
    let mut close_errors=Vec::new();if let Err(error)=stop_result {close_errors.push(format!("stop: {error}"));}
    while service.occupied() && boot_time_ns()<close {
        if let Err(error)=service.poll() {close_errors.push(format!("closure poll: {error}"));break;}
        thread::sleep(Duration::from_millis(2));
    }
    let last=service.snapshot();let closed=!service.occupied() && last.reaped && last.output_eof && last.exit_status.is_some() && last.namespace_handles_open && boot_time_ns()<close && close_errors.is_empty();
    publish(&root,"service-closure.json",json!({"closed":closed,"before_stop":before_stop,"immediate_after_stop":after_stop,"after_actual_wait_EOF":snapshot(last),"original_close_until_ns":close,"completed_at_ns":boot_time_ns(),"errors":close_errors}));
    let passed=desired.is_ok() && closed;
    match desired {Ok(value)=>publish(&root,"passed.json",value),Err(error)=>{let message=error.downcast_ref::<String>().cloned().or_else(||error.downcast_ref::<&str>().map(|v|v.to_string())).unwrap_or_else(||"non-string panic".to_string());publish(&root,"failed.json",json!({"message":message,"closed":closed,"failed_at_ns":boot_time_ns()}));}}
    println!("{}",json!({"passed":passed,"case":"lifecycle","service_closed":closed}));
    if !closed {std::process::abort();}
    drop(service);
    std::process::exit(if passed {0}else{101});
}
