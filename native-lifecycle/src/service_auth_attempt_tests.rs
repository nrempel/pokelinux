//! Host actor/typed-Provider tests. Authentication/closure stamps and successful
//! storage completion are authored models. No Source IPC, PAM/broker scope,
//! durable storage write or installed restoration is exercised here.
use super::refresh_session;
use crate::{
    Permission, Reply,
    authority::{AuthenticatedCompletion, Authority, ClosedAuthenticationAttempt, Decision},
    boottime_ns,
    session::{NativeFocus, OwnedFocus, Provider, SAMPLE_BUDGET_NS, Snapshot},
    storage::{Durable, Grant},
};
use std::{io, process::Child};

const ATTEMPT_BUDGET_NS: u64 = 20_000_000_000;

struct Facts(Option<Snapshot>);
impl Provider for Facts {
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        Ok(self.0.clone())
    }
    fn stop(&mut self) {}
    fn occupied(&self) -> bool {
        false
    }
}

fn own_locker() -> OwnedFocus {
    OwnedFocus::new(
        NativeFocus {
            pid: std::process::id(),
            uid: rustix::process::getuid().as_raw(),
            gid: rustix::process::getgid().as_raw(),
        },
        rustix::process::pidfd_open(
            rustix::process::getpid(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap(),
    )
}

fn facts(locker: OwnedFocus) -> Facts {
    let sampled_at_ns = boottime_ns();
    Facts(Some(Snapshot {
        source: "33333333333333333333333333333333".into(),
        revision: 1,
        sequence: 1,
        sampled_at_ns,
        until: sampled_at_ns + SAMPLE_BUDGET_NS,
        locked: true,
        focus: None,
        locker: Some(locker),
        lock_generation: Some(1),
        protocol_unlock: None,
    }))
}

fn poll(authority: &mut Authority, facts: &mut Facts) {
    refresh_session(authority, &mut Some(facts as &mut dyn Provider)).unwrap();
}

fn authority(facts: &mut Facts) -> Authority {
    let mut authority = Authority::loading(vec![]).unwrap();
    authority.activate(Durable {
        grants: vec![Grant {
            app: "messages".into(),
            permission: Permission::MessagesRead,
            allowed: true,
        }],
        receipts: vec!["retained-history".into()],
        lock_pending: true,
    });
    authority.require_session().unwrap();
    poll(&mut authority, facts);
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        crate::authority::Decision::Immediate(Reply::Ok)
    ));
    assert!(!authority.session_terminal());
    authority
}

fn assert_policy_retained(authority: &mut Authority) {
    assert!(authority.durable.lock_pending);
    assert_eq!(authority.durable.receipts, ["retained-history"]);
    assert_eq!(authority.durable.grants.len(), 1);
    let grant = &authority.durable.grants[0];
    assert_eq!(grant.app, "messages");
    assert_eq!(grant.permission, Permission::MessagesRead);
    assert!(grant.allowed);
    assert!(matches!(
        authority.snapshot(false, false),
        Reply::Snapshot { locked: true, .. }
    ));
}

fn assert_fresh(sample: &Snapshot, until: u64) {
    let now = boottime_ns();
    assert!(now < until, "unrelated original attempt deadline expired");
    assert!(now < sample.until, "unrelated 100ms Source lease expired");
}

#[test]
fn auth_attempt_provider_control_preserves_exact_lock_and_durable_denial() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    poll(&mut authority, &mut facts);
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.locker = Some(own_locker());
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    assert_policy_retained(&mut authority);
}

// Internal begin takes (now, absolute BOOTTIME deadline). Its opaque non-Clone
// handle is inert. current checks and latches lifetime only; it cannot authorize
// a worker, report authentication success, produce a candidate or clear policy.
// The preceding frozen API-prerequisite RED executed no attempt test bodies.

