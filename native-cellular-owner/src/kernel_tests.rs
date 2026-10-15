use super::*;
use pokelinux_cellular_boundary::generated::{
    android::hardware::radio::data::DataRequestReason::DataRequestReason,
    org::pokelinux::datatest::{CaptureSetup::CaptureSetup, IDataFixture::IDataFixture},
};
use pokelinux_cellular_boundary::{
    AccessNetwork, RadioError, RadioResponseInfo, RadioResponseType,
};
use pokelinux_network_state::KernelSnapshot;
use rsbinder::{
    FromIBinder, ProcessState, Proxy,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::mpsc::TryRecvError,
    thread,
    time::Duration,
};
const DATA_HASH: &str = "cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df";
const FIXTURE_HASH: &str = "b74bae7e2fba9b8fbb574174a24e7431dbb6281b";
const SERIAL: i32 = 713;
fn before(until: u64) {
    assert!(boot_ns() < until, "original BOOTTIME cutoff");
}
fn namespace() -> NamespaceIdentity {
    let m = File::open("/proc/thread-self/ns/net")
        .unwrap()
        .metadata()
        .unwrap();
    NamespaceIdentity {
        dev: m.dev(),
        ino: m.ino(),
    }
}
fn ns_json(ns: NamespaceIdentity) -> Value {
    json!({"dev":ns.dev,"ino":ns.ino})
}
fn write_once(root: &Path, name: &str, v: Value, until: u64) {
    before(until);
    let pending = root.join(format!("{name}.pending"));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .unwrap();
    let data = serde_json::to_vec(&v).unwrap();
    assert!(data.len() <= 16384);
    f.write_all(&data).unwrap();
    f.write_all(b"\n").unwrap();
    f.sync_all().unwrap();
    fs::rename(pending, root.join(name)).unwrap();
    before(until);
}
fn wait_json(root: &Path, name: &str, until: u64) -> Value {
    loop {
        before(until);
        match File::open(root.join(name)) {
            Ok(f) => {
                let mut v = Vec::new();
                f.take(131073).read_to_end(&mut v).unwrap();
                assert!(v.len() <= 131072);
                before(until);
                return serde_json::from_slice(&v).unwrap();
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(e) => panic!("control read: {e}"),
        }
    }
}
fn barrier(f: &Strong<dyn IDataFixture>, responses: i32, indications: i32, until: u64) {
    before(until);
    f.barrier(
        1,
        1,
        responses,
        indications,
        i64::try_from(until.min(boot_ns() + 1_000_000_000)).unwrap(),
    )
    .unwrap();
    before(until);
}
fn counts(v: &CaptureSetup, responses: i32, indications: i32) {
    assert_eq!(
        (
            v.attachmentAttempts,
            v.attachmentsRetained,
            v.setupAttempts,
            v.setupPresent
        ),
        (1, 1, 1, true)
    );
    assert_eq!(
        (
            v.responseAttempts,
            v.responseSubmissions,
            v.indicationAttempts,
            v.indicationSubmissions
        ),
        (responses, responses, indications, indications)
    );
    assert_eq!((v.forbiddenCalls, v.rejectedCalls), (0, 0));
}
fn receive(owner: &DataOwner, until: u64) -> Event {
    loop {
        before(until);
        assert!(!owner.shared.fault.load(Ordering::SeqCst));
        match owner.received.try_recv() {
            Ok(event) => return event,
            Err(TryRecvError::Disconnected) => panic!("callback disconnected"),
            Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(1)),
        }
    }
}
fn exact_response(event: &Event, serial: i32) -> Value {
    let Event::Setup(packet, uid) = event else {
        panic!("actual setup callback required")
    };
    let (info, result) = packet.as_ref();
    assert_eq!(*uid, 1000);
    assert_eq!(info.r#type, RadioResponseType::SOLICITED);
    assert_eq!(info.serial, serial);
    assert_eq!(info.error, RadioError::NONE);
    assert!(fixture::bounded_result(result));
    let value = fixture::result_json(&fixture::copy_result(result).unwrap());
    assert_eq!(value, fixture::result_json(&fixture::result()));
    value
}
fn kernel_candidate(owner: &mut DataOwner, until: u64) -> KernelSnapshot {
    loop {
        before(until);
        match owner.observer.refresh(until) {
            Ok(v) => return v,
            Err(ObserveError::ConcurrentChange) => thread::sleep(Duration::from_millis(1)),
            Err(e) => panic!("actual kernel candidate {e:?}"),
        }
    }
}
#[test]
#[ignore = "requires owned actual ARM Binder peers and independently created sole-FD TUN"]
fn actual_binder_kernel_join() {
    let control = std::env::var_os("POKELINUX_DATA_OWNER_CONTROL").unwrap();
    let control = Path::new(&control);
    let until: u64 = std::env::var("POKELINUX_DATA_INTEROP_UNTIL_NS")
        .unwrap()
        .parse()
        .unwrap();
    before(until);
    let generation = std::env::var("POKELINUX_DATA_INTEROP_GENERATION").unwrap();
    assert_eq!(generation.len(), 32);
    assert!(
        generation
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    let status = fs::read_to_string("/proc/self/status").unwrap();
    for line in [
        "Uid:\t1000\t1000\t1000\t1000",
        "Gid:\t1000\t1000\t1000\t1000",
        "CapEff:\t0000000000000000",
    ] {
        assert!(status.lines().any(|v| v == line));
    }
    let ns = namespace();
    let device = std::env::var("POKELINUX_DATA_INTEROP_DEVICE").unwrap();
    let process = ProcessState::init(&device, 4).unwrap();
    ProcessState::start_thread_pool();
    let context = process.context_object().unwrap();
    assert!(context.is_remote());
    assert_eq!(context.descriptor(), "android.os.IServiceManager");
    let manager = BpServiceManager::from_binder(context).unwrap();
    before(until);
    let transferred = manager
        .checkService("org.pokelinux.fixture.cellular-data")
        .unwrap()
        .unwrap();
    assert!(transferred.is_remote());
    assert_eq!(
        transferred.descriptor(),
        "org.pokelinux.datatest.IDataFixture"
    );
    let fixture: Strong<dyn IDataFixture> = FromIBinder::try_from(transferred).unwrap();
    assert_eq!(fixture.getInterfaceVersion().unwrap(), 1);
    assert_eq!(fixture.getInterfaceHash().unwrap(), FIXTURE_HASH);
    assert_eq!(fixture.generation().unwrap(), generation);
    let endpoint = fixture.endpoint().unwrap();
    assert!(endpoint.as_binder().is_remote());
    assert_eq!(endpoint.getInterfaceVersion().unwrap(), 3);
    assert_eq!(endpoint.getInterfaceHash().unwrap(), DATA_HASH);
    before(until);
    let mut owner = DataOwner::new(endpoint, 1000, SERIAL, until).unwrap();
    let (response, indication) = owner.callbacks();
    owner
        .endpoint()
        .setResponseFunctions(response, indication)
        .unwrap();
    before(until);
    let serial = owner.mark_may_have_submitted().unwrap();
    assert_eq!(serial, SERIAL);
    let profile = fixture::profile();
    let addresses = fixture::addresses();
    let dnses = fixture::dnses();
    let slice = fixture::slice();
    owner
        .endpoint()
        .setupDataCall(
            serial,
            AccessNetwork::NGRAN,
            &profile,
            false,
            DataRequestReason::NORMAL,
            &addresses,
            &dnses,
            7,
            Some(&slice),
            true,
        )
        .unwrap();
    barrier(&fixture, 0, 0, until);
    let captured = fixture.capture().unwrap();
    counts(&captured, 0, 0);
    assert_eq!(
        (
            captured.attachmentCallingPid,
            captured.setupCallingPid,
            captured.attachmentCallingUid,
            captured.setupCallingUid
        ),
        (0, 0, 1000, 1000)
    );
    let request = json!({"serial":captured.serial,"accessNetwork":captured.accessNetwork.0,"dataProfileInfo":fixture::profile_json(&captured.dataProfileInfo),
        "roamingAllowed":captured.roamingAllowed,"reason":captured.reason.0,"addresses":fixture::addresses_json(&captured.addresses),"dnses":captured.dnses,
        "pduSessionId":captured.pduSessionId,"sliceInfo":fixture::slice_json(captured.sliceInfo.as_ref()),"matchAllRuleAllowed":captured.matchAllRuleAllowed});
    let expected = json!({"serial":SERIAL,"accessNetwork":6,"dataProfileInfo":fixture::profile_json(&profile),"roamingAllowed":false,"reason":1,
        "addresses":fixture::addresses_json(&addresses),"dnses":dnses,"pduSessionId":7,"sliceInfo":fixture::slice_json(Some(&slice)),"matchAllRuleAllowed":true});
    assert_eq!(request, expected);
    assert_eq!(owner.disposition(), Disposition::OutcomeUnknown);
    write_once(
        control,
        "owner-ready.json",
        json!({"event":"data-owner-ready","network_namespace":ns_json(ns),"pid":std::process::id(),"uid":1000,
        "at_boottime_ns":boot_ns(),"original_until_ns":until,"generation":generation,"request":request,"attachment_attempts":1,"setup_attempts":1}),
        until,
    );
    let configured = wait_json(control, "configured.json", until);
    assert_eq!(configured["network_namespace"], ns_json(ns));
    let links = configured["links"].as_array().unwrap();
    let independent = links.iter().find(|v| v["ifname"] == "pklabi0").unwrap();
    let ifindex = u32::try_from(independent["ifindex"].as_u64().unwrap()).unwrap();
    assert!(ifindex > 1);
    assert_eq!(independent["mtu"], 1400);
    let kernel = kernel_candidate(&mut owner, until);
    assert_eq!(kernel.namespace, ns);
    let link = kernel
        .links
        .iter()
        .find(|v| v.ifindex == ifindex && v.name == "pklabi0")
        .unwrap();
    assert_eq!(link.mtu, 1400);
    assert_eq!(link.flags & (1 | 16), 1 | 16);
    let lifetime = link.observed_lifetime.unwrap();
    assert!(lifetime > 0);
    assert!(link.addresses.iter().any(|v| v.local
        == "192.0.2.2".parse::<std::net::IpAddr>().unwrap()
        && v.peer == Some("192.0.2.1".parse().unwrap())
        && v.prefix_len == 32));
    assert!(link.addresses.iter().any(|v| v.local
        == "2001:db8:1::2".parse::<std::net::IpAddr>().unwrap()
        && v.peer.is_none()
        && v.prefix_len == 64));
    assert!(
        link.addresses
            .iter()
            .all(|v| v.flags & (0x08 | 0x20 | 0x40) == 0)
    );
    let hold_started = boot_ns();
    let hold_until = hold_started + 100_000_000;
    assert!(hold_until < until);
    while boot_ns() < hold_until {
        before(until);
        assert!(matches!(
            owner.received.try_recv(),
            Err(TryRecvError::Empty)
        ));
        assert!(!owner.shared.fault.load(Ordering::SeqCst));
        thread::sleep(Duration::from_millis(1));
    }
    let wrong = RadioResponseInfo {
        r#type: RadioResponseType::SOLICITED,
        serial: SERIAL + 1,
        error: RadioError::NONE,
    };
    fixture.setupResponse(&wrong, &fixture::result()).unwrap();
    barrier(&fixture, 1, 0, until);
    let wrong_event = receive(&owner, until);
    exact_response(&wrong_event, SERIAL + 1);
    let right = RadioResponseInfo {
        r#type: RadioResponseType::SOLICITED,
        serial: SERIAL,
        error: RadioError::NONE,
    };
    fixture.setupResponse(&right, &fixture::result()).unwrap();
    barrier(&fixture, 2, 0, until);
    let right_event = receive(&owner, until);
    let provider_result = exact_response(&right_event, SERIAL);
    counts(&fixture.capture().unwrap(), 2, 0);
    assert!(!owner.shared.fault.load(Ordering::SeqCst));
    assert!(matches!(
        owner.received.try_recv(),
        Err(TryRecvError::Empty)
    ));
    let kernel = kernel_candidate(&mut owner, until);
    assert_eq!(kernel.namespace, ns);
    assert!(
        kernel
            .links
            .iter()
            .any(|v| v.ifindex == ifindex && v.observed_lifetime == Some(lifetime))
    );
    write_once(
        control,
        "prerequisites.json",
        json!({"event":"data-owner-prerequisites","network_namespace":ns_json(ns),"ifindex":ifindex,"lifetime":lifetime,
        "revision":kernel.revision,"link_dump_sequence":kernel.link_dump_sequence,"address_dump_sequence":kernel.address_dump_sequence,
        "provider_result":provider_result,"wrong_serial":SERIAL+1,"right_serial":SERIAL,"callback_uid":1000,"held_no_callback":true,
        "hold_started_ns":hold_started,"hold_until_ns":hold_until,"at_boottime_ns":boot_ns(),"original_until_ns":until}),
        until,
    );
    assert!(owner.provider_report().is_none());
    let reduced = owner.reduce(wrong_event);
    if reduced == Err(Error::Unsupported) {
        assert_eq!(owner.disposition(), Disposition::OutcomeUnknown);
        assert!(owner.last_association().is_none());
        drop(owner);
        drop(fixture);
        before(until);
        write_once(
            control,
            "outcome.json",
            json!({"event":"data-owner-outcome","requirement_passed":false,"missing_native_Data_join":true,
            "reason":"Unsupported","network_namespace":ns_json(ns),"provider_disposition":"OutcomeUnknown","association_present":false,
            "owner_dropped_before_result":true,"original_until_ns":until,"closed_at_ns":boot_ns()}),
            until,
        );
        panic!(
            "MISSING_NATIVE_DATA_JOIN after actual raw Binder callbacks and complete kernel candidate"
        );
    }
    assert_eq!(reduced, Ok(false));
    assert!(owner.provider_report().is_none());
    assert_eq!(owner.disposition(), Disposition::OutcomeUnknown);
    assert!(owner.last_association().is_none());
    assert_eq!(owner.reduce(right_event), Ok(true));
    assert_eq!(
        owner.disposition(),
        Disposition::ProviderAccepted { cid: 9 }
    );
    let poll_started = boot_ns();
    let association = owner.poll().unwrap().unwrap().clone();
    assert!(
        association.observed_at_boottime_ns >= poll_started
            && association.observed_at_boottime_ns < until
    );
    assert!(association.revision >= kernel.revision);
    assert_eq!(association.namespace, ns);
    assert_eq!(
        (association.ifindex, association.lifetime),
        (ifindex, lifetime)
    );
    assert_eq!(
        fixture::result_json(owner.provider_report().unwrap()),
        provider_result
    );
    write_once(
        control,
        "associated.json",
        json!({"event":"data-owner-associated","network_namespace":ns_json(ns),"ifindex":ifindex,"lifetime":lifetime,
        "revision":association.revision,"observed_at_boottime_ns":association.observed_at_boottime_ns,"provider_cid":9,"serial":SERIAL,
        "wrong_serial_ignored":true,"provider_disposition":"ProviderAccepted","original_until_ns":until,"at_boottime_ns":boot_ns()}),
        until,
    );
    let removed = wait_json(control, "removed.json", until);
    assert_eq!(removed["network_namespace"], ns_json(ns));
    for key in ["links", "addresses"] {
        assert!(
            !removed[key]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["ifname"] == "pklabi0" || v["ifindex"] == ifindex)
        );
    }
    assert!(owner.poll().unwrap().is_none());
    assert_eq!(
        owner.disposition(),
        Disposition::ProviderAccepted { cid: 9 }
    );
    assert_eq!(
        fixture::result_json(owner.provider_report().unwrap()),
        provider_result
    );
    fixture.listChanged(&[]).unwrap();
    barrier(&fixture, 2, 1, until);
    let empty = receive(&owner, until);
    assert!(matches!(&empty,Event::List(_,v,1000) if v.is_empty()));
    assert_eq!(owner.reduce(empty), Ok(true));
    assert!(owner.poll().unwrap().is_none());
    assert_eq!(
        owner.disposition(),
        Disposition::ProviderAccepted { cid: 9 }
    );
    counts(&fixture.capture().unwrap(), 2, 1);
    let _ = fixture.die();
    let died = receive(&owner, until);
    assert!(matches!(&died, Event::Died));
    assert_eq!(owner.reduce(died), Ok(true));
    assert!(owner.poll().unwrap().is_none());
    assert_eq!(
        owner.disposition(),
        Disposition::ProviderAccepted { cid: 9 }
    );
    assert_eq!(
        fixture::result_json(owner.provider_report().unwrap()),
        provider_result
    );
    let death_at = boot_ns();
    drop(owner);
    drop(fixture);
    assert_eq!(namespace(), ns);
    write_once(
        control,
        "outcome.json",
        json!({"event":"data-owner-outcome","requirement_passed":true,"missing_native_Data_join":false,
        "reason":"ObservedThenWithdrawn","network_namespace":ns_json(ns),"provider_disposition":"ProviderAccepted","provider_cid":9,
        "association_present":false,"kernel_loss_withdrawn":true,"empty_list_observed":true,"endpoint_death_observed_at_ns":death_at,
        "original_until_ns":until,"owner_dropped_before_result":true,"closed_at_ns":boot_ns()}),
        until,
    );
}

// Additional actual-boundary coverage against the unchanged repaired library.
// These cases exercise only public DataOwner methods, never its inbox/reducer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PublicCase {
    Link,
    List,
    Death,
}
impl PublicCase {
    fn label(self) -> &'static str {
        match self {
            Self::Link => "setup-link-loss",
            Self::List => "empty-list-loss",
            Self::Death => "endpoint-loss",
        }
    }
    fn test(self) -> &'static str {
        match self {
            Self::Link => "kernel_tests::actual_public_poll_setup_and_link_loss",
            Self::List => "kernel_tests::actual_public_poll_empty_list_loss",
            Self::Death => "kernel_tests::actual_public_poll_endpoint_loss",
        }
    }
}
fn public_history(owner: &DataOwner, expected: &Value) {
    assert_eq!(
        owner.disposition(),
        Disposition::ProviderAccepted { cid: 9 }
    );
    assert_eq!(
        fixture::result_json(owner.provider_report().unwrap()),
        *expected
    );
}
fn public_control_link(value: &Value, ns: NamespaceIdentity) -> u32 {
    assert_eq!(value["network_namespace"], ns_json(ns));
    let links = value["links"].as_array().unwrap();
    let selected: Vec<_> = links.iter().filter(|v| v["ifname"] == "pklabi0").collect();
    assert_eq!(selected.len(), 1);
    let link = selected[0];
    assert_eq!(link["mtu"], 1400);
    let flags = link["flags"].as_array().unwrap();
    assert!(flags.contains(&json!("UP")) && flags.contains(&json!("POINTOPOINT")));
    let ifindex = u32::try_from(link["ifindex"].as_u64().unwrap()).unwrap();
    assert!(ifindex > 1);
    let devices = value["addresses"].as_array().unwrap();
    let records = devices
        .iter()
        .find(|v| v["ifindex"] == ifindex && v["ifname"] == "pklabi0")
        .unwrap()["addr_info"]
        .as_array()
        .unwrap();
    assert!(records.iter().any(|v| v["family"] == "inet"
        && v["local"] == "192.0.2.2"
        && v["address"] == "192.0.2.1"
        && v["prefixlen"] == 32));
    assert!(
        records.iter().any(|v| v["family"] == "inet6"
            && v["local"] == "2001:db8:1::2"
            && v["prefixlen"] == 64)
    );
    ifindex
}
fn public_join(owner: &mut DataOwner, until: u64) -> Association {
    loop {
        before(until);
        let started = boot_ns();
        match owner.poll() {
            Ok(Some(v)) => {
                let v = v.clone();
                assert!(v.observed_at_boottime_ns >= started && v.observed_at_boottime_ns < until);
                return v;
            }
            Ok(None) | Err(Error::Kernel(ObserveError::ConcurrentChange)) => {
                thread::sleep(Duration::from_millis(1))
            }
            Err(e) => panic!("public poll join failed {e:?}"),
        }
    }
}
fn actual_public_poll_case(case: PublicCase) {
    let control = std::env::var_os("POKELINUX_DATA_OWNER_CONTROL").unwrap();
    let control = Path::new(&control);
    let until: u64 = std::env::var("POKELINUX_DATA_INTEROP_UNTIL_NS")
        .unwrap()
        .parse()
        .unwrap();
    before(until);
    let generation = std::env::var("POKELINUX_DATA_INTEROP_GENERATION").unwrap();
    assert_eq!(generation.len(), 32);
    assert!(
        generation
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    let status = fs::read_to_string("/proc/self/status").unwrap();
    for line in [
        "Uid:\t1000\t1000\t1000\t1000",
        "Gid:\t1000\t1000\t1000\t1000",
        "CapEff:\t0000000000000000",
    ] {
        assert!(status.lines().any(|v| v == line));
    }
    let ns = namespace();
    let device = std::env::var("POKELINUX_DATA_INTEROP_DEVICE").unwrap();
    let process = ProcessState::init(&device, 4).unwrap();
    ProcessState::start_thread_pool();
    let context = process.context_object().unwrap();
    assert!(context.is_remote());
    assert_eq!(context.descriptor(), "android.os.IServiceManager");
    let manager = BpServiceManager::from_binder(context).unwrap();
    before(until);
    let transferred = manager
        .checkService("org.pokelinux.fixture.cellular-data")
        .unwrap()
        .unwrap();
    assert!(transferred.is_remote());
    assert_eq!(
        transferred.descriptor(),
        "org.pokelinux.datatest.IDataFixture"
    );
    let fixture: Strong<dyn IDataFixture> = FromIBinder::try_from(transferred).unwrap();
    assert_eq!(fixture.getInterfaceVersion().unwrap(), 1);
    assert_eq!(fixture.getInterfaceHash().unwrap(), FIXTURE_HASH);
    assert_eq!(fixture.generation().unwrap(), generation);
    let endpoint = fixture.endpoint().unwrap();
    assert!(endpoint.as_binder().is_remote());
    assert_eq!(endpoint.getInterfaceVersion().unwrap(), 3);
    assert_eq!(endpoint.getInterfaceHash().unwrap(), DATA_HASH);
    before(until);
    let mut owner = DataOwner::new(endpoint, 1000, SERIAL, until).unwrap();
    let (response, indication) = owner.callbacks();
    owner
        .endpoint()
        .setResponseFunctions(response, indication)
        .unwrap();
    assert_eq!(owner.mark_may_have_submitted().unwrap(), SERIAL);
    let profile = fixture::profile();
    let addresses = fixture::addresses();
    let dnses = fixture::dnses();
    let slice = fixture::slice();
    owner
        .endpoint()
        .setupDataCall(
            SERIAL,
            AccessNetwork::NGRAN,
            &profile,
            false,
            DataRequestReason::NORMAL,
            &addresses,
            &dnses,
            7,
            Some(&slice),
            true,
        )
        .unwrap();
    barrier(&fixture, 0, 0, until);
    let captured = fixture.capture().unwrap();
    counts(&captured, 0, 0);
    assert_eq!(
        (
            captured.attachmentCallingPid,
            captured.setupCallingPid,
            captured.attachmentCallingUid,
            captured.setupCallingUid
        ),
        (0, 0, 1000, 1000)
    );
    let request = json!({"serial":captured.serial,"accessNetwork":captured.accessNetwork.0,"dataProfileInfo":fixture::profile_json(&captured.dataProfileInfo),
        "roamingAllowed":captured.roamingAllowed,"reason":captured.reason.0,"addresses":fixture::addresses_json(&captured.addresses),"dnses":captured.dnses,
        "pduSessionId":captured.pduSessionId,"sliceInfo":fixture::slice_json(captured.sliceInfo.as_ref()),"matchAllRuleAllowed":captured.matchAllRuleAllowed});
    assert_eq!(
        request,
        json!({"serial":SERIAL,"accessNetwork":6,"dataProfileInfo":fixture::profile_json(&profile),"roamingAllowed":false,"reason":1,
        "addresses":fixture::addresses_json(&addresses),"dnses":dnses,"pduSessionId":7,"sliceInfo":fixture::slice_json(Some(&slice)),"matchAllRuleAllowed":true})
    );
    assert_eq!(owner.disposition(), Disposition::OutcomeUnknown);
    assert!(owner.provider_report().is_none());
    assert!(owner.poll().unwrap().is_none());
    let info = RadioResponseInfo {
        r#type: RadioResponseType::SOLICITED,
        serial: SERIAL,
        error: RadioError::NONE,
    };
    fixture.setupResponse(&info, &fixture::result()).unwrap();
    barrier(&fixture, 1, 0, until);
    let provider_result = fixture::result_json(&fixture::result());
    loop {
        before(until);
        assert!(
            owner.poll().unwrap().is_none(),
            "no actual TUN exists before native ready"
        );
        if owner.disposition() == (Disposition::ProviderAccepted { cid: 9 }) {
            break;
        }
        assert_eq!(owner.disposition(), Disposition::OutcomeUnknown);
        thread::sleep(Duration::from_millis(1));
    }
    public_history(&owner, &provider_result);
    assert!(owner.last_association().is_none());
    counts(&fixture.capture().unwrap(), 1, 0);
    write_once(
        control,
        "owner-ready.json",
        json!({"event":"data-public-poll-ready","case":case.label(),"test":case.test(),"network_namespace":ns_json(ns),
        "pid":std::process::id(),"uid":1000,"at_boottime_ns":boot_ns(),"original_until_ns":until,"generation":generation,"request":request,
        "provider_result":provider_result,"provider_disposition":"ProviderAccepted","provider_cid":9,"association_present":false,
        "setup_via_public_poll":true,"attachment_attempts":1,"setup_attempts":1,"response_attempts":1}),
        until,
    );
    let configured = wait_json(control, "configured.json", until);
    let ifindex = public_control_link(&configured, ns);
    let joined = public_join(&mut owner, until);
    assert_eq!(joined.namespace, ns);
    assert_eq!(joined.ifindex, ifindex);
    assert!(joined.lifetime > 0);
    public_history(&owner, &provider_result);
    write_once(
        control,
        "associated.json",
        json!({"event":"data-public-poll-associated","case":case.label(),"test":case.test(),"network_namespace":ns_json(ns),
        "ifindex":ifindex,"lifetime":joined.lifetime,"revision":joined.revision,"observed_at_boottime_ns":joined.observed_at_boottime_ns,
        "provider_cid":9,"serial":SERIAL,"provider_disposition":"ProviderAccepted","original_until_ns":until,"at_boottime_ns":boot_ns()}),
        until,
    );
    let withdrawn_at;
    let mut confirmed_at = None;
    if case == PublicCase::Link {
        let removed = wait_json(control, "removed.json", until);
        assert_eq!(removed["network_namespace"], ns_json(ns));
        for key in ["links", "addresses"] {
            assert!(
                !removed[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["ifname"] == "pklabi0" || v["ifindex"] == ifindex)
            );
        }
        assert!(owner.poll().unwrap().is_none());
        withdrawn_at = boot_ns();
        public_history(&owner, &provider_result);
        counts(&fixture.capture().unwrap(), 1, 0);
    } else {
        let allowed = wait_json(control, "loss-allowed.json", until);
        assert_eq!(public_control_link(&allowed, ns), ifindex);
        let before_loss = public_join(&mut owner, until);
        assert_eq!(
            (
                before_loss.namespace,
                before_loss.ifindex,
                before_loss.lifetime
            ),
            (ns, ifindex, joined.lifetime)
        );
        public_history(&owner, &provider_result);
        if case == PublicCase::List {
            fixture.listChanged(&[]).unwrap();
            barrier(&fixture, 1, 1, until);
        } else {
            let _ = fixture.die();
        }
        loop {
            before(until);
            let absent = owner.poll().unwrap().is_none();
            public_history(&owner, &provider_result);
            if absent {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        withdrawn_at = boot_ns();
        assert!(owner.last_association().is_none());
        write_once(
            control,
            "provider-withdrawn.json",
            json!({"event":"data-public-poll-provider-withdrawn","case":case.label(),"test":case.test(),
            "network_namespace":ns_json(ns),"ifindex":ifindex,"lifetime":joined.lifetime,"provider_cid":9,"serial":SERIAL,
            "provider_disposition":"ProviderAccepted","association_present":false,"provider_result":provider_result,
            "original_until_ns":until,"at_boottime_ns":withdrawn_at}),
            until,
        );
        let confirmed = wait_json(control, "provider-loss-confirmed.json", until);
        assert_eq!(public_control_link(&confirmed, ns), ifindex);
        let observed = confirmed["observed_at_ns"].as_u64().unwrap();
        assert!(observed >= withdrawn_at && observed < until);
        confirmed_at = Some(observed);
        assert!(owner.poll().unwrap().is_none());
        public_history(&owner, &provider_result);
        if case == PublicCase::List {
            counts(&fixture.capture().unwrap(), 1, 1);
        }
    }
    // Explicit peer termination is fixture cleanup, not vendor deactivation.
    if case != PublicCase::Death {
        let _ = fixture.die();
    }
    public_history(&owner, &provider_result);
    drop(owner);
    drop(fixture);
    assert_eq!(namespace(), ns);
    write_once(
        control,
        "outcome.json",
        json!({"event":"data-public-poll-outcome","case":case.label(),"test":case.test(),"network_namespace":ns_json(ns),
        "requirement_passed":true,"provider_disposition":"ProviderAccepted","provider_cid":9,"serial":SERIAL,"provider_result":provider_result,
        "association_present":false,"setup_via_public_poll":true,"loss_via_public_poll":true,
        "kernel_loss_withdrawn":case==PublicCase::Link,"provider_loss_from_positive":case!=PublicCase::Link,
        "provider_loss_kernel_confirmed_at_ns":confirmed_at,"withdrawn_at_ns":withdrawn_at,"owner_dropped_before_result":true,
        "original_until_ns":until,"closed_at_ns":boot_ns()}),
        until,
    );
}
#[test]
#[ignore = "requires owned actual ARM Binder/TUN public-poll controller"]
fn actual_public_poll_setup_and_link_loss() {
    actual_public_poll_case(PublicCase::Link);
}
#[test]
#[ignore = "requires owned actual ARM Binder/TUN public-poll controller"]
fn actual_public_poll_empty_list_loss() {
    actual_public_poll_case(PublicCase::List);
}
#[test]
#[ignore = "requires owned actual ARM Binder/TUN public-poll controller"]
fn actual_public_poll_endpoint_loss() {
    actual_public_poll_case(PublicCase::Death);
}
