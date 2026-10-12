//! Actual frozen camera callbacks across real Binder into the device-neutral Rust owner.
use pokelinux_camera::{CaptureId, Generation, LeaseId, Owner, OwnerError, Receipt};
use pokelinux_camera_boundary::{
    BnCameraDeviceCallback, BufferCache, BufferRequest, BufferRequestStatus, BufferStatus,
    CaptureRequest, CaptureResult, ErrorCode, ICameraDeviceCallback, ICameraOwnershipProbe,
    NotifyMsg, StreamBuffer, StreamBufferRet,
};
use rsbinder::hub;
use rsbinder::{
    Binder, FromIBinder, Interface, Parcel, ParcelFileDescriptor, ProcessState, Remotable, Result,
    SIBinder, Status, StatusCode, Strong,
};
use std::{
    collections::BTreeMap,
    os::fd::OwnedFd,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};
const PACKAGE: &str = "org.pokelinux.camerapeer";
const COMPONENT: &str = "org.pokelinux.camerapeer.OwnershipPeer";
const DESCRIPTOR: &str = "org.pokelinux.cameratest.ICameraOwnershipProbe";
const GENERATION: Generation = Generation {
    session: 1,
    configuration: 1,
};
struct Watcher(SyncSender<Result<SIBinder>>);
fn peer_from_status(data: &mut Parcel) -> Result<SIBinder> {
    if data.read_i32()? != 1
        || data.read::<String>()? != PACKAGE
        || data.read::<String>()? != COMPONENT
        || data.read_i32()? != 37
        || data.read_i32()? != 1
    {
        return Err(StatusCode::BadValue);
    }
    let length = usize::try_from(data.read_i32()?).map_err(|_| StatusCode::BadValue)?;
    if length > data.data_avail() || data.read_i32()? != 0x4c444e42 {
        return Err(StatusCode::BadValue);
    }
    let begin = data.data_position();
    if data.read_i32()? != 1 || data.read::<String>()? != "peer" || data.read_i32()? != 15 {
        return Err(StatusCode::BadValue);
    }
    let peer: SIBinder = data.read()?;
    if data.data_position() - begin != length || data.read_i32()? != 0 || data.data_avail() != 0 {
        return Err(StatusCode::BadValue);
    }
    if !peer.is_remote() || peer.descriptor() != DESCRIPTOR {
        return Err(StatusCode::BadType);
    }
    Ok(peer)
}
impl Remotable for Watcher {
    fn descriptor() -> &'static str {
        "android.app.IInstrumentationWatcher"
    }
    fn on_transact(&self, code: u32, data: &mut Parcel, reply: &mut Parcel) -> Result<()> {
        if code != 1 {
            return Err(StatusCode::UnknownTransaction);
        }
        self.0
            .try_send(peer_from_status(data))
            .map_err(|_| StatusCode::FailedTransaction)?;
        reply.write_i32(0)
    }
    fn on_dump(&self, _: &mut dyn std::io::Write, _: &[String]) -> Result<()> {
        Err(StatusCode::InvalidOperation)
    }
}
fn bootstrap() -> std::result::Result<SIBinder, Box<dyn std::error::Error>> {
    ProcessState::init("/dev/binder", 2)?;
    ProcessState::start_thread_pool();
    let activity = hub::try_get_service("activity")?.ok_or("activity absent")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let watcher = Binder::new(Watcher(sender));
    let mut data = activity
        .as_remote()
        .ok_or("activity is local")?
        .prepare_transact(true)?;
    data.write_i32(1)?;
    data.write(PACKAGE)?;
    data.write(COMPONENT)?;
    data.write(&None::<String>)?;
    data.write_i32(0)?;
    data.write_i32(0)?;
    data.write(&watcher.as_binder())?;
    data.write(&None::<SIBinder>)?;
    data.write_i32(0)?;
    data.write(&None::<String>)?;
    let mut reply = activity
        .as_remote()
        .ok_or("activity is local")?
        .submit_transact(51, &data, 0)?
        .ok_or("no start reply")?;
    let status: Status = reply.read()?;
    if !status.is_ok() {
        return Err(status.into());
    }
    if reply.read_i32()? != 1 || reply.data_avail() != 0 {
        return Err("instrumentation refused".into());
    }
    let peer = receiver.recv_timeout(Duration::from_secs(5))??;
    peer.ping_binder()?;

    Ok(peer)
}
struct State {
    owner: Mutex<Owner>,
    mapping: Mutex<BTreeMap<i32, (CaptureId, LeaseId)>>,
    ignored: AtomicUsize,
    duplicates: AtomicUsize,
}
struct Callback(Arc<State>);
impl Interface for Callback {}
impl Callback {
    fn returned(&self, frame: i32, buffer: &StreamBuffer) -> rsbinder::BinderResult<()> {
        if buffer.streamId != 7
            || buffer.bufferId != 99
            || !buffer.buffer.fds.is_empty()
            || !buffer.acquireFence.fds.is_empty()
        {
            self.0.ignored.fetch_add(1, Ordering::SeqCst);
            return Ok(());
        }
        let Some((id, lease)) = self.0.mapping.lock().unwrap().get(&frame).copied() else {
            self.0.ignored.fetch_add(1, Ordering::SeqCst);
            return Ok(());
        };
        if buffer.releaseFence.fds.len() > 1 || !buffer.releaseFence.ints.is_empty() {
            return Err(StatusCode::BadValue.into());
        }
        let fence = buffer
            .releaseFence
            .fds
            .first()
            .map(ParcelFileDescriptor::try_clone)
            .transpose()?
            .map(OwnedFd::from);
        let result = self.0.owner.lock().unwrap().returned(
            GENERATION,
            id,
            lease,
            buffer.status == BufferStatus::ERROR,
            fence,
        );
        match result {
            Ok(true) => {}
            Ok(false) => {
                self.0.duplicates.fetch_add(1, Ordering::SeqCst);
            }
            Err(_) => {
                self.0.ignored.fetch_add(1, Ordering::SeqCst);
            }
        }
        Ok(())
    }
}
impl ICameraDeviceCallback for Callback {
    fn notify(&self, msgs: &[NotifyMsg]) -> rsbinder::BinderResult<()> {
        for message in msgs {
            match message {
                NotifyMsg::Error(error) => {
                    if let Some((id, _)) = self
                        .0
                        .mapping
                        .lock()
                        .unwrap()
                        .get(&error.frameNumber)
                        .copied()
                        && (error.errorCode == ErrorCode::ERROR_REQUEST
                            || error.errorCode == ErrorCode::ERROR_DEVICE)
                    {
                        let _ = self.0.owner.lock().unwrap().failed(GENERATION, id);
                    }
                }
                NotifyMsg::Shutter(shutter) => {
                    assert_eq!(shutter.timestamp, 100);
                    assert_eq!(shutter.readoutTimestamp, 101);
                    assert_eq!(shutter.streamGroupState.len(), 1);
                    assert_eq!(shutter.streamGroupState[0].activeStreamIds, [7]);
                    if let Some((id, _)) = self
                        .0
                        .mapping
                        .lock()
                        .unwrap()
                        .get(&shutter.frameNumber)
                        .copied()
                    {
                        let _ = self.0.owner.lock().unwrap().shutter(GENERATION, id);
                    }
                }
            }
        }
        Ok(())
    }
    fn processCaptureResult(&self, results: &[CaptureResult]) -> rsbinder::BinderResult<()> {
        for result in results {
            assert_eq!(result.fmqResultSize, 0);
            assert_eq!(result.inputBuffer.streamId, -1);
            for buffer in &result.outputBuffers {
                self.returned(result.frameNumber, buffer)?;
            }
            if result.partialResult == 1 {
                assert_eq!(result.result.metadata, [42]);
                assert_eq!(result.physicalCameraMetadata.len(), 1);
                assert_eq!(
                    result.physicalCameraMetadata[0].physicalCameraId,
                    "synthetic-physical"
                );
                assert_eq!(result.physicalCameraMetadata[0].metadata.metadata, [43]);
                if let Some((id, _)) = self
                    .0
                    .mapping
                    .lock()
                    .unwrap()
                    .get(&result.frameNumber)
                    .copied()
                {
                    let _ = self.0.owner.lock().unwrap().final_metadata(GENERATION, id);
                }
            }
        }
        Ok(())
    }
    fn requestStreamBuffers(
        &self,
        _: &[BufferRequest],
        _: &mut Vec<StreamBufferRet>,
    ) -> rsbinder::BinderResult<BufferRequestStatus> {
        Err(StatusCode::InvalidOperation.into())
    }
    fn returnStreamBuffers(&self, buffers: &[StreamBuffer]) -> rsbinder::BinderResult<()> {
        // Not used to infer a request frame: a HAL cache return is separate accounting.
        if !buffers.is_empty() {
            self.0.ignored.fetch_add(buffers.len(), Ordering::SeqCst);
        }
        Ok(())
    }
}
fn new_state() -> Arc<State> {
    Arc::new(State {
        owner: Mutex::new(Owner::new(GENERATION, 2, 8)),
        mapping: Mutex::new(BTreeMap::new()),
        ignored: AtomicUsize::new(0),
        duplicates: AtomicUsize::new(0),
    })
}
fn request(frame: i32, writer: Option<ParcelFileDescriptor>) -> CaptureRequest {
    let mut request = CaptureRequest {
        frameNumber: frame,
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
    if let Some(fd) = writer {
        buffer.buffer.fds.push(fd);
        buffer.buffer.ints = vec![37, 4];
    }
    request.outputBuffers.push(buffer);
    request
}
fn new_frame(
    state: &State,
    frame: i32,
    id: CaptureId,
    lease: LeaseId,
) -> rsbinder::Result<(ParcelFileDescriptor, CaptureRequest)> {
    let (reader, writer) = ParcelFileDescriptor::pipe()?;
    state
        .owner
        .lock()
        .unwrap()
        .admit(GENERATION, id, lease, writer.try_clone()?.into())
        .unwrap();
    state.mapping.lock().unwrap().insert(frame, (id, lease));
    Ok((reader, request(frame, Some(writer))))
}
fn receipt(state: &State, id: CaptureId, result: rsbinder::BinderResult<i32>) {
    state
        .owner
        .lock()
        .unwrap()
        .receipt(GENERATION, &[id], result.ok())
        .unwrap();
}
fn snapshot(state: &State, id: CaptureId, lease: LeaseId) -> pokelinux_camera::Snapshot {
    state.owner.lock().unwrap().snapshot(id, lease).unwrap()
}
fn signal(
    peer: &dyn ICameraOwnershipProbe,
    state: &State,
    frame: i32,
    id: CaptureId,
    lease: LeaseId,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let fd = state
        .owner
        .lock()
        .unwrap()
        .fence_probe(id, lease)
        .unwrap()
        .unwrap();
    let mut polls = [rustix::event::PollFd::new(
        &fd,
        rustix::event::PollFlags::IN,
    )];
    let zero = rustix::event::Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        rustix::event::poll(&mut polls, Some(&zero))?,
        0,
        "release signal must genuinely be pending"
    );
    peer.signalFence(frame)?;
    let wait = rustix::event::Timespec {
        tv_sec: 2,
        tv_nsec: 0,
    };
    assert_eq!(
        rustix::event::poll(&mut polls, Some(&wait))?,
        1,
        "bounded synthetic release FD"
    );
    let mut marker = [0];
    assert_eq!(rustix::io::read(&fd, &mut marker)?, 1);
    assert_eq!(
        marker,
        [b'F'],
        "only the synthetic F receipt signals this fixture fence"
    );
    state
        .owner
        .lock()
        .unwrap()
        .fence_signaled(GENERATION, id, lease)
        .unwrap();
    Ok(())
}
struct Death(Arc<State>, SyncSender<()>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.0.owner.lock().unwrap().service_died();
        let _ = self.1.try_send(());
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let binder = bootstrap()?;
    let peer: Strong<dyn ICameraOwnershipProbe> = FromIBinder::try_from(binder.clone())?;
    let baseline = peer.fdCount()?;
    assert!(baseline > 0);
    println!("camera-owner-frozen-bootstrap\tPASS");
    let state = new_state();
    let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
    peer.setCallback(&callback)?;
    let (_reader, capture_request) = new_frame(&state, 10, CaptureId(1), LeaseId(1))?;
    receipt(
        &state,
        CaptureId(1),
        peer.processCaptureRequest(&[capture_request], &[], 0),
    );
    assert_eq!(
        snapshot(&state, CaptureId(1), LeaseId(1)).receipt,
        Receipt::Accepted
    );
    peer.returnResult(10, 0)?;
    let pending = snapshot(&state, CaptureId(1), LeaseId(1));
    assert!(pending.returned);
    assert!(!pending.capture_complete);
    assert!(
        !pending.fence_ready && !pending.reusable,
        "returned buffer cannot be reused before release signal"
    );
    peer.returnResult(10, 1)?;
    let complete = snapshot(&state, CaptureId(1), LeaseId(1));
    assert!(
        complete.capture_complete && !complete.publication_eligible && !complete.reusable,
        "capture completion precedes fence-ready reuse"
    );
    signal(peer.as_ref(), &state, 10, CaptureId(1), LeaseId(1))?;
    assert!(snapshot(&state, CaptureId(1), LeaseId(1)).publication_eligible);
    peer.returnResult(10, 0)?;
    assert_eq!(state.duplicates.load(Ordering::SeqCst), 1);
    assert_eq!(snapshot(&state, CaptureId(1), LeaseId(1)).returns, 1);
    state
        .owner
        .lock()
        .unwrap()
        .reuse(GENERATION, CaptureId(2), LeaseId(1))
        .unwrap();
    state
        .mapping
        .lock()
        .unwrap()
        .insert(11, (CaptureId(2), LeaseId(1)));
    let cached = request(11, None);
    receipt(
        &state,
        CaptureId(2),
        peer.processCaptureRequest(&[cached], &[], 0),
    );
    peer.returnResult(10, 0)?;
    assert_eq!(
        snapshot(&state, CaptureId(2), LeaseId(1)).returns,
        1,
        "stale frame cannot release reused lease"
    );
    peer.returnResult(11, 4)?;
    assert!(
        !snapshot(&state, CaptureId(2), LeaseId(1)).returned,
        "unknown buffer must not release owned lease"
    );
    peer.returnResult(11, 2)?;
    let error = snapshot(&state, CaptureId(2), LeaseId(1));
    assert!(
        error.returned
            && error.capture_complete
            && !error.publication_eligible
            && !error.fence_ready
    );
    signal(peer.as_ref(), &state, 11, CaptureId(2), LeaseId(1))?;
    assert!(snapshot(&state, CaptureId(2), LeaseId(1)).reusable);
    println!("camera-owner-return-complete-fence\tPASS");
    println!("camera-owner-cached-error-duplicate\tPASS");
    let remove = BufferCache {
        streamId: 7,
        bufferId: 99,
    };
    assert_eq!(peer.processCaptureRequest(&[], &[remove], 0)?, 0);
    let cache_error = peer
        .processCaptureRequest(&[request(12, None)], &[], 0)
        .unwrap_err();
    assert_eq!(cache_error.service_specific_error(), 23);
    println!("camera-owner-cache-removal\tPASS");
    peer.releaseAll()?;
    drop(callback);
    drop(state);
    assert_eq!(
        peer.fdCount()?,
        baseline,
        "fixture releases cache/frame/fence descriptors"
    );
    for (frame, mode) in [(20, 1), (21, 3), (22, 4)] {
        let state = new_state();
        let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
        peer.setCallback(&callback)?;
        let (_reader, capture_request) = new_frame(&state, frame, CaptureId(1), LeaseId(1))?;
        receipt(
            &state,
            CaptureId(1),
            peer.processCaptureRequest(&[capture_request], &[], mode),
        );
        let status = snapshot(&state, CaptureId(1), LeaseId(1));
        assert_eq!(
            status.receipt,
            if mode == 1 {
                Receipt::Uncertain
            } else {
                Receipt::InvalidCount
            }
        );
        if mode != 1 {
            assert!(status.quarantined);
        }
        peer.returnResult(frame, 0)?;
        peer.returnResult(frame, 1)?;
        signal(peer.as_ref(), &state, frame, CaptureId(1), LeaseId(1))?;
        let status = snapshot(&state, CaptureId(1), LeaseId(1));
        assert_eq!(status.publication_eligible, mode == 1);
        assert_eq!(status.quarantined, mode != 1);
        peer.releaseAll()?;
    }
    println!("camera-owner-error-invalid-count\tPASS");
    for (frame, mode, expected) in [
        (70, 0, Receipt::Accepted),
        (71, 6, Receipt::Rejected),
        (72, 3, Receipt::InvalidCount),
        (73, 1, Receipt::Uncertain),
    ] {
        let state = new_state();
        let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
        peer.setCallback(&callback)?;
        let (_reader, capture_request) = new_frame(&state, frame, CaptureId(1), LeaseId(1))?;
        let observed = peer
            .processCaptureRequest(&[capture_request], &[], mode)
            .ok();
        state
            .owner
            .lock()
            .unwrap()
            .receipt(GENERATION, &[CaptureId(1)], observed)
            .unwrap();
        assert_eq!(snapshot(&state, CaptureId(1), LeaseId(1)).receipt, expected);
        assert_eq!(
            state
                .owner
                .lock()
                .unwrap()
                .receipt(GENERATION, &[CaptureId(1)], observed),
            Err(OwnerError::Duplicate),
            "resolved receipt is single-shot despite duplicate delivery of real Binder result"
        );
        if mode == 6 {
            peer.returnResult(frame, 0)?;
            peer.returnResult(frame, 1)?;
            signal(peer.as_ref(), &state, frame, CaptureId(1), LeaseId(1))?;
            let late_count = peer.processCaptureRequest(&[], &[], 3)?;
            assert_eq!(late_count, 1);
            assert_eq!(
                state
                    .owner
                    .lock()
                    .unwrap()
                    .receipt(GENERATION, &[CaptureId(1)], Some(late_count)),
                Err(OwnerError::Duplicate)
            );
            assert!(
                !snapshot(&state, CaptureId(1), LeaseId(1)).publication_eligible,
                "rejected receipt cannot become publishable after late accepted count"
            );
        }
        peer.releaseAll()?;
    }
    let state = new_state();
    let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
    peer.setCallback(&callback)?;
    let (_reader, capture_request) = new_frame(&state, 74, CaptureId(1), LeaseId(1))?;
    let observed = peer.processCaptureRequest(&[capture_request], &[], 0)?;
    assert_eq!(
        state.owner.lock().unwrap().receipt(
            GENERATION,
            &[CaptureId(1), CaptureId(1)],
            Some(observed)
        ),
        Err(OwnerError::Duplicate),
        "duplicate capture ID batch refused atomically"
    );
    assert_eq!(
        snapshot(&state, CaptureId(1), LeaseId(1)).receipt,
        Receipt::Pending
    );
    state
        .owner
        .lock()
        .unwrap()
        .receipt(GENERATION, &[CaptureId(1)], Some(observed))
        .unwrap();
    peer.releaseAll()?;
    println!("camera-owner-single-shot-receipts\tPASS");
    let state = new_state();
    let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
    peer.setCallback(&callback)?;
    let (_reader, capture_request) = new_frame(&state, 60, CaptureId(1), LeaseId(1))?;
    receipt(
        &state,
        CaptureId(1),
        peer.processCaptureRequest(&[capture_request], &[], 0),
    );
    peer.returnResult(60, 0)?;
    signal(peer.as_ref(), &state, 60, CaptureId(1), LeaseId(1))?;
    assert!(!snapshot(&state, CaptureId(1), LeaseId(1)).capture_complete);
    state
        .owner
        .lock()
        .unwrap()
        .reuse(GENERATION, CaptureId(2), LeaseId(1))
        .unwrap();
    state
        .mapping
        .lock()
        .unwrap()
        .insert(61, (CaptureId(2), LeaseId(1)));
    receipt(
        &state,
        CaptureId(2),
        peer.processCaptureRequest(&[request(61, None)], &[], 0),
    );
    peer.returnResult(60, 1)?;
    let historical = state
        .owner
        .lock()
        .unwrap()
        .snapshot(CaptureId(1), LeaseId(1));
    assert!(
        matches!(historical, Ok(status) if status.capture_complete && status.returned && status.fence_ready && !status.reusable && !status.publication_eligible),
        "late metadata completion survives cached lease reuse"
    );
    let current = snapshot(&state, CaptureId(2), LeaseId(1));
    assert!(
        !current.returned && !current.capture_complete && !current.fence_ready,
        "old metadata cannot complete the new lease owner"
    );
    peer.returnResult(60, 0)?;
    assert!(!snapshot(&state, CaptureId(2), LeaseId(1)).returned);
    state.owner.lock().unwrap().revoke();
    peer.releaseAll()?;
    println!("camera-owner-late-metadata-after-reuse\tPASS");
    let state = new_state();
    let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
    peer.setCallback(&callback)?;
    let (_reader, capture_request) = new_frame(&state, 30, CaptureId(1), LeaseId(1))?;
    let transport = peer.clone();
    let worker =
        std::thread::spawn(move || transport.processCaptureRequest(&[capture_request], &[], 5));
    let end = Instant::now() + Duration::from_secs(2);
    while !peer.held()? {
        assert!(Instant::now() < end, "held submit entry");
        std::thread::yield_now();
    }
    let before = Instant::now();
    state.owner.lock().unwrap().revoke();
    let revoked = snapshot(&state, CaptureId(1), LeaseId(1));
    assert!(
        !revoked.publication_eligible
            && revoked.quarantined
            && before.elapsed() < Duration::from_millis(100),
        "revoke/snapshot stay responsive during Binder hold"
    );
    assert_eq!(
        state
            .owner
            .lock()
            .unwrap()
            .replace_configuration(Generation {
                session: 1,
                configuration: 2
            }),
        Err(OwnerError::NotReusable)
    );
    peer.releaseHold()?;
    receipt(&state, CaptureId(1), worker.join().unwrap());
    peer.returnResult(30, 0)?;
    peer.returnResult(30, 1)?;
    signal(peer.as_ref(), &state, 30, CaptureId(1), LeaseId(1))?;
    let revoked = snapshot(&state, CaptureId(1), LeaseId(1));
    assert!(
        revoked.returned
            && revoked.capture_complete
            && revoked.fence_ready
            && revoked.quarantined
            && !revoked.publication_eligible
            && !revoked.reusable
    );
    assert_eq!(
        state
            .owner
            .lock()
            .unwrap()
            .replace_configuration(Generation {
                session: 1,
                configuration: 2
            }),
        Err(OwnerError::NotReusable)
    );
    assert_eq!(
        state.owner.lock().unwrap().fence_signaled(
            Generation {
                session: 1,
                configuration: 2
            },
            CaptureId(1),
            LeaseId(1)
        ),
        Err(OwnerError::WrongGeneration)
    );
    println!("camera-owner-held-revoke-generation\tPASS");
    peer.releaseAll()?;
    let state = new_state();
    let callback = BnCameraDeviceCallback::new_binder(Callback(state.clone()));
    peer.setCallback(&callback)?;
    let (_reader, capture_request) = new_frame(&state, 40, CaptureId(1), LeaseId(1))?;
    receipt(
        &state,
        CaptureId(1),
        peer.processCaptureRequest(&[capture_request], &[], 0),
    );
    peer.returnResult(40, 0)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let death = Arc::new(Death(state.clone(), sender));
    binder.link_to_death_arc(&death)?;
    assert_eq!(
        peer.die().unwrap_err().transaction_error(),
        StatusCode::DeadObject
    );
    receiver.recv_timeout(Duration::from_secs(5))?;
    let death_fence = state
        .owner
        .lock()
        .unwrap()
        .fence_probe(CaptureId(1), LeaseId(1))
        .unwrap()
        .unwrap();
    let mut polls = [rustix::event::PollFd::new(
        &death_fence,
        rustix::event::PollFlags::IN | rustix::event::PollFlags::HUP,
    )];
    let wait = rustix::event::Timespec {
        tv_sec: 2,
        tv_nsec: 0,
    };
    assert_eq!(rustix::event::poll(&mut polls, Some(&wait))?, 1);
    assert_eq!(
        rustix::io::read(&death_fence, &mut [0])?,
        0,
        "peer death yields pipe EOF, not a release-signal receipt"
    );
    let status = snapshot(&state, CaptureId(1), LeaseId(1));
    assert!(
        status.returned
            && !status.fence_ready
            && status.quarantined
            && !status.reusable
            && !status.publication_eligible
    );
    assert_eq!(
        state.owner.lock().unwrap().retained_leases(),
        1,
        "process death does not release quarantined buffer lease"
    );
    assert_eq!(
        state
            .owner
            .lock()
            .unwrap()
            .replace_configuration(Generation {
                session: 2,
                configuration: 1
            }),
        Err(OwnerError::NotReusable)
    );
    println!("camera-owner-death-quarantine\tPASS");
    Ok(())
}