#[test]
fn auth_attempt_exact_live_lock_admits_only_one_inert_handle() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert!(authority.begin_authentication_attempt(now, until).is_err());
    poll(&mut authority, &mut facts);
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.revision += 1;
    sample.locker = Some(own_locker());
    poll(&mut authority, &mut facts);
    authority.lifecycle(None, None, None).unwrap();
    assert!(authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_new_lock_admission_retires_handle_even_if_already_pending() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.current_authentication_attempt(&attempt, boottime_ns()));
    // Same Source, accepted generation, live locker and already-synced pending
    // intent. A successful new admission must still advance its own epoch.
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        crate::authority::Decision::Immediate(Reply::Ok)
    ));
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_new_accepted_operation_retires_same_process_handle() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.revision += 1;
    sample.lock_generation = Some(2);
    sample.locker = Some(own_locker());
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_provider_loss_is_terminal_without_handle_revival() {
    for replacement_source in [None, Some("44444444444444444444444444444444")] {
        let mut facts = facts(own_locker());
        let mut authority = authority(&mut facts);
        let now = boottime_ns();
        let until = now + ATTEMPT_BUDGET_NS;
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        let original = facts.0.clone().unwrap();
        match replacement_source {
            None => facts.0 = None,
            Some(source) => facts.0.as_mut().unwrap().source = source.into(),
        }
        poll(&mut authority, &mut facts);
        assert!(authority.session_terminal());
        assert_fresh(&original, until);
        assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
        facts.0 = Some(original.clone());
        poll(&mut authority, &mut facts);
        assert!(authority.session_terminal());
        assert_fresh(&original, until);
        assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
        assert_policy_retained(&mut authority);
    }
}

struct LockerChild(Child);
impl Drop for LockerChild {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().unwrap();
        }
        self.0.wait().unwrap();
    }
}

