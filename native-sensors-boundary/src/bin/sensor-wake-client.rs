//! Actual original SensorsV3 FMQ plus the same coordinator's frozen V1 Binder endpoint.
use pokelinux_sensors::wake::{AckOutcome, ReadOutcome, WakeError, WakeOwner};
use pokelinux_sensors::{ClientToken, Kind, Principal, Ticket};
use pokelinux_sensors_boundary::{
    BnSensorsCallback, Event, GrantorDescriptor, ISensorBoundaryControl, ISensors,
    ISensorsCallback, MQDescriptor, NativeHandle, SensorInfo, SynchronizedReadWrite, bootstrap,
    event_abi::{RawEvent, decode},
};
use pokelinux_suspend::{Attempt, BootClock, Clock, Coordinator, DescriptorBackend, Lifecycle};
use pokelinux_suspend_boundary::{
    BnSuspendControl, BnSystemSuspend, ISuspendBoundaryProbe,
    frontend::{Control, State, Suspend, features},
};
use rsbinder::fmq::MessageQueue;
use rsbinder::{FromIBinder, Interface, Strong};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::{
    fs::{self, OpenOptions},
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::Duration,
};
struct Callback;
impl Interface for Callback {}
impl ISensorsCallback for Callback {
    fn onDynamicSensorsConnected(&self, _: &[SensorInfo]) -> rsbinder::BinderResult<()> {
        Ok(())
    }
    fn onDynamicSensorsDisconnected(&self, _: &[i32]) -> rsbinder::BinderResult<()> {
        Ok(())
    }
}
fn wire<T: Default>(
    desc: rsbinder::fmq::Descriptor,
) -> rsbinder::Result<MQDescriptor<T, SynchronizedReadWrite>> {
    let builtin: rsbinder::fmq::MQDescriptor<T, rsbinder::fmq::SynchronizedReadWrite> =
        desc.try_into()?;
    Ok(MQDescriptor {
        grantors: builtin
            .grantors
            .into_iter()
            .map(|g| GrantorDescriptor {
                fdIndex: g.fdIndex,
                offset: g.offset,
                extent: g.extent,
            })
            .collect(),
        handle: NativeHandle {
            fds: builtin.handle.fds,
            ints: builtin.handle.ints,
        },
        quantum: builtin.quantum,
        flags: builtin.flags,
        ..Default::default()
    })
}
struct Queues {
    events: MessageQueue<RawEvent>,
    acks: MessageQueue<i32>,
}
impl Queues {
    fn new(sensor: &Strong<dyn ISensors>) -> Result<Self, Box<dyn std::error::Error>> {
        let events = MessageQueue::<RawEvent>::create(4, true)?;
        let acks = MessageQueue::<i32>::create(4, true)?;
        sensor.initialize(
            &wire::<Event>(events.descriptor()?)?,
            &wire::<i32>(acks.descriptor()?)?,
            &BnSensorsCallback::new_binder(Callback),
        )?;
        sensor.activate(77, true)?;
        Ok(Self { events, acks })
    }
    fn read(
        &mut self,
        owner: &WakeOwner,
        token: ClientToken,
        epoch: u64,
        abi: &[i32],
        coordinator: &Coordinator,
    ) -> Result<Ticket, WakeError> {
        owner.read_wake(token, epoch, || {
            let mut raw = [RawEvent::default()];
            match self.events.read(&mut raw) {
                Ok(false) => return ReadOutcome::Empty,
                Err(_) => return ReadOutcome::Uncertain,
                Ok(true) => {}
            }
            // This observation is after the actual FMQ counter advanced, before normalization.
            assert_eq!(
                coordinator.snapshot().wake_locks,
                1,
                "original FMQ consumed before native coordinator acquire"
            );
            assert_eq!(
                coordinator.attempt(),
                Attempt::Blocked,
                "actual injected state must remain blocked after queue consumption"
            );
            match decode(&raw[0], abi) {
                Ok(sample) => ReadOutcome::Sample(sample),
                Err(_) => ReadOutcome::Malformed,
            }
        })
    }
    fn ack(&mut self, owner: &WakeOwner, tickets: &[Ticket]) -> Result<(), WakeError> {
        owner.acknowledge(tickets, |plan| {
            match self.acks.write(&[plan.count]) {
                Ok(false) => return AckOutcome::Full,
                Err(_) => return AckOutcome::Uncertain,
                Ok(true) => {}
            }
            match self.acks.event_flag().unwrap().wake(1) {
                Ok(()) => AckOutcome::WrittenAndSignalled,
                Err(_) => AckOutcome::WrittenSignalFailed,
            }
        })
    }
}
struct Files {
    path: PathBuf,
    coordinator: Coordinator,
}
impl Files {
    fn new(
        root: &Path,
        label: &str,
        abort_read_only: bool,
        unlock_read_only: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = root.join(label);
        fs::create_dir(&path)?;
        let file = |name: &str, read_only: bool| -> std::io::Result<OwnedFd> {
            fs::write(path.join(name), b"")?;
            Ok(OpenOptions::new()
                .read(true)
                .write(!read_only)
                .open(path.join(name))?
                .into())
        };
        let count = file("count", false)?;
        fs::write(path.join("count"), b"7\n")?;
        let backend = DescriptorBackend::new(
            count,
            file("state", false)?,
            file("abort", abort_read_only)?,
            file("unlock", unlock_read_only)?,
        )?;
        let coordinator = Coordinator::new(backend, file("writer", false)?)
            .map_err(|e| format!("coordinator:{e:?}"))?;
        Ok(Self { path, coordinator })
    }
    fn no_mem(&self) {
        assert!(
            fs::read(self.path.join("state")).unwrap().is_empty(),
            "injected state file must not contain mem"
        );
    }
}
fn subscribe(owner: &WakeOwner) -> ClientToken {
    owner.grant(Principal(1));
    owner.subscribe(Principal(1), Kind::Acceleration).unwrap()
}
fn wait_count(coordinator: &Coordinator, expected: u32) {
    let end = BootClock.now() + Duration::from_secs(2);
    while coordinator.snapshot().wake_locks != expected {
        assert!(BootClock.now() < end, "native lease disposition deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
struct Death(WakeOwner, mpsc::SyncSender<()>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.0.backend_died();
        let _ = self.1.try_send(());
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("isolated FD directory missing")?,
    );
    fs::create_dir(&root)?;
    let ndk_client = std::env::args_os()
        .nth(2)
        .ok_or("qualified NDK V1 client missing")?;
    let sensor_binder = bootstrap::bootstrap()?;
    let peer: Strong<dyn ISensorBoundaryControl> = FromIBinder::try_from(sensor_binder.clone())?;
    let sensor = peer.sensors()?;
    assert_eq!(sensor.getInterfaceVersion()?, 3);
    assert_eq!(
        sensor.getInterfaceHash()?,
        "b051523be857524c99057b69f253a7d0bd295400"
    );
    let abi = peer.eventAbi()?;
    assert_eq!(abi, [96, 8, 0, 8, 12, 16, 1, 24, 28, 32, 36]);
    let files = Files::new(&root, "healthy", false, false)?;
    let coordinator = files.coordinator.clone();
    let _authority = coordinator.arm().unwrap();
    let suspend_binder = pokelinux_suspend_boundary::bootstrap::bootstrap(false)?;
    let suspend_peer: Strong<dyn ISuspendBoundaryProbe> = FromIBinder::try_from(suspend_binder)?;
    let state = State::new(coordinator.clone());
    let service = BnSystemSuspend::new_binder_with_features(Suspend(state.clone()), features());
    let control = BnSuspendControl::new_binder_with_features(Control(state), features());
    suspend_peer.install(&service, &control)?;
    let run_ndk = |mode: &str| -> Result<(), Box<dyn std::error::Error>> {
        let output = std::process::Command::new("/system/bin/timeout")
            .arg("15")
            .arg(&ndk_client)
            .arg(mode)
            .output()?;
        fs::write(root.join(format!("ndk-{mode}-stdout.txt")), &output.stdout)?;
        fs::write(root.join(format!("ndk-{mode}-stderr.txt")), &output.stderr)?;
        assert!(
            output.status.success(),
            "NDK {mode}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    };
    run_ndk("basic")?;
    wait_count(&coordinator, 0);
    files.no_mem();
    println!("sensor-wake-frozen-v1-shared-coordinator\tPASS");
    let owner = WakeOwner::new(1, 4, coordinator.clone());
    let token = subscribe(&owner);
    let mut q = Queues::new(&sensor)?;
    peer.produce(1, 100)?;
    let first = q
        .read(&owner, token, 1, &abi, &coordinator)
        .expect("real FMQ read must acquire native wake first");
    assert_eq!(owner.snapshot().native.wake_locks, 1);
    assert_eq!(owner.snapshot().pending, 1);
    assert_eq!(peer.pendingWake()?, 1);
    files.no_mem();
    println!("sensor-wake-acquire-before-original-read\tPASS");
    assert!(owner.processed(first).unwrap());
    assert!(q.acks.write(&[0, 0, 0, 0])?);
    assert_eq!(q.ack(&owner, &[first]), Err(WakeError::Full));
    assert_eq!(
        coordinator.attempt(),
        Attempt::Blocked,
        "full original ACK queue must retain native processing wake lease"
    );
    assert_eq!(owner.snapshot().native.wake_locks, 1);
    assert_eq!(peer.pendingWake()?, 1);
    files.no_mem();
    q.acks.event_flag().unwrap().wake(1)?;
    assert_eq!(peer.drainAcks()?, 0);
    q.ack(&owner, &[first]).unwrap();
    assert_eq!(peer.drainAcks()?, 1);
    wait_count(&coordinator, 0);
    println!("sensor-wake-processing-backpressure-handoff\tPASS");
    // Original generated NDK V1 reference and sensor work count in one coordinator.
    run_ndk("transfer-owner")?;
    wait_count(&coordinator, 1);
    peer.produce(1, 120)?;
    let shared = owner
        .read_wake(token, 1, || {
            let mut raw = [RawEvent::default()];
            assert!(q.events.read(&mut raw).unwrap());
            assert_eq!(
                coordinator.snapshot().wake_locks,
                2,
                "sensor and original V1 Binder object share one authority"
            );
            assert_eq!(coordinator.attempt(), Attempt::Blocked);
            ReadOutcome::Sample(decode(&raw[0], &abi).unwrap())
        })
        .unwrap();
    assert!(owner.processed(shared).unwrap());
    q.ack(&owner, &[shared]).unwrap();
    assert_eq!(peer.drainAcks()?, 1);
    wait_count(&coordinator, 1);
    assert_eq!(
        coordinator.attempt(),
        Attempt::Blocked,
        "retained Binder object outlives sensor ack disposition"
    );
    suspend_peer.dropRetained()?;
    wait_count(&coordinator, 0);
    assert_eq!(
        coordinator.attempt(),
        Attempt::Returned,
        "qualified handoff and retained-reference release allow one injected attempt"
    );
    assert_eq!(fs::read(files.path.join("state"))?, b"mem");
    fs::write(files.path.join("state"), b"")?;
    println!("sensor-wake-retained-v1-reference\tPASS");
    // Actual socket backpressure holds begin_wake outside the sensor mutex.
    // Revocation and both snapshots remain responsive; no read is admitted afterward.
    let responsive_path = root.join("responsive");
    fs::create_dir(&responsive_path)?;
    let private = |name: &str| -> std::io::Result<OwnedFd> {
        Ok(OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(responsive_path.join(name))?
            .into())
    };
    let count = private("count")?;
    fs::write(responsive_path.join("count"), b"7\n")?;
    let (mut outbound, mut drain) = UnixStream::pair()?;
    outbound.set_nonblocking(true)?;
    drain.set_nonblocking(true)?;
    let mut full = false;
    for _ in 0..1024 {
        match outbound.write(&[0; 4096]) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                full = true;
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }
    assert!(full, "bounded actual abort socket must become full");
    let responsive = Coordinator::new(
        DescriptorBackend::new(
            count,
            private("state")?,
            outbound.into(),
            private("unlock")?,
        )?,
        private("writer")?,
    )
    .map_err(|e| format!("responsive:{e:?}"))?;
    let _responsive_authority = responsive.arm().unwrap();
    let responsive_owner = WakeOwner::new(1, 2, responsive.clone());
    let responsive_token = subscribe(&responsive_owner);
    let mut responsive_q = Queues::new(&sensor)?;
    peer.produce(1, 150)?;
    let worker_owner = responsive_owner.clone();
    let worker_coordinator = responsive.clone();
    let worker_abi = abi.clone();
    let worker = std::thread::spawn(move || {
        let result = responsive_q.read(
            &worker_owner,
            responsive_token,
            1,
            &worker_abi,
            &worker_coordinator,
        );
        (result, responsive_q)
    });
    let deadline = BootClock.now() + Duration::from_millis(500);
    while responsive_owner.snapshot().acquiring != 1 {
        assert!(BootClock.now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let start = BootClock.now();
    assert_eq!(responsive_owner.snapshot().native.wake_locks, 0);
    responsive_owner.revoke(Principal(1));
    assert!(
        BootClock.now() - start < Duration::from_millis(100),
        "snapshot/revoke waited for native acquisition IO"
    );
    for _ in 0..1024 {
        match drain.read(&mut [0; 4096]) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e.into()),
        }
    }
    let (result, responsive_q) = worker.join().unwrap();
    assert_eq!(
        result,
        Err(WakeError::Policy(pokelinux_sensors::Error::Stale))
    );
    assert_eq!(responsive_q.events.available_to_read()?, 1);
    assert_eq!(responsive_q.acks.available_to_read()?, 0);
    assert_eq!(responsive_owner.snapshot().pending, 0);
    assert_eq!(responsive.snapshot().wake_locks, 0);
    assert!(fs::read(responsive_path.join("state"))?.is_empty());
    println!("sensor-wake-responsive-acquire-revocation\tPASS");
    // Real acquisition failures leave the original queue read counter untouched.
    for (label, abort, unlock) in [
        ("abort-failure", true, false),
        ("unlock-failure", false, true),
    ] {
        let failed = Files::new(&root, label, abort, unlock)?;
        let _authority = failed.coordinator.arm().unwrap();
        let failed_owner = WakeOwner::new(1, 2, failed.coordinator.clone());
        let token = subscribe(&failed_owner);
        let mut failed_q = Queues::new(&sensor)?;
        peer.produce(1, 200)?;
        let before = failed_q.events.available_to_read()?;
        assert_eq!(
            failed_q.read(&failed_owner, token, 1, &abi, &failed.coordinator),
            Err(WakeError::Acquire)
        );
        assert_eq!(failed_q.events.available_to_read()?, before);
        assert_eq!(failed_q.acks.available_to_read()?, 0);
        assert_eq!(failed_owner.snapshot().pending, 0);
        assert_eq!(failed.coordinator.snapshot().lifecycle, Lifecycle::Degraded);
        assert_eq!(failed.coordinator.attempt(), Attempt::Disabled);
        failed.no_mem();
    }
    println!("sensor-wake-real-fd-acquire-failures\tPASS");
    // Valid ack bytes were written, but a failed notification cannot be retried or retired.
    q = Queues::new(&sensor)?;
    peer.produce(1, 300)?;
    let uncertain = q.read(&owner, token, 1, &abi, &coordinator).unwrap();
    assert!(owner.processed(uncertain).unwrap());
    let mut writes = 1;
    let signal_owner = owner.clone();
    let mut signal_acks = q.acks;
    let fault = std::thread::spawn(move || {
        pokelinux_sensors_boundary::futex_fault::install_thread_wake_failure()
            .expect("owned guest-thread seccomp filter");
        let result = signal_owner.acknowledge(&[uncertain], |plan| {
            assert!(signal_acks.write(&[plan.count]).unwrap());
            let failed = signal_acks.event_flag().unwrap().wake(1);
            assert!(
                matches!(failed,Err(rsbinder::fmq::Error::Os(errno)) if errno.raw_os_error()==libc::EPERM),
                "actual FUTEX_WAKE_BITSET must return EPERM: {failed:?}"
            );
            AckOutcome::WrittenSignalFailed
        });
        (result, signal_acks)
    });
    let (result, acks) = fault.join().unwrap();
    q.acks = acks;
    assert_eq!(result, Err(WakeError::Uncertain));
    assert_eq!(owner.snapshot().quarantined, 1);
    assert_eq!(coordinator.snapshot().wake_locks, 1);
    assert_eq!(coordinator.attempt(), Attempt::Blocked);
    assert!(
        owner
            .acknowledge(&[uncertain], |_| {
                writes += 1;
                AckOutcome::WrittenAndSignalled
            })
            .is_err()
    );
    assert_eq!(
        writes, 1,
        "no duplicate ack after written-but-unsignalled disposition"
    );
    // The original receiver can still observe the atomically set bit and written count.
    assert_eq!(peer.drainAcks()?, 1);
    assert_eq!(
        owner.snapshot().native.wake_locks,
        1,
        "synthetic source count0 is not a native quarantine retirement receipt"
    );
    println!("sensor-wake-written-signal-failure-quarantine\tPASS");
    // Malformed data keeps its actual coordinator lease; replacing queues cannot reset it.
    q = Queues::new(&sensor)?;
    peer.produceMalformed()?;
    let malformed = owner.read_wake(token, 1, || {
        let mut raw = [RawEvent::default()];
        assert!(q.events.read(&mut raw).unwrap());
        assert_eq!(coordinator.snapshot().wake_locks, 2);
        assert!(decode(&raw[0], &abi).is_err());
        ReadOutcome::Malformed
    });
    assert_eq!(malformed, Err(WakeError::Malformed));
    assert_eq!(owner.snapshot().quarantined, 2);
    assert_eq!(coordinator.snapshot().wake_locks, 2);
    assert_eq!(coordinator.attempt(), Attempt::Blocked);
    files.no_mem();
    owner.reinitialize(2);
    peer.dropQueues()?;
    assert_eq!(owner.snapshot().pending, 2);
    assert_eq!(coordinator.snapshot().wake_locks, 2);
    println!("sensor-wake-malformed-generation-retention\tPASS");
    owner.grant(Principal(1));
    let current = owner.subscribe(Principal(1), Kind::Acceleration).unwrap();
    q = Queues::new(&sensor)?;
    peer.produce(1, 400)?;
    let dying = owner
        .read_wake(current, 2, || {
            let mut raw = [RawEvent::default()];
            assert!(q.events.read(&mut raw).unwrap());
            ReadOutcome::Sample(decode(&raw[0], &abi).unwrap())
        })
        .unwrap();
    let (sender, receiver) = mpsc::sync_channel(1);
    let death = Arc::new(Death(owner.clone(), sender));
    sensor_binder.link_to_death_arc(&death)?;
    assert!(peer.die().is_err());
    receiver.recv_timeout(Duration::from_secs(3))?;
    assert!(!owner.processed(dying).unwrap());
    assert!(
        owner
            .acknowledge(&[dying], |_| panic!(
                "dead source must not get acknowledgement"
            ))
            .is_err()
    );
    assert_eq!(owner.snapshot().native.wake_locks, 3);
    assert_eq!(owner.snapshot().pending, 3);
    assert_eq!(coordinator.attempt(), Attempt::Disabled);
    println!("sensor-wake-real-binder-death-retention\tPASS");
    let _ = sensor_binder.unlink_to_death_arc(&death);
    drop(death);
    drop(owner);
    wait_count(&coordinator, 0);
    assert_eq!(coordinator.snapshot().lifecycle, Lifecycle::Stopped);
    assert_eq!(coordinator.attempt(), Attempt::Disabled);
    assert!(coordinator.arm().is_err());
    files.no_mem();
    println!("sensor-wake-owner-drop-fences-before-release\tPASS");
    Ok(())
}
