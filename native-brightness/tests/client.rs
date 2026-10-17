//! Actual local root SEQPACKET edges with authored peer packets, not a daemon,
//! backend, permission grant, process retirement or physical display result.
use pokelinux_brightness::{
    client::{Client, Event},
    protocol::{Attributes, Reply, Request},
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType},
    time::{ClockId, clock_gettime},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    (t.tv_sec as u64) * 1_000_000_000 + t.tv_nsec as u64
}
fn sockets() -> (OwnedFd, OwnedFd) {
    let (a, b) = net::socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&b, true).unwrap();
    (a, b)
}
fn pair() -> (Client, OwnedFd) {
    let (a, b) = sockets();
    (Client::from_socket(a, None).unwrap(), b)
}
fn receive(fd: &OwnedFd) -> Request {
    let mut b = [0; 1024];
    let (n, full_len) = net::recv(fd, &mut b, RecvFlags::DONTWAIT | RecvFlags::TRUNC).unwrap();
    assert_eq!(n, full_len, "truncated request");
    serde_json::from_slice(&b[..n]).unwrap()
}
fn no_packet(fd: &OwnedFd) {
    let mut b = [0; 1024];
    match net::recv(fd, &mut b, RecvFlags::DONTWAIT) {
        Err(rustix::io::Errno::AGAIN | rustix::io::Errno::CONNRESET) | Ok((0, _)) => {}
        other => panic!("unexpected outgoing packet {other:?}"),
    }
}
fn send(fd: &OwnedFd, r: &Reply) {
    let b = serde_json::to_vec(r).unwrap();
    assert_eq!(
        net::send(fd, &b, SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap(),
        b.len()
    );
}
fn positive(id: u64, revision: u64, requested: u32, percent: Option<u8>) -> Reply {
    Reply {
        schema: 1,
        id,
        ok: true,
        revision,
        pending: false,
        observation: Some(Attributes {
            max: 400,
            requested,
            actual: Some(120),
        }),
        until_ns: (now() + 1_000_000_000).to_string(),
        requested_percent: percent,
        last_set_uncertain: false,
    }
}
fn observed(c: &mut Client, p: &OwnedFd) -> u64 {
    assert!(
        c.inspect(now() + 800_000_000).unwrap(),
        "explicit readonly Inspect must send once"
    );
    let req = receive(p);
    assert!(matches!(req, Request::Inspect { .. }));
    send(p, &positive(req.id(), 2, 200, None));
    assert!(matches!(c.poll(), Some(Event::Observed)));
    assert_eq!(c.snapshot().observation.unwrap().requested, 200);
    2
}
#[test]
fn persistent_verified_socket_inspects_then_sends_exactly_one_explicit_set() {
    let (mut c, p) = pair();
    let generation = c.incarnation();
    assert!(generation > 0);
    let rev = observed(&mut c, &p);
    assert!(c.set(77, 75, rev, now() + 500_000_000).unwrap());
    let req = receive(&p);
    assert!(matches!(
        req,
        Request::Set {
            percent: 75,
            expected_revision: 2,
            ..
        }
    ));
    assert!(c.snapshot().pending && c.snapshot().observation.is_none());
    for _ in 0..4 {
        assert!(c.poll().is_none());
        no_packet(&p);
    }
    send(&p, &positive(req.id(), 4, 300, Some(75)));
    match c.poll() {
        Some(Event::Completed { id, reply }) => {
            assert_eq!(id, 77);
            assert_eq!(reply.observation.unwrap().actual, Some(120));
        }
        _ => panic!("missing actual Set readback"),
    }
    assert_eq!(c.incarnation(), generation);
    assert_eq!(c.snapshot().observation.unwrap().requested, 300);
    assert!(!c.snapshot().pending);
    no_packet(&p);
}
#[test]
fn invalid_or_unobserved_set_sends_nothing_and_has_no_queue() {
    let (mut c, p) = pair();
    assert!(!c.set(1, 75, 1, now() + 100_000_000).unwrap());
    no_packet(&p);
    let rev = observed(&mut c, &p);
    for (id, percent, expected, until) in [
        (0, 75, rev, now() + 100_000_000),
        (1, 101, rev, now() + 100_000_000),
        (1, 75, rev + 1, now() + 100_000_000),
        (1, 75, rev, now()),
        (1, 75, rev, now() + 2_000_000_000),
    ] {
        assert!(!c.set(id, percent, expected, until).unwrap());
        no_packet(&p);
    }
    assert!(c.poll().is_none());
    no_packet(&p);
}
#[test]
fn actual_send_eagain_never_queues_or_flushes_on_poll() {
    let (a, p) = sockets();
    let alias = rustix::io::dup(&a).unwrap();
    let mut c = Client::from_socket(a, None).unwrap();
    let filler = [b'x'; 512];
    let mut n = 0;
    loop {
        match net::send(&alias, &filler, SendFlags::DONTWAIT | SendFlags::NOSIGNAL) {
            Ok(512) => {
                n += 1;
                assert!(n < 65536);
            }
            Err(rustix::io::Errno::AGAIN) => break,
            other => panic!("unexpected fill {other:?}"),
        }
    }
    assert!(!c.inspect(now() + 500_000_000).unwrap());
    let mut b = [0; 1024];
    for _ in 0..n {
        assert_eq!(net::recv(&p, &mut b, RecvFlags::DONTWAIT).unwrap().0, 512);
    }
    no_packet(&p);
    assert!(c.poll().is_none());
    no_packet(&p);
    assert!(c.inspect(now() + 500_000_000).unwrap());
    assert!(matches!(receive(&p), Request::Inspect { .. }));
}
#[test]
fn queued_positive_cannot_publish_after_original_request_deadline() {
    let (mut c, p) = pair();
    let until = now() + 80_000_000;
    assert!(c.inspect(until).unwrap());
    let req = receive(&p);
    send(&p, &positive(req.id(), 2, 200, None));
    while now() < until {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(matches!(c.poll(), Some(Event::Unavailable)));
    let s = c.snapshot();
    assert!(!s.connected && s.observation.is_none());
    no_packet(&p);
}
#[test]
fn malformed_wrong_id_oversize_and_uncertain_ack_are_terminal() {
    for case in 0..10 {
        let (mut c, p) = pair();
        assert!(c.inspect(now() + 500_000_000).unwrap());
        let req = receive(&p);
        let mut r = positive(req.id(), 2, 200, None);
        let bytes = match case {
            0 => b"{".to_vec(),
            1 => {
                r.id += 1;
                serde_json::to_vec(&r).unwrap()
            }
            2 => vec![b'x'; 513],
            3 => {
                r.schema = 2;
                serde_json::to_vec(&r).unwrap()
            }
            4 => {
                r.observation.as_mut().unwrap().actual = Some(401);
                serde_json::to_vec(&r).unwrap()
            }
            5 => {
                r.until_ns = "011000000000".into();
                serde_json::to_vec(&r).unwrap()
            }
            6 => {
                r.requested_percent = Some(50);
                serde_json::to_vec(&r).unwrap()
            }
            7 => {
                let mut value = serde_json::to_value(&r).unwrap();
                value.as_object_mut().unwrap().remove("requested_percent");
                serde_json::to_vec(&value).unwrap()
            }
            _ => {
                let mut b = serde_json::to_vec(&r).unwrap();
                b.pop();
                b.extend_from_slice(b",\"id\":99}");
                b
            }
        };
        if case == 9 {
            use rustix::fd::AsFd;
            let mut space = [std::mem::MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
            let mut ancillary = net::SendAncillaryBuffer::new(&mut space);
            let right = std::fs::File::open("/dev/null").unwrap();
            let fds = [right.as_fd()];
            assert!(ancillary.push(net::SendAncillaryMessage::ScmRights(&fds)));
            net::sendmsg(
                &p,
                &[std::io::IoSlice::new(&serde_json::to_vec(&r).unwrap())],
                &mut ancillary,
                SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
            )
            .unwrap();
        } else {
            net::send(&p, &bytes, SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap();
        }
        assert!(matches!(c.poll(), Some(Event::Unavailable)), "case {case}");
        assert!(!c.snapshot().connected);
        no_packet(&p);
    }
    // Set acknowledgements additionally require raw target/max and actor correlation.
    for case in 0..4 {
        let (mut c, p) = pair();
        let rev = observed(&mut c, &p);
        assert!(c.set(7, 75, rev, now() + 500_000_000).unwrap());
        let req = receive(&p);
        let mut r = positive(req.id(), 4, 300, Some(75));
        match case {
            0 => r.last_set_uncertain = true,
            1 => r.observation.as_mut().unwrap().requested = 299,
            2 => r.observation.as_mut().unwrap().max = 800,
            _ => r.revision = rev,
        }
        send(&p, &r);
        assert!(matches!(c.poll(), Some(Event::Unavailable)));
        assert!(c.snapshot().observation.is_none());
    }
}
#[test]
fn unsolicited_duplicate_reply_fences_before_another_set_is_sent() {
    let (mut c, p) = pair();
    let rev = observed(&mut c, &p);
    send(&p, &positive(999, 2, 200, None));
    assert!(!c.set(1, 75, rev, now() + 500_000_000).unwrap());
    assert!(!c.snapshot().connected);
    no_packet(&p);
}
#[test]
fn stream_type_and_relative_deployment_path_are_refused() {
    let (a, _b) = net::socketpair(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    assert!(Client::from_socket(a, None).is_err());
    assert!(Client::new("relative/socket".into()).is_err());
    assert!(Client::new("/tmp/../socket".into()).is_err());
}
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Site(PathBuf);
impl Site {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let p = std::env::temp_dir().join(format!(
            "bc-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn revoke_keeps_unknown_custody_and_only_readonly_inspect_reconnects() {
    let site = Site::new();
    let path = site.0.join("s");
    let listener = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&listener, true).unwrap();
    net::bind(&listener, &SocketAddrUnix::new(&path).unwrap()).unwrap();
    net::listen(&listener, 4).unwrap();
    let mut c = Client::new(path).unwrap();
    assert!(!c.set(1, 75, 2, now() + 500_000_000).unwrap());
    assert!(c.inspect(now() + 500_000_000).unwrap());
    let p = net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
    let req = receive(&p);
    send(&p, &positive(req.id(), 2, 200, None));
    assert!(matches!(c.poll(), Some(Event::Observed)));
    let old_generation = c.incarnation();
    assert!(c.set(1, 75, 2, now() + 500_000_000).unwrap());
    assert!(matches!(receive(&p), Request::Set { .. }));
    c.revoke();
    let s = c.snapshot();
    assert!(!s.connected && s.pending && s.last_set_uncertain && s.observation.is_none());
    assert!(!c.set(1, 75, 2, now() + 500_000_000).unwrap());
    no_packet(&p);
    assert!(c.inspect(now() + 500_000_000).unwrap());
    let fresh = net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
    let req = receive(&fresh);
    assert!(matches!(req, Request::Inspect { .. }));
    assert!(c.incarnation() > old_generation);
    let busy = Reply {
        schema: 1,
        id: req.id(),
        ok: false,
        revision: 3,
        pending: true,
        observation: None,
        until_ns: "0".into(),
        requested_percent: None,
        last_set_uncertain: true,
    };
    send(&fresh, &busy);
    assert!(matches!(c.poll(), Some(Event::Observed)));
    assert!(c.snapshot().pending);
    assert!(
        c.inspect(now() + 500_000_000).unwrap(),
        "cached remote pending must allow readonly reconciliation"
    );
    let req = receive(&fresh);
    assert!(matches!(req, Request::Inspect { .. }));
    let mut r = positive(req.id(), 4, 300, None);
    r.last_set_uncertain = true;
    send(&fresh, &r);
    assert!(matches!(c.poll(), Some(Event::Observed)));
    assert!(c.snapshot().last_set_uncertain);
    assert!(!c.snapshot().pending);
    assert!(
        c.set(2, 25, 4, now() + 500_000_000).unwrap(),
        "only new explicit intent may send another Set"
    );
    assert!(matches!(
        receive(&fresh),
        Request::Set {
            percent: 25,
            expected_revision: 4,
            ..
        }
    ));
}