#[test]
fn auth_attempt_retained_locker_death_retires_before_next_source_poll() {
    let mut child = LockerChild(
        std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap(),
    );
    let locker = OwnedFocus::new(
        NativeFocus {
            pid: child.0.id(),
            uid: rustix::process::getuid().as_raw(),
            gid: rustix::process::getgid().as_raw(),
        },
        rustix::process::pidfd_open(
            rustix::process::Pid::from_raw(child.0.id() as i32).unwrap(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap(),
    );
    let mut facts = facts(locker);
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_belongs_to_exact_actor_incarnation() {
    let locker = own_locker();
    let mut first_facts = facts(locker.clone());
    let mut second_facts = facts(locker);
    let mut first = authority(&mut first_facts);
    let mut second = authority(&mut second_facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = first.begin_authentication_attempt(now, until).unwrap();
    let second_attempt = second.begin_authentication_attempt(now, until).unwrap();
    assert_fresh(second_facts.0.as_ref().unwrap(), until);
    assert!(!second.current_authentication_attempt(&attempt, boottime_ns()));
    assert!(second.current_authentication_attempt(&second_attempt, boottime_ns()));
    assert!(first.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut first);
    assert_policy_retained(&mut second);
}

#[test]
fn auth_attempt_stable_heartbeat_cannot_extend_original_deadline() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + 50_000_000;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.sampled_at_ns = boottime_ns();
    sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
    assert!(until < sample.until);
    poll(&mut authority, &mut facts);
    assert!(authority.current_authentication_attempt(&attempt, boottime_ns()));
    // Exact absolute deadline is refused without sleeping or timer delivery;
    // the refreshed 100ms Source lease still extends beyond this instant.
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, until));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_unknown_accepted_generation_never_admits_a_handle() {
    let mut facts = facts(own_locker());
    facts.0.as_mut().unwrap().lock_generation = None;
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    assert!(
        authority
            .begin_authentication_attempt(now, now + ATTEMPT_BUDGET_NS)
            .is_err()
    );
    assert!(!authority.session_terminal());
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_observed_context_excursion_cannot_revive_when_facts_return() {
    for excursion in 0..3 {
        let mut facts = facts(own_locker());
        let mut authority = authority(&mut facts);
        let now = boottime_ns();
        let until = now + ATTEMPT_BUDGET_NS;
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        let original = facts.0.clone().unwrap();
        let sample = facts.0.as_mut().unwrap();
        sample.sequence += 1;
        sample.revision += 1;
        match excursion {
            0 => sample.lock_generation = Some(2),
            1 => sample.locker = None,
            2 => {
                sample.locked = false;
                sample.locker = None;
                sample.lock_generation = None;
                sample.protocol_unlock = Some(crate::session::ProtocolUnlock {
                    generation: 1,
                    sequence: sample.sequence,
                    observed_at_ns: sample.sampled_at_ns,
                });
            }
            _ => unreachable!(),
        }
        poll(&mut authority, &mut facts);
        assert!(!authority.session_terminal());
        // Return the operation context before calling current. A query-only
        // comparison would miss this excursion and incorrectly revive it.
        let mut returned = original;
        returned.sequence += 2;
        returned.revision += 2;
        facts.0 = Some(returned);
        poll(&mut authority, &mut facts);
        assert!(!authority.session_terminal());
        assert_fresh(facts.0.as_ref().unwrap(), until);
        assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
        assert!(
            authority
                .begin_authentication_attempt(boottime_ns(), until)
                .is_err()
        );
        assert_policy_retained(&mut authority);
    }
}

#[test]
fn auth_attempt_source_lease_expires_without_timer_and_never_frees_slot() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    let source_until = facts.0.as_ref().unwrap().until;
    assert!(source_until < until);
    assert_fresh(facts.0.as_ref().unwrap(), until);
    // Inject only the actor's current-time argument; no sleep/timer runs.
    assert!(!authority.current_authentication_attempt(&attempt, source_until));
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.sampled_at_ns = boottime_ns();
    sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
    poll(&mut authority, &mut facts);
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), until)
            .is_err()
    );
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_expired_deadline_is_sticky_and_slot_stays_occupied() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, until));
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), boottime_ns() + ATTEMPT_BUDGET_NS)
            .is_err()
    );
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_rejects_invalid_deadlines_without_occupying_slot() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    for until in [0, now - 1, now, now + 30_000_000_001, u64::MAX] {
        assert!(authority.begin_authentication_attempt(now, until).is_err());
    }
    assert!(
        authority
            .begin_authentication_attempt(now + 1_000_000_000, now + 2_000_000_000)
            .is_err()
    );
    let until = now + 30_000_000_000;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_shared_reference_cannot_hide_a_wrong_kernel_task() {
    let child = LockerChild(
        std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap(),
    );
    let forged = OwnedFocus::new(
        own_locker().metadata,
        rustix::process::pidfd_open(
            rustix::process::Pid::from_raw(child.0.id() as i32).unwrap(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap(),
    );
    // Source caches the exact same Arc, but its metadata names this parent
    // while its actual live pidfd names a different child. Begin must use
    // ProcessView.matches even for an unchanged shared reference.
    let mut malformed = facts(forged.clone());
    let mut invalid = authority(&mut malformed);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    assert_fresh(malformed.0.as_ref().unwrap(), until);
    assert!(invalid.begin_authentication_attempt(now, until).is_err());
    assert!(!invalid.session_terminal());
    assert_policy_retained(&mut invalid);

    // Also replace the descriptor after a genuine attempt was admitted, using
    // identical metadata and a newer semantic revision. Restore the original
    // context before any current query; the observed mismatch must stay retired.
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    let original = facts.0.clone().unwrap();
    let changed = facts.0.as_mut().unwrap();
    changed.sequence += 1;
    changed.revision += 1;
    changed.locker = Some(forged);
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    let mut returned = original;
    returned.sequence += 2;
    returned.revision += 2;
    facts.0 = Some(returned);
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_attempt_explicit_lifecycle_and_degrade_retire_without_reuse() {
    for change in 0..4 {
        let mut facts = facts(own_locker());
        let mut authority = authority(&mut facts);
        let now = boottime_ns();
        let until = now + ATTEMPT_BUDGET_NS;
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        match change {
            0 => authority.lifecycle(Some(true), None, None).unwrap(),
            1 => authority.lifecycle(Some(false), None, None).unwrap(),
            2 => {
                authority.lifecycle(None, Some(true), None).unwrap();
                authority.lifecycle(None, Some(false), None).unwrap();
                let sample = facts.0.as_mut().unwrap();
                sample.sequence += 1;
                sample.sampled_at_ns = boottime_ns();
                sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
                poll(&mut authority, &mut facts);
            }
            3 => authority.invalidate(),
            _ => unreachable!(),
        }
        assert_fresh(facts.0.as_ref().unwrap(), until);
        assert!(!authority.current_authentication_attempt(&attempt, boottime_ns()));
        assert!(
            authority
                .begin_authentication_attempt(boottime_ns(), until)
                .is_err()
        );
        assert_policy_retained(&mut authority);
    }
}

// The cfg-test-only builder authors trusted closure data. These tests exercise
// actor capacity handling, not an actual broker/scope/PAM cleanup proof. There
// is no production completion constructor or success/unlock ingress.
#[test]
fn auth_closed_capacity_allows_one_fresh_explicit_attempt_without_unlock() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let closed = ClosedAuthenticationAttempt::fixture(&old, until);
    let duplicate = ClosedAuthenticationAttempt::fixture(&old, until);
    assert!(authority.begin_authentication_attempt(now, until).is_err());
    assert!(authority.release_closed_authentication(closed));
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert_policy_retained(&mut authority);
    let now = boottime_ns();
    let fresh = authority
        .begin_authentication_attempt(now, now + ATTEMPT_BUDGET_NS)
        .unwrap();
    assert!(!authority.release_closed_authentication(duplicate));
    assert!(authority.current_authentication_attempt(&fresh, boottime_ns()));
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_closed_capacity_refuses_other_actor_and_changed_original_deadline() {
    let locker = own_locker();
    let mut first_facts = facts(locker.clone());
    let mut second_facts = facts(locker);
    let mut first = authority(&mut first_facts);
    let mut second = authority(&mut second_facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let first_handle = first.begin_authentication_attempt(now, until).unwrap();
    let second_handle = second.begin_authentication_attempt(now, until).unwrap();
    assert!(
        !first.release_closed_authentication(ClosedAuthenticationAttempt::fixture(
            &second_handle,
            until
        ))
    );
    assert!(
        !first.release_closed_authentication(ClosedAuthenticationAttempt::fixture(
            &first_handle,
            until + 1
        ))
    );
    assert!(first.current_authentication_attempt(&first_handle, boottime_ns()));
    assert!(second.current_authentication_attempt(&second_handle, boottime_ns()));
    assert_policy_retained(&mut first);
    assert_policy_retained(&mut second);
}

#[test]
fn auth_closed_capacity_does_not_release_an_absent_or_successor_slot() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let first = ClosedAuthenticationAttempt::fixture(&old, until);
    let absent = ClosedAuthenticationAttempt::fixture(&old, until);
    let replay = ClosedAuthenticationAttempt::fixture(&old, until);
    assert!(authority.release_closed_authentication(first));
    assert!(!authority.release_closed_authentication(absent));
    let fresh = authority
        .begin_authentication_attempt(boottime_ns(), until)
        .unwrap();
    assert!(!authority.release_closed_authentication(replay));
    assert!(authority.current_authentication_attempt(&fresh, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_closed_capacity_after_terminal_provider_loss_cannot_restore_the_source() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let closed = ClosedAuthenticationAttempt::fixture(&old, until);
    let original = facts.0.take().unwrap();
    poll(&mut authority, &mut facts);
    assert!(authority.session_terminal());
    assert!(authority.release_closed_authentication(closed));
    facts.0 = Some(original);
    poll(&mut authority, &mut facts);
    assert!(authority.session_terminal());
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), until)
            .is_err()
    );
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_closed_capacity_after_actual_source_expiry_keeps_locked_policy() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let closed = ClosedAuthenticationAttempt::fixture(&old, until);
    let source_until = facts.0.as_ref().unwrap().until;
    std::thread::sleep(
        std::time::Duration::from_nanos(source_until.saturating_sub(boottime_ns()))
            + std::time::Duration::from_millis(2),
    );
    assert!(boottime_ns() >= source_until);
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert!(authority.release_closed_authentication(closed));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), until)
            .is_err()
    );
    poll(&mut authority, &mut facts);
    assert!(authority.session_terminal());
    assert_eq!(facts.0.as_ref().unwrap().until, source_until);
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_closed_capacity_after_original_attempt_expiry_allows_only_a_new_deadline() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + 20_000_000;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let closed = ClosedAuthenticationAttempt::fixture(&old, until);
    std::thread::sleep(
        std::time::Duration::from_nanos(until.saturating_sub(boottime_ns()))
            + std::time::Duration::from_millis(2),
    );
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert!(authority.release_closed_authentication(closed));
    assert!(authority.begin_authentication_attempt(now, until).is_err());
    assert_fresh(facts.0.as_ref().unwrap(), boottime_ns() + ATTEMPT_BUDGET_NS);
    let now = boottime_ns();
    let fresh = authority
        .begin_authentication_attempt(now, now + ATTEMPT_BUDGET_NS)
        .unwrap();
    assert!(authority.current_authentication_attempt(&fresh, boottime_ns()));
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert_policy_retained(&mut authority);
}

