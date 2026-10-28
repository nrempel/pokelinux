#![cfg(feature = "isolated-fixture")]
//! Real kernel transport/credentials/pidfds with authored Source packets only.
//! The shared peer harness owns and actually waits its child.
include!("process.rs");

#[test]
fn establishment_first_snapshot_waits_for_two_fresh_packets() {
    use pokelinux_lifecycle::session::Provider;
    let (mut peer, mut observer) = start(0);
    peer.command("owned-focus");
    let first = Provider::poll(&mut observer).expect("first packet must validate provisionally");
    assert!(
        first.is_none(),
        "first Source sample escaped before an uninterrupted fresh pair; actual sequence={:?}",
        first.as_ref().map(|sample| sample.sequence)
    );
    assert!(observer.snapshot().is_none());
    assert!(Provider::first_rejection(&observer).is_none());

    let expiry = boottime_ns() + pokelinux_session::SAMPLE_BUDGET_NS + 20_000_000;
    while boottime_ns() < expiry {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(Provider::poll(&mut observer).unwrap().is_none());
    assert!(Provider::first_rejection(&observer).is_none());
    peer.command("owned-focus-heartbeat");
    assert!(Provider::poll(&mut observer).unwrap().is_none());
    peer.command("owned-focus-heartbeat");
    let established = Provider::poll(&mut observer)
        .unwrap()
        .expect("fresh advancing seq2/seq3 must establish authority");
    assert_eq!(established.sequence, 3);
    let focus = established.focus.as_ref().expect("actual owned focus");
    assert_eq!(focus.metadata.pid, peer.child.id());
    assert!(
        pokelinux_kernel_identity::ProcessView::open()
            .unwrap()
            .matches(focus.pidfd(), focus.pidfd(), peer.child.id())
            .unwrap()
    );
    let expiry = established.sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS + 20_000_000;
    while boottime_ns() < expiry {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(Provider::poll(&mut observer).is_err());
    assert!(Provider::first_rejection(&observer).is_some());
    peer.command("owned-focus-heartbeat");
    assert!(Provider::poll(&mut observer).is_err());
    assert!(observer.snapshot().is_none());
}

mod controls {
    use super::*;
    use pokelinux_lifecycle::session::{Provider, SourceRejectionReason as Reason};
    use serde_json::{Value, json};

    fn plain() -> Value {
        json!({"locked":false,"lock_generation":null,"protocol_unlock":null})
    }
    fn locked(generation: u64) -> Value {
        json!({"locked":true,"mapped":true,"lock_generation":generation,"protocol_unlock":null})
    }
    fn unlock(generation: u64) -> Value {
        json!({"locked":false,"lock_generation":null,
            "protocol_unlock":{"generation":generation,"sequence":"this","observed_at_ns":"sample"}})
    }
    fn repeated() -> Value {
        json!({"locked":false,"lock_generation":null,"protocol_unlock":"repeat"})
    }
    fn send(peer: &mut Peer, frames: Vec<Value>) {
        peer.command(&format!(
            "generation-wire:{}",
            serde_json::to_string(&frames).unwrap()
        ));
    }
    fn expire_now() {
        let until = boottime_ns() + pokelinux_session::SAMPLE_BUDGET_NS + 20_000_000;
        while boottime_ns() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn pending(observer: &mut Observer) {
        assert!(
            Provider::poll(observer)
                .expect("provisional validation must succeed")
                .is_none()
        );
        assert!(observer.snapshot().is_none());
        assert!(Provider::first_rejection(observer).is_none());
    }
    fn terminal(observer: &mut Observer, peer: &mut Peer, reason: Reason) {
        assert!(
            Provider::poll(observer).is_err(),
            "strict rejection required: {reason:?}"
        );
        let first = Provider::first_rejection(observer)
            .expect("actual rejection record")
            .clone();
        assert_eq!(first.reason, reason);
        assert!(observer.snapshot().is_none());
        send(peer, vec![plain(), plain()]);
        assert!(
            Provider::poll(observer).is_err(),
            "fresh packets must not revive terminal source"
        );
        assert_eq!(Provider::first_rejection(observer), Some(&first));
        assert!(observer.snapshot().is_none());
    }
    fn established(peer: &mut Peer, observer: &mut Observer) {
        send(peer, vec![plain(), plain()]);
        let sample = Provider::poll(observer)
            .unwrap()
            .expect("fresh actual pair must export");
        assert_eq!(sample.sequence, 2);
    }

    #[test]
    fn ready_pair_without_export_resets_on_later_gap() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![plain(), plain()]);
        Observer::poll(&mut observer).unwrap(); // drain only; deliberately never export
        expire_now();
        send(&mut peer, vec![plain()]);
        pending(&mut observer);
        send(&mut peer, vec![plain()]);
        assert_eq!(Provider::poll(&mut observer).unwrap().unwrap().sequence, 4);
    }

    #[test]
    fn unread_expired_pair_and_fresh_gap_in_same_drain_remain_private() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![plain(), plain()]);
        expire_now(); // all three packets below are inspected by one actual drain
        send(&mut peer, vec![plain()]);
        pending(&mut observer);
        send(&mut peer, vec![plain()]);
        assert_eq!(Provider::poll(&mut observer).unwrap().unwrap().sequence, 4);
    }

    #[test]
    fn equal_sample_timestamps_do_not_form_an_advancing_pair() {
        let (mut peer, mut observer) = start(0);
        // The adapted raw fixture advances wire sequence, while these two
        // deliberate equal sample timestamps come from the real host clock.
        let sampled = boottime_ns();
        peer.command(&format!("timestamp-heartbeat:{sampled}"));
        pending(&mut observer);
        peer.command(&format!("timestamp-heartbeat:{sampled}"));
        pending(&mut observer);
        peer.command("owned-focus-heartbeat");
        assert_eq!(Provider::poll(&mut observer).unwrap().unwrap().sequence, 3);
    }

    #[test]
    fn wire_sequence_history_survives_provisional_candidate_expiry() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![plain(), plain()]);
        Observer::poll(&mut observer).unwrap();
        expire_now();
        pending(&mut observer);
        peer.command("gap"); // intentional raw seq2, now replaying accepted raw seq2
        terminal(&mut observer, &mut peer, Reason::Sequence);
    }

    #[test]
    fn future_and_backward_samples_are_terminal_before_first_export() {
        for future in [true, false] {
            let (mut peer, mut observer) = start(0);
            send(&mut peer, vec![plain()]);
            Observer::poll(&mut observer).unwrap();
            let sample = if future {
                boottime_ns() + 1_000_000_000
            } else {
                1
            };
            peer.command(&format!("timestamp-heartbeat:{sample}"));
            terminal(
                &mut observer,
                &mut peer,
                if future {
                    Reason::SampleFuture
                } else {
                    Reason::SampleBackwards
                },
            );
        }
    }

    #[test]
    fn malformed_sequence_credentials_and_peer_fail_without_any_priming() {
        for (mode, reason) in [
            (
                "generation-wire:[{\"locked\":false,\"protocol_unlock\":null}]",
                Reason::MalformedFrame,
            ),
            ("gap", Reason::Sequence),
            ("delegated", Reason::SenderCredentials),
            ("owned-focus-file", Reason::PeerHandle),
        ] {
            let (mut peer, mut observer) = start(0);
            peer.command(mode);
            terminal(&mut observer, &mut peer, reason);
        }
    }

    #[test]
    fn expired_private_packets_still_validate_peer_and_protocol() {
        for invalid_peer in [true, false] {
            let (mut peer, mut observer) = start(0);
            if invalid_peer {
                peer.command("owned-focus-file");
            } else {
                send(&mut peer, vec![locked(2)]); // first accepted generation cannot be2
            }
            expire_now();
            terminal(
                &mut observer,
                &mut peer,
                if invalid_peer {
                    Reason::PeerHandle
                } else {
                    Reason::ProtocolFacts
                },
            );
        }
    }

    #[test]
    fn private_unlock_stays_masked_and_later_authority_event_exports_normally() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![locked(1)]);
        pending(&mut observer);
        expire_now();
        pending(&mut observer);
        send(&mut peer, vec![unlock(1)]);
        pending(&mut observer);
        send(&mut peer, vec![repeated()]);
        let first = Provider::poll(&mut observer).unwrap().unwrap();
        assert_eq!(first.sequence, 3);
        assert!(
            first.protocol_unlock.is_none(),
            "preauthority occurrence must not become authentication evidence"
        );
        send(&mut peer, vec![repeated()]);
        assert!(
            Provider::poll(&mut observer)
                .unwrap()
                .unwrap()
                .protocol_unlock
                .is_none()
        );
        send(&mut peer, vec![locked(2)]);
        let locked = Provider::poll(&mut observer).unwrap().unwrap();
        assert_eq!(locked.lock_generation, Some(2));
        assert_eq!(
            locked.locker.as_ref().unwrap().metadata.pid,
            peer.child.id()
        );
        send(&mut peer, vec![unlock(2)]);
        let current = Provider::poll(&mut observer).unwrap().unwrap();
        let event = current
            .protocol_unlock
            .expect("new postauthority exact occurrence");
        assert_eq!((event.generation, event.sequence), (2, 6));
        send(&mut peer, vec![repeated()]);
        assert_eq!(
            Provider::poll(&mut observer)
                .unwrap()
                .unwrap()
                .protocol_unlock,
            Some(event)
        );
    }

    #[test]
    fn event_on_first_export_is_masked_but_raw_occurrence_still_fences_change() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![locked(1), unlock(1)]);
        let first = Provider::poll(&mut observer).unwrap().unwrap();
        assert_eq!(first.sequence, 2);
        assert!(first.protocol_unlock.is_none());
        send(&mut peer, vec![unlock(1)]); // another occurrence is not the sticky exact record
        terminal(&mut observer, &mut peer, Reason::ProtocolFacts);
    }

    #[test]
    fn established_gap_is_permanent_even_when_new_packet_is_fresh() {
        let (mut peer, mut observer) = start(0);
        established(&mut peer, &mut observer);
        expire_now();
        send(&mut peer, vec![plain()]);
        terminal(&mut observer, &mut peer, Reason::SampleGap);
    }

    #[test]
    fn established_expired_intermediate_cannot_be_hidden_by_fresh_successor() {
        let (mut peer, mut observer) = start(0);
        established(&mut peer, &mut observer);
        send(&mut peer, vec![plain()]);
        expire_now();
        send(&mut peer, vec![plain()]);
        terminal(&mut observer, &mut peer, Reason::SampleExpired);
    }
}
