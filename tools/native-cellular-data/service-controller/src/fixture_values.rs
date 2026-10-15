//! Authored fixture constants/converters adapted from the pinned ABI helper.
//! Test infrastructure only; no configuration or callback success ingress.
use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
    DataCallFailCause::DataCallFailCause, LinkAddress::LinkAddress, OsAppId::OsAppId,
    PdpProtocolType::PdpProtocolType, Qos::Qos, SliceInfo::SliceInfo,
    TrafficDescriptor::TrafficDescriptor,
};
use pokelinux_cellular_boundary::{DataProfileInfo, SetupDataCallResult};
use serde_json::{Value, json};
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
