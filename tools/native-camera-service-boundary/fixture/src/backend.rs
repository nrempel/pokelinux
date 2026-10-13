//! Only this authored adapter owns frozen CameraV4/Binder IDs and synthetic pipe policy.
use crate::*;
use pokelinux_camera::{LeaseId, Owner};
use pokelinux_camera_service::worker::{Backend, Control};
use pokelinux_lifecycle::{boottime_ns, camera::Intent};
use rsbinder::{FromIBinder, Interface, ParcelFileDescriptor, ProcessState};
use std::{
    io,
    os::fd::OwnedFd,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Deferred {
    fault: String,
    retained: Option<Retained>,
}
struct Retained {
    _peer: rsbinder::Strong<dyn ICameraOwnershipProbe>,
    _callback: rsbinder::Strong<dyn ICameraDeviceCallback>,
    _death: Arc<Death>,
}
struct Death {
    owner: Arc<Mutex<Owner>>,
    control: Control,
}
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.owner.lock().unwrap().service_died();
        let _ = self.control.fail();
        std::process::exit(2);
    }
}
impl Deferred {
    pub fn new(fault: String) -> io::Result<Self> {
        if !matches!(
            fault.as_str(),
            "none" | "held" | "late-before-lookup" | "exit-before-reply"
        ) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            fault,
            retained: None,
        })
    }
}
struct Callback {
    owner: Arc<Mutex<Owner>>,
    intent: Intent,
}
impl Interface for Callback {}
impl ICameraDeviceCallback for Callback {
    fn notify(&self, messages: &[NotifyMsg]) -> rsbinder::BinderResult<()> {
        if messages.len() > 8 {
            return Err(rsbinder::StatusCode::BadValue.into());
        }
        for message in messages {
            if let NotifyMsg::Shutter(shutter) = message
                && shutter.frameNumber == 10
            {
                let _ = self
                    .owner
                    .lock()
                    .unwrap()
                    .shutter(self.intent.generation, self.intent.capture);
            }
        }
        Ok(())
    }
    fn processCaptureResult(&self, results: &[CaptureResult]) -> rsbinder::BinderResult<()> {
        if results.len() > 8 {
            return Err(rsbinder::StatusCode::BadValue.into());
        }
        for result in results {
            if result.frameNumber != 10
                || result.fmqResultSize != 0
                || result.outputBuffers.len() > 1
            {
                continue;
            }
            for buffer in &result.outputBuffers {
                if buffer.streamId != 7
                    || buffer.bufferId != 99
                    || buffer.releaseFence.fds.len() > 1
                    || !buffer.releaseFence.ints.is_empty()
                {
                    continue;
                }
                let fence = buffer
                    .releaseFence
                    .fds
                    .first()
                    .map(|fd| fd.try_clone())
                    .transpose()?
                    .map(OwnedFd::from);
                let _ = self.owner.lock().unwrap().returned(
                    self.intent.generation,
                    self.intent.capture,
                    LeaseId(1),
                    buffer.status == BufferStatus::ERROR,
                    fence,
                );
            }
            if result.partialResult == 1 && result.result.metadata == [42] {
                let _ = self
                    .owner
                    .lock()
                    .unwrap()
                    .final_metadata(self.intent.generation, self.intent.capture);
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
fn error(value: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("synthetic camera {value:?}"))
}
impl Backend for Deferred {
    fn capture(&mut self, intent: &Intent, control: &Control) -> io::Result<()> {
        if self.fault == "late-before-lookup" {
            while boottime_ns() < intent.until_ns {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        // No driver open, lookup, registration or submit before authorized command.
        control.check()?;
        let process = ProcessState::init("/dev/binder", 3).map_err(error)?;
        ProcessState::start_thread_pool();
        control.check()?;
        let binder = lookup(process).map_err(error)?;
        let peer: rsbinder::Strong<dyn ICameraOwnershipProbe> =
            FromIBinder::try_from(binder).map_err(error)?;
        let (reader, writer) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)?;
        let owner = Arc::new(Mutex::new(Owner::new(intent.generation, 1, 1)));
        owner
            .lock()
            .unwrap()
            .admit(intent.generation, intent.capture, LeaseId(1), reader)
            .map_err(error)?;
        let callback = BnCameraDeviceCallback::new_binder(Callback {
            owner: owner.clone(),
            intent: *intent,
        });
        let death = Arc::new(Death {
            owner: owner.clone(),
            control: control.clone(),
        });
        control.check()?;
        peer.as_binder().link_to_death_arc(&death).map_err(error)?;
        control.check()?;
        peer.setCallback(&callback).map_err(error)?;
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
        control.check()?;
        let count = peer
            .processCaptureRequest(&[request], &[], if self.fault == "held" { 5 } else { 0 })
            .ok();
        owner
            .lock()
            .unwrap()
            .receipt(intent.generation, &[intent.capture], count)
            .map_err(error)?;
        if self.fault == "exit-before-reply" {
            std::process::exit(0);
        }
        control.check()?;
        let snapshot = owner
            .lock()
            .unwrap()
            .snapshot(intent.capture, LeaseId(1))
            .map_err(error)?;
        control.report(snapshot.into())?;
        self.retained = Some(Retained {
            _peer: peer,
            _callback: callback,
            _death: death,
        });
        Ok(())
    }
}
