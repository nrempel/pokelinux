#![cfg(target_os = "linux")]
use omarchy_phone_core::lease_timer::LeaseTimer;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::time::Duration;
fn now() -> Duration {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(t.tv_sec as u64, t.tv_nsec as u32)
}
fn wait(timer: &LeaseTimer, millis: i64) -> bool {
    let mut fds = [PollFd::new(timer, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: millis / 1000,
            tv_nsec: (millis % 1000) * 1000000,
        }),
    )
    .unwrap();
    fds[0].revents().contains(PollFlags::IN)
}
#[test]
fn absolute_timer_expires_once_and_does_not_need_network_progress() {
    let timer = LeaseTimer::new().unwrap();
    assert!(!timer.consume().unwrap());
    let deadline = now() + Duration::from_millis(30);
    timer.arm(Some(deadline)).unwrap();
    assert!(wait(&timer, 1000));
    assert!(now() >= deadline);
    assert!(timer.consume().unwrap());
    assert!(!timer.consume().unwrap());
    assert!(!wait(&timer, 0));
}
#[test]
fn renewal_replaces_old_deadline_and_disarm_clears_pending_expiry() {
    let timer = LeaseTimer::new().unwrap();
    timer.arm(Some(Duration::ZERO)).unwrap();
    assert!(wait(&timer, 1000));
    timer.arm(Some(now() + Duration::from_secs(1))).unwrap();
    assert!(!timer.consume().unwrap());
    assert!(!wait(&timer, 20));
    timer.arm(None).unwrap();
    assert!(!wait(&timer, 20));
    timer.arm(Some(Duration::ZERO)).unwrap();
    assert!(wait(&timer, 1000));
    timer.arm(None).unwrap();
    assert!(!timer.consume().unwrap());
}
#[test]
fn out_of_range_deadline_is_rejected_without_disarming_current_timer() {
    let timer = LeaseTimer::new().unwrap();
    timer.arm(Some(Duration::ZERO)).unwrap();
    assert!(wait(&timer, 1000));
    assert!(timer.arm(Some(Duration::MAX)).is_err());
    assert!(timer.consume().unwrap());
}

#[test]
fn async_expiry_survives_cancelled_wait_and_rearming() {
    async_io::block_on(async {
        let timer = LeaseTimer::new().unwrap().into_async().unwrap();
        timer.arm(None).unwrap();
        let first = futures_lite::future::or(
            async {
                timer.expired().await.unwrap();
                true
            },
            async {
                async_io::Timer::after(Duration::from_millis(20)).await;
                false
            },
        )
        .await;
        assert!(!first);
        let deadline = now() + Duration::from_millis(30);
        timer.arm(Some(deadline)).unwrap();
        timer.expired().await.unwrap();
        assert!(now() >= deadline);
        timer.arm(Some(Duration::ZERO)).unwrap();
        timer.expired().await.unwrap();
        timer.arm(None).unwrap();
        assert!(
            !futures_lite::future::or(
                async {
                    timer.expired().await.unwrap();
                    true
                },
                async {
                    async_io::Timer::after(Duration::from_millis(20)).await;
                    false
                }
            )
            .await
        );
    });
}
