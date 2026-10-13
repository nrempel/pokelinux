//! Authored synthetic ARM Binder fixture, outside production phone services.
#[allow(clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/camera.rs"));
}
pub use generated::android::hardware::camera::device::{
    BufferCache::BufferCache,
    BufferRequest::BufferRequest,
    BufferRequestStatus::BufferRequestStatus,
    BufferStatus::BufferStatus,
    CaptureRequest::CaptureRequest,
    CaptureResult::CaptureResult,
    ICameraDeviceCallback::{BnCameraDeviceCallback, ICameraDeviceCallback},
    NotifyMsg::NotifyMsg,
    StreamBuffer::StreamBuffer,
    StreamBufferRet::StreamBufferRet,
};
pub use generated::org::pokelinux::cameratest::ICameraOwnershipProbe::{
    BnCameraOwnershipProbe, ICameraOwnershipProbe,
};
pub const NAME: &str = "org.pokelinux.fixture.camera-service-red";
pub const DESCRIPTOR: &str = "org.pokelinux.cameratest.ICameraOwnershipProbe";
pub const CAMERA_HASH: &str = "6e6878817de92e03fd52d5f10937714862ab4922";
pub mod backend;
pub mod pressure;

/// Same narrowly qualified fixture IServiceManager lookup as existing ARM tests.
pub fn lookup(
    process: &rsbinder::ProcessState,
) -> Result<rsbinder::SIBinder, Box<dyn std::error::Error>> {
    let manager = process.context_object()?;
    let remote = manager.as_remote().ok_or("fixture manager is local")?;
    let mut data = remote.prepare_transact(true)?;
    data.write(NAME)?;
    let mut reply = remote
        .submit_transact(3, &data, 0)?
        .ok_or("missing check reply")?;
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
        return Err("wrong remote camera fixture".into());
    }
    Ok(binder)
}