#[test]
fn auth_closed_capacity_after_new_lock_keeps_new_epoch_and_policy() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let closed = ClosedAuthenticationAttempt::fixture(&old, until);
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        crate::authority::Decision::Immediate(Reply::Ok)
    ));
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert!(authority.release_closed_authentication(closed));
    assert_policy_retained(&mut authority);
    let fresh = authority
        .begin_authentication_attempt(boottime_ns(), until)
        .unwrap();
    assert!(authority.current_authentication_attempt(&fresh, boottime_ns()));
    assert!(!authority.current_authentication_attempt(&old, boottime_ns()));
    assert_policy_retained(&mut authority);
}

// Whole-owner authenticated closure is authored only by the cfg-test fixture.
// There is no production constructor or success IPC. These tests cover actor
// authority; the one successful finish below models storage acknowledgement.
fn unlock_facts(facts: &mut Facts, generation: u64) {
    let sample = facts.0.as_mut().unwrap();
    sample.sequence += 1;
    sample.revision += 1;
    sample.sampled_at_ns = boottime_ns();
    sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
    sample.locked = false;
    sample.locker = None;
    sample.lock_generation = None;
    sample.protocol_unlock = Some(crate::session::ProtocolUnlock {
        generation,
        sequence: sample.sequence,
        observed_at_ns: sample.sampled_at_ns,
    });
}

