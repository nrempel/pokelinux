//! Smoke callbacks only. No raw owner, storage, ACK or production adapter.
use crate::*;
use rsbinder::{Interface, StatusCode};
use std::sync::mpsc::SyncSender;
pub struct Response;
impl Interface for Response {}
impl IRadioMessagingResponse for Response {
    fn sendSmsResponse(
        &self,
        _info: &RadioResponseInfo,
        _sms: &SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn acknowledgeRequest(&self, _serial: i32) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeIncomingGsmSmsWithPduResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeLastIncomingCdmaSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeLastIncomingGsmSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#deleteSmsOnRuimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#deleteSmsOnSimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[crate::generated::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo::CdmaBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getGsmBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[crate::generated::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo::GsmBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getSmscAddressResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_smsc: &str,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#reportSmsMemoryStatusResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendCdmaSmsExpectMoreResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendCdmaSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendImsSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendSmsExpectMoreResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setCdmaBroadcastActivationResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setGsmBroadcastActivationResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setGsmBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setSmscAddressResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#writeSmsToRuimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#writeSmsToSimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
}
pub type Arrival = (u8, u8, i32, Vec<u8>, u32);
pub struct Indication {
    pub slot: u8,
    pub queue: SyncSender<Arrival>,
}
impl Interface for Indication {}
impl Indication {
    fn queue(
        &self,
        kind: u8,
        indication_type: RadioIndicationType,
        raw: &[u8],
    ) -> rsbinder::BinderResult<()> {
        if raw.len() > 512 {
            return Err(StatusCode::BadValue.into());
        }
        self.queue
            .try_send((
                self.slot,
                kind,
                indication_type.0,
                raw.to_vec(),
                rsbinder::thread_state::get_calling_uid(),
            ))
            .map_err(|_| StatusCode::FailedTransaction.into())
    }
}
impl IRadioMessagingIndication for Indication {
    fn r#cdmaNewSms(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_msg: &crate::generated::android::hardware::radio::messaging::CdmaSmsMessage::CdmaSmsMessage,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#cdmaRuimSmsStorageFull(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newBroadcastSms(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_data: &[u8],
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSms(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        self.queue(0, _arg_type, _arg_pdu)
    }
    fn r#newSmsOnSim(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_record_number: i32,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newSmsStatusReport(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_pdu: &[u8],
    ) -> rsbinder::BinderResult<()> {
        self.queue(1, _arg_type, _arg_pdu)
    }
    fn r#simSmsStorageFull(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        Err(StatusCode::InvalidOperation.into())
    }
}
