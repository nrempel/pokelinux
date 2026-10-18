//! Frozen deterministic Rust owner behavior, not OS permission, kernel or UI proof.
//! Explicit numbers represent trusted CLOCK_BOOTTIME samples; no sleeps or clock hooks.
use pokelinux_connectivity_control::owner::{
    Action, Authority, Command, Finish, Id, Observation, Owner, Pending, Phase, Reject, Reply,
};

const T: u64 = 10_000_000_000;
const UNTIL: u64 = T + 1_000_000_000;

fn authority(instance: u64) -> Authority {
    Authority {
        principal: pokelinux_connectivity_control::owner::Principal {
            instance,
            generation: 7,
        },
        permission_epoch: 11,
        allowed: true,
    }
}
fn fact(connected: bool, now: u64) -> Option<Observation> {
    Some(Observation {
        connected,
        observed_at_ns: now,
    })
}
fn staged(owner: &mut Owner, auth: Authority, action: Action) -> Id {
    owner
        .stage(auth, action, UNTIL, T)
        .expect("a permitted explicit fixed-profile choice must stage")
}
fn dispatched(owner: &mut Owner, auth: Authority, action: Action) -> Id {
    let id = staged(owner, auth, action);
    assert_eq!(
        owner.dispatch(id, auth, T + 1),
        Ok(Command {
            id,
            action,
            original_until_ns: UNTIL,
        })
    );
    id
}

#[test]
fn denied_expired_and_unknown_requests_never_dispatch() {
    let mut owner = Owner::new();
    owner.observe(false, T);
    let mut denied = authority(1);
    denied.allowed = false;
    assert_eq!(
        owner.stage(denied, Action::Connect, UNTIL, T),
        Err(Reject::Denied)
    );
    assert_eq!(
        owner.stage(authority(1), Action::Connect, T, T),
        Err(Reject::Deadline)
    );
    assert_eq!(
        owner.stage(authority(1), Action::Connect, UNTIL + 1, T),
        Err(Reject::Deadline)
    );
    assert_eq!(
        owner.stage(authority(1), Action::Connect, u64::MAX, T),
        Err(Reject::Deadline)
    );
    assert_eq!(owner.dispatch(Id(99), authority(1), T), Err(Reject::Stale));
    let view = owner.view(T + 1);
    assert_eq!(view.observed, fact(false, T));
    assert_eq!(view.pending, None);
    assert!(!view.terminal_uncertain);
}

#[test]
fn explicit_connect_disconnect_each_dispatch_once_with_original_deadline() {
    let mut owner = Owner::new();
    let auth = authority(1);
    owner.observe(false, T - 1);
    let first = staged(&mut owner, auth, Action::Connect);
    assert_eq!(
        owner.view(T).pending,
        Some(Pending {
            command: Command {
                id: first,
                action: Action::Connect,
                original_until_ns: UNTIL
            },
            phase: Phase::Staged,
        })
    );
    let command = owner.dispatch(first, auth, T + 1).unwrap();
    assert_eq!(
        command,
        Command {
            id: first,
            action: Action::Connect,
            original_until_ns: UNTIL
        }
    );
    assert_eq!(
        owner.dispatch(first, auth, T + 2),
        Err(Reject::Stale),
        "no duplicate dispatch"
    );
    assert!(owner.finish(first, T + 3, Finish::Observed(true)));
    assert_eq!(owner.view(T + 4).observed, fact(true, T + 3));
    assert_eq!(
        owner.take_reply(first, auth, T + 4),
        Some(Reply {
            id: first,
            outcome: Finish::Observed(true)
        })
    );
    assert_eq!(owner.take_reply(first, auth, T + 5), None);
    let second = owner.stage(auth, Action::Disconnect, UNTIL, T + 6).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        owner.dispatch(second, auth, T + 7),
        Ok(Command {
            id: second,
            action: Action::Disconnect,
            original_until_ns: UNTIL,
        })
    );
    assert!(owner.finish(second, T + 8, Finish::Observed(false)));
    assert_eq!(owner.view(T + 9).observed, fact(false, T + 8));
    assert_eq!(
        owner.take_reply(second, auth, T + 9),
        Some(Reply {
            id: second,
            outcome: Finish::Observed(false)
        })
    );
    assert_eq!(owner.view(T + 10).pending, None);
}

#[test]
fn one_mutation_slot_survives_dispatch_and_requester_departure_until_finish() {
    let mut owner = Owner::new();
    let first = staged(&mut owner, authority(1), Action::Connect);
    assert_eq!(
        owner.stage(authority(2), Action::Disconnect, UNTIL, T + 1),
        Err(Reject::Busy)
    );
    owner.dispatch(first, authority(1), T + 2).unwrap();
    owner.withdraw(authority(1).principal, T + 3);
    assert_eq!(owner.view(T + 4).pending.unwrap().phase, Phase::Dispatched);
    assert_eq!(
        owner.stage(authority(2), Action::Disconnect, UNTIL, T + 4),
        Err(Reject::Busy)
    );
    assert!(!owner.finish(Id(first.0 + 1), T + 5, Finish::DefinitelyUnsent));
    assert!(owner.view(T + 5).pending.is_some());
    assert!(owner.finish(first, T + 6, Finish::Observed(true)));
    assert_eq!(owner.take_reply(first, authority(1), T + 7), None);
    assert!(
        owner
            .stage(authority(2), Action::Disconnect, UNTIL, T + 7)
            .is_ok()
    );
}