fn assert_no_authenticated_clear(authority: &mut Authority, now: u64) {
    assert!(matches!(
        authority.plan_authenticated_clear(now),
        Decision::Immediate(Reply::Denied | Reply::Unavailable)
    ));
    assert_policy_retained(authority);
}

#[test]
fn auth_restore_exact_completion_arms_atomically_without_freeing_capacity() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    let completion = AuthenticatedCompletion::fixture(&attempt, until);
    let replay = AuthenticatedCompletion::fixture(&attempt, until);
    let closed = ClosedAuthenticationAttempt::fixture(&attempt, until);
    assert!(authority.arm_authentication(completion, boottime_ns()));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), until)
            .is_err()
    );
    assert!(!authority.release_closed_authentication(closed));
    assert!(!authority.arm_authentication(replay, boottime_ns()));
    assert!(
        authority
            .begin_authentication_attempt(boottime_ns(), until)
            .is_err()
    );
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert_no_authenticated_clear(&mut authority, boottime_ns());
}

#[test]
fn auth_restore_completion_requires_exact_actor_and_original_deadline() {
    let locker = own_locker();
    let mut first_facts = facts(locker.clone());
    let mut second_facts = facts(locker);
    let mut first = authority(&mut first_facts);
    let mut second = authority(&mut second_facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let first_attempt = first.begin_authentication_attempt(now, until).unwrap();
    let second_attempt = second.begin_authentication_attempt(now, until).unwrap();
    assert!(!first.arm_authentication(
        AuthenticatedCompletion::fixture(&second_attempt, until),
        boottime_ns()
    ));
    assert!(!first.arm_authentication(
        AuthenticatedCompletion::fixture(&first_attempt, until + 1),
        boottime_ns()
    ));
    assert!(first.current_authentication_attempt(&first_attempt, boottime_ns()));
    assert!(second.current_authentication_attempt(&second_attempt, boottime_ns()));
    assert!(first.arm_authentication(
        AuthenticatedCompletion::fixture(&first_attempt, until),
        boottime_ns()
    ));
    assert_fresh(first_facts.0.as_ref().unwrap(), until);
    assert_policy_retained(&mut first);
    assert_policy_retained(&mut second);
}

#[test]
fn auth_restore_refuses_prearm_and_inexact_protocol_unlock_occurrences() {
    for invalid in 0..4 {
        let mut facts = facts(own_locker());
        let mut authority = authority(&mut facts);
        let now = boottime_ns();
        let until = now + ATTEMPT_BUDGET_NS;
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        let completion = AuthenticatedCompletion::fixture(&attempt, until);
        let before_arm = facts.0.as_ref().unwrap().clone();
        if invalid == 0 {
            unlock_facts(&mut facts, 1);
            poll(&mut authority, &mut facts);
            assert!(!authority.arm_authentication(completion, boottime_ns()));
        } else {
            assert!(authority.arm_authentication(completion, boottime_ns()));
            unlock_facts(&mut facts, 1);
            let event = facts.0.as_mut().unwrap().protocol_unlock.as_mut().unwrap();
            match invalid {
                1 => event.observed_at_ns = before_arm.sampled_at_ns,
                2 => event.sequence = before_arm.sequence,
                3 => event.generation = 2,
                _ => unreachable!(),
            }
            poll(&mut authority, &mut facts);
        }
        assert!(!authority.session_terminal());
        assert_fresh(facts.0.as_ref().unwrap(), until);
        assert_no_authenticated_clear(&mut authority, boottime_ns());
    }
}

#[test]
fn auth_restore_later_exact_unlock_plans_clear_but_waits_for_model_finish() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    assert_no_authenticated_clear(&mut authority, boottime_ns());
    assert_fresh(facts.0.as_ref().unwrap(), until);
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert_policy_retained(&mut authority);
    let Decision::Persist(intent) = authority.plan_authenticated_clear(boottime_ns()) else {
        panic!("later exact protocol unlock must plan a durable clear");
    };
    assert!(!intent.next.lock_pending);
    assert_eq!(intent.next.receipts, ["retained-history"]);
    assert_eq!(intent.next.grants.len(), 1);
    assert!(intent.next.grants[0].allowed);
    assert!(intent.until <= until);
    assert_policy_retained(&mut authority);
    // A second plan cannot issue another write while the first owns completion.
    assert_no_authenticated_clear(&mut authority, boottime_ns());
    // Model acknowledgement only: no storage child or filesystem write runs.
    assert_eq!(authority.finish(intent, true, boottime_ns()), Reply::Ok);
    assert!(!authority.durable.lock_pending);
    assert_eq!(authority.durable.receipts, ["retained-history"]);
    assert_eq!(authority.durable.grants.len(), 1);
    assert!(authority.durable.grants[0].allowed);
    assert!(matches!(
        authority.snapshot(false, false),
        Reply::Snapshot { locked: false, .. }
    ));
    assert!(matches!(
        authority.plan_authenticated_clear(boottime_ns()),
        Decision::Immediate(Reply::Denied | Reply::Unavailable)
    ));
}

