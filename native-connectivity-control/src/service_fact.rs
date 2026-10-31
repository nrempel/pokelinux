//! Typed private worker observations. Existing links never borrow a create epoch.
use pokelinux_network_state::NamespaceIdentity;
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkIdentity {
    VirtualPair {
        lifetime: u64,
    },
    WirelessOpen {
        wiphy: u32,
        wdev: u64,
        observed_lifetime: Option<u64>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fact {
    pub namespace: NamespaceIdentity,
    pub ifindex: u32,
    pub identity: LinkIdentity,
    pub connected: bool,
}
impl Fact {
    pub(crate) fn same_target(self, other: Self) -> bool {
        self.namespace == other.namespace
            && self.ifindex == other.ifindex
            && self.identity == other.identity
    }
}

pub(crate) fn record(kind: u8, id: u64, fact: Fact, at: u64) -> Vec<u8> {
    let mut b = vec![0; 40];
    b[0] = 2;
    b[1] = kind;
    b[2..10].copy_from_slice(&id.to_le_bytes());
    b[10..18].copy_from_slice(&at.to_le_bytes());
    b[18..26].copy_from_slice(&fact.namespace.dev.to_le_bytes());
    b[26..34].copy_from_slice(&fact.namespace.ino.to_le_bytes());
    b[34..38].copy_from_slice(&fact.ifindex.to_le_bytes());
    b[38] = u8::from(fact.connected);
    match fact.identity {
        LinkIdentity::VirtualPair { lifetime } => {
            b[39] = 1;
            b.extend_from_slice(&lifetime.to_le_bytes());
        }
        LinkIdentity::WirelessOpen {
            wiphy,
            wdev,
            observed_lifetime,
        } => {
            b[39] = 2;
            b.extend_from_slice(&wiphy.to_le_bytes());
            b.extend_from_slice(&wdev.to_le_bytes());
            b.push(u8::from(observed_lifetime.is_some()));
            b.extend_from_slice(&observed_lifetime.unwrap_or(0).to_le_bytes());
        }
    }
    b
}

pub(crate) fn decode_record(b: &[u8]) -> io::Result<(u8, u64, Fact, u64)> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidData);
    if b.len() < 40 || b[0] != 2 || !(1..=4).contains(&b[1]) || b[38] > 1 {
        return Err(invalid());
    }
    let n = |at| u64::from_le_bytes(b[at..at + 8].try_into().unwrap());
    let identity = match (b[39], b.len()) {
        (1, 48) if n(40) != 0 => LinkIdentity::VirtualPair { lifetime: n(40) },
        (2, 61) if n(44) != 0 => {
            let observed_lifetime = match (b[52], n(53)) {
                (0, 0) => None,
                (1, value) if value != 0 => Some(value),
                _ => return Err(invalid()),
            };
            LinkIdentity::WirelessOpen {
                wiphy: u32::from_le_bytes(b[40..44].try_into().unwrap()),
                wdev: n(44),
                observed_lifetime,
            }
        }
        _ => return Err(invalid()),
    };
    let fact = Fact {
        namespace: NamespaceIdentity {
            dev: n(18),
            ino: n(26),
        },
        ifindex: u32::from_le_bytes(b[34..38].try_into().unwrap()),
        identity,
        connected: b[38] != 0,
    };
    if fact.ifindex <= 1 {
        return Err(invalid());
    }
    Ok((b[1], n(2), fact, n(10)))
}
