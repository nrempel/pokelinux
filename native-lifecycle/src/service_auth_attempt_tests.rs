//! Host actor/typed-Provider tests. No Source IPC, PAM, authentication result,
//! unlock, storage write or restoration is exercised by this selection.
use super::refresh_session;
use crate::{
    Permission, Reply,
    authority::Authority,
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
