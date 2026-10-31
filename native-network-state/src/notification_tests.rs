//! Authored synthetic route notifications, not captured kernel-event evidence.
use super::*;

const IFINDEX: u32 = 42;

fn observer(epoch: Option<u64>, synchronized: bool) -> KernelObserver {
    // Notification bookkeeping never accesses either descriptor in these tests.
    KernelObserver {
        _retained_namespace: File::open("/dev/null").unwrap(),
        namespace: NamespaceIdentity { dev: 0, ino: 0 },
        socket: File::open("/dev/null").unwrap().into(),
        port: 0,
        sequence: 0,
        revision: 7,
        next_epoch: 41,
        synchronized,
        lifetimes: BTreeMap::from([(
            IFINDEX,
            Lifetime {
                name: "wlan0".to_owned(),
                epoch,
            },
        )]),
        terminal: None,
    }
}

fn attribute(body: &mut Vec<u8>, kind: u16, value: &[u8]) {
    body.extend_from_slice(&u16::try_from(value.len() + 4).unwrap().to_ne_bytes());
    body.extend_from_slice(&kind.to_ne_bytes());
    body.extend_from_slice(value);
    body.resize((body.len() + 3) & !3, 0);
}

fn link_body(ifindex: u32, name: &[u8], mtu: Option<u32>) -> Vec<u8> {
    let mut body = vec![0; 16];
    body[4..8].copy_from_slice(&ifindex.to_ne_bytes());
    body[8..12].copy_from_slice(&0x41_u32.to_ne_bytes());
    attribute(&mut body, 3, name);
    if let Some(mtu) = mtu {
        attribute(&mut body, 4, &mtu.to_ne_bytes());
    }
    body
}

fn notification(kind: u16, body: Vec<u8>) -> Message {
    Message {
        kind,
        flags: 0,
        sequence: 81,
        multicast_group: 1,
        body,
    }
}

fn sparse_wireless_body(ifindex: u32) -> Vec<u8> {
    let mut body = link_body(ifindex, b"wlan0\0", None);
    // IFLA_WIRELESS is opaque to this observer; the framing must remain valid.
    attribute(&mut body, 11, &[8, 0, 0x15, 0x8b, 0, 0, 0, 0]);
    body
}

fn preserves_lifetime(epoch: Option<u64>) {
    let mut observer = observer(epoch, true);
    assert_eq!(
        observer.notification(&notification(NEW_LINK, sparse_wireless_body(IFINDEX))),
        Ok(()),
        "synthetic existing-link wireless notification does not contain IFLA_MTU"
    );
    assert_eq!(observer.revision, 8);
    assert_eq!(observer.next_epoch, 41);
    assert_eq!(observer.lifetimes.len(), 1);
    assert_eq!(observer.lifetimes[&IFINDEX].epoch, epoch);
    assert_eq!(observer.lifetimes[&IFINDEX].name, "wlan0");
    assert!(observer.synchronized);
    assert!(observer.terminal.is_none());
}

#[test]
fn synthetic_sparse_update_preserves_preexisting_lifetime() {
    preserves_lifetime(None);
}

#[test]
fn synthetic_sparse_update_preserves_witnessed_lifetime() {
    preserves_lifetime(Some(41));
}

#[test]
fn synthetic_unknown_sparse_update_loses_evidence_without_creating_lifetime() {
    for synchronized in [false, true] {
        let mut observer = observer(Some(41), synchronized);
        assert_eq!(
            observer.notification(&notification(NEW_LINK, sparse_wireless_body(IFINDEX + 1))),
            Err(ObserveError::EvidenceLost)
        );
        assert_eq!(observer.revision, 7);
        assert_eq!(observer.next_epoch, 41);
        assert_eq!(observer.lifetimes.len(), 1);
        assert_eq!(observer.lifetimes[&IFINDEX].epoch, Some(41));
    }
}

#[test]
fn synthetic_dump_still_requires_mtu_and_preserves_snapshot_fields() {
    assert_eq!(
        parse_link(&sparse_wireless_body(IFINDEX)),
        Err(ObserveError::InvalidData)
    );
    let mut body = link_body(IFINDEX, b"wlan0\0", Some(1400));
    attribute(&mut body, 16, &[6]);
    assert_eq!(
        parse_link(&body),
        Ok(LinkSnapshot {
            ifindex: IFINDEX,
            name: "wlan0".to_owned(),
            observed_lifetime: None,
            flags: 0x41,
            mtu: 1400,
            operstate: Some(6),
            addresses: Vec::new(),
        })
    );
}