#[test]
fn departure_revocation_and_changed_grant_cancel_only_definitely_unsent_stage() {
    for revoked in [false, true] {
        let mut owner = Owner::new();
        let auth = authority(1);
        owner.observe(false, T - 1);
        let id = staged(&mut owner, auth, Action::Connect);
        owner.withdraw(auth.principal, T + 1);
        assert_eq!(owner.dispatch(id, auth, T + 2), Err(Reject::Stale));
        let view = owner.view(T + 3);
        assert_eq!(view.pending, None);
        assert_eq!(view.observed, fact(false, T - 1));
        assert!(!view.terminal_uncertain);
        let mut current = auth;
        current.allowed = false;
        if revoked {
            current.permission_epoch += 1;
        }
        assert_eq!(
            owner.stage(current, Action::Connect, UNTIL, T + 4),
            Err(Reject::Denied)
        );
    }
    let mut owner = Owner::new();
    let auth = authority(1);
    let id = staged(&mut owner, auth, Action::Connect);
    let mut regranted = auth;
    regranted.permission_epoch += 1;
    assert_eq!(owner.dispatch(id, regranted, T + 1), Err(Reject::Denied));
    assert_eq!(
        owner.view(T + 2).pending,
        None,
        "regrant cannot revive an old staged choice"
    );
    assert!(
        owner
            .stage(regranted, Action::Connect, UNTIL, T + 2)
            .is_ok()
    );
}

#[test]
fn dispatched_connect_survives_settings_close_or_revocation_but_reply_does_not() {
    for revoked in [false, true] {
        let mut owner = Owner::new();
        let auth = authority(1);
        owner.observe(false, T - 1);
        let id = dispatched(&mut owner, auth, Action::Connect);
        owner.withdraw(auth.principal, T + 2);
        assert!(owner.finish(id, T + 3, Finish::Observed(true)));
        assert_eq!(owner.view(T + 4).observed, fact(true, T + 3));
        assert_eq!(owner.take_reply(id, auth, T + 4), None);
        let mut no_control = auth;
        no_control.allowed = false;
        if revoked {
            no_control.permission_epoch += 1;
        }
        assert_eq!(
            owner.stage(no_control, Action::Disconnect, UNTIL, T + 5),
            Err(Reject::Denied)
        );
        assert_eq!(
            owner.view(T + 6).observed,
            fact(true, T + 3),
            "no implicit Disconnect/rollback"
        );
        assert_eq!(owner.view(T + 6).pending, None);
    }
}

#[test]
fn definitely_unsent_finish_preserves_latest_independent_fact_and_its_timestamp() {
    let mut owner = Owner::new();
    let auth = authority(1);
    owner.observe(false, T - 1);
    let id = dispatched(&mut owner, auth, Action::Connect);
    owner.observe(true, T + 2);
    assert!(owner.finish(id, T + 3, Finish::DefinitelyUnsent));
    let view = owner.view(T + 4);
    assert_eq!(
        view.observed,
        fact(true, T + 2),
        "finish must not replace newer actual readback with staging history"
    );
    assert_eq!(view.pending, None);
    assert!(!view.terminal_uncertain);
    assert_eq!(
        owner.take_reply(id, auth, T + 4),
        Some(Reply {
            id,
            outcome: Finish::DefinitelyUnsent
        })
    );
    assert!(owner.stage(auth, Action::Disconnect, UNTIL, T + 5).is_ok());
}

#[test]
fn sent_uncertainty_is_terminal_even_after_readback_without_replay_or_rollback() {
    let mut owner = Owner::new();
    let auth = authority(1);
    owner.observe(false, T - 1);
    let id = dispatched(&mut owner, auth, Action::Connect);
    assert!(owner.finish(id, T + 2, Finish::Uncertain));
    let uncertain = owner.view(T + 3);
    assert_eq!(uncertain.pending, None);
    assert_eq!(uncertain.observed, None);
    assert!(uncertain.terminal_uncertain);
    assert_eq!(
        owner.take_reply(id, auth, T + 3),
        Some(Reply {
            id,
            outcome: Finish::Uncertain
        })
    );
    owner.observe(true, T + 4);
    assert_eq!(owner.view(T + 5).observed, fact(true, T + 4));
    assert!(owner.view(T + 5).terminal_uncertain);
    assert!(
        !owner.finish(id, T + 6, Finish::Observed(true)),
        "late facts cannot retroactively ACK the uncertain operation"
    );
    assert_eq!(owner.take_reply(id, auth, T + 6), None);
    assert_eq!(
        owner.stage(authority(2), Action::Disconnect, UNTIL, T + 7),
        Err(Reject::Uncertain)
    );
    assert_eq!(owner.view(T + 8).observed, fact(true, T + 4));
}

#[test]
fn original_boottime_expiry_never_revives_requester_or_releases_background_custody() {
    let mut unsent = Owner::new();
    let auth = authority(1);
    let id = staged(&mut unsent, auth, Action::Connect);
    unsent.tick(UNTIL);
    assert_eq!(unsent.view(UNTIL).pending, None);
    assert_eq!(unsent.dispatch(id, auth, UNTIL), Err(Reject::Stale));
    assert_eq!(unsent.take_reply(id, auth, UNTIL), None);
    let mut sent = Owner::new();
    let id = dispatched(&mut sent, auth, Action::Connect);
    sent.tick(UNTIL);
    assert_eq!(sent.view(UNTIL).pending.unwrap().phase, Phase::Dispatched);
    assert_eq!(
        sent.stage(authority(2), Action::Disconnect, UNTIL + 100, UNTIL),
        Err(Reject::Busy)
    );
    assert!(sent.finish(id, UNTIL + 1, Finish::Observed(true)));
    assert_eq!(sent.view(UNTIL + 2).observed, fact(true, UNTIL + 1));
    assert_eq!(
        sent.take_reply(id, auth, UNTIL + 2),
        None,
        "late global fact cannot revive expired requester reply"
    );
    assert_eq!(sent.view(UNTIL + 2).pending, None);
}
