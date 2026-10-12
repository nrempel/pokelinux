//! Synthetic camera-boundary validation only; no qualified camera HAL backend.
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/camera.rs"));
}
pub use bindings::android::hardware::common::NativeHandle::NativeHandle;
pub use bindings::org::pokelinux::cameratest::ICameraBoundaryProbe::ICameraBoundaryProbe;

/// A successful submit receipt is separate from asynchronous capture completion.
#[derive(Debug, PartialEq, Eq)]
pub enum SubmissionReceipt {
    Accepted(i32),
    /// The peer may retain/work on buffers even though no count is observable.
    Uncertain {
        service_error: i32,
        transport_error: rsbinder::StatusCode,
    },
}

pub fn classify_submission(result: rsbinder::BinderResult<i32>) -> SubmissionReceipt {
    match result {
        Ok(count) => SubmissionReceipt::Accepted(count),
        Err(status) => SubmissionReceipt::Uncertain {
            service_error: status.service_specific_error(),
            transport_error: status.transaction_error(),
        },
    }
}
