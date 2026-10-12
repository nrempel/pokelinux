//! Disposable framework bootstrap, generated Rust proxy into original NDK dispatcher.
use pokelinux_camera_boundary::{
    ICameraBoundaryProbe, NativeHandle, SubmissionReceipt, classify_submission,
};
use rsbinder::{
    Binder, FromIBinder, Interface, Parcel, ParcelFileDescriptor, ProcessState, Remotable, Result,
    SIBinder, Status, StatusCode, Strong, hub,
};
use std::{
    io::{Read, Write},
    sync::{
        Arc,
        mpsc::{self, SyncSender},
    },
    time::Duration,
};
const PACKAGE: &str = "org.pokelinux.camerapeer";
const COMPONENT: &str = "org.pokelinux.camerapeer.Peer";
const DESCRIPTOR: &str = "org.pokelinux.cameratest.ICameraBoundaryProbe";
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
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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

    let proxy: Strong<dyn ICameraBoundaryProbe> = FromIBinder::try_from(peer.clone())?;
    println!("camera-rust-bootstrap\tPASS");
    let baseline_peer = proxy.fdCount()?;
    let baseline_client = fd_count()?;
    for mode in [0, 1] {
        let (reader, writer) = ParcelFileDescriptor::pipe()?;
        let mut handle = NativeHandle::default();
        handle.fds.push(writer);
        handle.ints = vec![37, 4];
        let receipt = classify_submission(proxy.submit(&handle, mode));
        match mode {
            0 => assert_eq!(receipt, SubmissionReceipt::Accepted(1)),
            _ => assert_eq!(
                receipt,
                SubmissionReceipt::Uncertain {
                    service_error: 17,
                    transport_error: StatusCode::Ok
                }
            ),
        }
        drop(handle);
        let returned = proxy.returnedHandle()?;
        assert_eq!(returned.ints, [37, 4]);
        assert_eq!(returned.fds.len(), 1);
        proxy.release()?;
        read_byte(&reader, b'R')?;
        (&returned.fds[0]).write_all(b"C")?;
        read_byte(&reader, b'C')?;
        drop(returned);
        eof(&reader)?;
        println!(
            "camera-rust-{}\tPASS",
            if mode == 0 {
                "fd-independent"
            } else {
                "error-uncertain"
            }
        );
    }
    for mode in (0..64).map(|n| n % 2) {
        let (reader, writer) = ParcelFileDescriptor::pipe()?;
        let mut handle = NativeHandle::default();
        handle.fds.push(writer);
        handle.ints = vec![37, 4];
        let receipt = classify_submission(proxy.submit(&handle, mode));
        assert!(matches!(
            (mode, receipt),
            (0, SubmissionReceipt::Accepted(1))
                | (
                    1,
                    SubmissionReceipt::Uncertain {
                        service_error: 17,
                        transport_error: StatusCode::Ok
                    }
                )
        ));
        drop(handle);
        proxy.release()?;
        read_byte(&reader, b'R')?;
        eof(&reader)?;
    }
    assert_eq!(proxy.fdCount()?, baseline_peer);
    assert_eq!(fd_count()?, baseline_client);
    println!("camera-rust-fd-budget\t64\tPASS");
    let (sender, receiver) = mpsc::sync_channel(1);
    let recipient = Arc::new(Death(sender));
    peer.link_to_death_arc(&recipient)?;
    let (reader, writer) = ParcelFileDescriptor::pipe()?;
    let mut handle = NativeHandle::default();
    handle.fds.push(writer);
    handle.ints = vec![37, 4];
    assert!(matches!(
        classify_submission(proxy.submit(&handle, 2)),
        SubmissionReceipt::Uncertain {
            transport_error: StatusCode::DeadObject,
            ..
        }
    ));
    drop(handle);
    read_byte(&reader, b'D')?;
    eof(&reader)?;
    receiver.recv_timeout(Duration::from_secs(5))?;
    assert_eq!(
        proxy.fdCount().unwrap_err().transaction_error(),
        StatusCode::DeadObject
    );
    println!("camera-rust-service-death\tPASS");
    Ok(())
}
struct Death(SyncSender<()>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        let _ = self.0.try_send(());
    }
}
fn fd_count() -> std::io::Result<usize> {
    Ok(std::fs::read_dir("/proc/self/fd")?.count())
}
fn ready(reader: &ParcelFileDescriptor) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut fds = [rustix::event::PollFd::new(
        reader,
        rustix::event::PollFlags::IN | rustix::event::PollFlags::HUP,
    )];
    let timeout = rustix::event::Timespec {
        tv_sec: 2,
        tv_nsec: 0,
    };
    assert_eq!(
        rustix::event::poll(&mut fds, Some(&timeout))?,
        1,
        "bounded FD observation"
    );
    Ok(())
}
fn read_byte(
    reader: &ParcelFileDescriptor,
    expected: u8,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    ready(reader)?;
    let mut byte = [0];
    (&*reader).read_exact(&mut byte)?;
    assert_eq!(byte, [expected]);
    Ok(())
}
fn eof(reader: &ParcelFileDescriptor) -> std::result::Result<(), Box<dyn std::error::Error>> {
    ready(reader)?;
    assert_eq!(
        (&*reader).read(&mut [0])?,
        0,
        "all transferred writers closed"
    );
    Ok(())
}
