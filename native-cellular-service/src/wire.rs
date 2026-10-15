//! Bounded local observations, never a command or connectivity authority.
use pokelinux_cellular_boundary::SetupDataCallResult;
use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
    LinkAddress::LinkAddress as BinderAddress, Qos::Qos, SliceInfo::SliceInfo as BinderSlice,
    TrafficDescriptor::TrafficDescriptor as BinderDescriptor,
};
use pokelinux_cellular_owner::{DataOwner, Disposition as OwnerDisposition};
use pokelinux_process_owner::supervisor::ProcessIdentity;
use serde::{Deserialize, Deserializer, Serialize};
use std::io::{self, Write};

pub const MAX_RECORD_BYTES: usize = 4096;
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn text(value: &str, max: usize) -> bool {
    value.len() <= max && !value.contains('\0')
}
// A missing nullable field must be rejected; an explicitly present null is valid.
pub(crate) fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub schema: u8,
    pub incarnation: String,
    pub attempt: u64,
    pub revision: u64,
    pub serial: i32,
    pub until_ns: u64,
    pub sampled_at_ns: u64,
    pub disposition: Disposition,
    #[serde(deserialize_with = "required_nullable")]
    pub provider_report: Option<Report>,
    #[serde(deserialize_with = "required_nullable")]
    pub association: Option<Association>,
    pub observer_initial_sync_completed: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub fault: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Disposition {
    NotSubmitted,
    OutcomeUnknown,
    ProviderAccepted { cid: i32 },
    ProviderRejected { radio_error: i32, cause: i32 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
    pub dev: u64,
    pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Association {
    pub namespace: Namespace,
    pub ifindex: u32,
    pub lifetime: u64,
    pub revision: u64,
    pub observed_at_boottime_ns: u64,
}
/// All 18 generated result fields, within the owner's current bounded subset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub cause: i32,
    pub suggested_retry_time: i64,
    pub cid: i32,
    pub active: i32,
    pub r#type: i32,
    pub ifname: String,
    pub addresses: Vec<LinkAddress>,
    pub dnses: Vec<String>,
    pub gateways: Vec<String>,
    pub pcscf: Vec<String>,
    pub mtu_v4: i32,
    pub mtu_v6: i32,
    pub default_qos: QosNoinit,
    pub qos_sessions: Vec<()>,
    pub handover_failure_mode: i8,
    pub pdu_session_id: i32,
    #[serde(deserialize_with = "required_nullable")]
    pub slice_info: Option<SliceInfo>,
    pub traffic_descriptors: Vec<TrafficDescriptor>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkAddress {
    pub address: String,
    pub address_properties: i32,
    pub deprecation_time: i64,
    pub expiration_time: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SliceInfo {
    pub slice_service_type: i8,
    pub slice_differentiator: i32,
    pub mapped_hplmn_sst: i8,
    pub mapped_hplmn_sd: i32,
    pub status: i8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QosNoinit {
    pub noinit: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrafficDescriptor {
    #[serde(deserialize_with = "required_nullable")]
    pub dnn: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub os_app_id: Option<OsAppId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OsAppId {
    pub os_app_id: Vec<u8>,
}
impl From<&BinderAddress> for LinkAddress {
    fn from(value: &BinderAddress) -> Self {
        Self {
            address: value.address.clone(),
            address_properties: value.addressProperties,
            deprecation_time: value.deprecationTime,
            expiration_time: value.expirationTime,
        }
    }
}
impl From<&BinderSlice> for SliceInfo {
    fn from(value: &BinderSlice) -> Self {
        Self {
            slice_service_type: value.sliceServiceType,
            slice_differentiator: value.sliceDifferentiator,
            mapped_hplmn_sst: value.mappedHplmnSst,
            mapped_hplmn_sd: value.mappedHplmnSd,
            status: value.status,
        }
    }
}
impl From<&BinderDescriptor> for TrafficDescriptor {
    fn from(value: &BinderDescriptor) -> Self {
        Self {
            dnn: value.dnn.clone(),
            os_app_id: value.osAppId.as_ref().map(|value| OsAppId {
                os_app_id: value.osAppId.clone(),
            }),
        }
    }
}
impl Report {
    pub fn from_generated(value: &SetupDataCallResult) -> io::Result<Self> {
        // Check before copying variable-length generated data.
        if !text(&value.ifname, 15)
            || value.addresses.len() > 4
            || value
                .addresses
                .iter()
                .any(|value| !text(&value.address, 128))
            || [&value.dnses, &value.gateways, &value.pcscf]
                .iter()
                .any(|values| values.len() > 4 || values.iter().any(|value| !text(value, 128)))
            || !value.qosSessions.is_empty()
            || value.trafficDescriptors.len() > 1
            || value.trafficDescriptors.iter().any(|value| {
                value.dnn.as_ref().is_some_and(|value| !text(value, 128))
                    || value
                        .osAppId
                        .as_ref()
                        .is_some_and(|value| value.osAppId.len() > 256)
            })
        {
            return Err(invalid());
        }
        let Qos::Noinit(noinit) = &value.defaultQos else {
            return Err(invalid());
        };
        Ok(Self {
            cause: value.cause.0,
            suggested_retry_time: value.suggestedRetryTime,
            cid: value.cid,
            active: value.active,
            r#type: value.r#type.0,
            ifname: value.ifname.clone(),
            addresses: value.addresses.iter().map(LinkAddress::from).collect(),
            dnses: value.dnses.clone(),
            gateways: value.gateways.clone(),
            pcscf: value.pcscf.clone(),
            mtu_v4: value.mtuV4,
            mtu_v6: value.mtuV6,
            default_qos: QosNoinit { noinit: *noinit },
            qos_sessions: Vec::new(),
            handover_failure_mode: value.handoverFailureMode,
            pdu_session_id: value.pduSessionId,
            slice_info: value.sliceInfo.as_ref().map(SliceInfo::from),
            traffic_descriptors: value
                .trafficDescriptors
                .iter()
                .map(TrafficDescriptor::from)
                .collect(),
        })
    }
    fn validate(&self) -> io::Result<()> {
        if !text(&self.ifname, 15)
            || self.addresses.len() > 4
            || self
                .addresses
                .iter()
                .any(|value| !text(&value.address, 128))
            || [&self.dnses, &self.gateways, &self.pcscf]
                .iter()
                .any(|values| values.len() > 4 || values.iter().any(|value| !text(value, 128)))
            || !self.qos_sessions.is_empty()
            || self.traffic_descriptors.len() > 1
            || self.traffic_descriptors.iter().any(|value| {
                value.dnn.as_ref().is_some_and(|value| !text(value, 128))
                    || value
                        .os_app_id
                        .as_ref()
                        .is_some_and(|value| value.os_app_id.len() > 256)
            })
        {
            return Err(invalid());
        }
        Ok(())
    }
}
impl Frame {
    #[allow(clippy::too_many_arguments)]
    pub fn from_owner(
        identity: ProcessIdentity,
        revision: u64,
        serial: i32,
        until_ns: u64,
        sampled_at_ns: u64,
        owner: &DataOwner,
        fault: Option<String>,
    ) -> io::Result<Self> {
        let disposition = match owner.disposition() {
            OwnerDisposition::NotSubmitted => Disposition::NotSubmitted,
            OwnerDisposition::OutcomeUnknown => Disposition::OutcomeUnknown,
            OwnerDisposition::ProviderAccepted { cid } => Disposition::ProviderAccepted { cid },
            OwnerDisposition::ProviderRejected { radio_error, cause } => {
                Disposition::ProviderRejected { radio_error, cause }
            }
        };
        let value = Self {
            schema: 1,
            incarnation: identity.hex(),
            attempt: 1,
            revision,
            serial,
            until_ns,
            sampled_at_ns,
            disposition,
            provider_report: owner
                .provider_report()
                .map(Report::from_generated)
                .transpose()?,
            association: owner.last_association().map(|value| Association {
                namespace: Namespace {
                    dev: value.namespace.dev,
                    ino: value.namespace.ino,
                },
                ifindex: value.ifindex,
                lifetime: value.lifetime,
                revision: value.revision,
                observed_at_boottime_ns: value.observed_at_boottime_ns,
            }),
            // DataOwner::new returns only after the retained observer's initial sync.
            observer_initial_sync_completed: true,
            fault,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema != 1
            || self.incarnation.len() != 32
            || !self
                .incarnation
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.attempt != 1
            || self.revision == 0
            || self.until_ns == 0
            || self.sampled_at_ns == 0
            || self.fault.as_ref().is_some_and(|value| !text(value, 128))
        {
            return Err(invalid());
        }
        if let Some(report) = &self.provider_report {
            report.validate()?;
        }
        let consistent = match (&self.disposition, &self.provider_report) {
            (Disposition::NotSubmitted | Disposition::OutcomeUnknown, None) => true,
            (Disposition::ProviderAccepted { cid }, Some(report)) => {
                report.cid == *cid && report.cause == 0
            }
            (Disposition::ProviderRejected { radio_error, cause }, Some(report)) => {
                report.cause == *cause && (*radio_error != 0 || *cause != 0)
            }
            _ => false,
        };
        if !consistent
            || (!self.observer_initial_sync_completed
                && (!matches!(self.disposition, Disposition::NotSubmitted)
                    || self.provider_report.is_some()
                    || self.association.is_some()))
        {
            return Err(invalid());
        }
        if let Some(value) = &self.association
            && (!matches!(self.disposition, Disposition::ProviderAccepted { .. })
                || !self.observer_initial_sync_completed
                || self.fault.is_some()
                || value.namespace.ino == 0
                || value.ifindex == 0
                || value.lifetime == 0
                || value.revision == 0
                || value.observed_at_boottime_ns == 0
                || value.observed_at_boottime_ns > self.sampled_at_ns
                || self.sampled_at_ns >= self.until_ns)
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut writer = LimitedWriter(Vec::with_capacity(MAX_RECORD_BYTES));
        serde_json::to_writer(&mut writer, self).map_err(|_| invalid())?;
        writer.0.push(b'\n');
        Ok(writer.0)
    }
}
struct LimitedWriter(Vec<u8>);
impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > (MAX_RECORD_BYTES - 1).saturating_sub(self.0.len()) {
            return Err(invalid());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Exactly one complete bounded record, with strict duplicate/shape validation.
pub fn decode_exact(bytes: &[u8]) -> io::Result<Frame> {
    if bytes.is_empty()
        || bytes.len() > MAX_RECORD_BYTES
        || bytes.last() != Some(&b'\n')
        || bytes[..bytes.len() - 1].contains(&b'\n')
    {
        return Err(invalid());
    }
    // Deserialize directly into strict DTOs. An intermediate Value would erase
    // duplicate keys before the typed visitor can reject them.
    let frame: Frame = serde_json::from_slice(&bytes[..bytes.len() - 1]).map_err(|_| invalid())?;
    frame.validate()?;
    Ok(frame)
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
