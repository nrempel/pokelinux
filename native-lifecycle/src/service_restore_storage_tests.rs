//! Actual Stop with a synced but unacknowledged clear, followed by the real
//! storage worker's unclean-history refusal. Auth/Source/history remain authored.
use super::*;
use crate::storage::{Event, Worker};

struct OwnedProbe(Worker);
impl Drop for OwnedProbe {
    fn drop(&mut self) {
        self.0.drain();
    }
}

fn refuse_unclean_policy(case: &mut Case) -> io::Result<()> {
    let before = fs::read(case.policy.join("policy.json"))?;
    let live = fs::read(case.policy.join("owner.live"))?;
    check(
        !case.policy.join("owner.clean").exists(),
        "unresolved clear acquired a clean seal",
    )?;
    let runtime =
        PathBuf::from("/run").join(format!("restore-probe-{}", &crate::fresh_id()?[..10]));
    fs::create_dir(&runtime)?;
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755))?;
    let mut probe = OwnedProbe(Worker::spawn(
        &runtime,
        &case.policy,
        &case.install.join("lifecycle-daemon"),
        None,
    )?);
    let until = boottime_ns().saturating_add(1_000_000_000);
    let token = probe.0.start_load(vec![APP.into()], until)?;
    let mut result = None;
    let observed = wait_until(until, || match probe.0.poll()? {
        Some(Event::Complete {
            token: actual,
            success,
            state,
        }) => {
            check(actual == token, "fresh worker returned wrong token")?;
            result = Some(!success && state.is_none());
            Ok(true)
        }
        Some(_) => Err(io::Error::other(
            "expected qualified failed Load, not uncertain process failure",
        )),
        None => Ok(false),
    });
    // Establish actual child closure before either success/failure assertion.
    probe.0.drain();
    check(!probe.0.occupied(), "fresh worker was not actually reaped")?;
    observed?;
    check(
        result == Some(true),
        "fresh actual worker reopened uncertain history",
    )?;
    check(
        fs::read(case.policy.join("policy.json"))? == before
            && fs::read(case.policy.join("owner.live"))? == live
            && !case.policy.join("owner.clean").exists(),
        "refused Load changed uncertain history",
    )?;
    case.observations.push(json!({
        "fresh_actual_storage_worker_load_refused": true,
        "qualified_token_matched_completion": true,
        "fresh_worker_owned_drain_completed": true,
        "policy_and_live_owner_bytes_unchanged": true,
        "clean_seal_absent": true,
        "fresh_actor_restart_executed": false,
    }));
    Ok(())
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn held_clear_stop_leaves_unclean_history_and_fresh_worker_refuses() {
    let mut case = Case::new(None, true).expect("ordinary held-clear fixture setup");
    let body = (|| {
        let before = case.policy()?;
        case.arm()?;
        case.unlock()?;
        let pid = case.worker.as_ref().unwrap().0;
        wait_until(boottime_ns() + 900_000_000, || {
            let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
            Ok(stat
                .rsplit_once(") ")
                .and_then(|(_, tail)| tail.split_whitespace().next())
                .is_some_and(|state| state == "T" || state == "t")
                && case.policy()?["lock_pending"] == false)
        })?;
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        check(
            snapshot["storage_busy"] == true
                && snapshot["ready"] == true
                && snapshot["locked"] == true,
            "held clear lost denial or responsive Snapshot",
        )?;
        check(
            case.read()? == json!({"result":"unavailable"}),
            "held clear released app history",
        )?;
        let pending = case.policy()?;
        check(
            pending["grants"] == before["grants"] && pending["receipts"] == before["receipts"],
            "held false clear changed retained history",
        )?;
        case.observations.push(json!({
            "held_synced_false_policy": pending,
            "snapshot_while_exact_worker_stopped": snapshot,
            "worker_independently_resumed_or_killed": false,
        }));
        Ok(())
    })();
    // Existing normal Stop must signal/drain the retained stopped child itself.
    // The driver never resumes, kills or waits that storage child independently.
    let closed = case.close();
    let body = if body.is_ok() && closed.is_ok() {
        body.and_then(|_| {
            check(
                case.policy()?["lock_pending"] == false,
                "Stop pretended unresolved false bytes were undone",
            )?;
            refuse_unclean_policy(&mut case)
        })
    } else {
        body
    };
    case.report("held-clear-stop-restart", &body, &closed);
    assert!(closed.is_ok(), "actual fixture closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
}
