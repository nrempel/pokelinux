//! Radio-simulator integration only. The harness controls emulator-5554;
//! this test has only the enrolled TLS endpoint, never ADB or a physical phone.
#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    Identity,
    enrollment::Enrollment,
    tls::WatchCancel,
    watch::{SourceState, WatchQuery},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires the owned Android emulator radio and enrollment"]
fn watch_tracks_emulator_radio_state() {
    let path = std::env::var_os("PHONE_EMULATOR_ENROLLMENT").unwrap();
    let expected = match std::env::var("PHONE_EXPECT_RINGING").unwrap().as_str() {
        "1" => 1,
        "0" => 0,
        _ => panic!("explicit radio expectation required"),
    };
    let enrollment = Enrollment::load(Path::new(&path)).unwrap();
    let end = Instant::now() + Duration::from_secs(20);
    let mut cursor = None;
    loop {
        let query = WatchQuery::new(
            Identity::try_from(uuid::Uuid::new_v4().simple().to_string()).unwrap(),
            cursor,
            500,
        )
        .unwrap();
        let snapshot = enrollment
            .client()
            .watch(enrollment.address(), &query, &WatchCancel::default())
            .unwrap();
        if snapshot.ringing().len() == expected {
            assert_ne!(snapshot.source_state(), SourceState::Unavailable);
            if expected == 1 {
                assert_eq!(snapshot.source_state(), SourceState::Observed);
                assert!(snapshot.ringing()[0].id() > 0);
            }
            break;
        }
        assert!(Instant::now() < end, "Android radio state was not observed");
        cursor = Some(snapshot.cursor());
    }
    if expected == 1 {
        // Exercise the same independent worker used by the daemon, with no UI.
        let mut observer = omarchy_phone_core::background::BackgroundObserver::start(
            enrollment.client().clone(),
            enrollment.address(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(lease) = observer.lease().unwrap() {
                assert_eq!(lease.snapshot().ringing().len(), 1);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background ringing lease missing"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let stopped = Instant::now();
        observer.shutdown().unwrap();
        assert!(stopped.elapsed() < Duration::from_secs(2));
        assert!(observer.lease().unwrap().is_none());
    }
}
