//! Generated Rust V1 proxy -> original generated NDK V1 dispatcher, real Binder.
use pokelinux_suspend::{BootClock, Clock};
use pokelinux_suspend_boundary::{ISuspendBoundaryProbe, WakeLockType, bootstrap};
use rsbinder::{FromIBinder, Interface, Strong};
use std::time::Duration;
fn count(
    peer: &Strong<dyn ISuspendBoundaryProbe>,
    expected: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = BootClock.now() + Duration::from_secs(3);
    while peer.ndkSnapshot()?[0] != expected {
        if BootClock.now() >= deadline {
            return Err("NDK object lifetime deadline".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let peer: Strong<dyn ISuspendBoundaryProbe> =
        FromIBinder::try_from(bootstrap::bootstrap(true)?)?;
    let service = peer.ndkService()?;
    assert!(service.as_binder().is_remote());
    assert_eq!(service.getInterfaceVersion()?, 1);
    assert_eq!(
        service.getInterfaceHash()?,
        "082a58c2505b703702f34ab8abcb1d1b5e890be4"
    );
    let baseline = peer.ndkSnapshot()?;
    let first = service.acquireWakeLock(WakeLockType::PARTIAL, "rust-native")?;
    let second = first.clone();
    assert!(first.as_binder().is_remote());
    assert_eq!(first.getInterfaceVersion()?, 1);
    assert_eq!(
        first.getInterfaceHash()?,
        "082a58c2505b703702f34ab8abcb1d1b5e890be4"
    );
    let snapshot = peer.ndkSnapshot()?;
    assert_eq!(snapshot[0], 1);
    assert_eq!(snapshot[3], 2000);
    assert_eq!(snapshot[4], std::process::id() as i64);
    println!("rust-proxy-ndk-v1-dispatch-identity\tPASS");
    drop(first);
    count(&peer, 1)?;
    second.release()?;
    count(&peer, 0)?;
    second.release()?;
    drop(second);
    assert_eq!(peer.ndkSnapshot()?[2], baseline[2] + 1);
    println!("rust-proxy-ndk-release-once\tPASS");
    let held = service.acquireWakeLock(WakeLockType::FULL, "destroy")?;
    count(&peer, 1)?;
    drop(held);
    count(&peer, 0)?;
    assert_eq!(peer.ndkSnapshot()?[2], baseline[2] + 2);
    println!("rust-proxy-ndk-last-reference\tPASS");
    Ok(())
}
