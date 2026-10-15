//! Bounded copies of unchanged generated types, not replacement wire types.
use pokelinux_cellular_boundary::SetupDataCallResult;
use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
    LinkAddress::LinkAddress, OsAppId::OsAppId, Qos::Qos, SliceInfo::SliceInfo,
    TrafficDescriptor::TrafficDescriptor,
};
use rsbinder::StatusCode;
fn text(s: &str, limit: usize) -> bool {
    s.len() <= limit && !s.contains('\0')
}
fn strings(values: &[String]) -> bool {
    values.len() <= 4 && values.iter().all(|s| text(s, 128))
}
fn bounded_descriptor(v: &TrafficDescriptor) -> bool {
    v.dnn.as_ref().is_none_or(|s| text(s, 128))
        && v.osAppId.as_ref().is_none_or(|v| v.osAppId.len() <= 256)
}
pub fn bounded_result(v: &SetupDataCallResult) -> bool {
    text(&v.ifname, 15)
        && v.addresses.len() <= 4
        && v.addresses.iter().all(|a| text(&a.address, 128))
        && strings(&v.dnses)
        && strings(&v.gateways)
        && strings(&v.pcscf)
        && matches!(v.defaultQos, Qos::Noinit(_))
        && v.qosSessions.is_empty()
        && v.trafficDescriptors.len() <= 1
        && v.trafficDescriptors.iter().all(bounded_descriptor)
}
fn copy_slice(v: &SliceInfo) -> SliceInfo {
    SliceInfo {
        sliceServiceType: v.sliceServiceType,
        sliceDifferentiator: v.sliceDifferentiator,
        mappedHplmnSst: v.mappedHplmnSst,
        mappedHplmnSd: v.mappedHplmnSd,
        status: v.status,
    }
}
fn copy_descriptor(v: &TrafficDescriptor) -> TrafficDescriptor {
    TrafficDescriptor {
        dnn: v.dnn.clone(),
        osAppId: v.osAppId.as_ref().map(|v| OsAppId {
            osAppId: v.osAppId.clone(),
        }),
    }
}
fn copy_address(v: &LinkAddress) -> LinkAddress {
    LinkAddress {
        address: v.address.clone(),
        addressProperties: v.addressProperties,
        deprecationTime: v.deprecationTime,
        expirationTime: v.expirationTime,
    }
}
pub fn copy_result(v: &SetupDataCallResult) -> Result<SetupDataCallResult, StatusCode> {
    if !bounded_result(v) {
        return Err(StatusCode::BadValue);
    }
    let Qos::Noinit(noinit) = &v.defaultQos else {
        return Err(StatusCode::BadValue);
    };
    Ok(SetupDataCallResult {
        cause: v.cause,
        suggestedRetryTime: v.suggestedRetryTime,
        cid: v.cid,
        active: v.active,
        r#type: v.r#type,
        ifname: v.ifname.clone(),
        addresses: v.addresses.iter().map(copy_address).collect(),
        dnses: v.dnses.clone(),
        gateways: v.gateways.clone(),
        pcscf: v.pcscf.clone(),
        mtuV4: v.mtuV4,
        mtuV6: v.mtuV6,
        defaultQos: Qos::Noinit(*noinit),
        qosSessions: Vec::new(),
        handoverFailureMode: v.handoverFailureMode,
        pduSessionId: v.pduSessionId,
        sliceInfo: v.sliceInfo.as_ref().map(copy_slice),
        trafficDescriptors: v.trafficDescriptors.iter().map(copy_descriptor).collect(),
    })
}
