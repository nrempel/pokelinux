//! Actual Binder callback implementations; only normal setup/list are accepted.
use crate::typed;
use pokelinux_cellular_boundary::generated::android::hardware::radio::{
    RadioIndicationType::RadioIndicationType,
    data::{
        KeepaliveStatus::KeepaliveStatus, PcoDataInfo::PcoDataInfo, SlicingConfig::SlicingConfig,
    },
};
use pokelinux_cellular_boundary::{
    IRadioDataIndication, IRadioDataResponse, RadioResponseInfo, RadioResponseType,
    SetupDataCallResult,
};
use rsbinder::{BinderResult, Interface, StatusCode};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
};
pub enum Event {
    Setup(Box<(RadioResponseInfo, SetupDataCallResult)>, u32),
    List(RadioIndicationType, Vec<SetupDataCallResult>, u32),
    Died,
}
pub struct Shared {
    pub queue: SyncSender<Event>,
    pub fault: AtomicBool,
    pub closed: AtomicBool,
}
impl Shared {
    fn refused(&self) -> BinderResult<()> {
        self.fault.store(true, Ordering::SeqCst);
        Err(StatusCode::InvalidOperation.into())
    }
    fn send(&self, event: Event) -> BinderResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(StatusCode::DeadObject.into());
        }
        self.queue.try_send(event).map_err(|_| {
            self.fault.store(true, Ordering::SeqCst);
            StatusCode::FailedTransaction.into()
        })
    }
}
pub struct Response(pub Arc<Shared>);
impl Interface for Response {}
macro_rules! refuse_info {
    ($($name:ident),+ $(,)?) => { $(fn $name(&self,_:&RadioResponseInfo)->BinderResult<()> {self.0.refused()})+ };
}
impl IRadioDataResponse for Response {
    fn setupDataCallResponse(
        &self,
        info: &RadioResponseInfo,
        result: &SetupDataCallResult,
    ) -> BinderResult<()> {
        if info.r#type != RadioResponseType::SOLICITED {
            return self.0.refused();
        }
        let owned = typed::copy_result(result)
            .inspect_err(|_| self.0.fault.store(true, Ordering::SeqCst))?;
        self.0.send(Event::Setup(
            Box::new((
                RadioResponseInfo {
                    r#type: info.r#type,
                    serial: info.serial,
                    error: info.error,
                },
                owned,
            )),
            rsbinder::thread_state::get_calling_uid(),
        ))
    }
    fn acknowledgeRequest(&self, _: i32) -> BinderResult<()> {
        self.0.refused()
    }
    fn allocatePduSessionIdResponse(&self, _: &RadioResponseInfo, _: i32) -> BinderResult<()> {
        self.0.refused()
    }
    fn getDataCallListResponse(
        &self,
        _: &RadioResponseInfo,
        _: &[SetupDataCallResult],
    ) -> BinderResult<()> {
        self.0.refused()
    }
    fn getSlicingConfigResponse(
        &self,
        _: &RadioResponseInfo,
        _: &SlicingConfig,
    ) -> BinderResult<()> {
        self.0.refused()
    }
    fn startKeepaliveResponse(
        &self,
        _: &RadioResponseInfo,
        _: &KeepaliveStatus,
    ) -> BinderResult<()> {
        self.0.refused()
    }
    refuse_info!(
        cancelHandoverResponse,
        deactivateDataCallResponse,
        releasePduSessionIdResponse,
        setDataAllowedResponse,
        setDataProfileResponse,
        setDataThrottlingResponse,
        setInitialAttachApnResponse,
        startHandoverResponse,
        stopKeepaliveResponse
    );
}
pub struct Indication(pub Arc<Shared>);
impl Interface for Indication {}
impl IRadioDataIndication for Indication {
    fn dataCallListChanged(
        &self,
        kind: RadioIndicationType,
        calls: &[SetupDataCallResult],
    ) -> BinderResult<()> {
        if kind != RadioIndicationType::UNSOLICITED || calls.len() > 1 {
            return self.0.refused();
        }
        let owned = calls
            .iter()
            .map(typed::copy_result)
            .collect::<Result<Vec<_>, _>>()
            .inspect_err(|_| self.0.fault.store(true, Ordering::SeqCst))?;
        self.0.send(Event::List(
            kind,
            owned,
            rsbinder::thread_state::get_calling_uid(),
        ))
    }
    fn keepaliveStatus(&self, _: RadioIndicationType, _: &KeepaliveStatus) -> BinderResult<()> {
        self.0.refused()
    }
    fn pcoData(&self, _: RadioIndicationType, _: &PcoDataInfo) -> BinderResult<()> {
        self.0.refused()
    }
    fn unthrottleApn(
        &self,
        _: RadioIndicationType,
        _: &pokelinux_cellular_boundary::DataProfileInfo,
    ) -> BinderResult<()> {
        self.0.refused()
    }
    fn slicingConfigChanged(&self, _: RadioIndicationType, _: &SlicingConfig) -> BinderResult<()> {
        self.0.refused()
    }
}
pub struct Death(pub Arc<Shared>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        if self.0.queue.try_send(Event::Died).is_err() {
            self.0.fault.store(true, Ordering::SeqCst);
        }
    }
}
