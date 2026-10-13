//! Actual generated Binder controls and one frozen-type/FD owner smoke.
use native_camera_service_boundary_fixture::*;
use pokelinux_camera::{CaptureId, Generation, LeaseId, Owner, Receipt};
use rsbinder::{FromIBinder, Interface, ParcelFileDescriptor, ProcessState};
use std::{
    os::fd::OwnedFd,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const GENERATION: Generation = Generation {
    session: 1,
    configuration: 1,
};
fn owner_result<T>(
    value: Result<T, pokelinux_camera::OwnerError>,
) -> Result<T, Box<dyn std::error::Error>> {
    value.map_err(|error| format!("camera owner {error:?}").into())
}
struct Callback(Arc<Mutex<Owner>>);
impl Interface for Callback {}
impl ICameraDeviceCallback for Callback {
    fn notify(&self, messages: &[NotifyMsg]) -> rsbinder::BinderResult<()> {
        for message in messages {
            if let NotifyMsg::Shutter(shutter) = message {
                if shutter.frameNumber != 10 || shutter.timestamp != 100 {
                    return Err(rsbinder::StatusCode::BadValue.into());
                }
                self.0
                    .lock()
                    .unwrap()
                    .shutter(GENERATION, CaptureId(1))
                    .map_err(|_| rsbinder::StatusCode::BadValue)?;
            }
        }
        Ok(())
    }
    fn processCaptureResult(&self, results: &[CaptureResult]) -> rsbinder::BinderResult<()> {
        for result in results {
            if result.frameNumber != 10
                || result.fmqResultSize != 0
                || result.inputBuffer.streamId != -1
            {
                return Err(rsbinder::StatusCode::BadValue.into());
            }
            for buffer in &result.outputBuffers {
                if buffer.streamId != 7
                    || buffer.bufferId != 99
                    || buffer.releaseFence.fds.len() != 1
                {
                    return Err(rsbinder::StatusCode::BadValue.into());
                }
                let fence = OwnedFd::from(buffer.releaseFence.fds[0].try_clone()?);
                self.0
                    .lock()
                    .unwrap()
                    .returned(GENERATION, CaptureId(1), LeaseId(1), false, Some(fence))
                    .map_err(|_| rsbinder::StatusCode::BadValue)?;
            }
            if result.partialResult == 1 && result.result.metadata == [42] {
                self.0
                    .lock()
                    .unwrap()
                    .final_metadata(GENERATION, CaptureId(1))
                    .map_err(|_| rsbinder::StatusCode::BadValue)?;
            }
        }
        Ok(())
    }
    fn requestStreamBuffers(
        &self,
        _: &[BufferRequest],
        _: &mut Vec<StreamBufferRet>,
    ) -> rsbinder::BinderResult<BufferRequestStatus> {
        Err(rsbinder::StatusCode::InvalidOperation.into())
    }
    fn returnStreamBuffers(&self, _: &[StreamBuffer]) -> rsbinder::BinderResult<()> {
        Err(rsbinder::StatusCode::InvalidOperation.into())
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1).ok_or("probe command required")?;
    let process = ProcessState::init("/dev/binder", 3)?;
    ProcessState::start_thread_pool();
    let binder = lookup(process)?;
    let peer: rsbinder::Strong<dyn ICameraOwnershipProbe> = FromIBinder::try_from(binder)?;
    if arg == "info" {
        println!(
            "{}",
            serde_json::json!({"remote":peer.as_binder().is_remote(), "version":peer.getInterfaceVersion()?, "hash":peer.getInterfaceHash()?, "submitted":peer.submittedCount()?, "registered":peer.callbackCount()?, "held":peer.held()?, "fds":peer.fdCount()?})
        );
    } else if arg == "die" {
        let _ = peer.die();
    } else if arg == "release" {
        peer.releaseHold()?;
    } else if arg == "callbacks" {
        peer.returnResult(10, 0)?;
        peer.returnResult(10, 1)?;
    } else if arg == "signal" {
        peer.signalFence(10)?;
    } else if arg == "smoke" {
        let baseline = peer.fdCount()?;
        let (reader, writer) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)?;
        let owner = Arc::new(Mutex::new(Owner::new(GENERATION, 1, 1)));
        owner_result(
            owner
                .lock()
                .unwrap()
                .admit(GENERATION, CaptureId(1), LeaseId(1), reader),
        )?;
        let callback = BnCameraDeviceCallback::new_binder(Callback(owner.clone()));
        peer.setCallback(&callback)?;
        let mut request = CaptureRequest {
            frameNumber: 10,
            ..CaptureRequest::default()
        };
        request.inputBuffer.streamId = -1;
        request.settings.metadata = vec![1, 2, 3];
        let mut buffer = StreamBuffer {
            streamId: 7,
            bufferId: 99,
            status: BufferStatus::OK,
            ..StreamBuffer::default()
        };
        buffer.buffer.fds = vec![ParcelFileDescriptor::new(writer)];
        buffer.buffer.ints = vec![37, 4];
        request.outputBuffers = vec![buffer];
        let count = peer.processCaptureRequest(&[request], &[], 0)?;
        owner_result(
            owner
                .lock()
                .unwrap()
                .receipt(GENERATION, &[CaptureId(1)], Some(count)),
        )?;
        assert_eq!(peer.submittedCount()?, 1);
        peer.returnResult(10, 0)?;
        peer.returnResult(10, 1)?;
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let observed = owner_result(owner.lock().unwrap().snapshot(CaptureId(1), LeaseId(1)))?;
            if observed.returned && observed.capture_complete {
                break;
            }
            if Instant::now() >= until {
                return Err("generated asynchronous callbacks did not settle capture".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let pending = owner_result(owner.lock().unwrap().snapshot(CaptureId(1), LeaseId(1)))?;
        assert!(
            pending.returned
                && pending.capture_complete
                && !pending.fence_ready
                && !pending.publication_eligible
                && pending.receipt == Receipt::Accepted
        );
        let fence = owner_result(owner.lock().unwrap().fence_probe(CaptureId(1), LeaseId(1)))?
            .ok_or("missing fence")?;
        let mut fds = [rustix::event::PollFd::new(
            &fence,
            rustix::event::PollFlags::IN,
        )];
        assert_eq!(
            rustix::event::poll(
                &mut fds,
                Some(&rustix::event::Timespec {
                    tv_sec: 0,
                    tv_nsec: 0
                })
            )?,
            0
        );
        peer.signalFence(10)?;
        assert_eq!(
            rustix::event::poll(
                &mut fds,
                Some(&rustix::event::Timespec {
                    tv_sec: 1,
                    tv_nsec: 0
                })
            )?,
            1
        );
        let mut signal = [0u8; 1];
        assert_eq!(rustix::io::read(&fence, &mut signal)?, 1);
        assert_eq!(&signal, b"F");
        owner_result(
            owner
                .lock()
                .unwrap()
                .fence_signaled(GENERATION, CaptureId(1), LeaseId(1)),
        )?;
        assert!(
            owner_result(owner.lock().unwrap().snapshot(CaptureId(1), LeaseId(1)))?
                .publication_eligible
        );
        peer.releaseAll()?;
        assert_eq!(peer.fdCount()?, baseline);
        println!(
            "{}",
            serde_json::json!({"remote":true,"frozen_callback_version":4,"frozen_callback_hash":CAMERA_HASH,"submitted":1,"owned_fd":true,"completion_before_fence":true,"explicit_pipe_signal":true,"peer_fd_budget":true,"vendor_execution":false,"sync_file_verified":false})
        );
    } else {
        return Err("unknown probe command".into());
    }
    Ok(())
}
