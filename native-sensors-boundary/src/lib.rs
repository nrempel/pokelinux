//! Exact frozen wire types; ABI mapping is a separate explicitly audited leaf.
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/sensor.rs"));
}
pub use bindings::android::hardware::sensors::{
    Event::Event,
    ISensors::ISensors,
    ISensorsCallback::{BnSensorsCallback, ISensorsCallback},
    SensorInfo::SensorInfo,
};
pub use bindings::org::pokelinux::sensortest::{
    ISensorBoundaryControl::ISensorBoundaryControl,
    ISensorClient::{BnSensorClient, ISensorClient},
};
pub mod event_abi;

pub use bindings::android::hardware::common::{
    NativeHandle::NativeHandle,
    fmq::{
        GrantorDescriptor::GrantorDescriptor, MQDescriptor::MQDescriptor,
        SynchronizedReadWrite::SynchronizedReadWrite,
    },
};