#[test]
fn auth_restore_forced_unlock_without_event_cannot_be_repaired_by_later_event() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    unlock_facts(&mut facts, 1);
    facts.0.as_mut().unwrap().protocol_unlock = None;
    poll(&mut authority, &mut facts);
    assert!(!authority.session_terminal());
    assert_no_authenticated_clear(&mut authority, boottime_ns());
    // Source later supplies a well-shaped event; the observed forced excursion
    // already retired this reservation and cannot become an authenticated cause.
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert_no_authenticated_clear(&mut authority, boottime_ns());
}

#[test]
fn auth_restore_new_lock_source_loss_and_original_expiries_never_revive() {
    for retirement in 0..4 {
        let mut facts = facts(own_locker());
        let mut authority = authority(&mut facts);
        let now = boottime_ns();
        let until = now
            + if retirement == 3 {
                20_000_000
            } else {
                ATTEMPT_BUDGET_NS
            };
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        assert!(authority.arm_authentication(
            AuthenticatedCompletion::fixture(&attempt, until),
            boottime_ns()
        ));
        match retirement {
            0 => assert!(matches!(
                authority.plan_lock(boottime_ns()),
                Decision::Immediate(Reply::Ok)
            )),
            1 => {
                let retained = facts.0.take().unwrap();
                poll(&mut authority, &mut facts);
                assert!(authority.session_terminal());
                facts.0 = Some(retained);
            }
            2 | 3 => {
                let source_until = facts.0.as_ref().unwrap().until;
                let expires = if retirement == 2 { source_until } else { until };
                // Bounded actual expiration; the Source budget remains 100ms.
                std::thread::sleep(
                    std::time::Duration::from_nanos(expires.saturating_sub(boottime_ns()))
                        + std::time::Duration::from_millis(2),
                );
                assert!(boottime_ns() >= expires);
                if retirement == 3 {
                    assert!(
                        boottime_ns() < source_until,
                        "Source expired before attempt control"
                    );
                }
                assert_no_authenticated_clear(&mut authority, boottime_ns());
            }
            _ => unreachable!(),
        }
        // A later fresh sample keeps the same Source and exact historical
        // generation; it cannot renew the retired reservation/deadline/epoch.
        unlock_facts(&mut facts, 1);
        poll(&mut authority, &mut facts);
        assert_no_authenticated_clear(&mut authority, boottime_ns());
    }
}

