use super::*;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::thread;
use std::time::Duration;

const NAME: &str = "pklcap0";

fn namespace() -> NamespaceIdentity {
    let metadata = File::open("/proc/thread-self/ns/net")
        .unwrap()
        .metadata()
        .unwrap();
    NamespaceIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
    }
}

fn write_once(root: &Path, name: &str, value: Value) {
    let pending = root.join(format!("{name}.pending"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .unwrap();
    serde_json::to_writer(&mut file, &value).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
    fs::rename(pending, root.join(name)).unwrap();
}

fn wait_json(root: &Path, name: &str, until: u64) -> Value {
    loop {
        assert!(
            boot_time_ns() < until,
            "original kernel test BOOTTIME cutoff while waiting for {name}"
        );
        match File::open(root.join(name)) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(131_073).read_to_end(&mut bytes).unwrap();
                assert!(bytes.len() <= 131_072);
                let value = serde_json::from_slice(&bytes).unwrap();
                assert!(boot_time_ns() < until);
                return value;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("kernel control read: {error}"),
        }
    }
}

fn check_namespace(value: &Value, expected: NamespaceIdentity) {
    assert_eq!(
        value["network_namespace"]["dev"].as_u64(),
        Some(expected.dev)
    );
    assert_eq!(
        value["network_namespace"]["ino"].as_u64(),
        Some(expected.ino)
    );
}

fn expected_link(value: &Value, expected: NamespaceIdentity) -> u32 {
    check_namespace(value, expected);
    let links = value["links"].as_array().unwrap();
    let matches: Vec<_> = links.iter().filter(|link| link["ifname"] == NAME).collect();
    assert_eq!(matches.len(), 1, "independent actual TUN link control");
    let link = matches[0];
    assert_eq!(link["mtu"], 1400);
    let flags = link["flags"].as_array().unwrap();
    assert!(flags.contains(&json!("UP")) && flags.contains(&json!("POINTOPOINT")));
    let ifindex = u32::try_from(link["ifindex"].as_u64().unwrap()).unwrap();
    assert!(ifindex > 1);
    let addresses = value["addresses"].as_array().unwrap();
    let device = addresses
        .iter()
        .find(|device| device["ifindex"] == ifindex && device["ifname"] == NAME)
        .unwrap();
    let records = device["addr_info"].as_array().unwrap();
    assert!(records.iter().any(|address| address["family"] == "inet"
        && address["local"] == "192.0.2.2"
        && address["address"] == "192.0.2.1"
        && address["prefixlen"] == 32));
    assert!(records.iter().any(|address| address["family"] == "inet6"
        && address["local"] == "2001:db8:1::2"
        && address["prefixlen"] == 64));
    ifindex
}

#[test]
#[ignore = "requires actual isolated ARM kernel TUN and independent sole-FD owner"]
fn actual_kernel_link_lifetime() {
    let control =
        std::env::var_os("POKELINUX_NETWORK_TEST_CONTROL").expect("kernel control directory");
    let control = Path::new(&control);
    let until: u64 = std::env::var("POKELINUX_NETWORK_TEST_UNTIL")
        .unwrap()
        .parse()
        .unwrap();
    let original_namespace = namespace();
    let process_status = fs::read_to_string("/proc/self/status").unwrap();
    assert!(
        process_status
            .lines()
            .any(|line| line == "Uid:\t1000\t1000\t1000\t1000")
    );
    assert!(
        process_status
            .lines()
            .any(|line| line == "Gid:\t1000\t1000\t1000\t1000")
    );
    assert!(
        process_status
            .lines()
            .any(|line| line == "CapEff:\t0000000000000000")
    );
    let subscribed = KernelObserver::subscribe_current_namespace(until);
    let status = match &subscribed {
        Ok(_) => "subscribed",
        Err(ObserveError::Unsupported) => "unsupported",
        Err(error) => panic!("observer setup prerequisite failed: {error:?}"),
    };
    write_once(
        control,
        "observer-ready.json",
        json!({"network_namespace": {"dev": original_namespace.dev, "ino": original_namespace.ino},
        "status": status, "pid": std::process::id(), "uid": 1000, "cap_eff": "0000000000000000", "at_boottime_ns": boot_time_ns()}),
    );
    let configured = wait_json(control, "configured.json", until);
    let ifindex = expected_link(&configured, original_namespace);
    assert_eq!(namespace(), original_namespace);
    println!("ACTUAL-KERNEL-POSITIVE-CONTROL ifindex={ifindex} namespace={original_namespace:?}");
    {
        let mut observer = match subscribed {
            Ok(observer) => observer,
            Err(ObserveError::Unsupported) => {
                write_once(
                    control,
                    "outcome.json",
                    json!({"network_namespace": {"dev": original_namespace.dev, "ino": original_namespace.ino},
                "setup_control_qualified": true, "missing_observer_capability": true, "requirement_passed": false,
                "native_observer_returned_Unsupported_without_owned_object": true,
                "original_until_ns": until, "closed_at_ns": boot_time_ns()}),
                );
                panic!(
                    "MISSING_NATIVE_KERNEL_OBSERVER after independently qualified actual TUN/link/local-peer-address control"
                )
            }
            Err(error) => panic!("unexpected observer result {error:?}"),
        };
        let observed = observer
            .refresh(until)
            .expect("complete native link/address observation");
        assert_eq!(observed.namespace, original_namespace);
        assert!(
            observed.link_dump_sequence > 0
                && observed.address_dump_sequence > observed.link_dump_sequence
        );
        assert!(observed.observed_at_boottime_ns < until);
        let link = observed
            .links
            .iter()
            .find(|link| link.ifindex == ifindex && link.name == NAME)
            .unwrap();
        assert_eq!(link.mtu, 1400);
        assert_eq!(link.flags & (0x1 | 0x10), 0x1 | 0x10);
        let lifetime = link
            .observed_lifetime
            .expect("actual post-subscription create notification");
        assert!(lifetime > 0);
        assert!(link.addresses.iter().any(|address| address.local
            == "192.0.2.2".parse::<IpAddr>().unwrap()
            && address.peer == Some("192.0.2.1".parse::<IpAddr>().unwrap())
            && address.prefix_len == 32));
        assert!(link.addresses.iter().any(|address| address.local
            == "2001:db8:1::2".parse::<IpAddr>().unwrap()
            && address.peer.is_none()
            && address.prefix_len == 64));
        assert!(
            link.addresses
                .iter()
                .all(|address| address.flags & (0x20 | 0x40 | 0x08) == 0)
        );
        write_once(
            control,
            "observed.json",
            json!({"network_namespace": {"dev": observed.namespace.dev, "ino": observed.namespace.ino},
        "ifname": link.name, "ifindex": ifindex, "lifetime": lifetime, "revision": observed.revision,
        "link_dump_sequence": observed.link_dump_sequence, "address_dump_sequence": observed.address_dump_sequence,
        "at_boottime_ns": observed.observed_at_boottime_ns}),
        );
        let removed = wait_json(control, "removed.json", until);
        check_namespace(&removed, original_namespace);
        assert!(
            !removed["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["ifname"] == NAME || value["ifindex"] == ifindex)
        );
        assert!(
            !removed["addresses"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["ifname"] == NAME || value["ifindex"] == ifindex)
        );
        let withdrawn = observer
            .refresh(until)
            .expect("complete native observation after actual sole-TUN-FD close");
        assert_eq!(withdrawn.namespace, original_namespace);
        assert!(withdrawn.revision > observed.revision);
        assert!(withdrawn.link_dump_sequence > observed.address_dump_sequence);
        assert!(withdrawn.address_dump_sequence > withdrawn.link_dump_sequence);
        assert!(
            withdrawn.observed_at_boottime_ns >= observed.observed_at_boottime_ns
                && withdrawn.observed_at_boottime_ns < until
        );
        assert!(
            !withdrawn
                .links
                .iter()
                .any(|link| link.ifindex == ifindex || link.name == NAME)
        );
        write_once(
            control,
            "withdrawn.json",
            json!({"network_namespace": {"dev": withdrawn.namespace.dev, "ino": withdrawn.namespace.ino},
        "retired_ifindex": ifindex, "retired_lifetime": lifetime, "revision": withdrawn.revision,
            "link_dump_sequence": withdrawn.link_dump_sequence, "address_dump_sequence": withdrawn.address_dump_sequence,
        "at_boottime_ns": withdrawn.observed_at_boottime_ns}),
        );
        println!("ACTUAL-NATIVE-KERNEL-OBSERVATION-AND-WITHDRAWAL lifetime={lifetime}");
    } // The owned observer leaves scope before the final receipt is written.
    write_once(
        control,
        "outcome.json",
        json!({"network_namespace": {"dev": original_namespace.dev, "ino": original_namespace.ino},
        "setup_control_qualified": true, "missing_observer_capability": false, "requirement_passed": true,
        "native_observer_dropped_before_result": true, "original_until_ns": until, "closed_at_ns": boot_time_ns()}),
    );
}
