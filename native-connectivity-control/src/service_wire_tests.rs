//! Typed record component tests, not radio or process-readiness evidence.
use super::*;

#[test]
fn backend_failure_retains_operation_and_original_error() {
    for source in [
        crate::kernel::Error::Deadline,
        crate::kernel::Error::Conflict,
        crate::kernel::Error::Observation(pokelinux_network_state::ObserveError::EvidenceLost),
    ] {
        let error = backend_result::<(), _>("refresh_wireless", Err(source)).unwrap_err();
        let description = format!("{error:?}");
        assert!(description.contains("refresh_wireless"), "{description}");
        assert!(
            description.contains(&format!("{source:?}")),
            "{description}"
        );
    }
}

fn service() -> Service {
    let current = File::open("/proc/thread-self/ns/net").unwrap();
    let namespace = metadata(&current).unwrap();
    Service {
        supervisor: Supervisor::new(std::env::temp_dir().join("unused-wire-test-journal")),
        identity: None,
        _current: current,
        _peer: None,
        selected: Selected::WirelessOpen { ifindex: 7 },
        namespace,
        owner: owner::Owner::default(),
        observed: None,
        handshake: false,
        terminal: false,
        stopping: false,
        output_eof: false,
        reaped: false,
        exit_status: None,
        main_caps_dropped: false,
        start_until: boot_time_ns() + 1_000_000_000,
    }
}

fn wireless(namespace: NamespaceIdentity) -> Vec<u8> {
    let mut b = vec![0; 61];
    b[0] = 2;
    b[1] = 1;
    b[10..18].copy_from_slice(&boot_time_ns().to_le_bytes());
    b[18..26].copy_from_slice(&namespace.dev.to_le_bytes());
    b[26..34].copy_from_slice(&namespace.ino.to_le_bytes());
    b[34..38].copy_from_slice(&7_u32.to_le_bytes());
    b[39] = 2;
    b[44..52].copy_from_slice(&9_u64.to_le_bytes());
    b
}

#[test]
fn existing_wireless_record_accepts_absent_witnessed_lifetime() {
    let mut s = service();
    let bytes = wireless(s.namespace);
    assert!(
        s.accept(&bytes).is_ok(),
        "typed existing-link record must not require a virtual create lifetime"
    );
    let (fact, _) = s.observed.unwrap();
    assert_eq!(fact.ifindex, 7);
    assert!(!fact.connected);
}

#[test]
fn wireless_record_rejects_invalid_identity_and_noncanonical_lifetime() {
    for offset in [0, 38, 39, 52, 53] {
        let mut s = service();
        let mut bytes = wireless(s.namespace);
        bytes[offset] = 7;
        assert!(s.accept(&bytes).is_err(), "invalid byte at {offset}");
        assert!(s.observed.is_none());
    }
    for offset in [18, 26, 34, 44] {
        let mut s = service();
        assert!(s.accept(&wireless(s.namespace)).is_ok());
        let mut changed = wireless(s.namespace);
        changed[offset] ^= 1;
        assert!(s.accept(&changed).is_err(), "identity changed at {offset}");
    }
}

#[test]
fn record_rejects_old_version_truncation_and_wrong_selected_backend() {
    let namespace = service().namespace;
    let bytes = wireless(namespace);
    for size in 0..bytes.len() {
        assert!(service().accept(&bytes[..size]).is_err());
    }
    let mut old = bytes.clone();
    old[0] = 1;
    assert!(service().accept(&old).is_err());
    let mut virtual_service = service();
    virtual_service.selected = Selected::VirtualPair;
    assert!(virtual_service.accept(&bytes).is_err());
    let virtual_fact = Fact {
        namespace,
        ifindex: 7,
        connected: false,
        identity: LinkIdentity::VirtualPair { lifetime: 1 },
    };
    let virtual_record = record(1, 0, virtual_fact, boot_time_ns());
    assert!(service().accept(&virtual_record).is_err());
    assert!(virtual_service.accept(&virtual_record).is_ok());
}

#[test]
fn wireless_start_refuses_wrong_actual_namespace_before_worker_admission() {
    let file = File::open("/proc/thread-self/ns/net").unwrap();
    let actual = metadata(&file).unwrap();
    let profile = Profile::WirelessOpen {
        target: crate::startup::Target {
            namespace: Namespace {
                dev: actual.dev,
                ino: actual.ino ^ 1,
            },
            ifindex: 7,
            name: "wlan0".into(),
        },
        frequency_mhz: 2412,
        bssid: [2, 0, 0, 0, 0, 1],
        ssid: b"authored".to_vec(),
    };
    let before = std::fs::read_to_string("/proc/self/status").unwrap();
    let outcome = Service::start(
        Path::new("/nonexistent-worker"),
        profile,
        std::env::temp_dir().join("unused-wrong-namespace-admission"),
        boot_time_ns() + 1_000_000_000,
    );
    assert_eq!(outcome.err().unwrap().kind(), io::ErrorKind::InvalidData);
    let after = std::fs::read_to_string("/proc/self/status").unwrap();
    let caps = |s: String| {
        s.lines()
            .filter(|l| l.starts_with("Cap") || l.starts_with("NoNewPrivs:"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(caps(before), caps(after));
}

#[test]
fn old_positional_worker_startup_refuses_before_capability_changes() {
    assert_eq!(
        worker(&["/proc/1/fd/3".into(), "1".into(), "2".into(), "3".into()])
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
