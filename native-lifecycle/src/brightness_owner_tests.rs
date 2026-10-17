//! Actual local root SEQPACKET peers and authored readback packets. The existing
//! Client is independently qualified first. No daemon/backend/hardware, effect
//! permission grant, actual remote worker custody or physical luminance proof.
use super::Owner;
use crate::{
    boottime_ns,
    brightness::{Event, Intent, Provider},
};
use pokelinux_brightness::{
    client::{Client, Event as ClientEvent},
    protocol::{Attributes, Reply, Request},
};
use rustix::{
    fd::OwnedFd,
    net::{self, AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
fn sockets() -> (OwnedFd, OwnedFd) {
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let (a, b) = net::socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&b, true).unwrap();
    let cred = net::sockopt::socket_peercred(&b).unwrap();
    assert_eq!(
        (
            cred.pid.as_raw_nonzero().get() as u32,
            cred.uid.as_raw(),
            cred.gid.as_raw()
        ),
        (std::process::id(), 0, 0)
    );
    (a, b)
}
fn receive(peer: &OwnedFd) -> Request {
    let mut body = [0; 513];
    let (n, full) = net::recv(peer, &mut body, RecvFlags::DONTWAIT | RecvFlags::TRUNC).unwrap();
    assert_eq!(n, full);
    assert!(n <= 512);
    serde_json::from_slice(&body[..n]).unwrap()
}
fn no_packet(peer: &OwnedFd) {
    let mut body = [0; 513];
    match net::recv(peer, &mut body, RecvFlags::DONTWAIT) {
        Err(rustix::io::Errno::AGAIN) | Ok((0, _)) => {}
        other => panic!("unexpected implicit packet {other:?}"),
    }
}
fn send(peer: &OwnedFd, reply: &Reply) {
    let body = serde_json::to_vec(reply).unwrap();
    assert!(body.len() <= 512);
    assert_eq!(
        net::send(peer, &body, SendFlags::DONTWAIT | SendFlags::NOSIGNAL).unwrap(),
        body.len()
    );
}
fn reply(id: u64, revision: u64, attrs: Attributes, percent: Option<u8>, until: u64) -> Reply {
    Reply {
        schema: 1,
        id,
        ok: true,
        revision,
        pending: false,
        observation: Some(attrs),
        until_ns: until.to_string(),
        requested_percent: percent,
        last_set_uncertain: false,
    }
}
fn observed_client(attrs: Attributes, until: u64) -> (Client, OwnedFd, Reply) {
    let (a, peer) = sockets();
    let mut client = Client::from_socket(a, None).unwrap();
    assert!(client.inspect(boottime_ns() + 500_000_000).unwrap());
    let request = receive(&peer);
    assert!(matches!(request, Request::Inspect { .. }));
    let packet = reply(request.id(), 2, attrs, None, until);
    send(&peer, &packet);
    assert!(matches!(client.poll(), Some(ClientEvent::Observed)));
    assert_eq!(client.snapshot().observation, Some(attrs));
    (client, peer, packet)
}
#[test]
fn owner_maps_half_up_u32_attributes_with_distinct_optional_actual_and_original_until() {
    for (attrs, observed, actual) in [
        (
            Attributes {
                max: 400,
                requested: 130,
                actual: Some(120),
            },
            33,
            Some(30),
        ),
        (
            Attributes {
                max: 3,
                requested: 1,
                actual: None,
            },
            33,
            None,
        ),
        (
            Attributes {
                max: 3,
                requested: 2,
                actual: Some(1),
            },
            67,
            Some(33),
        ),
        (
            Attributes {
                max: u32::MAX,
                requested: u32::MAX,
                actual: Some(0),
            },
            100,
            Some(0),
        ),
    ] {
        let until = boottime_ns() + 1_000_000_000;
        let (client, peer, packet) = observed_client(attrs, until);
        println!("PASS independently credentialed Client attribute control");
        let owner = Owner::from_client(client);
        let state = owner.state();
        assert!(
            state.connected && state.available,
            "MISSING_OWNER_READBACK_MAPPING"
        );
        assert_eq!(
            (
                state.revision,
                state.pending,
                state.requested_percent,
                state.observed_percent,
                state.actual_percent
            ),
            (2, false, None, Some(observed), actual)
        );
        assert_eq!(state.until_ns, packet.until_ns);
        no_packet(&peer);
    }
}
#[test]
fn owner_forwards_one_explicit_actor_intent_and_correlates_discrete_readback_not_luminance() {
    let (client, peer, _) = observed_client(
        Attributes {
            max: 3,
            requested: 1,
            actual: Some(0),
        },
        boottime_ns() + 1_000_000_000,
    );
    let mut owner = Owner::from_client(client);
    let incarnation = owner.incarnation().to_owned();
    let intent = Intent {
        id: 77,
        percent: 75,
        expected_revision: 2,
        until_ns: boottime_ns() + 500_000_000,
    };
    assert!(
        owner.submit(&intent).unwrap(),
        "MISSING_OWNER_EXPLICIT_SET_FORWARDING"
    );
    let request = receive(&peer);
    match &request {
        Request::Set {
            percent,
            expected_revision,
            until_ns,
            ..
        } => assert_eq!(
            (*percent, *expected_revision, until_ns.as_str()),
            (75, 2, intent.until_ns.to_string().as_str())
        ),
        other => panic!("wrong explicit request {other:?}"),
    }
    assert!(owner.state().pending);
    assert!(!owner.state().available);
    assert_eq!(owner.state().until_ns, "0");
    for _ in 0..3 {
        assert!(owner.poll().is_none());
        no_packet(&peer);
    }
    send(
        &peer,
        &reply(
            request.id(),
            4,
            Attributes {
                max: 3,
                requested: 2,
                actual: Some(1),
            },
            Some(75),
            boottime_ns() + 1_000_000_000,
        ),
    );
    match owner.poll() {
        Some(Event::Completed { id, state }) => {
            assert_eq!(id, 77);
            assert_eq!(
                (
                    state.requested_percent,
                    state.observed_percent,
                    state.actual_percent
                ),
                (Some(75), Some(67), Some(33))
            );
            assert!(!state.pending && state.available);
        }
        _ => panic!("missing correlated owner completion"),
    }
    assert_eq!(owner.incarnation(), incarnation);
    no_packet(&peer);
}
#[test]
fn owner_expires_cached_attribute_view_and_never_restores_request_on_readonly_reconciliation() {
    let until = boottime_ns() + 100_000_000;
    let (client, peer, _) = observed_client(
        Attributes {
            max: 400,
            requested: 300,
            actual: None,
        },
        until,
    );
    let mut owner = Owner::from_client(client);
    assert!(owner.state().available, "MISSING_OWNER_FRESH_VIEW");
    while boottime_ns() < until {
        std::thread::sleep(Duration::from_millis(2));
    }
    let state = owner.state();
    assert!(!state.available);
    assert_eq!(
        (
            state.requested_percent,
            state.observed_percent,
            state.actual_percent,
            state.until_ns.as_str()
        ),
        (None, None, None, "0")
    );
    let _ = owner.poll();
    no_packet(&peer);
}
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Site(PathBuf);
impl Site {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "bo-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn owner_revoke_keeps_pending_and_only_readonly_inspect_changes_incarnation_or_connects() {
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
    let mut owner = Owner::new(&path).unwrap();
    let initial = owner.incarnation().to_owned();
    let intent = Intent {
        id: 9,
        percent: 75,
        expected_revision: 2,
        until_ns: boottime_ns() + 500_000_000,
    };
    assert!(!owner.submit(&intent).unwrap());
    assert!(owner.poll().is_none());
    assert_eq!(owner.incarnation(), initial);
    assert!(matches!(
        net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC),
        Err(rustix::io::Errno::AGAIN)
    ));
    assert!(
        owner.inspect(boottime_ns() + 500_000_000).unwrap(),
        "MISSING_OWNER_READONLY_CONNECT"
    );
    let peer = net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
    let request = receive(&peer);
    assert!(matches!(request, Request::Inspect { .. }));
    let old = owner.incarnation().to_owned();
    assert_ne!(old, initial);
    send(
        &peer,
        &reply(
            request.id(),
            2,
            Attributes {
                max: 400,
                requested: 200,
                actual: Some(180),
            },
            None,
            boottime_ns() + 1_000_000_000,
        ),
    );
    assert!(matches!(owner.poll(), Some(Event::Observed)));
    let intent = Intent {
        until_ns: boottime_ns() + 500_000_000,
        ..intent
    };
    assert!(owner.submit(&intent).unwrap());
    assert!(matches!(receive(&peer), Request::Set { .. }));
    owner.revoke();
    let state = owner.state();
    assert!(!state.available && !state.connected && state.pending);
    assert_eq!(
        (
            state.requested_percent,
            state.observed_percent,
            state.actual_percent,
            state.until_ns.as_str()
        ),
        (None, None, None, "0")
    );
    assert!(!owner.submit(&intent).unwrap());
    assert!(owner.poll().is_none());
    assert_eq!(owner.incarnation(), old);
    no_packet(&peer);
    assert!(matches!(
        net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC),
        Err(rustix::io::Errno::AGAIN)
    ));
    assert!(owner.inspect(boottime_ns() + 500_000_000).unwrap());
    let fresh = net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC).unwrap();
    let request = receive(&fresh);
    assert!(matches!(request, Request::Inspect { .. }));
    assert_ne!(owner.incarnation(), old);
    let busy = Reply {
        schema: 1,
        id: request.id(),
        ok: false,
        revision: 3,
        pending: true,
        observation: None,
        until_ns: "0".into(),
        requested_percent: None,
        last_set_uncertain: true,
    };
    send(&fresh, &busy);
    assert!(matches!(owner.poll(), Some(Event::Observed)));
    assert!(owner.state().pending && !owner.state().available);
    assert!(owner.inspect(boottime_ns() + 500_000_000).unwrap());
    let request = receive(&fresh);
    assert!(matches!(request, Request::Inspect { .. }));
    let mut readback = reply(
        request.id(),
        4,
        Attributes {
            max: 400,
            requested: 300,
            actual: Some(120),
        },
        None,
        boottime_ns() + 1_000_000_000,
    );
    readback.last_set_uncertain = true;
    send(&fresh, &readback);
    assert!(matches!(owner.poll(), Some(Event::Observed)));
    let state = owner.state();
    assert!(state.available && !state.pending);
    assert_eq!(
        (
            state.requested_percent,
            state.observed_percent,
            state.actual_percent
        ),
        (None, Some(75), Some(30))
    );
    for _ in 0..3 {
        assert!(owner.poll().is_none());
        no_packet(&fresh);
    }
}
#[test]
fn owner_false_submission_has_no_poll_queue_or_hidden_connection() {
    let (client, peer, _) = observed_client(
        Attributes {
            max: 400,
            requested: 200,
            actual: Some(120),
        },
        boottime_ns() + 1_000_000_000,
    );
    let mut owner = Owner::from_client(client);
    let intent = Intent {
        id: 12,
        percent: 75,
        expected_revision: 999,
        until_ns: boottime_ns() + 500_000_000,
    };
    assert!(!owner.submit(&intent).unwrap());
    for _ in 0..4 {
        assert!(owner.poll().is_none());
        no_packet(&peer);
    }
}
