#[allow(non_snake_case)]
pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/suspend.rs"));
}
pub use bindings::android::system::suspend::{
    ISystemSuspend::{BnSystemSuspend, ISystemSuspend},
    IWakeLock::{BnWakeLock, IWakeLock},
    WakeLockType::WakeLockType,
};
pub use bindings::org::pokelinux::suspendtest::{
    ISuspendBoundaryProbe::ISuspendBoundaryProbe,
    ISuspendControl::{BnSuspendControl, ISuspendControl},
};
pub mod bootstrap;

pub mod frontend;