#[test]
fn synthetic_malformed_or_duplicate_known_attributes_remain_invalid() {
    let mut malformed = vec![vec![0; 15], link_body(0, b"wlan0\0", None)];
    malformed.push(link_body(u32::MAX, b"wlan0\0", None));
    malformed.push(link_body(IFINDEX, b"wlan0", None));
    malformed.push(link_body(IFINDEX, b"\0", None));
    malformed.push(link_body(IFINDEX, b"wl\0an0\0", None));
    malformed.push(link_body(IFINDEX, b"wl\xffan0\0", None));
    malformed.push(link_body(IFINDEX, b"abcdefghijklmnop\0", None));
    let mut missing_name = vec![0; 16];
    missing_name[4..8].copy_from_slice(&IFINDEX.to_ne_bytes());
    malformed.push(missing_name);
    for (kind, value) in [(4, vec![0; 3]), (16, vec![0; 2])] {
        let mut body = sparse_wireless_body(IFINDEX);
        attribute(&mut body, kind, &value);
        malformed.push(body);
    }
    for (kind, value) in [(3, b"wlan0\0".to_vec()), (4, vec![0; 4]), (16, vec![6])] {
        let mut body = sparse_wireless_body(IFINDEX);
        if kind != 3 {
            attribute(&mut body, kind, &value);
        }
        attribute(&mut body, kind, &value);
        malformed.push(body);
    }
    for suffix in [vec![3, 0, 11, 0], vec![8, 0, 11, 0], vec![1]] {
        let mut body = sparse_wireless_body(IFINDEX);
        body.extend_from_slice(&suffix);
        malformed.push(body);
    }
    for (case, body) in malformed.into_iter().enumerate() {
        assert_eq!(
            parse_link(&body),
            Err(ObserveError::InvalidData),
            "dump case {case}"
        );
        for kind in [NEW_LINK, DEL_LINK] {
            let mut observer = observer(Some(41), true);
            assert_eq!(
                observer.notification(&notification(kind, body.clone())),
                Err(ObserveError::InvalidData),
                "synthetic kind {kind} case {case}"
            );
            assert_eq!(observer.revision, 7);
            assert_eq!(observer.next_epoch, 41);
            assert_eq!(observer.lifetimes[&IFINDEX].epoch, Some(41));
        }
    }
}

#[test]
fn synthetic_notification_still_requires_link_multicast_group() {
    let mut observer = observer(None, true);
    let mut message = notification(NEW_LINK, sparse_wireless_body(IFINDEX));
    message.multicast_group = 0x10;
    assert_eq!(
        observer.notification(&message),
        Err(ObserveError::InvalidData)
    );
    assert_eq!(observer.revision, 7);
    assert_eq!(observer.lifetimes[&IFINDEX].epoch, None);
}

#[test]
fn synthetic_full_delete_recreate_starts_a_distinct_lifetime() {
    let mut observer = observer(Some(41), true);
    let full = link_body(IFINDEX, b"wlan0\0", Some(1400));
    assert_eq!(
        observer.notification(&notification(DEL_LINK, full.clone())),
        Ok(())
    );
    assert!(!observer.lifetimes.contains_key(&IFINDEX));
    assert_eq!(observer.notification(&notification(NEW_LINK, full)), Ok(()));
    assert_eq!(observer.lifetimes[&IFINDEX].epoch, Some(42));
    assert_eq!(observer.next_epoch, 42);
    assert_eq!(observer.revision, 9);
}

#[test]
fn synthetic_sparse_delete_retires_known_lifetime() {
    let mut observer = observer(Some(41), true);
    assert_eq!(
        observer.notification(&notification(DEL_LINK, sparse_wireless_body(IFINDEX))),
        Ok(())
    );
    assert!(observer.lifetimes.is_empty());
    assert_eq!(observer.next_epoch, 41);
    assert_eq!(observer.revision, 8);
}
