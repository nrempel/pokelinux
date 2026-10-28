use super::*;
use pokelinux_network_state::{AddressSnapshot, LinkSnapshot, ObserveError};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn target() -> ExistingLinkIdentity {
    ExistingLinkIdentity {
        namespace: NamespaceIdentity { dev: 5, ino: 42 },
        ifindex: 3,
        name: "wlan1".into(),
    }
}

fn snapshot() -> KernelSnapshot {
    KernelSnapshot {
        namespace: target().namespace,
        revision: 7,
        link_dump_sequence: 11,
        address_dump_sequence: 12,
        observed_at_boottime_ns: 100,
        links: vec![LinkSnapshot {
            ifindex: 3,
            name: "wlan1".into(),
            observed_lifetime: None,
            flags: 1,
            mtu: 1500,
            operstate: Some(6),
            addresses: vec![],
        }],
    }
}

fn fixed_address() -> AddressSnapshot {
    AddressSnapshot {
        local: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
        peer: None,
        prefix_len: 30,
        scope: 0,
        flags: 0x80,
    }
}

#[test]
fn selected_profile_requires_exact_initial_state() {
    let mut value = snapshot();
    let absent = selected_profile(&value, &target(), None, false).unwrap();
    assert!(!absent.configured);
    assert_eq!(absent.observed_lifetime, None);
    assert_eq!(absent.observed_at_boottime_ns, 100);
    value.links[0].addresses.push(fixed_address());
    assert!(
        selected_profile(&value, &target(), None, true)
            .unwrap()
            .configured
    );
    // An already configured interface cannot silently satisfy a fresh Up's
    // required initial absence, including after an earlier uncertain action.
    assert_eq!(
        selected_profile(&value, &target(), None, false),
        Err(Error::Conflict)
    );
    value.links[0].addresses.clear();
    assert_eq!(
        selected_profile(&value, &target(), None, true),
        Err(Error::Conflict)
    );
}

#[test]
fn selected_identity_and_unwitnessed_lifetime_are_exact() {
    let original = snapshot();
    let mut changes = vec![];
    let mut value = original.clone();
    value.namespace.ino += 1;
    changes.push(value);
    let mut value = original.clone();
    value.links[0].ifindex += 1;
    changes.push(value);
    let mut value = original.clone();
    value.links[0].name = "other0".into();
    changes.push(value);
    let mut value = original.clone();
    value.links[0].flags = 0;
    changes.push(value);
    let mut value = original.clone();
    value.links[0].observed_lifetime = Some(77);
    changes.push(value);
    let mut value = original.clone();
    value.links.clear();
    changes.push(value);
    for changed in changes {
        assert_eq!(
            selected_profile(&changed, &target(), None, false),
            Err(Error::Conflict)
        );
    }
}

#[test]
fn shared_fixed_profile_rejects_conflicting_ipv4() {
    for altered in [
        AddressSnapshot {
            local: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2)),
            ..fixed_address()
        },
        AddressSnapshot {
            peer: Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2))),
            ..fixed_address()
        },
        AddressSnapshot {
            prefix_len: 32,
            ..fixed_address()
        },
        AddressSnapshot {
            scope: 253,
            ..fixed_address()
        },
    ] {
        let mut value = snapshot();
        value.links[0].addresses.push(altered);
        assert_eq!(
            selected_profile(&value, &target(), None, true),
            Err(Error::Conflict)
        );
    }
    let mut value = snapshot();
    value.links[0].addresses = vec![fixed_address(), fixed_address()];
    assert_eq!(
        selected_profile(&value, &target(), None, true),
        Err(Error::Conflict)
    );
    // Preserve the existing virtual backend's IPv4-only predicate.
    value.links[0].addresses = vec![AddressSnapshot {
        local: IpAddr::V6(Ipv6Addr::LOCALHOST),
        peer: None,
        prefix_len: 128,
        scope: 254,
        flags: 0,
    }];
    assert!(
        !selected_profile(&value, &target(), None, false)
            .unwrap()
            .configured
    );
}