#[test]
fn auth_restore_old_completion_cannot_arm_an_absent_or_successor_attempt() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let old = authority.begin_authentication_attempt(now, until).unwrap();
    let absent = AuthenticatedCompletion::fixture(&old, until);
    let replay = AuthenticatedCompletion::fixture(&old, until);
    assert!(
        authority.release_closed_authentication(ClosedAuthenticationAttempt::fixture(&old, until))
    );
    assert!(!authority.arm_authentication(absent, boottime_ns()));
    let now = boottime_ns();
    let next_until = now + ATTEMPT_BUDGET_NS;
    let fresh = authority
        .begin_authentication_attempt(now, next_until)
        .unwrap();
    assert!(!authority.arm_authentication(replay, boottime_ns()));
    assert!(authority.current_authentication_attempt(&fresh, boottime_ns()));
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&fresh, next_until),
        boottime_ns()
    ));
    assert_fresh(facts.0.as_ref().unwrap(), next_until);
    assert_no_authenticated_clear(&mut authority, boottime_ns());
}

#[test]
fn auth_restore_new_lock_during_clear_refuses_ack_and_keeps_synced_bytes_truthful() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    let Decision::Persist(clear) = authority.plan_authenticated_clear(boottime_ns()) else {
        panic!("exact later event must own a clear");
    };
    // Cached durable true cannot acknowledge a new lock while this owned clear
    // may already have synced false bytes. This models a known completion only.
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        Decision::Immediate(Reply::Unavailable)
    ));
    assert_eq!(
        authority.finish(clear, true, boottime_ns()),
        Reply::Unavailable
    );
    assert!(
        !authority.durable.lock_pending,
        "retain the known false commit truthfully"
    );
    assert!(
        authority.ready(),
        "known superseded commit must permit the newer true write"
    );
    assert!(matches!(
        authority.snapshot(false, false),
        Reply::Snapshot { locked: true, .. }
    ));
    assert_eq!(authority.durable.receipts, ["retained-history"]);
    assert_eq!(authority.durable.grants.len(), 1);
    assert!(authority.durable.grants[0].allowed);
}

#[test]
fn auth_restore_source_expiry_after_clear_submission_never_publishes_unlock() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    let Decision::Persist(clear) = authority.plan_authenticated_clear(boottime_ns()) else {
        panic!("clear missing")
    };
    let expires = facts.0.as_ref().unwrap().until;
    std::thread::sleep(
        std::time::Duration::from_nanos(expires.saturating_sub(boottime_ns()))
            + std::time::Duration::from_millis(2),
    );
    assert_eq!(
        authority.finish(clear, true, boottime_ns()),
        Reply::Unavailable
    );
    assert!(!authority.durable.lock_pending);
    assert!(authority.is_degraded());
    assert!(matches!(
        authority.snapshot(false, false),
        Reply::Snapshot { locked: true, .. }
    ));
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    assert!(matches!(
        authority.plan_authenticated_clear(boottime_ns()),
        Decision::Immediate(Reply::Denied | Reply::Unavailable)
    ));
    assert_eq!(authority.durable.receipts, ["retained-history"]);
}

