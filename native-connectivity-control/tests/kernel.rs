//! Actual Linux namespace boundary; intentionally skipped without the owned fixture.
use pokelinux_connectivity_control::kernel::VirtualLink;
use std::{fs::File, os::unix::fs::MetadataExt};
fn until() -> u64 {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(now.tv_sec).unwrap() * 1_000_000_000
        + u64::try_from(now.tv_nsec).unwrap()
        + 1_000_000_000
}
#[test]
#[ignore = "requires two retained fresh private namespaces and exclusive administration"]
fn actual_rust_owned_virtual_link_creation() {
    assert_eq!(
        std::env::var("POKELINUX_CONNECTIVITY_ISOLATED").as_deref(),
        Ok("1")
    );
    let peer = File::open(std::env::var("POKELINUX_CONNECTIVITY_PEER_NS").unwrap()).unwrap();
    let peer_identity = peer.metadata().unwrap();
    let local = File::open("/proc/thread-self/ns/net")
        .unwrap()
        .metadata()
        .unwrap();
    assert_ne!(peer_identity.ino(), local.ino());
    let link = VirtualLink::create(peer, until()).expect("Rust must create the owned virtual pair");
    let fact = link
        .fact()
        .expect("creation must retain truthful local state");
    assert_eq!(
        (fact.namespace.dev, fact.namespace.ino),
        (local.dev(), local.ino())
    );
    assert!(fact.ifindex > 1 && fact.lifetime > 0);
    assert!(!fact.connected, "creation grants no IPv4 connection");
    println!(
        "OWNED_VIRTUAL_CREATED namespace={} ifindex={} lifetime={}",
        fact.namespace.ino, fact.ifindex, fact.lifetime
    );
}
