//! Actual C ABI, UID11050 and credentialed SEQPACKET boundary. Temporary peers only.
use pokelinux_settings_client::ffi::*;
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
    io::{self, IoSliceMut, Read, Write},
    mem::MaybeUninit,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt, process::CommandExt},
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
struct Api(*mut Handle);
impl Api {
    fn new(path: &Path) -> Self {
        let mut h = std::ptr::null_mut();
        let b = path.as_os_str().as_bytes();
        // SAFETY: complete live exclusive output and path storage.
        assert_eq!(
            unsafe { pr_settings_client_new_v1(b.as_ptr(), b.len(), &mut h) },
            OK
        );
        assert!(!h.is_null());
        Self(h)
    }
    fn view(&self) -> View {
        let mut v = View::default();
        // SAFETY: live handle and complete disjoint exclusive View.
        assert_eq!(unsafe { pr_settings_client_view_v1(self.0, &mut v) }, OK);
        v
    }
    fn set(&self, p: u32) -> i32 {
        // SAFETY: live handle, no concurrent free.
        unsafe { pr_settings_client_set_v1(self.0, p) }
    }
    fn until(&self, what: &str, budget: u64, f: impl Fn(View) -> bool) -> View {
        let end = now() + budget;
        loop {
            let v = self.view();
            if f(v) {
                return v;
            }
            assert!(now() < end, "{what}: last view {v:?}");
            pause();
        }
    }
    fn close(&mut self) {
        if self.0.is_null() {
            return;
        }
        let started = now();
        // SAFETY: unique live handle; join repeatedly polls the same owned worker.
        unsafe {
            assert_eq!(pr_settings_client_free_v1(self.0), REFUSED);
            assert_eq!(pr_settings_client_stop_v1(self.0), OK);
            loop {
                match pr_settings_client_join_v1(self.0) {
                    OK => break,
                    WAIT => pause(),
                    other => panic!("join {other}"),
                }
                assert!(now() < started + 200_000_000, "original Stop bound");
            }
            assert_eq!(pr_settings_client_free_v1(self.0), OK);
        }
        self.0 = std::ptr::null_mut();
        assert!(
            now() < started + 200_000_000,
            "Stop and actual join must remain bounded"
        );
        assert_eq!(
            fs::read_dir("/proc/self/task").unwrap().count(),
            1,
            "no detached worker"
        );
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.close();
    }
}
fn child(case: &str, path: &Path) {
    assert_eq!(rustix::process::getuid().as_raw(), 11050);
    assert_eq!(rustix::process::geteuid().as_raw(), 11050);
    assert_eq!(std::mem::size_of::<View>(), 48);
    assert_eq!(std::mem::align_of::<View>(), 8);
    let mut sentinel = View {
        revision: 91,
        ..View::default()
    };
    // SAFETY: invalid pointer forms must be rejected before dereference.
    unsafe {
        assert_eq!(
            pr_settings_client_new_v1(std::ptr::null(), 0, std::ptr::null_mut()),
            INVALID
        );
        assert_eq!(
            pr_settings_client_view_v1(std::ptr::null_mut(), &mut sentinel),
            INVALID
        );
        assert_eq!(sentinel.revision, 91);
    }
    let mut a = Api::new(path);
    let ready = a.until("actual root status", 800_000_000, |v| v.available == 1);
    assert_eq!(ready.observed_percent, 50);
    assert_eq!(a.set(50), REFUSED, "only explicit25/75 controls");
    match case {
        "happy" => {
            assert_eq!(ready.can_set, 1);
            assert_eq!(a.set(75), OK);
            assert_eq!(a.set(25), REFUSED, "one pending explicit intent");
            let v = a.until("actual completed readback", 900_000_000, |v| {
                v.observed_percent == 74
            });
            assert_eq!(v.actual_percent, 72);
            assert_eq!(v.requested_percent, 75);
            assert_eq!(v.pending, 0);
            assert_eq!(v.can_set, 0, "completion cannot grant");
            assert_eq!(a.set(25), REFUSED);
            a.until("fresh grant status", 800_000_000, |v| {
                v.can_set == 1 && v.revision == 8
            });
            assert_eq!(a.set(25), OK);
            a.until("second explicit click", 900_000_000, |v| {
                v.observed_percent == 25 && v.revision == 9
            });
        }
        "controls" => {
            assert_eq!(ready.can_set, 0);
            assert_eq!(ready.status, CONTROLS_UNAVAILABLE);
            assert_eq!(a.set(75), REFUSED);
            let v = a.until("explicit denial", 800_000_000, |v| v.status == DENIED);
            assert_eq!(v.available, 0);
            assert_eq!(v.observed_percent, -1);
            assert_eq!(a.set(75), REFUSED);
        }
        "validation" => {
            let v = a.until("invalid packet clears prior live state", 650_000_000, |v| {
                v.available == 0
            });
            assert!(
                now() < ready.until_ns,
                "not merely original snapshot expiry"
            );
            assert_eq!(v.can_set, 0);
            assert_eq!(v.observed_percent, -1);
            assert_eq!(v.actual_percent, -1);
            assert_eq!(a.set(75), REFUSED);
        }
        "expiry" => {
            a.until("getter samples expiry independently", 400_000_000, |v| {
                v.available == 0
            });
            assert_eq!(a.set(75), REFUSED);
            assert!(now() >= ready.until_ns);
        }
        "held" => {
            assert_eq!(a.set(75), OK);
            let v = a.until("original Set deadline", 1_250_000_000, |v| {
                v.status == UNCERTAIN
            });
            assert_eq!(v.available, 0);
            assert_eq!(v.pending, 0);
            assert_eq!(v.observed_percent, -1);
            a.until("readonly recovery", 800_000_000, |v| v.can_set == 1);
            thread::sleep(Duration::from_millis(320));
        }
        "stale" => {
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            assert_eq!(line, "queue\n");
            assert_eq!(a.set(75), OK);
            println!("QUEUED");
            io::stdout().flush().unwrap();
            let v = a.until("queued revision mismatch cancels", 800_000_000, |v| {
                v.revision == 8 && v.pending == 0
            });
            assert_eq!(v.can_set, 1);
            thread::sleep(Duration::from_millis(300));
        }
        "pending-usable" | "pending-changed" | "pending-expiry" | "pending-stop" => {
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            assert_eq!(line, "queue\n");
            let before = now();
            assert_eq!(
                a.set(75),
                OK,
                "explicit click accepted before pending Status reply"
            );
            let accepted = now();
            println!("ACCEPTED_CLICK {accepted}");
            io::stdout().flush().unwrap();
            match case {
                "pending-usable" => {
                    let v = a.until(
                        "ACCEPTED_CLICK_MUST_SURVIVE_TRANSIENT_READONLY_PENDING",
                        950_000_000,
                        |v| {
                            v.revision == 8 && v.observed_percent == 75 && v.requested_percent == 75
                        },
                    );
                    assert_eq!(v.pending, 0);
                    assert!(now() < before + 1_000_000_000, "original click deadline");
                }
                "pending-changed" => {
                    a.until(
                        "changed revision visibly retires queued click",
                        900_000_000,
                        |v| v.revision == 8 && v.pending == 0 && v.can_set == 1,
                    );
                    while now() < accepted + 1_050_000_000 {
                        pause();
                    }
                }
                "pending-expiry" => {
                    let v = a.until(
                        "original unsent click expiry is visible",
                        1_200_000_000,
                        |v| v.status == UNAVAILABLE && v.pending == 0,
                    );
                    assert!(now() >= before + 1_000_000_000);
                    assert_eq!(v.available, 0);
                    println!("CLICK_EXPIRED");
                    io::stdout().flush().unwrap();
                    a.until(
                        "readonly recovery cannot replay expired click",
                        700_000_000,
                        |v| v.can_set == 1,
                    );
                    thread::sleep(Duration::from_millis(100));
                }
                "pending-stop" => {
                    line.clear();
                    io::stdin().read_line(&mut line).unwrap();
                    assert_eq!(line, "stop\n");
                }
                _ => unreachable!(),
            }
        }
        "stop" => {
            assert_eq!(a.set(75), OK);
            thread::sleep(Duration::from_millis(50));
        }
        _ => panic!("unknown child case"),
    }
    a.close();
    println!("CLIENT_PASS {case}");
}
struct Peer {
    child: Child,
    out: Vec<u8>,
    err: Vec<u8>,
    out_eof: bool,
    err_eof: bool,
    exit: Option<i32>,
    wait_at: Option<u64>,
    until: u64,
}
impl Peer {
    fn spawn(case: &str, path: &Path) -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .arg("client")
            .arg(case)
            .arg(path)
            .uid(11050)
            .gid(11050)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        for fd in [
            child.stdout.as_ref().unwrap() as &dyn std::os::fd::AsFd,
            child.stderr.as_ref().unwrap() as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd).unwrap();
            rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        }
        Self {
            child,
            out: vec![],
            err: vec![],
            out_eof: false,
            err_eof: false,
            exit: None,
            wait_at: None,
            until: now() + 5_000_000_000,
        }
    }
    fn pump(&mut self) {
        fn read(stream: &mut impl Read, b: &mut Vec<u8>, eof: &mut bool) {
            if *eof {
                return;
            }
            let mut bytes = [0; 1024];
            loop {
                match stream.read(&mut bytes) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        b.extend_from_slice(&bytes[..n]);
                        assert!(b.len() < 32768)
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("child PIPE {e}"),
                }
            }
        }
        read(
            self.child.stdout.as_mut().unwrap(),
            &mut self.out,
            &mut self.out_eof,
        );
        read(
            self.child.stderr.as_mut().unwrap(),
            &mut self.err,
            &mut self.err_eof,
        );
        if self.exit.is_none() {
            if let Some(s) = self.child.try_wait().unwrap() {
                self.exit = Some(s.code().unwrap_or(-1));
                self.wait_at = Some(now());
            }
        }
    }
    fn finish(&mut self) -> bool {
        loop {
            self.pump();
            if self.exit.is_some() && self.out_eof && self.err_eof {
                break;
            }
            assert!(now() < self.until, "original child wait/EOF cutoff");
            pause();
        }
        println!(
            "CHILD_RECEIPT {}",
            json!({"pid":self.child.id(),"exit":self.exit,"actual_wait_ns":self.wait_at,
            "stdout_eof":self.out_eof,"stderr_eof":self.err_eof,"original_until_ns":self.until,"closed_ns":now()})
        );
        if self.exit != Some(0) {
            eprintln!("{}", String::from_utf8_lossy(&self.err));
        }
        self.exit == Some(0)
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
        }
        while !(self.exit.is_some() && self.out_eof && self.err_eof) && now() < self.until {
            self.pump();
            pause();
        }
        if !(self.exit.is_some() && self.out_eof && self.err_eof) {
            std::process::abort();
        }
    }
}
struct Socket {
    root: PathBuf,
    path: PathBuf,
    listener: OwnedFd,
}
impl Socket {
    fn new(id: usize) -> Self {
        let root = PathBuf::from(format!(
            "/tmp/settings-boundary-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let path = root.join("app.sock");
        let listener = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&listener, true).unwrap();
        net::bind(&listener, &SocketAddrUnix::new(&path).unwrap()).unwrap();
        net::listen(&listener, 8).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        Self {
            root,
            path,
            listener,
        }
    }
    fn request(&self, p: &mut Peer) -> Option<(OwnedFd, Value)> {
        let fd = loop {
            p.pump();
            if p.exit.is_some() {
                return None;
            }
            assert!(now() < p.until);
            match net::accept_with(&self.listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
                Ok(f) => break f,
                Err(rustix::io::Errno::AGAIN) => pause(),
                Err(e) => panic!("accept {e}"),
            }
        };
        let peer = net::sockopt::socket_peercred(&fd).unwrap();
        assert_eq!(peer.uid.as_raw(), 11050);
        assert_eq!(peer.gid.as_raw(), 11050);
        net::sockopt::set_socket_passcred(&fd, true).unwrap();
        let mut b = [0; 4096];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1))];
        loop {
            let mut control = RecvAncillaryBuffer::new(&mut space);
            match net::recvmsg(
                &fd,
                &mut [IoSliceMut::new(&mut b)],
                &mut control,
                RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
            ) {
                Ok(m) => {
                    assert!(m.bytes > 0);
                    assert!(!m.flags.intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC));
                    let c: Vec<_> = control.drain().collect();
                    assert_eq!(c.len(), 1);
                    match c[0] {
                        RecvAncillaryMessage::ScmCredentials(c) => assert_eq!(c, peer),
                        _ => panic!("request credentials"),
                    }
                    return Some((fd, serde_json::from_slice(&b[..m.bytes]).unwrap()));
                }
                Err(rustix::io::Errno::AGAIN) => {
                    p.pump();
                    if p.exit.is_some() {
                        return None;
                    }
                    assert!(now() < p.until);
                    pause();
                }
                Err(e) => panic!("request {e}"),
            }
        }
    }
}
impl Drop for Socket {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap();
        fs::remove_dir(&self.root).unwrap();
    }
}
fn state(rev: u64, observed: u8, requested: Option<u8>, ttl: u64) -> Value {
    json!({"connected":true,"available":true,"revision":rev,"pending":false,
    "requested_percent":requested,"observed_percent":observed,"actual_percent":if observed==74{Some(72)}else{None},"until_ns":(now()+ttl).to_string()})
}
fn status(s: Value, can_set: bool) -> Value {
    json!({"result":"brightness_state","state":s,"can_set":can_set})
}
fn send(fd: &OwnedFd, value: Value) {
    send_raw(fd, value.to_string().as_bytes());
}
fn send_raw(fd: &OwnedFd, b: &[u8]) {
    assert_eq!(
        net::send(fd, b, SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap(),
        b.len()
    );
}
fn is_status(v: &Value) {
    assert_eq!(*v, json!({"op":"brightness_status"}));
}
fn is_set(v: &Value, p: u8, rev: u64) {
    assert_eq!(v["op"], "brightness_set");
    let r = &v["request"];
    assert_eq!(r["percent"], p);
    assert_eq!(r["expected_revision"], rev);
    assert!(r["ttl_ms"]
        .as_u64()
        .is_some_and(|v| (1..=1000).contains(&v)));
    assert_eq!(r.as_object().unwrap().len(), 3);
    assert_eq!(v.as_object().unwrap().len(), 2);
}
fn eof(fd: &OwnedFd, p: &mut Peer, budget: u64) {
    let end = now() + budget;
    let mut b = [0; 8];
    loop {
        match net::recv(fd, &mut b, RecvFlags::DONTWAIT) {
            Ok((0, _)) => break,
            Ok(_) => panic!("unexpected extra request"),
            Err(rustix::io::Errno::AGAIN) => {
                p.pump();
                assert!(now() < end, "original channel close deadline");
                pause()
            }
            Err(rustix::io::Errno::CONNRESET) => break,
            Err(e) => panic!("EOF {e}"),
        }
    }
}
fn scenario(id: usize, case: &str, fault: Option<&str>) -> bool {
    let s = Socket::new(id);
    let mut p = Peer::spawn(case, &s.path);
    let Some((fd, request)) = s.request(&mut p) else {
        return p.finish();
    };
    is_status(&request);
    send(
        &fd,
        status(
            state(
                7,
                50,
                None,
                if case == "expiry" {
                    120_000_000
                } else {
                    2_000_000_000
                },
            ),
            case != "controls",
        ),
    );
    drop(fd);
    let mut revision = 7;
    let mut sets = 0;
    let mut reads = 1;
    while let Some((fd, request)) = s.request(&mut p) {
        if request["op"] == "brightness_set" {
            sets += 1;
            if case == "happy" {
                let (percent, rev, obs) = if sets == 1 { (75, 7, 74) } else { (25, 8, 25) };
                is_set(&request, percent, rev);
                revision = rev + 1;
                send(
                    &fd,
                    json!({"result":"brightness_completed","state":state(revision,obs,Some(percent),2_000_000_000)}),
                );
            } else if case == "held" || case == "stop" {
                assert_eq!(sets, 1, "no automatic replay");
                is_set(&request, 75, 7);
                eof(&fd, &mut p, 1_200_000_000);
            } else {
                panic!("stale or unauthorized Set sent {request}");
            }
        } else {
            is_status(&request);
            reads += 1;
            match case {
                "controls" => {
                    send(&fd, json!({"result":"denied"}));
                    eof(&fd, &mut p, 800_000_000);
                }
                "validation" => {
                    let mut v = status(state(7, 50, None, 2_000_000_000), true);
                    match fault.unwrap() {
                        "duplicate" => {
                            let raw = v.to_string().replacen(
                                "\"can_set\":true",
                                "\"can_set\":true,\"can_set\":false",
                                1,
                            );
                            send_raw(&fd, raw.as_bytes());
                        }
                        "missing" => {
                            v["state"].as_object_mut().unwrap().remove("actual_percent");
                            send(&fd, v);
                        }
                        "unknown" => {
                            v["extra"] = json!(1);
                            send(&fd, v);
                        }
                        "range" => {
                            v["state"]["observed_percent"] = json!(101);
                            send(&fd, v);
                        }
                        "canonical" => {
                            v["state"]["until_ns"] = json!(format!("0{}", now() + 1_000_000_000));
                            send(&fd, v);
                        }
                        "future" => {
                            v["state"]["until_ns"] = json!((now() + 60_000_000_000).to_string());
                            send(&fd, v);
                        }
                        "rights" => {
                            use std::{io::IoSlice, os::fd::AsFd};
                            let null = fs::File::open("/dev/null").unwrap();
                            let rights = [null.as_fd()];
                            let mut space =
                                [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
                            let mut a = net::SendAncillaryBuffer::new(&mut space);
                            assert!(a.push(net::SendAncillaryMessage::ScmRights(&rights)));
                            let b = v.to_string();
                            assert_eq!(
                                net::sendmsg(
                                    &fd,
                                    &[IoSlice::new(b.as_bytes())],
                                    &mut a,
                                    SendFlags::NOSIGNAL
                                )
                                .unwrap(),
                                b.len()
                            );
                        }
                        "pid" => {
                            let mut writer = Command::new(std::env::current_exe().unwrap())
                                .arg("writer")
                                .arg(v.to_string())
                                .stdin(Stdio::null())
                                .stdout(Stdio::from(fd))
                                .stderr(Stdio::piped())
                                .spawn()
                                .unwrap();
                            let deadline = now() + 300_000_000;
                            loop {
                                if let Some(exit) = writer.try_wait().unwrap() {
                                    assert!(exit.success());
                                    break;
                                }
                                assert!(now() < deadline);
                                pause();
                            }
                            let mut err = String::new();
                            writer
                                .stderr
                                .take()
                                .unwrap()
                                .read_to_string(&mut err)
                                .unwrap();
                            assert!(err.is_empty());
                            println!(
                                "WRITER_RECEIPT {}",
                                json!({"pid":writer.id(),"actual_wait_ns":now(),"stderr_eof":true,"stdout":"owned peer socket","original_until_ns":deadline})
                            );
                            continue;
                        }
                        _ => panic!("fault"),
                    }
                    eof(&fd, &mut p, 650_000_000);
                }
                "expiry" => {
                    eof(&fd, &mut p, 600_000_000);
                }
                "stale" if reads == 2 => {
                    p.child
                        .stdin
                        .as_mut()
                        .unwrap()
                        .write_all(b"queue\n")
                        .unwrap();
                    let end = now() + 400_000_000;
                    while !String::from_utf8_lossy(&p.out).contains("QUEUED") {
                        p.pump();
                        assert!(p.exit.is_none() && now() < end, "queued child marker");
                        pause();
                    }
                    revision = 8;
                    send(&fd, status(state(8, 50, None, 2_000_000_000), true));
                }
                _ => {
                    send(
                        &fd,
                        status(
                            state(
                                revision,
                                if revision == 8 { 74 } else { 50 },
                                None,
                                2_000_000_000,
                            ),
                            true,
                        ),
                    );
                }
            }
        }
    }
    let pass = p.finish();
    if pass {
        assert_eq!(
            sets,
            if case == "happy" {
                2
            } else if case == "held" || case == "stop" {
                1
            } else {
                0
            }
        );
    }
    pass
}
// The existing root peer holds an already-entered Status transaction while the
// unprivileged client accepts ONE explicit C ABI click. No service/policy mock.
fn pending_scenario(id: usize, case: &str) -> bool {
    let s = Socket::new(id);
    let mut p = Peer::spawn(case, &s.path);
    let (fd, request) = s.request(&mut p).unwrap();
    is_status(&request);
    send(&fd, status(state(7, 50, None, 2_000_000_000), true));
    drop(fd);
    let (fd, request) = s.request(&mut p).unwrap();
    is_status(&request);
    p.child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"queue\n")
        .unwrap();
    let accepted = loop {
        p.pump();
        if let Some(line) = String::from_utf8_lossy(&p.out)
            .lines()
            .find(|v| v.starts_with("ACCEPTED_CLICK "))
        {
            break line
                .strip_prefix("ACCEPTED_CLICK ")
                .unwrap()
                .parse::<u64>()
                .unwrap();
        }
        assert!(
            p.exit.is_none() && now() < p.until,
            "accepted C ABI click marker"
        );
        pause();
    };
    let original_until = accepted + 1_000_000_000;
    let pending = || {
        let mut value = state(7, 50, None, 2_000_000_000);
        value["pending"] = json!(true);
        if case == "pending-expiry" {
            value["until_ns"] = json!(original_until.to_string());
        }
        status(value, false)
    };
    send(&fd, pending());
    drop(fd);
    let mut sets = 0;
    let mut reads = 2;
    let mut first_usable_release = None;
    while let Some((fd, request)) = s.request(&mut p) {
        println!(
            "PENDING_PACKET {}",
            json!({"case":case,"packet":request,"at_ns":now(),"original_click_until_upper_ns":original_until})
        );
        if request["op"] == "brightness_set" {
            sets += 1;
            assert_eq!(case, "pending-usable", "invalidated click must never send");
            assert_eq!(sets, 1, "one explicit click permits one Set packet");
            is_set(&request, 75, 7);
            assert!(now() < original_until, "no renewed click budget");
            assert!(
                request["request"]["ttl_ms"].as_u64().unwrap()
                    <= (original_until - first_usable_release.unwrap()) / 1_000_000
            );
            send(
                &fd,
                json!({"result":"brightness_completed","state":state(8,75,Some(75),2_000_000_000)}),
            );
        } else {
            is_status(&request);
            reads += 1;
            match case {
                "pending-usable" => {
                    // This precedes any Set caused by releasing fresh usable state.
                    // Receiver time includes transit and cannot bound the sender TTL.
                    if sets == 0 && first_usable_release.is_none() {
                        first_usable_release = Some(now());
                    }
                    send(
                        &fd,
                        status(
                            state(
                                if sets == 0 { 7 } else { 8 },
                                if sets == 0 { 50 } else { 75 },
                                None,
                                2_000_000_000,
                            ),
                            true,
                        ),
                    );
                }
                "pending-changed" => send(&fd, status(state(8, 50, None, 2_000_000_000), true)),
                "pending-expiry" if now() < accepted + 750_000_000 => send(&fd, pending()),
                "pending-expiry" => {
                    loop {
                        p.pump();
                        if String::from_utf8_lossy(&p.out).contains("CLICK_EXPIRED")
                            || p.exit.is_some()
                        {
                            break;
                        }
                        assert!(now() < p.until, "expiry marker within original child bound");
                        pause();
                    }
                    if p.exit.is_none() {
                        send(&fd, status(state(7, 50, None, 2_000_000_000), true));
                    }
                }
                "pending-stop" => {
                    p.child
                        .stdin
                        .as_mut()
                        .unwrap()
                        .write_all(b"stop\n")
                        .unwrap();
                    eof(&fd, &mut p, 200_000_000);
                }
                _ => unreachable!(),
            }
        }
    }
    let pass = p.finish();
    println!(
        "PENDING_CASE {}",
        json!({"case":case,"accepted_at_ns":accepted,"original_click_until_upper_ns":original_until,"status_packets":reads,"set_packets":sets,"child_passed":pass})
    );
    if pass {
        assert_eq!(sets, usize::from(case == "pending-usable"));
    }
    pass
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|v| v == "client") {
        child(&args[2], Path::new(&args[3]));
        return;
    }
    if args.get(1).is_some_and(|v| v == "writer") {
        io::stdout().write_all(args[2].as_bytes()).unwrap();
        return;
    }
    assert_eq!(
        rustix::process::getuid().as_raw(),
        0,
        "root authored server required"
    );
    if args.get(1).is_some_and(|v| v == "pending-only") {
        let mut failed = 0;
        for (id, case) in [
            "happy",
            "pending-usable",
            "pending-changed",
            "pending-expiry",
            "pending-stop",
        ]
        .into_iter()
        .enumerate()
        {
            let result = std::panic::catch_unwind(|| {
                if case == "happy" {
                    scenario(id, case, None)
                } else {
                    pending_scenario(id, case)
                }
            });
            let passed = matches!(result, Ok(true));
            println!(
                "GROUP {case} {}",
                if passed { "PASS" } else { "ASSERT_FAIL" }
            );
            failed += usize::from(!passed);
        }
        println!("RESULT pending_groups=5 failed={failed}");
        std::process::exit(if failed == 0 { 0 } else { 1 });
    }
    let mut failed = 0;
    let mut id = 0;
    for case in [
        "happy",
        "controls",
        "validation",
        "expiry",
        "held",
        "stale",
        "stop",
    ] {
        let faults = if case == "validation" {
            vec![
                Some("duplicate"),
                Some("missing"),
                Some("unknown"),
                Some("range"),
                Some("canonical"),
                Some("future"),
                Some("rights"),
                Some("pid"),
            ]
        } else {
            vec![None]
        };
        let mut passed = true;
        for fault in faults {
            id += 1;
            let result = std::panic::catch_unwind(|| scenario(id, case, fault));
            if !matches!(result, Ok(true)) {
                passed = false;
            }
        }
        println!(
            "GROUP {case} {}",
            if passed { "PASS" } else { "ASSERT_FAIL" }
        );
        if !passed {
            failed += 1;
        }
    }
    println!("RESULT groups=7 failed={failed}");
    std::process::exit(if failed == 0 { 0 } else { 1 });
}
