#![cfg(feature = "isolated-fixture")]
//! Actual Source datagrams, credentials and pidfds with a real BOOTTIME gap.
//! The shared peer harness retains and actually waits its child.
include!("process.rs");

#[test]
fn established_timing_loss_withdraws_authority_without_closing_receiver() {
    use pokelinux_lifecycle::session::Provider;

    let (mut peer, mut observer) = start(0);
    establish(
        &mut peer,
        &mut observer,
        "owned-focus",
        "owned-focus-heartbeat",
    );
    let sample = observer
        .snapshot()
        .expect("fresh pair must export authority");
    assert_eq!(sample.sequence, 2);
    assert_eq!(sample.focus.as_ref().unwrap().pid, peer.child.id());
    let sampled_at = sample.sampled_at_ns;
    let original_expiry = sampled_at + pokelinux_session::SAMPLE_BUDGET_NS;
    eprintln!(
        "ACTUAL_SOURCE_ESTABLISHED sequence=2 sampled_at_ns={sampled_at} original_expiry_ns={original_expiry}"
    );

    // This deadline comes from the already exported sample, never a fake clock.
    let hold_until = original_expiry + 20_000_000;
    while boottime_ns() < hold_until {
        std::thread::sleep(Duration::from_millis(1));
    }
    peer.command("owned-focus-heartbeat"); // Actual next wire sequence is 3.
    let after_gap = Observer::poll(&mut observer);
    assert!(
        after_gap.is_ok(),
        "timing loss must not terminate the Source receiver; poll={after_gap:?}; rejection={:?}",
        Provider::first_rejection(&observer)
    );
    assert!(
        observer.snapshot().is_none(),
        "timing loss must withdraw the previously exported authority"
    );

    // Preserve the raw sequence/peer/protocol chain; do not recreate the source.
    // Successful polls here require no renewed snapshot through the old API.
    for sequence in [4, 5] {
        peer.command("owned-focus-heartbeat");
        Observer::poll(&mut observer).unwrap_or_else(|error| {
            panic!("valid packet {sequence} was rejected after timing loss: {error}")
        });
    }
}

mod recovery {
    use super::*;
    use pokelinux_lifecycle::session::{Provider, SourceRejectionReason, Update};
    use serde_json::{Value, json};

    fn send(peer: &mut Peer, frames: Vec<Value>) {
        peer.command(&format!(
            "generation-wire:{}",
            serde_json::to_string(&frames).unwrap()
        ));
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
    fn expire_export(observer: &mut Observer) -> u64 {
        let until =
            observer.snapshot().unwrap().sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS;
        await_boottime(until + 10_000_000);
        assert!(observer.snapshot().is_none());
        boottime_ns()
    }
    fn recovering(observer: &mut Observer) {
        assert!(matches!(
            Provider::poll(observer).unwrap(),
            Update::Recovering
        ));
        assert!(observer.snapshot().is_none());
        assert!(Provider::first_rejection(observer).is_none());
    }

    #[test]
    fn pending_loss_precedes_a_post_discovery_pair_in_one_drain() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        let observed_by = expire_export(&mut observer);
        peer.command("owned-focus-heartbeat");
        peer.command("owned-focus-heartbeat");
        Observer::poll(&mut observer).unwrap();
        assert!(
            observer.snapshot().is_none(),
            "pending loss was hidden by queued recovery"
        );
        let staged_revision = observer.revision();
        assert!(observer.snapshot().is_none());
        assert_eq!(observer.revision(), staged_revision);
        let loss = provider_loss(&mut observer);
        assert!(loss.observed_at_ns <= observed_by);
        assert_eq!(loss.source, SOURCE);
        assert_eq!(observer.revision(), staged_revision);
        let current = provider_current(&mut observer);
        assert_eq!(current.revision, staged_revision + 1);
        assert_eq!(provider_current(&mut observer).revision, current.revision);
        assert_eq!(current.sequence, 4);
        assert!(current.sampled_at_ns > loss.observed_at_ns);
        assert_eq!(
            current.until,
            current.sampled_at_ns + pokelinux_session::SAMPLE_BUDGET_NS
        );
    }

