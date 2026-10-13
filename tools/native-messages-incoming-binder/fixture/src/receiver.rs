//! Dedicated receive-only synthetic V3 adapter; generated callbacks enqueue only.
use crate::*;
use pokelinux_lifecycle::boottime_ns;
use pokelinux_messages::{
    incoming::{Backend, Control},
    projection::Store,
};
use pokelinux_messaging::receipt::Slot;
use rsbinder::{FromIBinder, ProcessState, Strong};
use rsbinder::{Interface, StatusCode};
use std::{io, sync::Arc};
pub struct Response(Control);
impl Interface for Response {}
impl IRadioMessagingResponse for Response {
    fn sendSmsResponse(
        &self,
        _info: &RadioResponseInfo,
        _sms: &SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn acknowledgeRequest(&self, _serial: i32) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeIncomingGsmSmsWithPduResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeLastIncomingCdmaSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#acknowledgeLastIncomingGsmSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#deleteSmsOnRuimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#deleteSmsOnSimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[crate::generated::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo::CdmaBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getGsmBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_configs: &[crate::generated::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo::GsmBroadcastSmsConfigInfo],
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#getSmscAddressResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_smsc: &str,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#reportSmsMemoryStatusResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendCdmaSmsExpectMoreResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendCdmaSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendImsSmsResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#sendSmsExpectMoreResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_sms: &crate::generated::android::hardware::radio::messaging::SendSmsResult::SendSmsResult,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setCdmaBroadcastActivationResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setCdmaBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setGsmBroadcastActivationResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setGsmBroadcastConfigResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#setSmscAddressResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#writeSmsToRuimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#writeSmsToSimResponse(
        &self,
        _arg_info: &crate::generated::android::hardware::radio::RadioResponseInfo::RadioResponseInfo,
        _arg_index: i32,
    ) -> rsbinder::BinderResult<()> {
        self.0.fail("unexpected_response_callback");
        Err(StatusCode::InvalidOperation.into())
    }
}
pub struct Indication {
    pub callback: pokelinux_messages::incoming::Callback,
}
impl Interface for Indication {}
impl Indication {
    fn queue(
        &self,
        kind: u8,
        indication_type: RadioIndicationType,
        raw: &[u8],
    ) -> rsbinder::BinderResult<()> {
        if indication_type != RadioIndicationType::UNSOLICITED
            || rsbinder::thread_state::get_calling_uid() != 1000
        {
            self.callback.fail("unqualified_callback_type_or_identity");
            return Err(StatusCode::BadValue.into());
        }
        let kind = if kind == 0 {
            pokelinux_messaging::receipt::IncomingKind::Deliver
        } else {
            pokelinux_messaging::receipt::IncomingKind::StatusReport
        };
        self.callback
            .admit(kind, raw)
            .map_err(|_| StatusCode::FailedTransaction.into())
    }
}
impl IRadioMessagingIndication for Indication {
    fn r#cdmaNewSms(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_msg: &crate::generated::android::hardware::radio::messaging::CdmaSmsMessage::CdmaSmsMessage,
    ) -> rsbinder::BinderResult<()> {
        self.callback.fail("unsupported_incoming_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#cdmaRuimSmsStorageFull(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
    ) -> rsbinder::BinderResult<()> {
        self.callback.fail("unsupported_incoming_callback");
        Err(StatusCode::InvalidOperation.into())
    }
    fn r#newBroadcastSms(
        &self,
        _arg_type: crate::generated::android::hardware::radio::RadioIndicationType::RadioIndicationType,
        _arg_data: &[u8],
    ) -> rsbinder::BinderResult<()> {
        self.callback.fail("unsupported_incoming_callback");
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
        self.callback.fail("unsupported_incoming_callback");
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
        self.callback.fail("unsupported_incoming_callback");
        Err(StatusCode::InvalidOperation.into())
    }
}