#[test]
fn malformed_expired_and_wrong_namespace_attach_are_unsent() {
    for (index, name) in [
        (0, "wlan1"),
        (1, "wlan1"),
        (3, "lo"),
        (3, ""),
        (3, "bad/name"),
        (3, "too-long-interface"),
    ] {
        let mut selected = target();
        selected.ifindex = index;
        selected.name = name.into();
        assert!(matches!(
            ExistingIpv4Profile::attach_existing(selected, false, boot_time_ns() + 1_000_000_000),
            Err(Error::InvalidData)
        ));
    }
    assert!(matches!(
        ExistingIpv4Profile::attach_existing(target(), false, boot_time_ns()),
        Err(Error::Deadline)
    ));
    assert!(matches!(
        ExistingIpv4Profile::attach_existing(target(), false, boot_time_ns() + 2_000_000_000),
        Err(Error::Deadline)
    ));
    let actual = identity(&File::open("/proc/thread-self/ns/net").unwrap()).unwrap();
    let mut wrong = target();
    wrong.namespace = NamespaceIdentity {
        ino: actual.ino.checked_add(1).unwrap(),
        ..actual
    };
    assert!(matches!(
        ExistingIpv4Profile::attach_existing(wrong, false, boot_time_ns() + 1_000_000_000),
        Err(Error::Conflict)
    ));
}

#[test]
fn normal_cli_has_exact_modes_and_bounded_selectors() {
    let valid: Vec<String> = ["profile-down", "5", "42", "3", "wlan1", "100"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let (configured, selected, until) = parse_cli(&valid).unwrap();
    assert!(!configured);
    assert_eq!(selected, target());
    assert_eq!(until, 100);
    for (index, text) in [
        (0, "disconnect"),
        (1, "05"),
        (2, "0"),
        (3, "4294967296"),
        (3, "1"),
        (4, "bad/name"),
        (5, "+100"),
    ] {
        let mut bad = valid.clone();
        bad[index] = text.into();
        assert_eq!(
            parse_cli(&bad).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(
        parse_cli(&valid[..5]).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn retained_origin_binds_real_creating_thread_and_namespace() {
    let actual = identity(&File::open("/proc/thread-self/ns/net").unwrap()).unwrap();
    let origin = Origin::retain(actual).unwrap();
    assert_eq!(origin.check(), Ok(()));
    assert_eq!(
        std::thread::spawn(move || origin.check()).join().unwrap(),
        Err(Error::Conflict)
    );
    let mut changed = Origin::retain(actual).unwrap();
    changed.namespace.ino += 1;
    assert_eq!(changed.check(), Err(Error::Conflict));
}

fn terminal_control(terminal: bool) -> ExistingIpv4Profile {
    // Authored component state with real retained namespace/netlink descriptors.
    // The impossible selected index prevents mutation even if a guard regresses;
    // this does not claim actual link creation, deletion or configuration.
    let namespace = identity(&File::open("/proc/thread-self/ns/net").unwrap()).unwrap();
    ExistingIpv4Profile {
        origin: Origin::retain(namespace).unwrap(),
        observer: KernelObserver::subscribe_current_namespace(boot_time_ns() + 1_000_000_000)
            .unwrap(),
        mutator: Mutator::new().unwrap(),
        target: ExistingLinkIdentity {
            namespace,
            ifindex: u32::MAX,
            name: "retired0".into(),
        },
        observed_lifetime: None,
        configured: false,
        terminal,
    }
}

#[test]
fn terminal_state_fences_every_further_method_and_deadline() {
    let mut profile = terminal_control(true);
    assert_eq!(profile.refresh(0), Err(Error::Uncertain));
    assert_eq!(profile.set_configured(true, 0), Err(Error::Uncertain));
    assert_eq!(
        profile.refresh(boot_time_ns() + 1_000_000_000),
        Err(Error::Uncertain)
    );
    assert_eq!(
        profile.set_configured(false, boot_time_ns() + 1_000_000_000),
        Err(Error::Uncertain)
    );
}

#[test]
fn observer_failure_retires_the_profile_without_renewal() {
    let mut profile = terminal_control(false);
    assert_eq!(
        profile.observer.refresh(boot_time_ns()),
        Err(ObserveError::Deadline)
    );
    assert_eq!(
        profile.refresh(boot_time_ns() + 1_000_000_000),
        Err(Error::Observation(ObserveError::Deadline))
    );
    assert_eq!(
        profile.set_configured(true, boot_time_ns() + 1_000_000_000),
        Err(Error::Uncertain)
    );
    assert_eq!(
        profile.refresh(boot_time_ns() + 1_000_000_000),
        Err(Error::Uncertain)
    );
}
