//! One explicitly authorized add-PDN attempt and opaque owned-role protocol.
//! Intent/sync/transport observations never mean modem success or safe retry.
use pokelinux_process_owner::{
    bootstrap::BoundParent,
    deadline::{BootClock, Clock, Deadline},
    supervisor::ProcessIdentity,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    net::{Ipv4Addr, Ipv6Addr},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::Duration,
};
const MAX_PACKET: usize = 8192;
const MAX_CONFIG: usize = 16384;
const MAX_CANONICAL: usize = 4096;
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn check(ok: bool) -> io::Result<()> {
    if ok { Ok(()) } else { Err(invalid()) }
}
fn positive(n: u64) -> bool {
    n > 0 && n < i64::MAX as u64
}
fn boottime() -> u64 {
    BootClock
        .now()
        .as_nanos()
        .try_into()
        .expect("Linux BOOTTIME fits u64")
}
pub fn hex(bytes: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(D[(b >> 4) as usize] as char);
        s.push(D[(b & 15) as usize] as char);
    }
    s
}
pub fn parse_hex<const N: usize>(value: &str) -> io::Result<[u8; N]> {
    check(value.len() == N * 2)?;
    let mut out = [0; N];
    for (dst, pair) in out
        .iter_mut()
        .zip(value.as_bytes().as_chunks::<2>().0.iter())
    {
        let digit = |x: u8| -> io::Result<u8> {
            match x {
                b'0'..=b'9' => Ok(x - b'0'),
                b'a'..=b'f' => Ok(x - b'a' + 10),
                _ => Err(invalid()),
            }
        };
        *dst = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Ok(out)
}
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Admit {
    pub schema: u8,
    pub incarnation: String,
    pub attempt: u64,
    pub serial: i32,
    pub request: i32,
    pub slot: i32,
    pub payload_sha256: String,
    pub deadline_ns: u64,
}
impl Admit {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.schema == 1
                && self.attempt == 1
                && self.serial == 1
                && self.request == 105
                && self.slot == 1
                && positive(self.deadline_ns),
        )?;
        parse_hex::<16>(&self.incarnation)?;
        parse_hex::<32>(&self.payload_sha256)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmitReply {
    pub schema: u8,
    pub admitted: bool,
    pub reason: String,
    pub incarnation: String,
    pub attempt: u64,
    pub request_revision: u64,
}
impl AdmitReply {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.schema == 1
                && self.attempt == 1
                && self.request_revision < u64::MAX
                && (!self.admitted || self.request_revision > 0)
                && !self.reason.is_empty()
                && self.reason.len() <= 96
                && self
                    .reason
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'-'),
        )?;
        parse_hex::<16>(&self.incarnation)?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestOutcome {
    NotSent,
    OutcomeUnknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryResponse {
    pub vendor_error: i32,
    pub payload_hex: String,
    pub ingress_sequence: u64,
    pub received_at_ns: u64,
    pub publication_revision: u64,
}
impl HistoryResponse {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.payload_hex.len() <= 8192
                && self.payload_hex.len().is_multiple_of(2)
                && self
                    .payload_hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && positive(self.ingress_sequence)
                && positive(self.received_at_ns)
                && positive(self.publication_revision),
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestHistory {
    pub request_revision: u64,
    pub intent: String,
    pub incarnation: String,
    pub attempt: u64,
    pub serial: i32,
    pub client_id: i32,
    pub request: i32,
    pub slot: i32,
    pub payload_sha256: String,
    pub deadline_ns: u64,
    pub outcome: RequestOutcome,
    #[serde(deserialize_with = "required_option")]
    pub response: Option<HistoryResponse>,
}
impl RequestHistory {
    pub fn validate(&self) -> io::Result<()> {
        check(
            positive(self.request_revision)
                && self.attempt == 1
                && self.serial == 1
                && self.client_id >= 0
                && self.request == 105
                && self.slot == 1
                && positive(self.deadline_ns),
        )?;
        parse_hex::<16>(&self.intent)?;
        parse_hex::<16>(&self.incarnation)?;
        parse_hex::<32>(&self.payload_sha256)?;
        if let Some(r) = &self.response {
            r.validate()?;
            check(
                self.outcome == RequestOutcome::OutcomeUnknown
                    && r.received_at_ns < self.deadline_ns,
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub incarnation: ProcessIdentity,
    pub intent: [u8; 16],
    pub deadline_ns: u64,
    pub client_id: i32,
    pub payload_sha256: [u8; 32],
    pub payload_length: u32,
}
impl Operation {
    pub fn validate(&self) -> io::Result<()> {
        check(
            positive(self.deadline_ns)
                && self.client_id >= 0
                && self.payload_length > 0
                && self.payload_length as usize <= omarchy_ims::add_pdn::MAX_ENCODED_BYTES,
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: u8,
    add_pdn: OwnedAddPdn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CivilTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    utc_offset_minutes: i16,
}
impl CivilTime {
    fn wire(self) -> omarchy_ims::add_pdn::WireTime {
        omarchy_ims::add_pdn::WireTime {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            utc_offset_minutes: self.utc_offset_minutes,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedAddPdn {
    pdn_type_wire: u8,
    ip_type_wire: u8,
    pdn_ipv4: Ipv4Addr,
    pdn_ipv6: Ipv6Addr,
    hd_enabled_wire: u8,
    operator_id_wire: u8,
    registration_style_wire: u8,
    new_pdn_wire: u8,
    volte_auto_config_wire: u8,
    rcs_auto_config_wire: u8,
    offering_type_wire: u16,
    registration_type_wire: u8,
    init_type_wire: u8,
    ims_net_type_wire: u8,
    #[serde(deserialize_with = "required_option")]
    mac: Option<[u8; 6]>,
    pcscf_ipv4: Vec<Ipv4Addr>,
    pcscf_ipv6: Vec<Ipv6Addr>,
    primary_dns_ipv4: Ipv4Addr,
    primary_dns_ipv6: Ipv6Addr,
    secondary_dns_ipv4: Ipv4Addr,
    secondary_dns_ipv6: Ipv6Addr,
    last_access_network_wire: u8,
    #[serde(deserialize_with = "required_option")]
    last_access_time: Option<CivilTime>,
    #[serde(deserialize_with = "required_option")]
    time: Option<CivilTime>,
    tac_or_lac_wire: u32,
    cell_id32_wire: u32,
    remaining_timer_wire: u32,
    #[serde(deserialize_with = "required_option")]
    mcc_mnc: Option<String>,
    #[serde(deserialize_with = "required_option")]
    country: Option<String>,
    mtu_wire: u16,
    rcs_capability_wire: u32,
    vendor_capability_wire: u16,
    extended_rcs_capability_wire: u32,
    #[serde(deserialize_with = "required_option")]
    e911_aid: Option<String>,
    band_wire: u32,
    cell_id64_wire: u64,
    network_interface_index_wire: u32,
    sip_delegate_capability_wire: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalConfig {
    config: Box<Config>,
    canonical: Vec<u8>,
    digest: [u8; 32],
    source: [u8; 32],
}
impl CanonicalConfig {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        check(!bytes.is_empty() && bytes.len() <= MAX_CONFIG)?;
        let config: Config = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        check(config.schema == 1)?;
        let canonical = serde_json::to_vec(&config).map_err(|_| invalid())?;
        check(canonical.len() <= MAX_CANONICAL)?;
        let mut owned = Self {
            config: Box::new(config),
            canonical,
            digest: [0; 32],
            source: Sha256::digest(bytes).into(),
        };
        owned.digest = Sha256::digest(owned.payload()?).into();
        Ok(owned)
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub fn source_sha256(&self) -> [u8; 32] {
        self.source
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn payload(&self) -> io::Result<Vec<u8>> {
        omarchy_ims::add_pdn::encode(&self.as_add_pdn()).map_err(|_| invalid())
    }
    pub fn as_add_pdn(&self) -> omarchy_ims::add_pdn::AddPdn<'_> {
        let c = &self.config.add_pdn;
        omarchy_ims::add_pdn::AddPdn {
            pdn_type_wire: c.pdn_type_wire,
            ip_type_wire: c.ip_type_wire,
            pdn_ipv4: c.pdn_ipv4,
            pdn_ipv6: c.pdn_ipv6,
            hd_enabled_wire: c.hd_enabled_wire,
            operator_id_wire: c.operator_id_wire,
            registration_style_wire: c.registration_style_wire,
            new_pdn_wire: c.new_pdn_wire,
            volte_auto_config_wire: c.volte_auto_config_wire,
            rcs_auto_config_wire: c.rcs_auto_config_wire,
            offering_type_wire: c.offering_type_wire,
            registration_type_wire: c.registration_type_wire,
            init_type_wire: c.init_type_wire,
            ims_net_type_wire: c.ims_net_type_wire,
            mac: c.mac,
            pcscf_ipv4: &c.pcscf_ipv4,
            pcscf_ipv6: &c.pcscf_ipv6,
            primary_dns_ipv4: c.primary_dns_ipv4,
            primary_dns_ipv6: c.primary_dns_ipv6,
            secondary_dns_ipv4: c.secondary_dns_ipv4,
            secondary_dns_ipv6: c.secondary_dns_ipv6,
            last_access_network_wire: c.last_access_network_wire,
            last_access_time: c.last_access_time.map(CivilTime::wire),
            time: c.time.map(CivilTime::wire),
            tac_or_lac_wire: c.tac_or_lac_wire,
            cell_id32_wire: c.cell_id32_wire,
            remaining_timer_wire: c.remaining_timer_wire,
            mcc_mnc: c.mcc_mnc.as_deref(),
            country: c.country.as_deref(),
            mtu_wire: c.mtu_wire,
            rcs_capability_wire: c.rcs_capability_wire,
            vendor_capability_wire: c.vendor_capability_wire,
            extended_rcs_capability_wire: c.extended_rcs_capability_wire,
            e911_aid: c.e911_aid.as_deref(),
            band_wire: c.band_wire,
            cell_id64_wire: c.cell_id64_wire,
            network_interface_index_wire: c.network_interface_index_wire,
            sip_delegate_capability_wire: c.sip_delegate_capability_wire,
        }
    }
}
/// Startup only. The fixed path and root/service-GID ownership grant no dispatch.
pub fn load_config() -> io::Result<CanonicalConfig> {
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(
            (rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC)
                .bits() as i32,
        )
        .open("/etc/pokelinux")?;
    let md = parent.metadata()?;
    check(
        md.is_dir()
            && md.uid() == 0
            && md.gid() == rustix::process::getegid().as_raw()
            && md.mode() & 0o7777 == 0o750,
    )?;
    let fd = rustix::fs::openat(
        &parent,
        "radio-add-pdn.json",
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?;
    let mut file = File::from(fd);
    let before = file.metadata()?;
    check(
        before.is_file()
            && before.uid() == 0
            && before.gid() == md.gid()
            && before.mode() & 0o7777 == 0o640
            && before.len() > 0
            && before.len() <= MAX_CONFIG as u64,
    )?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_CONFIG as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    check(
        bytes.len() as u64 == before.len()
            && after.len() == before.len()
            && after.dev() == before.dev()
            && after.ino() == before.ino()
            && after.mode() == before.mode()
            && after.uid() == before.uid()
            && after.gid() == before.gid()
            && after.mtime() == before.mtime()
            && after.mtime_nsec() == before.mtime_nsec(),
    )?;
    CanonicalConfig::from_bytes(&bytes)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncReceipt {
    pub completed_at_ns: u64,
    pub device: u64,
    pub inode: u64,
    pub uid: u32,
    pub mode: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParentMessage {
    Write {
        operation: Operation,
        writer: ProcessIdentity,
    },
    Finish {
        operation: Operation,
        writer: ProcessIdentity,
    },
    Dispatch {
        operation: Operation,
        config: CanonicalConfig,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildMessage {
    Synced {
        operation: Operation,
        writer: ProcessIdentity,
        receipt: SyncReceipt,
    },
    Request {
        operation: Operation,
        revision: u64,
        outcome: RequestOutcome,
        response: Option<HistoryResponse>,
    },
}
fn header(tag: u8) -> Vec<u8> {
    let mut b = Vec::with_capacity(256);
    b.extend_from_slice(b"RADO");
    b.extend_from_slice(&[1, tag, 0, 0]);
    b.extend_from_slice(&[0; 4]);
    b
}
fn finish(mut b: Vec<u8>) -> io::Result<Vec<u8>> {
    check(b.len() <= MAX_PACKET)?;
    let n = b.len() as u32;
    b[8..12].copy_from_slice(&n.to_le_bytes());
    Ok(b)
}
fn put_op(b: &mut Vec<u8>, o: &Operation) -> io::Result<()> {
    o.validate()?;
    b.extend_from_slice(o.incarnation.as_bytes());
    b.extend_from_slice(&o.intent);
    b.extend_from_slice(&o.deadline_ns.to_le_bytes());
    b.extend_from_slice(&o.client_id.to_le_bytes());
    b.extend_from_slice(&o.payload_sha256);
    b.extend_from_slice(&o.payload_length.to_le_bytes());
    b.extend_from_slice(&1u64.to_le_bytes());
    for n in [1i32, 105, 1] {
        b.extend_from_slice(&n.to_le_bytes());
    }
    Ok(())
}
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let end = self.at.checked_add(n).ok_or_else(invalid)?;
        let b = self.bytes.get(self.at..end).ok_or_else(invalid)?;
        self.at = end;
        Ok(b)
    }
    fn arr<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| invalid())
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.arr()?))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.arr()?))
    }
    fn i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_le_bytes(self.arr()?))
    }
    fn done(&self) -> io::Result<()> {
        check(self.at == self.bytes.len())
    }
    fn op(&mut self) -> io::Result<Operation> {
        let o = Operation {
            incarnation: ProcessIdentity::from_bytes(self.arr()?),
            intent: self.arr()?,
            deadline_ns: self.u64()?,
            client_id: self.i32()?,
            payload_sha256: self.arr()?,
            payload_length: self.u32()?,
        };
        check(self.u64()? == 1 && self.i32()? == 1 && self.i32()? == 105 && self.i32()? == 1)?;
        o.validate()?;
        Ok(o)
    }
}
fn start(bytes: &[u8]) -> io::Result<(u8, Cursor<'_>)> {
    check((12..=MAX_PACKET).contains(&bytes.len()))?;
    check(
        &bytes[..4] == b"RADO"
            && bytes[4] == 1
            && bytes[6..8] == [0, 0]
            && u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid())?) as usize
                == bytes.len(),
    )?;
    Ok((bytes[5], Cursor { bytes, at: 12 }))
}
impl ParentMessage {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut b = header(match self {
            Self::Write { .. } => 1,
            Self::Finish { .. } => 2,
            Self::Dispatch { .. } => 3,
        });
        match self {
            Self::Write { operation, writer } | Self::Finish { operation, writer } => {
                put_op(&mut b, operation)?;
                b.extend_from_slice(writer.as_bytes());
            }
            Self::Dispatch { operation, config } => {
                put_op(&mut b, operation)?;
                check(
                    config.digest() == operation.payload_sha256
                        && config.payload()?.len() == operation.payload_length as usize,
                )?;
                b.extend_from_slice(&(config.canonical.len() as u32).to_le_bytes());
                b.extend_from_slice(&config.canonical);
            }
        }
        finish(b)
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        let (tag, mut c) = start(bytes)?;
        let operation = c.op()?;
        let m = match tag {
            1 => Self::Write {
                operation,
                writer: ProcessIdentity::from_bytes(c.arr()?),
            },
            2 => Self::Finish {
                operation,
                writer: ProcessIdentity::from_bytes(c.arr()?),
            },
            3 => {
                let n = c.u32()? as usize;
                check(n <= MAX_CANONICAL)?;
                let raw = c.take(n)?;
                let config = CanonicalConfig::from_bytes(raw)?;
                check(
                    raw == config.canonical_bytes()
                        && config.digest() == operation.payload_sha256
                        && config.payload()?.len() == operation.payload_length as usize,
                )?;
                Self::Dispatch { operation, config }
            }
            _ => return Err(invalid()),
        };
        c.done()?;
        Ok(m)
    }
}
impl ChildMessage {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut b = header(match self {
            Self::Synced { .. } => 17,
            Self::Request { .. } => 18,
        });
        match self {
            Self::Synced {
                operation,
                writer,
                receipt,
            } => {
                put_op(&mut b, operation)?;
                check(
                    positive(receipt.completed_at_ns)
                        && receipt.completed_at_ns < operation.deadline_ns
                        && receipt.inode > 0
                        && receipt.mode & 0o170000 == 0o100000
                        && receipt.mode & 0o7777 == 0o600,
                )?;
                b.extend_from_slice(writer.as_bytes());
                for n in [receipt.completed_at_ns, receipt.device, receipt.inode] {
                    b.extend_from_slice(&n.to_le_bytes());
                }
                b.extend_from_slice(&receipt.uid.to_le_bytes());
                b.extend_from_slice(&receipt.mode.to_le_bytes());
            }
            Self::Request {
                operation,
                revision,
                outcome,
                response,
            } => {
                put_op(&mut b, operation)?;
                check(positive(*revision))?;
                b.extend_from_slice(&revision.to_le_bytes());
                b.extend_from_slice(&[
                    match outcome {
                        RequestOutcome::OutcomeUnknown => 1,
                        RequestOutcome::NotSent => 2,
                    },
                    u8::from(response.is_some()),
                    0,
                    0,
                ]);
                if let Some(r) = response {
                    r.validate()?;
                    check(
                        *outcome == RequestOutcome::OutcomeUnknown
                            && r.received_at_ns < operation.deadline_ns
                            && r.publication_revision <= *revision,
                    )?;
                    b.extend_from_slice(&r.vendor_error.to_le_bytes());
                    let payload = decode_payload(&r.payload_hex)?;
                    b.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                    for n in [r.ingress_sequence, r.received_at_ns, r.publication_revision] {
                        b.extend_from_slice(&n.to_le_bytes());
                    }
                    b.extend_from_slice(&payload);
                }
            }
        }
        finish(b)
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        let (tag, mut c) = start(bytes)?;
        let operation = c.op()?;
        let m = match tag {
            17 => Self::Synced {
                operation,
                writer: ProcessIdentity::from_bytes(c.arr()?),
                receipt: SyncReceipt {
                    completed_at_ns: c.u64()?,
                    device: c.u64()?,
                    inode: c.u64()?,
                    uid: c.u32()?,
                    mode: c.u32()?,
                },
            },
            18 => {
                let revision = c.u64()?;
                let outcome = match c.take(1)?[0] {
                    1 => RequestOutcome::OutcomeUnknown,
                    2 => RequestOutcome::NotSent,
                    _ => return Err(invalid()),
                };
                let present = c.take(1)?[0];
                check(present <= 1 && c.take(2)? == [0, 0])?;
                let response = if present == 1 {
                    let vendor_error = c.i32()?;
                    let n = c.u32()? as usize;
                    check(n <= 4096)?;
                    let ingress_sequence = c.u64()?;
                    let received_at_ns = c.u64()?;
                    let publication_revision = c.u64()?;
                    Some(HistoryResponse {
                        vendor_error,
                        payload_hex: hex(c.take(n)?),
                        ingress_sequence,
                        received_at_ns,
                        publication_revision,
                    })
                } else {
                    None
                };
                Self::Request {
                    operation,
                    revision,
                    outcome,
                    response,
                }
            }
            _ => return Err(invalid()),
        };
        c.done()?;
        m.encode()?;
        Ok(m)
    }
}
fn decode_payload(s: &str) -> io::Result<Vec<u8>> {
    check(s.len() <= 8192 && s.len().is_multiple_of(2))?;
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| {
            let s = std::str::from_utf8(x).map_err(|_| invalid())?;
            Ok(parse_hex::<1>(s)?[0])
        })
        .collect()
}
pub fn intent_record(o: &Operation, writer: ProcessIdentity) -> io::Result<Vec<u8>> {
    o.validate()?;
    Ok(format!("pokelinux-add-pdn-intent-v1\nradio={}\nwriter={}\nintent={}\nattempt=1\nserial=1\nclient_id={}\nrequest=105\nslot=1\npayload_length={}\npayload_sha256={}\ndeadline_ns={}\n",o.incarnation.hex(),writer.hex(),hex(&o.intent),o.client_id,o.payload_length,hex(&o.payload_sha256),o.deadline_ns).into_bytes())
}
fn writer_before(session: Deadline, operation: Option<&Operation>) -> io::Result<()> {
    let n = BootClock.now();
    if session.elapsed(n)
        || operation.is_some_and(|o| Deadline::at(Duration::from_nanos(o.deadline_ns)).elapsed(n))
    {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
/// Finite prewarmed role, before Binder initialization. A partial marker survives
/// every error. Only matching Finish can yield normal0; ACK is not a native wait.
pub fn run_intent_writer(
    journal: &Path,
    session: Deadline,
    parent: &mut BoundParent,
) -> io::Result<()> {
    loop {
        writer_before(session, None)?;
        if let Some(bytes) = parent.receive_message(session)? {
            let ParentMessage::Write { operation, writer } = ParentMessage::decode(&bytes)? else {
                return Err(invalid());
            };
            check(writer == parent.identity())?;
            writer_before(session, Some(&operation))?;
            let dirpath = journal.parent().ok_or_else(invalid)?;
            let name = journal.file_name().ok_or_else(invalid)?;
            check(journal.is_absolute() && name.as_encoded_bytes().ends_with(b".add-pdn"))?;
            let dir = OpenOptions::new()
                .read(true)
                .custom_flags(
                    (rustix::fs::OFlags::DIRECTORY
                        | rustix::fs::OFlags::NOFOLLOW
                        | rustix::fs::OFlags::CLOEXEC)
                        .bits() as i32,
                )
                .open(dirpath)?;
            let md = dir.metadata()?;
            check(
                md.is_dir()
                    && md.uid() == rustix::process::geteuid().as_raw()
                    && md.mode() & 0o7777 == 0o700,
            )?;
            let record = intent_record(&operation, writer)?;
            writer_before(session, Some(&operation))?;
            let fd = rustix::fs::openat(
                &dir,
                name,
                rustix::fs::OFlags::WRONLY
                    | rustix::fs::OFlags::CREATE
                    | rustix::fs::OFlags::EXCL
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            )?;
            let mut file = File::from(fd);
            let md = file.metadata()?;
            check(
                md.is_file()
                    && md.uid() == rustix::process::geteuid().as_raw()
                    && md.mode() & 0o7777 == 0o600,
            )?;
            file.write_all(&record)?;
            writer_before(session, Some(&operation))?;
            file.sync_all()?;
            writer_before(session, Some(&operation))?;
            dir.sync_all()?;
            writer_before(session, Some(&operation))?;
            let completed_at_ns = boottime();
            let md = file.metadata()?;
            check(
                md.len() == record.len() as u64
                    && md.is_file()
                    && md.mode() & 0o7777 == 0o600
                    && md.uid() == rustix::process::geteuid().as_raw(),
            )?;
            let receipt = SyncReceipt {
                completed_at_ns,
                device: md.dev(),
                inode: md.ino(),
                uid: md.uid(),
                mode: md.mode(),
            };
            let until = Deadline::at(Duration::from_nanos(operation.deadline_ns));
            writer_before(session, Some(&operation))?;
            parent.send_message(
                until,
                &ChildMessage::Synced {
                    operation: operation.clone(),
                    writer,
                    receipt,
                }
                .encode()?,
            )?;
            loop {
                writer_before(session, Some(&operation))?;
                if let Some(bytes) = parent.receive_message(until)? {
                    let ParentMessage::Finish {
                        operation: got,
                        writer: who,
                    } = ParentMessage::decode(&bytes)?
                    else {
                        return Err(invalid());
                    };
                    check(got == operation && who == writer)?;
                    writer_before(session, Some(&operation))?;
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