struct Death(Control);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.0.fail("backend_dead");
    }
}
struct SlotAttachment {
    _endpoint: Strong<dyn IRadioMessaging>,
    _response: Strong<dyn IRadioMessagingResponse>,
    _indication: Strong<dyn IRadioMessagingIndication>,
}
struct Attachment {
    _probe: Strong<dyn IIncomingMessagesProbe>,
    _death: Arc<Death>,
    _slots: Vec<SlotAttachment>,
}
pub struct Synthetic {
    generation: String,
    fault: String,
    attachment: Option<Attachment>,
    control: Option<Control>,
}
impl Synthetic {
    pub fn new(generation: String, fault: String) -> io::Result<Self> {
        if ![
            "none",
            "hold-after-raw",
            "exit-before-complete",
            "hold-before-complete",
            "small-ipc",
        ]
        .contains(&fault.as_str())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            generation,
            fault,
            attachment: None,
            control: None,
        })
    }
}
impl Backend for Synthetic {
    fn initialize(&mut self, store: &mut Store, control: &Control, until: u64) -> io::Result<()> {
        self.control = Some(control.clone());
        let result = (|| -> Result<Attachment, Box<dyn std::error::Error>> {
            let process = ProcessState::init("/dev/binder", 3)?;
            ProcessState::start_thread_pool();
            let binder = lookup_probe(process)?;
            let probe: Strong<dyn IIncomingMessagesProbe> = FromIBinder::try_from(binder.clone())?;
            if probe.generation()? != self.generation {
                return Err("different synthetic peer generation".into());
            }
            let death = Arc::new(Death(control.clone()));
            binder.link_to_death_arc(&death)?;
            let mut slots = Vec::new();
            for slot in 0..2 {
                let endpoint = probe.endpoint(slot)?;
                if !endpoint.as_binder().is_remote()
                    || endpoint.getInterfaceVersion() != Ok(3)
                    || endpoint.getInterfaceHash() != Ok(HASH.into())
                {
                    return Err("wrong frozen V3 endpoint".into());
                }
                let source = store.attach_test_incoming(Slot(slot as u8))?;
                let callback = control.bind(source)?;
                let response = BnRadioMessagingResponse::new_binder(Response(control.clone()));
                let indication = BnRadioMessagingIndication::new_binder(Indication { callback });
                if boottime_ns() >= until {
                    return Err("original startup deadline expired".into());
                }
                endpoint.setResponseFunctions(&response, &indication)?;
                // Explicit synthetic fixture confirmation only; never factory registration proof.
                let remaining = until.checked_sub(boottime_ns()).ok_or("startup expired")?;
                probe.barrier(slot, 1, 0, (remaining / 1_000_000).clamp(1, 1000) as i32)?;
                slots.push(SlotAttachment {
                    _endpoint: endpoint,
                    _response: response,
                    _indication: indication,
                });
            }
            if self.fault == "small-ipc" {
                control.fixture_send_buffer(1024)?;
            }
            Ok(Attachment {
                _probe: probe,
                _death: death,
                _slots: slots,
            })
        })();
        match result {
            Ok(attachment) => {
                self.attachment = Some(attachment);
                Ok(())
            }
            Err(_) => {
                control.fail("source_attachment_failed");
                Err(io::ErrorKind::NotConnected.into())
            }
        }
    }
    fn after_raw(&mut self) -> io::Result<()> {
        if self.fault == "hold-after-raw" {
            loop {
                std::thread::park();
            }
        }
        Ok(())
    }
    fn before_complete(&mut self) -> io::Result<()> {
        if self.fault == "exit-before-complete" {
            std::process::exit(73);
        }
        if self.fault == "hold-before-complete" {
            loop {
                std::thread::park();
            }
        }
        Ok(())
    }
}

impl Drop for Synthetic {
    fn drop(&mut self) {
        // Isolated exit marker proves actual start-send EAGAIN without callback filesystem IO.
        if self.fault == "small-ipc"
            && self
                .control
                .as_ref()
                .is_some_and(Control::fixture_start_eagain)
        {
            std::process::exit(74);
        }
    }
}
