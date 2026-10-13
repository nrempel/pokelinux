//! Real Binder scaffold checks only; no storage or installed authority evidence.
use native_messages_incoming_binder_fixture::{
    smoke_callbacks::{Indication, Response},
    *,
};
use rsbinder::{FromIBinder, ProcessState, Strong};
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
struct Death(mpsc::SyncSender<()>);
impl rsbinder::DeathRecipient for Death {
    fn binder_died(&self, _: &rsbinder::WIBinder) {
        let _ = self.0.try_send(());
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generation = std::env::args()
        .nth(1)
        .ok_or("explicit fresh synthetic generation required")?;
    let process = ProcessState::init("/dev/binder", 3)?;
    ProcessState::start_thread_pool();
    let b = lookup_probe(process)?;
    assert!(b.is_remote());
    assert_eq!(b.descriptor(), DESCRIPTOR);
    let p: Strong<dyn IIncomingMessagesProbe> = FromIBinder::try_from(b.clone())?;
    assert_eq!(p.generation()?, generation);
    let (events, received) = mpsc::sync_channel(8);
    let (notice, died) = mpsc::sync_channel(1);
    let death = Arc::new(Death(notice));
    b.link_to_death_arc(&death)?;
    let mut callbacks = Vec::new();
    for slot in 0..2u8 {
        let e = p.endpoint(i32::from(slot))?;
        assert!(e.as_binder().is_remote());
        assert_eq!(e.getInterfaceVersion()?, 3);
        assert_eq!(e.getInterfaceHash()?, HASH);
        let r = BnRadioMessagingResponse::new_binder(Response);
        let i = BnRadioMessagingIndication::new_binder(Indication {
            slot,
            queue: events.clone(),
        });
        e.setResponseFunctions(&r, &i)?;
        p.barrier(i32::from(slot), 1, 0, 1000)?;
        callbacks.push((e, r, i));
    }
    let raw = [0u8, 1, 2, 3];
    for (slot, kind) in [(0, 0), (0, 0), (1, 1)] {
        p.incoming(slot, kind, &raw, 0)?;
        let event = received.recv_timeout(Duration::from_secs(2))?;
        assert_eq!(
            (i32::from(event.0), i32::from(event.1), event.2),
            (slot, kind, 0)
        );
        assert_eq!(event.3, raw);
        assert_eq!(event.4, 1000);
    }
    // Explicit adversarial second attachment proves this fixture refuses replacement.
    let (e, r, i) = &callbacks[0];
    e.setResponseFunctions(r, i)?;
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        let c = p.counters(0)?;
        if c[0] == 2 {
            assert_eq!(c[1], 1);
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let counters: Vec<_> = (0..2)
        .map(|slot| p.counters(slot))
        .collect::<Result<_, _>>()?;
    assert_eq!(&counters[0][3..], &[0, 0, 0, 0]);
    assert_eq!(&counters[1][3..], &[0, 0, 0, 0]);
    let _ = p.die();
    died.recv_timeout(Duration::from_secs(3))?;
    println!(
        "{}",
        serde_json::json!({"passed":true,"generation":generation,"callbacks":3,"slots":2,"equal_payload_callbacks":2,"callback_uid":1000,"counters":counters,"death_notified":true,"second_attachment_refused":true,"storage_qualified":false,"installed_authority_qualified":false})
    );
    Ok(())
}