    #[test]
    fn queued_pair_before_loss_discovery_cannot_reestablish_authority() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        let initial = observer.snapshot().unwrap().sampled_at_ns;
        await_boottime(initial + 40_000_000);
        peer.command("owned-focus-heartbeat");
        peer.command("owned-focus-heartbeat");
        assert!(
            boottime_ns() < initial + pokelinux_session::SAMPLE_BUDGET_NS,
            "fixture missed the original exported lease"
        );
        await_boottime(initial + pokelinux_session::SAMPLE_BUDGET_NS + 10_000_000);
        let loss = provider_loss(&mut observer);
        recovering(&mut observer);
        peer.command("owned-focus-heartbeat");
        recovering(&mut observer);
        peer.command("owned-focus-heartbeat");
        let current = provider_current(&mut observer);
        assert_eq!(current.sequence, 6);
        assert!(current.sampled_at_ns > loss.observed_at_ns);
    }

    #[test]
    fn expired_recovery_candidate_needs_two_new_fresh_samples() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        expire_export(&mut observer);
        provider_loss(&mut observer);
        peer.command("owned-focus-heartbeat");
        recovering(&mut observer);
        await_boottime(boottime_ns() + pokelinux_session::SAMPLE_BUDGET_NS + 10_000_000);
        peer.command("owned-focus-heartbeat");
        recovering(&mut observer);
        peer.command("owned-focus-heartbeat");
        assert_eq!(provider_current(&mut observer).sequence, 5);
    }

    #[test]
    fn a_drained_pair_expiring_before_export_does_not_commit_recovery() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        let first_loss_observed_by = expire_export(&mut observer);
        peer.command("owned-focus-heartbeat");
        peer.command("owned-focus-heartbeat");
        Observer::poll(&mut observer).unwrap();
        assert!(observer.snapshot().is_none());
        await_boottime(boottime_ns() + pokelinux_session::SAMPLE_BUDGET_NS + 10_000_000);
        peer.command("owned-focus-heartbeat");
        let loss = provider_loss(&mut observer);
        assert!(
            loss.observed_at_ns <= first_loss_observed_by,
            "private pair committed recovery and replaced the original pending loss"
        );
        recovering(&mut observer);
        peer.command("owned-focus-heartbeat");
        assert_eq!(provider_current(&mut observer).sequence, 6);
    }

    #[test]
    fn both_recovery_samples_must_still_be_fresh_at_export() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        expire_export(&mut observer);
        provider_loss(&mut observer);
        peer.command("owned-focus-heartbeat");
        let first_sent_by = boottime_ns();
        Observer::poll(&mut observer).unwrap();
        await_boottime(first_sent_by + 60_000_000);
        let second_sent_after = boottime_ns();
        peer.command("owned-focus-heartbeat");
        Observer::poll(&mut observer).unwrap();
        await_boottime(first_sent_by + pokelinux_session::SAMPLE_BUDGET_NS + 10_000_000);
        assert!(
            boottime_ns() < second_sent_after + pokelinux_session::SAMPLE_BUDGET_NS,
            "fixture's latest sample also expired before export"
        );
        recovering(&mut observer);
        peer.command("owned-focus-heartbeat");
        assert_eq!(provider_current(&mut observer).sequence, 5);
    }

    #[test]
    fn unlock_during_recovery_is_masked_without_erasing_raw_eligibility() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![locked(1), locked(1)]);
        assert_eq!(provider_current(&mut observer).lock_generation, Some(1));
        expire_export(&mut observer);
        provider_loss(&mut observer);
        send(&mut peer, vec![unlock(1)]);
        recovering(&mut observer);
        send(&mut peer, vec![repeated()]);
        let current = provider_current(&mut observer);
        assert_eq!(current.sequence, 4);
        assert!(current.protocol_unlock.is_none());
        send(&mut peer, vec![repeated()]);
        assert!(provider_current(&mut observer).protocol_unlock.is_none());
        send(&mut peer, vec![locked(2)]);
        assert_eq!(provider_current(&mut observer).lock_generation, Some(2));
        send(&mut peer, vec![unlock(2)]);
        let event = provider_current(&mut observer).protocol_unlock.unwrap();
        assert_eq!((event.generation, event.sequence), (2, 7));
    }

    #[test]
    fn already_exported_unlock_does_not_reappear_after_recovery() {
        let (mut peer, mut observer) = start(0);
        send(&mut peer, vec![locked(1), locked(1)]);
        provider_current(&mut observer);
        send(&mut peer, vec![unlock(1)]);
        let unlocked = provider_current(&mut observer);
        assert_eq!(unlocked.protocol_unlock.as_ref().unwrap().sequence, 3);
        let unlocked_revision = unlocked.revision;
        expire_export(&mut observer);
        provider_loss(&mut observer);
        send(&mut peer, vec![repeated(), repeated()]);
        let recovered = provider_current(&mut observer);
        assert!(recovered.protocol_unlock.is_none());
        assert!(
            recovered.revision > unlocked_revision,
            "changed exported unlock eligibility reused the previous semantic revision"
        );
        send(&mut peer, vec![unlock(1)]);
        assert!(Provider::poll(&mut observer).is_err());
        assert_eq!(
            Provider::first_rejection(&observer).unwrap().reason,
            SourceRejectionReason::ProtocolFacts
        );
    }

    #[test]
    fn malformed_identity_sequence_time_and_protocol_errors_stay_terminal_after_loss() {
        for (mode, reason) in [
            ("generation-wire:[{\"locked\":false,\"protocol_unlock\":null}]".to_owned(), SourceRejectionReason::MalformedFrame),
            ("delegated".to_owned(), SourceRejectionReason::SenderCredentials),
            ("owned-focus-file".to_owned(), SourceRejectionReason::PeerHandle),
            ("gap".to_owned(), SourceRejectionReason::Sequence),
            ("timestamp-heartbeat:1".to_owned(), SourceRejectionReason::SampleBackwards),
            (format!("timestamp-heartbeat:{}", boottime_ns() + 5_000_000_000), SourceRejectionReason::SampleFuture),
            ("generation-wire:[{\"locked\":true,\"mapped\":true,\"lock_generation\":2,\"protocol_unlock\":null}]".to_owned(), SourceRejectionReason::ProtocolFacts),
        ] {
            let (mut peer, mut observer) = start(0);
            establish(&mut peer, &mut observer, "owned-focus", "owned-focus-heartbeat");
            expire_export(&mut observer);
            provider_loss(&mut observer);
            peer.command(&mode);
            assert!(Provider::poll(&mut observer).is_err(), "strict error was softened: {reason:?}");
            let first = Provider::first_rejection(&observer).unwrap().clone();
            assert_eq!(first.reason, reason);
            peer.command("owned-focus-heartbeat");
            assert!(Provider::poll(&mut observer).is_err());
            assert_eq!(Provider::first_rejection(&observer), Some(&first));
            assert!(observer.snapshot().is_none());
        }
    }

    #[test]
    fn source_death_while_recovering_remains_terminal() {
        let (mut peer, mut observer) = start(0);
        establish(
            &mut peer,
            &mut observer,
            "owned-focus",
            "owned-focus-heartbeat",
        );
        expire_export(&mut observer);
        provider_loss(&mut observer);
        peer.command("die");
        peer.reap();
        assert!(Provider::poll(&mut observer).is_err());
        assert_eq!(
            Provider::first_rejection(&observer).unwrap().reason,
            SourceRejectionReason::SourceNotLiving
        );
        assert!(Provider::poll(&mut observer).is_err());
        assert!(observer.snapshot().is_none());
    }
}
