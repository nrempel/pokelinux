use omarchy_phone_core::{
    Identity,
    notification_policy::Dismissals,
    watch::{Snapshot, decode_snapshot},
};
fn snapshot(generation: &str, state: &str, calls: &[(u32, &str)]) -> Snapshot {
    let id = Identity::try_from("dddddddddddddddddddddddddddddddd".to_owned()).unwrap();
    let body = serde_json::to_vec(&serde_json::json!({
        "v":1,"id":id,"generation":generation,"revision":1,
        "source":"android.telecom.InCallService","source_state":state,
        "ringing":calls.iter().map(|(id,episode)| serde_json::json!({"id":id,"episode":episode})).collect::<Vec<_>>()
    })).unwrap();
    decode_snapshot(&body, &id).unwrap()
}
const G: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const A: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const B: &str = "cccccccccccccccccccccccccccccccc";
#[test]
fn dismissal_survives_observation_gaps_but_not_a_new_call_episode() {
    let mut policy = Dismissals::default();
    let ringing = snapshot(G, "observed", &[(7, A)]);
    assert!(policy.has_undismissed(&ringing));
    policy.dismiss(&ringing, &ringing);
    assert!(!policy.has_undismissed(&ringing));
    policy.observe(&snapshot(G, "unavailable", &[]));
    policy.observe(&snapshot(G, "detached", &[]));
    assert!(!policy.has_undismissed(&ringing));
    let next = snapshot(G, "observed", &[(7, B)]);
    policy.observe(&next);
    assert!(policy.has_undismissed(&next));
}
#[test]
fn delayed_dismissal_only_suppresses_calls_represented_by_that_banner() {
    let mut policy = Dismissals::default();
    let original = snapshot(G, "observed", &[(7, A)]);
    let current = snapshot(G, "observed", &[(7, A), (8, B)]);
    policy.dismiss(&original, &current);
    assert!(!policy.has_undismissed(&original));
    assert!(policy.has_undismissed(&current));
    policy.dismiss(&current, &current);
    assert!(!policy.has_undismissed(&current));
}
#[test]
fn stale_generation_dismissal_cannot_suppress_a_reused_call_id() {
    let mut policy = Dismissals::default();
    let original = snapshot(G, "observed", &[(7, A)]);
    policy.dismiss(&original, &original);
    let next = snapshot(B, "observed", &[(7, A)]);
    policy.dismiss(&original, &next);
    assert!(policy.has_undismissed(&next));
    assert!(!policy.has_undismissed(&snapshot(B, "observed", &[])));
    assert!(!policy.has_undismissed(&snapshot(B, "unavailable", &[])));
}

#[test]
fn dismissal_during_an_observation_gap_uses_last_confirmed_episodes() {
    let mut policy = Dismissals::default();
    let original = snapshot(G, "observed", &[(7, A)]);
    policy.observe(&original);
    policy.dismiss(&original, &snapshot(G, "unavailable", &[]));
    assert!(!policy.has_undismissed(&original));
    policy.observe(&snapshot(G, "observed", &[]));
    assert!(policy.has_undismissed(&original));
}

#[test]
fn banner_attempts_survive_gaps_and_allow_new_episodes() {
    use omarchy_phone_core::notification_policy::BannerAttempts;
    let mut attempts = BannerAttempts::default();
    let dismissals = Dismissals::default();
    let original = snapshot(G, "observed", &[(7, A)]);
    assert!(attempts.claim(&original, &dismissals));
    assert!(!attempts.claim(&original, &dismissals));
    attempts.observe(&snapshot(G, "unavailable", &[]));
    attempts.observe(&snapshot(G, "detached", &[]));
    assert!(!attempts.claim(&original, &dismissals));
    let waiting = snapshot(G, "observed", &[(7, A), (8, B)]);
    assert!(attempts.claim(&waiting, &dismissals));
    assert!(!attempts.claim(&waiting, &dismissals));
    // Retiring the first episode must not repeat the surviving second one.
    assert!(!attempts.claim(&snapshot(G, "observed", &[(8, B)]), &dismissals));
    assert!(attempts.claim(&snapshot(G, "observed", &[(7, G), (8, B)]), &dismissals));
}

#[test]
fn a_dismissed_new_episode_does_not_reoffer_an_already_attempted_call() {
    use omarchy_phone_core::notification_policy::BannerAttempts;
    let mut attempts = BannerAttempts::default();
    let mut dismissals = Dismissals::default();
    assert!(attempts.claim(&snapshot(G, "observed", &[(7, A)]), &dismissals));
    let current = snapshot(G, "observed", &[(7, A), (8, B)]);
    dismissals.dismiss(&snapshot(G, "observed", &[(8, B)]), &current);
    // Aggregate has_undismissed is true for call 7. Eligibility must instead
    // intersect unattempted AND undismissed episodes, not the two set totals.
    assert!(dismissals.has_undismissed(&current));
    assert!(!attempts.claim(&current, &dismissals));
}

#[test]
fn attempt_state_is_process_local_but_dismissal_still_applies() {
    use omarchy_phone_core::notification_policy::BannerAttempts;
    let original = snapshot(G, "observed", &[(7, A)]);
    let mut dismissals = Dismissals::default();
    let mut before_restart = BannerAttempts::default();
    assert!(before_restart.claim(&original, &dismissals));
    let mut after_restart = BannerAttempts::default();
    assert!(after_restart.claim(&original, &dismissals));
    dismissals.dismiss(&original, &original);
    assert!(!BannerAttempts::default().claim(&original, &dismissals));
    assert!(after_restart.claim(&snapshot(B, "observed", &[(7, A)]), &dismissals));
    assert!(!after_restart.claim(&snapshot(B, "observed", &[]), &dismissals));
    assert!(!after_restart.claim(&snapshot(B, "unavailable", &[]), &dismissals));
}