#[test]
fn auth_restore_required_true_is_owned_once_under_first_new_lock_deadline() {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    let Decision::Persist(clear) = authority.plan_authenticated_clear(boottime_ns()) else {
        panic!("clear missing")
    };
    let first_lock = boottime_ns();
    assert!(matches!(
        authority.plan_lock(first_lock),
        Decision::Immediate(Reply::Unavailable)
    ));
    assert!(
        authority.take_required_lock_write(boottime_ns()).is_none(),
        "clear still owns completion"
    );
    assert_eq!(
        authority.finish(clear, true, boottime_ns()),
        Reply::Unavailable
    );
    assert!(authority.ready());
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        Decision::Immediate(Reply::Unavailable)
    ));
    let pending = authority
        .take_required_lock_write(boottime_ns())
        .expect("newer true must replace known false");
    assert!(pending.next.lock_pending);
    assert!(
        pending.until <= first_lock + 1_000_000_000,
        "later admission cannot renew first unresolved lock deadline"
    );
    assert_eq!(pending.next.receipts, ["retained-history"]);
    assert_eq!(pending.next.grants.len(), 1);
    assert!(pending.next.grants[0].allowed);
    assert!(
        authority.take_required_lock_write(boottime_ns()).is_none(),
        "never duplicate owned write"
    );
    assert_eq!(authority.finish(pending, true, boottime_ns()), Reply::Ok);
    assert_policy_retained(&mut authority);
    assert!(authority.take_required_lock_write(boottime_ns()).is_none());
    assert!(matches!(
        authority.plan_lock(boottime_ns()),
        Decision::Immediate(Reply::Ok)
    ));
}

fn assert_actual_completion_deadline(required_true: bool) {
    let mut facts = facts(own_locker());
    let mut authority = authority(&mut facts);
    let now = boottime_ns();
    let until = now + ATTEMPT_BUDGET_NS;
    let attempt = authority.begin_authentication_attempt(now, until).unwrap();
    assert!(authority.arm_authentication(
        AuthenticatedCompletion::fixture(&attempt, until),
        boottime_ns()
    ));
    unlock_facts(&mut facts, 1);
    poll(&mut authority, &mut facts);
    let Decision::Persist(mut intent) = authority.plan_authenticated_clear(boottime_ns()) else {
        panic!("clear missing")
    };
    if required_true {
        assert!(matches!(
            authority.plan_lock(boottime_ns()),
            Decision::Immediate(Reply::Unavailable)
        ));
        assert_eq!(
            authority.finish(intent, true, boottime_ns()),
            Reply::Unavailable
        );
        intent = authority.take_required_lock_write(boottime_ns()).unwrap();
    }
    // Shorten only this modeled IO deadline, never the Source or attempt lease.
    // A queued callback's stale time cannot resurrect an elapsed BOOTTIME budget.
    let callback_sample = boottime_ns();
    intent.until = callback_sample + 20_000_000;
    std::thread::sleep(std::time::Duration::from_millis(22));
    assert!(boottime_ns() >= intent.until);
    assert_fresh(facts.0.as_ref().unwrap(), until);
    assert_eq!(
        authority.finish(intent, true, callback_sample),
        Reply::Unavailable
    );
    assert!(authority.is_degraded());
    assert!(matches!(
        authority.snapshot(false, false),
        Reply::Snapshot { locked: true, .. }
    ));
    assert_eq!(authority.durable.lock_pending, required_true);
    assert_eq!(authority.durable.receipts, ["retained-history"]);
}

#[test]
fn auth_restore_synced_completion_checks_actual_clear_deadline() {
    assert_actual_completion_deadline(false);
}

#[test]
fn auth_restore_synced_completion_checks_actual_required_true_deadline() {
    assert_actual_completion_deadline(true);
}
