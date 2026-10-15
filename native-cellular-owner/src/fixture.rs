//! Bounded copies of unchanged generated types, not replacement wire types.
use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
    ApnAuthType::ApnAuthType, DataCallFailCause::DataCallFailCause, LinkAddress::LinkAddress,
    OsAppId::OsAppId, PdpProtocolType::PdpProtocolType, Qos::Qos, SliceInfo::SliceInfo,
    TrafficDescriptor::TrafficDescriptor,
};
use pokelinux_cellular_boundary::{DataProfileInfo, SetupDataCallResult};
use rsbinder::StatusCode;
use serde_json::{Value, json};
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
pub fn slice() -> SliceInfo {
    SliceInfo {
        sliceServiceType: 1,
        sliceDifferentiator: 0x010203,
        mappedHplmnSst: 2,
        mappedHplmnSd: 0x040506,
        status: 2,
    }
}
pub fn descriptor() -> TrafficDescriptor {
    TrafficDescriptor {
        dnn: Some("interop.invalid".into()),
        osAppId: Some(OsAppId {
            osAppId: vec![1, 2, 0, 255],
        }),
    }
}
pub fn addresses() -> Vec<LinkAddress> {
    ["192.0.2.2/32", "2001:db8:1::2/64"]
        .into_iter()
        .map(|address| LinkAddress {
            address: address.into(),
            addressProperties: 0,
            deprecationTime: i64::MAX,
            expirationTime: i64::MAX,
        })
        .collect()
}
pub fn dnses() -> Vec<String> {
    vec!["192.0.2.53".into(), "2001:db8::53".into()]
}
pub fn profile() -> DataProfileInfo {
    DataProfileInfo {
        profileId: 2,
        apn: "interop.invalid".into(),
        protocol: PdpProtocolType::IPV4V6,
        roamingProtocol: PdpProtocolType::IP,
        authType: ApnAuthType::NO_PAP_NO_CHAP,
        user: String::new(),
        password: String::new(),
        r#type: 1,
        maxConnsTime: 11,
        maxConns: 3,
        waitTime: 13,
        enabled: true,
        supportedApnTypesBitmap: 64,
        bearerBitmap: 0x4000,
        mtuV4: 1400,
        mtuV6: 1400,
        preferred: false,
        persistent: true,
        alwaysOn: false,
        trafficDescriptor: descriptor(),
        infrastructureBitmap: 1,
    }
}
pub fn result() -> SetupDataCallResult {
    SetupDataCallResult {
        cause: DataCallFailCause::NONE,
        suggestedRetryTime: -1,
        cid: 9,
        active: 2,
        r#type: PdpProtocolType::IPV4V6,
        ifname: "pklabi0".into(),
        addresses: addresses(),
        dnses: dnses(),
        gateways: vec!["192.0.2.1".into()],
        pcscf: vec!["192.0.2.20".into(), "2001:db8::20".into()],
        mtuV4: 1400,
        mtuV6: 1400,
        defaultQos: Qos::Noinit(false),
        qosSessions: Vec::new(),
        handoverFailureMode: 0,
        pduSessionId: 7,
        sliceInfo: Some(slice()),
        trafficDescriptors: vec![descriptor()],
    }
}
pub fn slice_json(v: Option<&SliceInfo>) -> Value {
    v.map(|v| {
        json!({"sliceServiceType":v.sliceServiceType,"sliceDifferentiator":v.sliceDifferentiator,
        "mappedHplmnSst":v.mappedHplmnSst,"mappedHplmnSd":v.mappedHplmnSd,"status":v.status})
    })
    .unwrap_or(Value::Null)
}
fn descriptor_json(v: &TrafficDescriptor) -> Value {
    json!({"dnn":v.dnn,"osAppId":v.osAppId.as_ref().map(|v| &v.osAppId)})
}
pub fn addresses_json(v: &[LinkAddress]) -> Value {
    json!(
        v.iter()
            .map(
                |v| json!({"address":v.address,"addressProperties":v.addressProperties,
        "deprecationTime":v.deprecationTime,"expirationTime":v.expirationTime})
            )
            .collect::<Vec<_>>()
    )
}
pub fn profile_json(v: &DataProfileInfo) -> Value {
    json!({"profileId":v.profileId,"apn":v.apn,"protocol":v.protocol.0,"roamingProtocol":v.roamingProtocol.0,
        "authType":v.authType.0,"user":v.user,"password":v.password,"type":v.r#type,
        "maxConnsTime":v.maxConnsTime,"maxConns":v.maxConns,"waitTime":v.waitTime,"enabled":v.enabled,
        "supportedApnTypesBitmap":v.supportedApnTypesBitmap,"bearerBitmap":v.bearerBitmap,
        "mtuV4":v.mtuV4,"mtuV6":v.mtuV6,"preferred":v.preferred,"persistent":v.persistent,
        "alwaysOn":v.alwaysOn,"trafficDescriptor":descriptor_json(&v.trafficDescriptor),
        "infrastructureBitmap":v.infrastructureBitmap})
}
pub fn result_json(v: &SetupDataCallResult) -> Value {
    let qos = match &v.defaultQos {
        Qos::Noinit(value) => json!({"noinit":value}),
        _ => json!({"unsupported":true}),
    };
    json!({"cause":v.cause.0,"suggestedRetryTime":v.suggestedRetryTime,"cid":v.cid,"active":v.active,
        "type":v.r#type.0,"ifname":v.ifname,"addresses":addresses_json(&v.addresses),"dnses":v.dnses,
        "gateways":v.gateways,"pcscf":v.pcscf,"mtuV4":v.mtuV4,"mtuV6":v.mtuV6,"defaultQos":qos,
        "qosSessions":v.qosSessions.iter().map(|_| "unsupported").collect::<Vec<_>>(),
        "handoverFailureMode":v.handoverFailureMode,"pduSessionId":v.pduSessionId,
        "sliceInfo":slice_json(v.sliceInfo.as_ref()),
        "trafficDescriptors":v.trafficDescriptors.iter().map(descriptor_json).collect::<Vec<_>>()})
}
