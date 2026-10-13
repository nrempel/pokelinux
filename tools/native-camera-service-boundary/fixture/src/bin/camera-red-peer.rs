//! Independent generated public CameraV4 peer. No Bionic or original/vendor DSOs.
use generated::android::hardware::camera::device::{
    ShutterMsg::ShutterMsg, StreamGroupState::StreamGroupState,
};
use native_camera_service_boundary_fixture::*;
use rsbinder::{Interface, ParcelFileDescriptor, ProcessState, Strong};
use std::{
    collections::BTreeMap,
    io::Write,
    os::fd::OwnedFd,
    sync::{Condvar, Mutex},
};
struct Frame {
    _buffer: OwnedFd,
    fence_read: Option<OwnedFd>,
    fence_write: Option<OwnedFd>,
}
#[derive(Default)]
struct State {
    callback: Option<Strong<dyn ICameraDeviceCallback>>,
    frames: BTreeMap<i32, Frame>,
    submitted: i32,
    registered: i32,
    holding: bool,
}
struct Peer(Mutex<State>, Condvar);
impl Interface for Peer {}
fn bad(code: i32) -> rsbinder::Status {
    rsbinder::Status::new_service_specific_error(code, None)
}
impl ICameraOwnershipProbe for Peer {
    fn setCallback(
        &self,
        callback: &Strong<dyn ICameraDeviceCallback>,
    ) -> rsbinder::BinderResult<()> {
        if !callback.as_binder().is_remote()
            || callback.getInterfaceVersion()? != 4
            || callback.getInterfaceHash()? != CAMERA_HASH
        {
            return Err(bad(29));
        }
        let mut state = self.0.lock().unwrap();
        state.callback = Some(callback.clone());
        state.registered += 1;
        Ok(())
    }
    fn processCaptureRequest(
        &self,
        requests: &[CaptureRequest],
        remove: &[BufferCache],
        mode: i32,
    ) -> rsbinder::BinderResult<i32> {
        let mut state = self.0.lock().unwrap();
        if state.callback.is_none()
            || requests.len() != 1
            || !remove.is_empty()
            || !matches!(mode, 0 | 5)
            || state.submitted >= 8
        {
            return Err(bad(20));
        }
        let request = &requests[0];
        if request.outputBuffers.len() != 1
            || request.inputBuffer.streamId != -1
            || request.fmqSettingsSize != 0
            || request.settings.metadata != [1, 2, 3]
            || state.frames.contains_key(&request.frameNumber)
        {
            return Err(bad(21));
        }
        let buffer = &request.outputBuffers[0];
        if buffer.streamId != 7
            || buffer.bufferId != 99
            || buffer.status != BufferStatus::OK
            || buffer.buffer.fds.len() != 1
            || buffer.buffer.ints != [37, 4]
            || !buffer.acquireFence.fds.is_empty()
            || !buffer.releaseFence.fds.is_empty()
        {
            return Err(bad(22));
        }
        let retained = OwnedFd::from(buffer.buffer.fds[0].try_clone()?);
        state.frames.insert(
            request.frameNumber,
            Frame {
                _buffer: retained,
                fence_read: None,
                fence_write: None,
            },
        );
        state.submitted += 1;
        if mode == 5 {
            state.holding = true;
            while state.holding {
                state = self.1.wait(state).unwrap();
            }
        }
        Ok(1)
    }
    fn returnResult(&self, frame: i32, behavior: i32) -> rsbinder::BinderResult<()> {
        let (callback, result, notification) = {
            let mut state = self.0.lock().unwrap();
            let callback = state.callback.as_ref().ok_or_else(|| bad(25))?.clone();
            let owned = state.frames.get_mut(&frame).ok_or_else(|| bad(25))?;
            let mut result = CaptureResult {
                frameNumber: frame,
                ..CaptureResult::default()
            };
            result.inputBuffer.streamId = -1;
            let notification = if behavior == 1 {
                result.partialResult = 1;
                result.result.metadata = vec![42];
                let mut shutter = ShutterMsg {
                    frameNumber: frame,
                    timestamp: 100,
                    readoutTimestamp: 101,
                    ..ShutterMsg::default()
                };
                shutter.streamGroupState = vec![StreamGroupState {
                    groupId: 4,
                    activeStreamIds: vec![7],
                }];
                Some(NotifyMsg::Shutter(shutter))
            } else if behavior == 0 {
                if owned.fence_read.is_none() {
                    let (read, write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
                        .map_err(rsbinder::StatusCode::from)?;
                    owned.fence_read = Some(read);
                    owned.fence_write = Some(write);
                }
                let mut buffer = StreamBuffer {
                    streamId: 7,
                    bufferId: 99,
                    status: BufferStatus::OK,
                    ..StreamBuffer::default()
                };
                buffer.releaseFence.fds = vec![ParcelFileDescriptor::new(
                    owned.fence_read.as_ref().unwrap().try_clone()?,
                )];
                result.outputBuffers = vec![buffer];
                None
            } else {
                return Err(bad(25));
            };
            (callback, result, notification)
        };
        if let Some(notification) = notification {
            callback.notify(&[notification])?;
        }
        callback.processCaptureResult(&[result])
    }
    fn signalFence(&self, frame: i32) -> rsbinder::BinderResult<()> {
        let mut state = self.0.lock().unwrap();
        let frame = state.frames.get_mut(&frame).ok_or_else(|| bad(25))?;
        let writer = frame.fence_write.take().ok_or_else(|| bad(26))?;
        if rustix::io::write(&writer, b"F").map_err(rsbinder::StatusCode::from)? != 1 {
            return Err(bad(26));
        }
        Ok(())
    }
    fn held(&self) -> rsbinder::BinderResult<bool> {
        Ok(self.0.lock().unwrap().holding)
    }
    fn releaseHold(&self) -> rsbinder::BinderResult<()> {
        self.0.lock().unwrap().holding = false;
        self.1.notify_all();
        Ok(())
    }
    fn releaseAll(&self) -> rsbinder::BinderResult<()> {
        self.0.lock().unwrap().frames.clear();
        Ok(())
    }
    fn fdCount(&self) -> rsbinder::BinderResult<i32> {
        Ok(std::fs::read_dir("/proc/self/fd")
            .map_err(|_| bad(27))?
            .count() as i32)
    }
    fn die(&self) -> rsbinder::BinderResult<()> {
        std::process::exit(0)
    }
    fn submittedCount(&self) -> rsbinder::BinderResult<i32> {
        Ok(self.0.lock().unwrap().submitted)
    }
    fn callbackCount(&self) -> rsbinder::BinderResult<i32> {
        Ok(self.0.lock().unwrap().registered)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let process = ProcessState::init("/dev/binder", 3)?;
    ProcessState::start_thread_pool();
    let peer =
        BnCameraOwnershipProbe::new_binder(Peer(Mutex::new(State::default()), Condvar::new()));
    let manager = process.context_object()?;
    let remote = manager.as_remote().ok_or("manager is local")?;
    let mut data = remote.prepare_transact(true)?;
    data.write(NAME)?;
    data.write(&peer.as_binder())?;
    data.write(&false)?;
    data.write_i32(8)?;
    let mut reply = remote
        .submit_transact(5, &data, 0)?
        .ok_or("missing registration reply")?;
    let status: rsbinder::Status = reply.read()?;
    if !status.is_ok() || reply.data_avail() != 0 {
        return Err("fixture registration failed".into());
    }
    println!("READY-CAMERA pid={}", std::process::id());
    std::io::stdout().flush()?;
    ProcessState::join_thread_pool()?;
    Ok(())
}
