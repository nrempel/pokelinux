#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    Identity,
    notification_events::{NoticeReceiver, notice_channel},
    notification_generation::Notice,
    watch::decode_snapshot,
};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};
fn ready(receiver: &NoticeReceiver, millis: i64) -> bool {
    let mut fds = [PollFd::new(receiver, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: millis / 1000,
            tv_nsec: millis % 1000 * 1_000_000,
        }),
    )
    .unwrap();
    fds[0].revents().contains(PollFlags::IN)
}
fn dismissal() -> Notice {
    let id = Identity::try_from("dddddddddddddddddddddddddddddddd".to_owned()).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,
        "generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,
        "source":"android.telecom.InCallService","source_state":"observed",
        "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]}))
    .unwrap();
    Notice::Dismissed(decode_snapshot(&bytes, &id).unwrap())
}
#[test]
fn notices_wake_poll_and_preserve_order_without_persistent_readiness() {
    let (sender, receiver) = notice_channel().unwrap();
    assert!(!ready(&receiver, 0));
    sender.send(Notice::OpenPhone).unwrap();
    sender.send(dismissal()).unwrap();
    assert!(ready(&receiver, 100));
    let batch = receiver.drain().unwrap();
    assert!(!batch.closed);
    let [Notice::OpenPhone, Notice::Dismissed(snapshot)] = batch.notices.as_slice() else {
        panic!("lost event order")
    };
    assert_eq!(snapshot.ringing()[0].id(), 7);
    assert!(!ready(&receiver, 0));
    assert!(receiver.drain().unwrap().notices.is_empty());
}
#[test]
fn full_queue_refuses_without_waiting_and_sender_loss_still_wakes_consumer() {
    let (sender, receiver) = notice_channel().unwrap();
    for _ in 0..16 {
        sender.send(Notice::OpenPhone).unwrap();
    }
    assert_eq!(
        sender.send(dismissal()).unwrap_err().kind(),
        ErrorKind::WouldBlock
    );
    drop(sender);
    assert!(ready(&receiver, 100));
    let batch = receiver.drain().unwrap();
    assert_eq!(batch.notices.len(), 16);
    // A bounded drain must preserve wakeup for the terminal channel state.
    assert!(ready(&receiver, 100));
    let terminal = receiver.drain().unwrap();
    assert!(terminal.notices.is_empty());
    assert!(terminal.closed);
}
#[test]
fn consumer_loss_is_reported_to_the_notification_loop() {
    let (sender, receiver) = notice_channel().unwrap();
    drop(receiver);
    assert_eq!(
        sender.send(Notice::OpenPhone).unwrap_err().kind(),
        ErrorKind::BrokenPipe
    );
}
#[test]
fn concurrent_delivery_and_close_have_no_lost_wakeup() {
    let (sender, receiver) = notice_channel().unwrap();
    let producer = std::thread::spawn(move || {
        for _ in 0..2000 {
            loop {
                match sender.send(Notice::OpenPhone) {
                    Ok(()) => break,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => std::thread::yield_now(),
                    Err(error) => panic!("unexpected send failure: {error}"),
                }
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = 0;
    loop {
        assert!(Instant::now() < deadline, "delivery deadline");
        assert!(ready(&receiver, 1000), "lost notice or closure wakeup");
        let batch = receiver.drain().unwrap();
        assert!(batch.notices.len() <= 16);
        received += batch.notices.len();
        if batch.closed {
            break;
        }
    }
    producer.join().unwrap();
    assert_eq!(received, 2000);
}
