#[allow(clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/sms_receipt.rs"));
}
pub use generated::android::hardware::radio::messaging::{
    GsmSmsMessage::GsmSmsMessage,
    IRadioMessaging::IRadioMessaging,
    IRadioMessagingIndication::{BnRadioMessagingIndication, IRadioMessagingIndication},
    IRadioMessagingResponse::{BnRadioMessagingResponse, IRadioMessagingResponse},
    SendSmsResult::SendSmsResult,
};
pub use generated::android::hardware::radio::{
    RadioError::RadioError, RadioResponseInfo::RadioResponseInfo,
    RadioResponseType::RadioResponseType,
};
pub use generated::org::pokelinux::smsreceipttest::ISmsReceiptProbe::ISmsReceiptProbe;

pub use generated::android::hardware::radio::RadioIndicationType::RadioIndicationType;
pub use generated::android::hardware::radio::messaging::SmsAcknowledgeFailCause::SmsAcknowledgeFailCause;
