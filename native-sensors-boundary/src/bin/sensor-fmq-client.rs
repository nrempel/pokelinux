//! Original public NDK FMQ/EventFlag ↔ Rust owner through exact SensorsV3 Binder.
use pokelinux_sensors::{AckPlan, ClientToken, Error, Kind, Owner, Principal, Ticket};
use pokelinux_sensors_boundary::{
    BnSensorClient, BnSensorsCallback, Event, ISensorBoundaryControl, ISensorClient, ISensors,
    ISensorsCallback, SensorInfo,
    event_abi::{EVENT_SIZE, RawEvent, decode},
};
use pokelinux_sensors_boundary::{
    GrantorDescriptor, MQDescriptor, NativeHandle, SynchronizedReadWrite,
};
use rsbinder::fmq::MessageQueue;
use rsbinder::{
    Binder, FromIBinder, Interface, Parcel, ProcessState, Remotable, Result, SIBinder, Status,
    StatusCode, Strong, hub,
};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, SyncSender},
};
use std::time::{Duration, Instant};
const PACKAGE: &str = "org.pokelinux.sensorpeer";
const COMPONENT: &str = "org.pokelinux.sensorpeer.Peer";
const DESCRIPTOR: &str = "org.pokelinux.sensortest.ISensorBoundaryControl";
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
struct Client(Arc<Mutex<Owner>>);
impl Interface for Client {}
impl ISensorClient for Client {
    fn subscribe(&self, claimed_uid: i32, kind: i32) -> rsbinder::BinderResult<i64> {
        let authenticated = rsbinder::get_calling_uid();
        if claimed_uid < 0 || claimed_uid as u32 != authenticated {
            return Err(StatusCode::PermissionDenied.into());
        }
        let kind = match kind {
            0 => Kind::Acceleration,
            1 => Kind::Location,
            _ => return Err(StatusCode::BadValue.into()),
        };
        self.0
            .lock()
            .unwrap()
            .subscribe(Principal(u64::from(authenticated)), kind)
            .map(|token| token.0 as i64)
            .map_err(|_| StatusCode::PermissionDenied.into())
    }
}
struct Death(Arc<Mutex<Owner>>, SyncSender<()>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        self.0.lock().unwrap().backend_died();
        let _ = self.1.try_send(());
    }
}
// Frozen schemas use qualified references rather than imports, so generate their exact
// declarations and adapt the runtime-validated descriptor fields without touching schemas.
fn wire_descriptor<T: Default>(
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
    fn new(sensor: &Strong<dyn ISensors>) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let events = MessageQueue::<RawEvent>::create(4, true)?;
        let acks = MessageQueue::<i32>::create(4, true)?;
        let event_desc: MQDescriptor<Event, SynchronizedReadWrite> =
            wire_descriptor(events.descriptor()?)?;
        let ack_desc: MQDescriptor<i32, SynchronizedReadWrite> =
            wire_descriptor(acks.descriptor()?)?;
        let cb = BnSensorsCallback::new_binder(Callback);
        sensor.initialize(&event_desc, &ack_desc, &cb)?;
        // Each parcelable/descriptor is transient. Both processes own independent FD/mapping lifetimes.
        drop(event_desc);
        drop(ack_desc);
        sensor.activate(77, true)?;
        Ok(Self { events, acks })
    }
    fn read(
        &mut self,
        peer: &Strong<dyn ISensorBoundaryControl>,
        abi: &[i32],
        owner: &Arc<Mutex<Owner>>,
        token: ClientToken,
        epoch: u64,
    ) -> std::result::Result<Ticket, Box<dyn std::error::Error>> {
        // Hold the service state across the bounded local read/admission so revocation
        // cannot remove permission after the read counter advanced but before ownership.
        let ticket = {
            let mut owner = owner.lock().unwrap();
            let ticket = owner
                .reserve(token, epoch, true)
                .map_err(|_| "admission refused")?;
            let mut raw = [RawEvent::default()];
            match self.events.read(&mut raw) {
                Ok(true) => {}
                Ok(false) => {
                    owner.cancel_unread(ticket).unwrap();
                    return Err("queue empty".into());
                }
                Err(error) => {
                    owner.reject_decoding(ticket).unwrap();
                    return Err(error.into());
                }
            }
            let sample = match decode(&raw[0], abi) {
                Ok(sample) => sample,
                Err(error) => {
                    owner.reject_decoding(ticket).unwrap();
                    return Err(error.into());
                }
            };
            assert_eq!(sample.acceleration[1..], [1.25, -2.5]);
            owner
                .decoded(ticket, sample)
                .map_err(|_| "sample normalization refused")?;
            ticket
        };
        self.events.event_flag().unwrap().wake(2)?;
        assert_eq!(peer.awaitReadFlag()?, 2);
        Ok(ticket)
    }
    fn ack(
        &mut self,
        peer: &Strong<dyn ISensorBoundaryControl>,
        owner: &Arc<Mutex<Owner>>,
        plan: AckPlan,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert!(self.acks.write(&[plan.count])?);
        // Successful queue write transfers the acknowledgement; no credit is released on preparation.
        owner
            .lock()
            .unwrap()
            .commit_ack(plan)
            .map_err(|_| "ack commit refused")?;
        self.acks.event_flag().unwrap().wake(1)?;
        assert_eq!(peer.drainAcks()?, plan.count);
        Ok(())
    }
}
fn assert_owner_error<T: std::fmt::Debug>(result: std::result::Result<T, Error>, error: Error) {
    assert_eq!(result.unwrap_err(), error);
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let expected_uid: u32 = std::env::args()
        .nth(1)
        .ok_or("trusted fixture UID missing")?
        .parse()?;
    assert!(
        expected_uid >= 10000,
        "installed fixture must be an app principal"
    );
    let binder = bootstrap()?;
    let peer: Strong<dyn ISensorBoundaryControl> = FromIBinder::try_from(binder.clone())?;
    assert_eq!(
        peer.uid()? as u32,
        expected_uid,
        "trusted package-manager UID"
    );
    let sensor = peer.sensors()?;
    assert_eq!(sensor.getInterfaceVersion()?, 3);
    assert_eq!(
        sensor.getInterfaceHash()?,
        "b051523be857524c99057b69f253a7d0bd295400"
    );
    let infos = sensor.getSensorsList()?;
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].sensorHandle, 77);
    assert_eq!(infos[0].flags & 1, 1);
    let abi = peer.eventAbi()?;
    assert_eq!(abi[0] as usize, EVENT_SIZE);
    assert_eq!(
        abi,
        [96, 8, 0, 8, 12, 16, 1, 24, 28, 32, 36],
        "exact compiler-probed Event layout"
    );
    println!("sensor-frozen-ndk-abi-bootstrap\tPASS");
    let owner = Arc::new(Mutex::new(Owner::new(1, 2)));
    let principal = Principal(u64::from(expected_uid));
    let client = BnSensorClient::new_binder(Client(owner.clone()));
    assert!(
        peer.requestSubscription(&client, expected_uid as i32, 0)
            .is_err(),
        "no permission grant"
    );
    owner.lock().unwrap().grant(principal);
    assert!(
        peer.requestSubscription(&client, 0, 0).is_err(),
        "claimed UID must not authenticate caller"
    );
    assert!(
        peer.requestSubscription(&client, expected_uid as i32, 1)
            .is_err(),
        "location permission is separate and unsupported"
    );
    let token = ClientToken(peer.requestSubscription(&client, expected_uid as i32, 0)? as u64);
    let baseline = peer.fdCount()?;
    let mut q = Queues::new(&sensor)?;
    assert!(
        peer.fdCount()? <= baseline + 2,
        "exact descriptor duplicate count is bounded"
    );
    println!("sensor-authenticated-per-client\tPASS");
    peer.produce(2, 100)?;
    assert_eq!(
        q.events
            .event_flag()
            .unwrap()
            .wait(1, Some(Duration::from_secs(1)))?,
        1
    );
    let first = q.read(&peer, &abi, &owner, token, 1)?;
    let second = q.read(&peer, &abi, &owner, token, 1)?;
    assert_eq!(owner.lock().unwrap().wake_responsibilities(), 2);
    assert_eq!(peer.pendingWake()?, 2);
    // Genuine runtime red: the initial bug actually sends an early ack through the original queue.
    let premature = owner.lock().unwrap().prepare_ack(&[first]);
    if let Ok(plan) = premature {
        q.ack(&peer, &owner, plan)?;
    }
    assert_eq!(
        peer.pendingWake()?,
        2,
        "unprocessed admitted wake event was acknowledged over actual FMQ"
    );
    assert_owner_error(premature, Error::NotProcessed);
    let elapsed = peer.awaitAckTimeout()?;
    assert!(
        (900..=2000).contains(&elapsed),
        "original one-second EventFlag wait was unbounded or early: {elapsed}"
    );
    assert_eq!(
        peer.pendingWake()?,
        2,
        "a wait timeout is not an acknowledgement"
    );
    println!("sensor-original-one-second-wait\tPASS");
    println!("sensor-admitted-before-ack\tPASS");
    assert!(owner.lock().unwrap().processed(second).unwrap());
    assert_owner_error(
        owner.lock().unwrap().prepare_ack(&[second, second]),
        Error::Duplicate,
    );
    assert_eq!(peer.pendingWake()?, 2);
    let plan = owner.lock().unwrap().prepare_ack(&[second]).unwrap();
    assert_eq!(plan.count, 1);
    assert_eq!(
        owner.lock().unwrap().wake_responsibilities(),
        2,
        "preparation retains credits"
    );
    assert_owner_error(
        owner
            .lock()
            .unwrap()
            .commit_ack(AckPlan { count: 2, ..plan }),
        Error::Unknown,
    );
    q.ack(&peer, &owner, plan)?;
    assert_eq!(peer.pendingWake()?, 1);
    assert_owner_error(owner.lock().unwrap().prepare_ack(&[second]), Error::Unknown);
    assert!(owner.lock().unwrap().processed(first).unwrap());
    let plan = owner.lock().unwrap().prepare_ack(&[first]).unwrap();
    q.ack(&peer, &owner, plan)?;
    assert_eq!(peer.pendingWake()?, 0);
    assert_eq!(owner.lock().unwrap().pending(), 0);
    println!("sensor-exact-split-ack-accounting\tPASS");
    // Real transport full conditions; a refused write cannot consume an ownership credit.
    peer.produce(4, 200)?;
    assert!(peer.produce(1, 300).is_err());
    assert_eq!(peer.pendingWake()?, 4);
    let a = q.read(&peer, &abi, &owner, token, 1)?;
    let b = q.read(&peer, &abi, &owner, token, 1)?;
    let unread = q.events.available_to_read()?;
    assert_eq!(unread, 2);
    assert_owner_error(owner.lock().unwrap().can_admit(token, 1), Error::Capacity);
    assert_eq!(
        q.events.available_to_read()?,
        unread,
        "bounded admission leaves read counter untouched"
    );
    assert!(owner.lock().unwrap().processed(a).unwrap());
    assert!(owner.lock().unwrap().processed(b).unwrap());
    let plan = owner.lock().unwrap().prepare_ack(&[a, b]).unwrap();
    assert!(q.acks.write(&[0, 0, 0, 0])?);
    assert!(!q.acks.write(&[plan.count])?);
    owner.lock().unwrap().abort_ack(plan).unwrap();
    assert_eq!(owner.lock().unwrap().wake_responsibilities(), 2);
    q.acks.event_flag().unwrap().wake(1)?;
    assert_eq!(peer.drainAcks()?, 0);
    assert_eq!(peer.pendingWake()?, 4);
    let plan = owner.lock().unwrap().prepare_ack(&[a, b]).unwrap();
    q.ack(&peer, &owner, plan)?;
    for _ in 0..2 {
        let ticket = q.read(&peer, &abi, &owner, token, 1)?;
        assert!(owner.lock().unwrap().processed(ticket).unwrap());
        let plan = owner.lock().unwrap().prepare_ack(&[ticket]).unwrap();
        q.ack(&peer, &owner, plan)?;
    }
    assert_eq!(peer.pendingWake()?, 0);
    println!("sensor-bounded-queue-backpressure\tPASS");
    peer.produce(1, 400)?;
    let revoked = q.read(&peer, &abi, &owner, token, 1)?;
    owner.lock().unwrap().revoke(principal);
    assert!(
        peer.requestSubscription(&client, expected_uid as i32, 0)
            .is_err()
    );
    assert_owner_error(owner.lock().unwrap().can_admit(token, 1), Error::Permission);
    assert!(
        !owner.lock().unwrap().processed(revoked).unwrap(),
        "revoked client cannot publish"
    );
    let plan = owner.lock().unwrap().prepare_ack(&[revoked]).unwrap();
    q.ack(&peer, &owner, plan)?;
    assert_eq!(
        peer.pendingWake()?,
        0,
        "service-owned processed disposal still acknowledges"
    );
    println!("sensor-revocation-publication-fence\tPASS");
    // Exact descriptor FD lifetime remains independent after the NDK receiver closes/replaces it.
    owner.lock().unwrap().grant(principal);
    let old_token = ClientToken(peer.requestSubscription(&client, expected_uid as i32, 0)? as u64);
    peer.produce(1, 500)?;
    let stale = q.read(&peer, &abi, &owner, old_token, 1)?;
    peer.produce(1, 501)?;
    let old_flag = q.events.event_flag().unwrap();
    peer.dropQueues()?;
    assert_eq!(peer.fdCount()?, baseline);
    assert_eq!(q.events.available_to_read()?, 1);
    owner.lock().unwrap().reinitialize(2);
    assert!(!owner.lock().unwrap().processed(stale).unwrap());
    assert_owner_error(owner.lock().unwrap().prepare_ack(&[stale]), Error::Stale);
    assert_owner_error(owner.lock().unwrap().can_admit(old_token, 1), Error::Stale);
    let mut fresh = Queues::new(&sensor)?;
    assert_eq!(fresh.events.available_to_read()?, 0);
    assert_eq!(fresh.acks.available_to_read()?, 0);
    let mut old = [RawEvent::default()];
    assert!(q.events.read(&mut old)?);
    assert_eq!(decode(&old[0], &abi)?.timestamp_ns, 501);
    old_flag.wake(2)?;
    assert!(
        fresh
            .events
            .event_flag()
            .unwrap()
            .wait(2, Some(Duration::from_millis(30)))
            .is_err(),
        "old event word must not signal new generation"
    );
    assert_eq!(
        owner.lock().unwrap().wake_responsibilities(),
        1,
        "stale wake remains quarantined; queue replacement is not cleanup"
    );
    println!("sensor-stale-generation-fd-lifetime\tPASS");
    // Independent malformed original generated Event: no raw event is silently lost
    // after advancing the FMQ read counter, even when normalization cannot proceed.
    let malformed_owner = Arc::new(Mutex::new(Owner::new(3, 2)));
    malformed_owner.lock().unwrap().grant(principal);
    let malformed_client = BnSensorClient::new_binder(Client(malformed_owner.clone()));
    let malformed_token =
        ClientToken(peer.requestSubscription(&malformed_client, expected_uid as i32, 0)? as u64);
    let mut malformed_queue = Queues::new(&sensor)?;
    peer.produceMalformed()?;
    assert_eq!(
        malformed_queue
            .events
            .event_flag()
            .unwrap()
            .wait(1, Some(Duration::from_secs(1)))?,
        1
    );
    assert!(
        malformed_queue
            .read(&peer, &abi, &malformed_owner, malformed_token, 3)
            .is_err()
    );
    assert_eq!(malformed_queue.events.available_to_read()?, 0);
    assert_eq!(
        malformed_owner.lock().unwrap().wake_responsibilities(),
        1,
        "malformed raw Event lost its admitted wake responsibility"
    );
    assert_eq!(
        peer.pendingWake()?,
        1,
        "malformed event must not be acknowledged"
    );
    assert_owner_error(
        malformed_owner.lock().unwrap().prepare_ack(&[Ticket(1)]),
        Error::Stale,
    );
    assert_eq!(malformed_queue.acks.available_to_read()?, 0);
    peer.dropQueues()?;
    assert_eq!(
        malformed_owner.lock().unwrap().wake_responsibilities(),
        1,
        "source queue close cannot clear service quarantine"
    );
    println!("sensor-malformed-event-quarantine\tPASS");
    // Reinitialize/drop cycles own and bound all peer descriptors.
    for _ in 0..32 {
        let cycle = Queues::new(&sensor)?;
        assert!(peer.fdCount()? <= baseline + 2);
        drop(cycle);
        peer.dropQueues()?;
        assert_eq!(peer.fdCount()?, baseline);
    }
    fresh = Queues::new(&sensor)?;
    let current = ClientToken(peer.requestSubscription(&client, expected_uid as i32, 0)? as u64);
    peer.produce(1, 600)?;
    assert_eq!(
        fresh
            .events
            .event_flag()
            .unwrap()
            .wait(1, Some(Duration::from_secs(1)))?,
        1
    );
    let dying = fresh.read(&peer, &abi, &owner, current, 2)?;
    let (tx, rx) = mpsc::sync_channel(1);
    let death = Arc::new(Death(owner.clone(), tx));
    binder.link_to_death_arc(&death)?;
    assert!(peer.die().is_err());
    rx.recv_timeout(Duration::from_secs(3))?;
    assert!(!owner.lock().unwrap().processed(dying).unwrap());
    assert_owner_error(
        owner.lock().unwrap().prepare_ack(&[dying]),
        Error::Quarantined,
    );
    assert_owner_error(owner.lock().unwrap().can_admit(current, 2), Error::Stale);
    assert_eq!(
        owner.lock().unwrap().wake_responsibilities(),
        2,
        "Binder death cannot reap sensor wake responsibilities"
    );
    assert_eq!(
        fresh.events.available_to_read()?,
        0,
        "kernel mapping stays alive after peer process death"
    );
    assert!(
        fresh.acks.write(&[0])?,
        "mapping ownership survives peer death; policy still prohibits ack"
    );
    println!("sensor-bounded-death-quarantine\tPASS");
    let timeout = Instant::now();
    assert!(
        fresh
            .events
            .event_flag()
            .unwrap()
            .wait(1, Some(Duration::from_millis(40)))
            .is_err()
    );
    assert!(
        timeout.elapsed() >= Duration::from_millis(30)
            && timeout.elapsed() < Duration::from_secs(1)
    );
    println!("sensor-original-eventflag-timeout\tPASS");
    Ok(())
}
