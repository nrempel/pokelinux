//! Independent synthetic fixture types; no installed service or raw store.
#[allow(clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/incoming.rs"));
}
pub use generated::android::hardware::radio::messaging::{
    IRadioMessaging::IRadioMessaging,
    IRadioMessagingIndication::{BnRadioMessagingIndication, IRadioMessagingIndication},
    IRadioMessagingResponse::{BnRadioMessagingResponse, IRadioMessagingResponse},
    SendSmsResult::SendSmsResult,
};
pub use generated::android::hardware::radio::{
    RadioIndicationType::RadioIndicationType, RadioResponseInfo::RadioResponseInfo,
};
pub use generated::org::pokelinux::messagesincomingtest::IIncomingMessagesProbe::IIncomingMessagesProbe;
pub const NAME: &str = "org.pokelinux.fixture.incoming-messages";
pub const DESCRIPTOR: &str = "org.pokelinux.messagesincomingtest.IIncomingMessagesProbe";
pub const HASH: &str = "30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb";
pub mod smoke_callbacks;

/// The dedicated synthetic manager implements the pinned Android16 legacy check
/// transaction3. This is fixture lookup, not production/vendor discovery.
pub fn lookup_probe(
    process: &rsbinder::ProcessState,
) -> Result<rsbinder::SIBinder, Box<dyn std::error::Error>> {
    let manager = process.context_object()?;
    let remote = manager.as_remote().ok_or("fixture manager is local")?;
    let mut data = remote.prepare_transact(true)?;
    data.write(NAME)?;
    let mut reply = remote
        .submit_transact(3, &data, 0)?
        .ok_or("fixture check returned no reply")?;
    let status: rsbinder::Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    let binder: rsbinder::SIBinder = reply.read()?;
    if reply.data_avail() != 0
        || !binder.is_remote()
        || binder == manager
        || binder.descriptor() != DESCRIPTOR
    {
        return Err("wrong transferred fixture probe".into());
    }
    Ok(binder)
}

pub mod receiver;
