//! Actual SafeEngine/UID11050 Unix protocol components, authored root peers only.
//! No real actor/backend, OS app grant, launched Settings, C ABI, GUI or network.
use pokelinux_settings_client::{
    connectivity::{Engine, View as NetView},
    ffi::{OK, WAIT},
};
use rustix::{
    fd::OwnedFd,
    net::{
        self, AddressFamily, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendFlags, SocketAddrUnix, SocketFlags, SocketType,
    },
    time::{clock_gettime, ClockId},
};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, IoSlice, IoSliceMut, Read, Write},
    mem::MaybeUninit,
    os::{
        fd::AsFd,
        unix::{fs::PermissionsExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn pause() {
    thread::sleep(Duration::from_millis(2));
}
// Exact existing Peer custody block; Socket now retains the actor's real
// persistent transaction transport until EOF, with readonly-only reconnects.
include!("support/connectivity-boundary.rs");
struct Api {
    engine: Engine,
    closed: bool,
}
impl Api {
    fn new(path: &Path) -> Self {
        Self {
            engine: Engine::new(path.into()).unwrap(),
            closed: false,
        }
    }
    fn view(&self) -> NetView {
        self.engine.view().unwrap()
    }
    fn until(&self, end: u64, f: impl Fn(NetView) -> bool) -> NetView {
        loop {
            assert!(now() < end, "original getter cutoff");
            let v = self.view();
            assert!(now() < end, "getter returned late: {v:?}");
            if f(v) {
                return v;
            }
            pause();
        }
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        let before = now();
        let end = before + 200_000_000;
        assert!(self.engine.stop());
        loop {
            match self.engine.join() {
                OK => break,
                WAIT => pause(),
                other => panic!("SDK join {other}"),
            };
            assert!(now() < end);
        }
        assert!(self.engine.joined() && now() < end);
        let tasks = fs::read_dir("/proc/self/task")
            .unwrap()
            .collect::<io::Result<Vec<_>>>()
            .unwrap()
            .len();
        assert_eq!(tasks, 1, "independent no-detached-thread witness");
        assert!(now() < end);
        println!(
            "SDK_JOIN {}",
            json!({"actual_joined":true,"tasks":tasks,"before_ns":before,"completed_ns":now(),"original_until_ns":end})
        );
        self.closed = true;
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.close();
    }
}
fn tag(v: NetView) -> String {
    serde_json::to_value(v.status)
        .unwrap()
        .as_str()
        .unwrap()
        .into()
}
fn settled(v: NetView, configured: bool) -> bool {
    let t = now();
    v.transport_connected
        && v.available
        && !v.pending
        && v.configured == Some(configured)
        && v.observed_at_ns > 0
        && v.observed_at_ns <= t
        && t < v.until_ns
        && v.observed_at_ns.checked_add(250_000_000) == Some(v.until_ns)
        && matches!(tag(v).as_str(), "ready" | "controls_unavailable")
}
fn line(expected: &str) {
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    assert_eq!(s, expected);
}
fn choice(api: &Api, ms: u64) -> u64 {
    let end = now() + ms * 1_000_000;
    assert!(api.engine.set(true, end));
    assert!(now() < end);
    println!("CHOICE {}", json!({"until_ns":end,"accepted_ns":now()}));
    io::stdout().flush().unwrap();
    end
}
fn child(case: &str, path: &Path) {
    assert_eq!(rustix::process::getuid().as_raw(), 11050);
    assert_eq!(rustix::process::geteuid().as_raw(), 11050);
    let caps_text = fs::read_to_string("/proc/self/status").unwrap();
    let mut caps = serde_json::Map::new();
    for name in [
        "CapEff",
        "CapPrm",
        "CapInh",
        "CapAmb",
        "CapBnd",
        "NoNewPrivs",
    ] {
        let text = caps_text
            .lines()
            .find_map(|v| v.strip_prefix(&format!("{name}:")))
            .unwrap()
            .trim();
        if name == "NoNewPrivs" {
            assert_eq!(text, "1");
        } else {
            assert_eq!(
                u64::from_str_radix(text, 16).unwrap(),
                if name == "CapBnd" { 0xe0 } else { 0 }
            );
        }
        caps.insert(name.into(), json!(text));
    }
    println!(
        "CHILD_CAPS {}",
        json!({"pid":std::process::id(),"uid":11050,"gid":11050,"sets":caps})
    );
    let mut a = Api::new(path);
    let ready = a.until(now() + 800_000_000, |v| settled(v, false));
    assert!(ready.can_control);
    println!("READY");
    io::stdout().flush().unwrap();
    match case {
        "bad" => {
            let v = a.until(ready.until_ns, |v| !v.available);
            assert!(now() < ready.until_ns, "invalid packet, not lease expiry");
            assert_eq!(v.configured, None);
            assert!(!v.can_control);
            assert!(!a.engine.set(true, now() + 100_000_000));
        }
        "expiry" => {
            let v = a.until(ready.until_ns + 50_000_000, |v| !v.available);
            assert!(now() >= ready.until_ns);
            assert_eq!(v.configured, None);
            assert_eq!(v.observed_at_ns, 0);
            assert_eq!(v.until_ns, 0);
            assert!(!v.can_control);
        }
        "positive" => {
            let end = choice(&a, 700);
            a.until(end, |v| settled(v, true));
        }
        "queue" | "queue-pending" | "queue-uncontrolled" | "queue-expired" | "queue-stop" => {
            line("queue\n");
            let end = choice(&a, 600);
            if case == "queue-stop" {
                line("stop\n");
            } else if case == "queue" {
                a.until(end, |v| settled(v, true));
            } else {
                let v = a.until(end, |v| {
                    v.available
                        && !v.pending
                        && v.configured == Some(false)
                        && tag(v) == "not_sent"
                        && v.observed_at_ns > ready.observed_at_ns
                });
                a.until(end + 300_000_000, |next| {
                    next.available
                        && !next.pending
                        && next.configured == Some(false)
                        && tag(next) == "not_sent"
                        && next.observed_at_ns > v.observed_at_ns
                });
                println!("RECOVERED_NOT_SENT");
                io::stdout().flush().unwrap();
                line("stop\n");
            }
        }
        "held" => {
            let end = choice(&a, 300);
            let v = a.until(end + 200_000_000, |v| tag(v) == "uncertain");
            assert!(now() >= end);
            assert!(!v.pending);
            let v = a.until(end + 1_000_000_000, |v| {
                v.available && v.configured == Some(true) && tag(v) == "uncertain"
            });
            assert!(!v.pending);
            println!("RECOVERED_UNCERTAIN");
            io::stdout().flush().unwrap();
            line("stop\n");
        }
        _ => panic!("unknown component child"),
    }
    a.close();
    println!("CLIENT_PASS {case}");
}
fn state(configured: Option<bool>) -> Value {
    let at = now();
    json!({"connected":true,"available":true,"configured":configured,"pending":false,
        "observed_at_ns":at.to_string(),"until_ns":(at+250_000_000).to_string()})
}
fn status(s: Value, can: bool) -> Value {
    json!({"result":"connectivity_state","state":s,"can_control":can})
}
fn send(fd: &OwnedFd, v: Value) {
    let b = v.to_string();
    assert_eq!(
        net::send(fd, b.as_bytes(), SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap(),
        b.len()
    );
}
fn is_status(v: &Value) {
    assert_eq!(*v, json!({"op":"connectivity_status"}));
}
fn is_set(v: &Value, end: u64) {
    assert_eq!(
        *v,
        json!({"op":"connectivity_set","request":{"connected":true,"until_ns":end.to_string()}})
    );
    assert!(
        now() < end,
        "actual complete Set arrived under the original requested deadline"
    );
}
fn marker(p: &mut Peer, prefix: &str) -> Value {
    loop {
        p.pump();
        if let Some(v) = String::from_utf8_lossy(&p.out)
            .lines()
            .find_map(|l| l.strip_prefix(prefix))
        {
            return serde_json::from_str(v).unwrap();
        }
        assert!(
            p.exit.is_none() && now() < p.until,
            "actual child marker {prefix}"
        );
        pause();
    }
}
fn eof(fd: &OwnedFd, p: &mut Peer) {
    let end = (now() + 800_000_000).min(p.until);
    let mut b = [0; 8];
    let mut resets = 0;
    loop {
        assert!(now() < end, "original socket EOF cutoff");
        match net::recv(fd, &mut b, RecvFlags::DONTWAIT) {
            Ok((0, _)) => {
                println!(
                    "SOCKET_EOF {}",
                    json!({"phase":"eof","pid":p.child.id(),"resets":resets,"zero_byte_EOF":1})
                );
                break;
            }
            Err(rustix::io::Errno::CONNRESET) => {
                resets += 1;
                println!(
                    "SOCKET_RESET {}",
                    json!({"phase":"eof","pid":p.child.id(),"count":resets})
                );
            }
            Err(rustix::io::Errno::AGAIN) => {
                p.pump();
                pause();
            }
            other => panic!("request EOF {other:?}"),
        }
    }
}
fn drain_after_exit(s: &Socket, p: &Peer) -> usize {
    assert!(p.exit.is_some() && p.out_eof && p.err_eof);
    let mut sets = 0;
    let mut sockets = Vec::new();
    if let Some(fd) = s.current.borrow_mut().take() {
        sockets.push(fd);
    }
    loop {
        assert!(now() < p.until, "original post-wait accept cutoff");
        match net::accept_with(&s.listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
            Ok(fd) => sockets.push(fd),
            Err(rustix::io::Errno::AGAIN) => break,
            Err(e) => panic!("post-wait accept {e}"),
        }
    }
    for fd in sockets {
        net::sockopt::set_socket_passcred(&fd, true).unwrap();
        let mut resets = 0;
        let mut packets = 0;
        // Linux datagram receive reports sock_error before its still-queued data:
        // https://raw.githubusercontent.com/torvalds/linux/master/net/core/datagram.c
        // ECONNRESET therefore retains this exact FD until real recvmsg(0).
        loop {
            assert!(now() < p.until, "original post-wait drain cutoff");
            let mut b = [0; 4096];
            let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1))];
            let mut a = RecvAncillaryBuffer::new(&mut space);
            let m = match net::recvmsg(
                &fd,
                &mut [IoSliceMut::new(&mut b)],
                &mut a,
                RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
            ) {
                Ok(m) => m,
                Err(rustix::io::Errno::CONNRESET) => {
                    resets += 1;
                    println!(
                        "SOCKET_RESET {}",
                        json!({"phase":"post_wait","pid":p.child.id(),"count":resets})
                    );
                    continue;
                }
                Err(rustix::io::Errno::AGAIN) => {
                    pause();
                    continue;
                }
                Err(e) => panic!("post-wait receive {e}"),
            };
            assert!(!m.flags.intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC));
            if m.bytes == 0 {
                println!(
                    "SOCKET_EOF {}",
                    json!({"phase":"post_wait","pid":p.child.id(),"resets":resets,"packets":packets,"zero_byte_EOF":1})
                );
                break;
            }
            packets += 1;
            let c: Vec<_> = a.drain().collect();
            assert_eq!(c.len(), 1);
            match c[0] {
                RecvAncillaryMessage::ScmCredentials(c) => {
                    assert_eq!(c.pid.as_raw_nonzero().get() as u32, p.child.id());
                    assert_eq!(c.uid.as_raw(), 11050);
                    assert_eq!(c.gid.as_raw(), 11050);
                }
                _ => panic!("queued credentials"),
            }
            let v: Value = serde_json::from_slice(&b[..m.bytes]).unwrap();
            if v["op"] == "connectivity_set" {
                sets += 1;
            } else {
                is_status(&v);
            }
        }
    }
    sets
}
fn no_packet(fd: &OwnedFd) {
    let mut b = [0; 8];
    assert!(
        matches!(
            net::recv(fd, &mut b, RecvFlags::DONTWAIT | RecvFlags::PEEK),
            Err(rustix::io::Errno::AGAIN)
        ),
        "one outstanding transaction; no effect before held Status reply"
    );
}
fn initial(s: &Socket, p: &mut Peer) -> u64 {
    let (fd, v) = s.request(p).unwrap();
    is_status(&v);
    let v = state(Some(false));
    let end = v["until_ns"].as_str().unwrap().parse().unwrap();
    send(&fd, status(v, true));
    end
}
fn rights(fd: &OwnedFd, v: Value) {
    let null = fs::File::open("/dev/null").unwrap();
    let fds = [null.as_fd()];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut a = net::SendAncillaryBuffer::new(&mut space);
    assert!(a.push(net::SendAncillaryMessage::ScmRights(&fds)));
    let b = v.to_string();
    assert_eq!(
        net::sendmsg(
            fd,
            &[IoSlice::new(b.as_bytes())],
            &mut a,
            SendFlags::NOSIGNAL
        )
        .unwrap(),
        b.len()
    );
}
fn wrong_pid(fd: OwnedFd, v: Value) {
    let end = now() + 300_000_000;
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("writer")
        .arg(v.to_string())
        .stdin(Stdio::from(fd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for f in [
        child.stdout.as_ref().unwrap() as &dyn AsFd,
        child.stderr.as_ref().unwrap() as &dyn AsFd,
    ] {
        let flags = rustix::fs::fcntl_getfl(f).unwrap();
        rustix::fs::fcntl_setfl(f, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
    }
    let mut p = Peer {
        child,
        out: vec![],
        err: vec![],
        out_eof: false,
        err_eof: false,
        exit: None,
        wait_at: None,
        until: end,
    };
    assert!(p.finish());
    assert!(String::from_utf8_lossy(&p.out).contains("WRITER_SENT"));
}
fn malformed_group(mut id: usize) {
    for fault in [
        "missing",
        "nullable",
        "canonical",
        "lease",
        "unknown",
        "unavailable-shape",
    ] {
        let s = Socket::new(id);
        id += 1;
        let mut p = Peer::spawn("bad", &s.path);
        initial(&s, &mut p);
        let (fd, v) = s.request(&mut p).unwrap();
        is_status(&v);
        let mut reply = status(state(Some(false)), true);
        match fault {
            "missing" => {
                reply["state"].as_object_mut().unwrap().remove("configured");
            }
            "nullable" => reply["state"]["configured"] = Value::Null,
            "canonical" => {
                let at = reply["state"]["observed_at_ns"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                reply["state"]["observed_at_ns"] = json!(format!("0{at}"));
            }
            "lease" => {
                let at = now();
                reply["state"]["observed_at_ns"] = json!(at.to_string());
                reply["state"]["until_ns"] = json!((at + 250_000_001).to_string());
            }
            "unknown" => reply["state"]["path"] = json!("/not-authority"),
            _ => {
                reply["state"]["available"] = json!(false);
                reply["state"]["observed_at_ns"] = json!("0");
                reply["state"]["until_ns"] = json!("0");
                reply["can_control"] = json!(false);
            }
        }
        send(&fd, reply);
        eof(&fd, &mut p);
        assert!(p.finish());
        assert_eq!(drain_after_exit(&s, &p), 0);
    }
}
fn credentials_group(mut id: usize) {
    for pid in [false, true] {
        let s = Socket::new(id);
        id += 1;
        let mut p = Peer::spawn("bad", &s.path);
        initial(&s, &mut p);
        let (fd, v) = s.request(&mut p).unwrap();
        is_status(&v);
        let reply = status(state(Some(false)), true);
        if pid {
            wrong_pid(fd, reply);
        } else {
            rights(&fd, reply);
            eof(&fd, &mut p);
        }
        assert!(p.finish());
        assert_eq!(drain_after_exit(&s, &p), 0);
    }
}
fn expiry_group(id: usize) {
    let s = Socket::new(id);
    let mut p = Peer::spawn("expiry", &s.path);
    initial(&s, &mut p);
    let (fd, v) = s.request(&mut p).unwrap();
    is_status(&v); // Actual outstanding RPC gets no reply.
    eof(&fd, &mut p);
    assert!(p.finish());
    assert_eq!(drain_after_exit(&s, &p), 0);
}
fn positive_group(id: usize) {
    let s = Socket::new(id);
    let mut p = Peer::spawn("positive", &s.path);
    initial(&s, &mut p);
    let choice = marker(&mut p, "CHOICE ");
    let end = choice["until_ns"].as_u64().unwrap();
    let mut sets = 0;
    while let Some((fd, v)) = s.request(&mut p) {
        assert_eq!(
            s.generation(),
            1,
            "effect stays on readonly-established connection"
        );
        if v["op"] == "connectivity_set" {
            sets += 1;
            assert_eq!(sets, 1);
            is_set(&v, end);
            send(
                &fd,
                json!({"result":"connectivity_completed","state":state(Some(true))}),
            );
        } else {
            is_status(&v);
            send(&fd, status(state(Some(sets == 1)), true));
        }
    }
    assert!(p.finish());
    sets += drain_after_exit(&s, &p);
    assert_eq!(sets, 1);
}
fn queued_group(mut id: usize) {
    for case in [
        "queue",
        "queue-pending",
        "queue-uncontrolled",
        "queue-expired",
        "queue-stop",
    ] {
        let s = Socket::new(id);
        id += 1;
        let mut p = Peer::spawn(case, &s.path);
        let old_until = initial(&s, &mut p);
        let (fd, v) = s.request(&mut p).unwrap();
        is_status(&v);
        assert_eq!(s.generation(), 1);
        p.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"queue\n")
            .unwrap();
        let choice = marker(&mut p, "CHOICE ");
        let end = choice["until_ns"].as_u64().unwrap();
        no_packet(&fd);
        if case == "queue-stop" {
            p.child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(b"stop\n")
                .unwrap();
            eof(&fd, &mut p);
            assert!(p.finish());
            assert_eq!(drain_after_exit(&s, &p), 0);
            continue;
        }
        if case == "queue-expired" {
            while now() < old_until + 10_000_000 {
                p.pump();
                assert!(p.exit.is_none() && now() < p.until);
                pause();
            }
            no_packet(&fd);
        } else {
            assert!(
                now() < old_until,
                "fresh held-readonly reply precedes the original cache expiry"
            );
        }
        let mut reply = state(Some(false));
        let pending = case == "queue-pending";
        reply["pending"] = json!(pending);
        send(&fd, status(reply, !pending && case != "queue-uncontrolled"));
        drop(fd);
        let should_send = case == "queue";
        let mut sets = 0;
        let mut stopped = false;
        while let Some((fd, v)) = s.request(&mut p) {
            if v["op"] == "connectivity_set" {
                assert!(should_send, "not_sent choice revived after pending/expiry");
                sets += 1;
                assert_eq!(sets, 1);
                assert_eq!(s.generation(), 1);
                is_set(&v, end);
                send(
                    &fd,
                    json!({"result":"connectivity_completed","state":state(Some(true))}),
                );
            } else {
                is_status(&v);
                send(&fd, status(state(Some(sets == 1)), true));
            }
            p.pump();
            if !should_send && String::from_utf8_lossy(&p.out).contains("RECOVERED_NOT_SENT") {
                p.child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(b"stop\n")
                    .unwrap();
                stopped = true;
                break;
            }
        }
        assert!(should_send || stopped);
        assert!(p.finish());
        sets += drain_after_exit(&s, &p);
        assert_eq!(sets, usize::from(should_send));
    }
}
fn held_group(id: usize) {
    let s = Socket::new(id);
    let mut p = Peer::spawn("held", &s.path);
    initial(&s, &mut p);
    let choice = marker(&mut p, "CHOICE ");
    let end = choice["until_ns"].as_u64().unwrap();
    let mut sets = 0;
    loop {
        let (fd, v) = s.request(&mut p).unwrap();
        if v["op"] == "connectivity_set" {
            sets += 1;
            assert_eq!(
                s.generation(),
                1,
                "first effect uses readonly-established transport"
            );
            is_set(&v, end);
            while now() < end + 30_000_000 {
                p.pump();
                assert!(now() < p.until);
                pause();
            }
            drop(fd);
            break;
        } else {
            is_status(&v);
            send(&fd, status(state(Some(false)), true));
        }
    }
    let mut settle_end = None;
    let mut stop_sent = false;
    while let Some((fd, v)) = s.request(&mut p) {
        if v["op"] == "connectivity_set" {
            panic!("expired sent Set was replayed: {v}");
        }
        is_status(&v);
        send(&fd, status(state(Some(true)), true));
        p.pump();
        if String::from_utf8_lossy(&p.out).contains("RECOVERED_UNCERTAIN") && settle_end.is_none() {
            settle_end = Some(now() + 350_000_000);
        }
        if !stop_sent && settle_end.is_some_and(|t| now() >= t) {
            p.child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(b"stop\n")
                .unwrap();
            stop_sent = true;
            break;
        }
    }
    assert!(stop_sent);
    assert!(p.finish());
    sets += drain_after_exit(&s, &p);
    assert_eq!(sets, 1);
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "client") {
        child(&args[2], Path::new(&args[3]));
        return;
    }
    if args.get(1).is_some_and(|s| s == "writer") {
        let b = args[2].as_bytes();
        assert_eq!(
            net::send(
                io::stdin().as_fd(),
                b,
                SendFlags::NOSIGNAL | SendFlags::DONTWAIT
            )
            .unwrap(),
            b.len()
        );
        println!("WRITER_SENT");
        return;
    }
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let mut failed = 0;
    for (id, (name, run)) in [
        ("state-shapes", malformed_group as fn(usize)),
        ("actual-credentials", credentials_group),
        ("getter-expiry-stop", expiry_group),
        ("exact-original-set", positive_group),
        ("held-readonly-choice", queued_group),
        ("sent-uncertainty-no-replay", held_group),
    ]
    .into_iter()
    .enumerate()
    {
        let result = std::panic::catch_unwind(|| run(100 + id * 10));
        if result.is_err() {
            failed += 1;
        }
        println!(
            "GROUP {name} {}",
            if result.is_ok() { "PASS" } else { "FAIL" }
        );
    }
    assert_eq!(failed, 0, "supplementary SafeEngine protocol groups");
}
