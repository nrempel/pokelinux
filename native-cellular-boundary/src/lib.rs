//! Frozen public Data V3 backend wire types; no cellular acquisition authority.
#[allow(clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/cellular.rs"));
}
pub use generated::android::hardware::radio::data::{
    DataProfileInfo::DataProfileInfo,
    IRadioData::IRadioData,
    IRadioDataIndication::{BnRadioDataIndication, IRadioDataIndication},
    IRadioDataResponse::{BnRadioDataResponse, IRadioDataResponse},
    SetupDataCallResult::SetupDataCallResult,
};
pub use generated::android::hardware::radio::{
    AccessNetwork::AccessNetwork, RadioError::RadioError, RadioResponseInfo::RadioResponseInfo,
    RadioResponseType::RadioResponseType,
};
