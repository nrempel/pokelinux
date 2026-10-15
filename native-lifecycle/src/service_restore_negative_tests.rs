//! Additional actual-service lifetime coverage. Auth and Source remain authored;
//! a Source barrier proves loop observation and grants no restoration authority.
use super::*;

fn submit(case: &Case, complete: bool) -> io::Result<bool> {
    let (reply, receive) = mpsc::sync_channel(1);
    case.commands
        .try_send(if complete {
            ControlCommand::Complete { reply }
        } else {
            ControlCommand::Begin {
                until: boottime_ns().saturating_add(20_000_000_000),
                reply,
            }
        })
        .map_err(|_| io::Error::other("bounded test mailbox unavailable"))?;
    receive
        .recv_timeout(Duration::from_secs(2))
        .map_err(io::Error::other)
}

fn source_barrier(case: &mut Case, revision: u64) -> io::Result<()> {
    let (reply, receive) = mpsc::sync_channel(1);
    case.commands
        .try_send(ControlCommand::SourceBarrier {
            expected_revision: revision,
            until: boottime_ns().saturating_add(WAIT_NS),
            reply,
        })
        .map_err(|_| io::Error::other("bounded Source barrier mailbox unavailable"))?;
    check(
        receive
            .recv_timeout(Duration::from_secs(2))
            .map_err(io::Error::other)?,
        "Source barrier refused before actual loop observation",
    )?;
    case.observations.push(json!({
        "actual_service_Source_observed_revision_at_least": revision,
        "barrier_at_ns": boottime_ns(),
        "barrier_has_authentication_authority": false
    }));
    Ok(())
}

fn publish_unlocked_without_event(case: &Case) -> io::Result<u64> {
    let mut sample = case
        .facts
        .lock()
        .map_err(|_| io::Error::other("authored Source mutex poisoned"))?;
    sample.revision += 1;
    sample.sequence += 1;
    sample.sampled_at_ns = boottime_ns();
    sample.until = sample.sampled_at_ns + SAMPLE_BUDGET_NS;
    sample.locked = false;
    sample.focus = case.app.clone();
    sample.locker = None;
    sample.lock_generation = None;
    sample.protocol_unlock = None;
    Ok(sample.revision)
}

fn publish_later_event(case: &mut Case) -> io::Result<()> {
    case.unlock()?;
    let revision = case
        .facts
        .lock()
        .map_err(|_| io::Error::other("authored Source mutex poisoned"))?
        .revision;
    source_barrier(case, revision)
}

fn retained_denial(case: &mut Case, before: &Value) -> io::Result<()> {
    let snapshot = case.admin(json!({"op":"snapshot"}))?;
    check(
        snapshot["ready"] == true
            && snapshot["locked"] == true
            && snapshot["storage_busy"] == false,
        "negative lifetime transition changed current locked policy",
    )?;
    let disk = case.policy()?;
    check(
        disk == *before,
        "negative lifetime transition changed durable bytes/state",
    )?;
    check(
        case.read()? == json!({"result":"denied"}),
        "retired authentication restored an actual UID11010 read",
    )?;
    case.observations
        .push(json!({"retained_denial_snapshot":snapshot,"retained_disk":disk}));
    Ok(())
}

fn actual_idle_lock(case: &mut Case) -> io::Result<()> {
    let mut child = Command::new(&case.python)
        .arg("-I")
        .arg(case.install.join("app.py"))
        .arg("--lock")
        .arg(case.runtime.join("lock.sock"))
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let status = match child_wait(&mut child, boottime_ns().saturating_add(WAIT_NS)) {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill();
            if child_wait(&mut child, boottime_ns().saturating_add(WAIT_NS)).is_err() {
                std::process::abort();
            }
            return Err(error);
        }
    };
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_string(&mut output)?;
    check(
        status.success()
            && output.len() <= 4096
            && !Path::new(&format!("/proc/{}", child.id())).exists(),
        "actual idle lock helper did not close normally",
    )?;
    let value: Value = serde_json::from_str(&output)?;
    check(
        value["identity"]["uid"] == 1000
            && value["identity"]["gid"] == 1000
            && value["identity"]["groups"] == json!([])
            && value["identity"]["cap_eff"] == "0000000000000000"
            && value["reply"] == json!({"result":"ok"}),
        "actual UID1000 idle lock did not receive durable Ok",
    )?;
    case.observations.push(json!({
        "actual_uid1000_idle_lock": value,
        "actual_parent_wait": status.code(),
        "stdout_eof": true,
        "proc_absent": true
    }));
    Ok(())
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn forced_unlock_without_event_then_later_event_cannot_restore_service_read() {
    let mut case = Case::new(None, false).expect("ordinary negative restoration setup");
    let body = (|| {
        let before = case.policy()?;
        case.arm()?;
        let forced_revision = publish_unlocked_without_event(&case)?;
        source_barrier(&mut case, forced_revision)?;
        // The later exact event cannot be coalesced with the earlier forced
        // sample: normal refresh_session completed before this barrier replied.
        retained_denial(&mut case, &before)?;
        publish_later_event(&mut case)?;
        retained_denial(&mut case, &before)
    })();
    let closed = case.close();
    case.report("forced-unlock-late-event-denial", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual negative fixture closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn actual_source_lease_expiry_then_fresh_event_cannot_restore_service() {
    let mut case = Case::new(None, false).expect("ordinary expiry restoration setup");
    let body = (|| {
        let before = case.policy()?;
        case.arm()?;
        let revision = case
            .facts
            .lock()
            .map_err(|_| io::Error::other("authored Source mutex poisoned"))?
            .revision;
        source_barrier(&mut case, revision)?;
        let shared = case.facts.clone();
        let held = shared
            .lock()
            .map_err(|_| io::Error::other("authored Source mutex poisoned"))?;
        let original_until = held.until;
        // Heartbeat blocks outside the actor. Provider.try_lock keeps its old
        // absolute lease; no test clock or longer Source deadline is supplied.
        while boottime_ns() <= original_until {
            thread::sleep(Duration::from_millis(2));
        }
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        check(
            boottime_ns() > original_until
                && snapshot["ready"] == true
                && snapshot["locked"] == true
                && snapshot["storage_busy"] == false,
            "actual Source expiry did not retain responsive locked Snapshot",
        )?;
        case.observations.push(json!({
            "held_original_Source_until_ns": original_until,
            "actual_after_expiry_ns": boottime_ns(),
            "snapshot_while_Source_mutex_held": snapshot
        }));
        drop(held);
        publish_later_event(&mut case)?;
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        check(
            snapshot["locked"] == true && snapshot["storage_busy"] == false,
            "fresh later facts revived authority after actual Source expiry",
        )?;
        check(
            case.policy()? == before,
            "actual expiry/later event changed durable state",
        )?;
        // Source loss is terminal for Apps; the normal loop may already retire
        // the real helper. Observe that boundary rather than demand a reply
        // from a dead app or relaunch a new principal to manufacture coverage.
        let app = case.app.as_ref().unwrap();
        wait_until(boottime_ns().saturating_add(WAIT_NS), || {
            Ok(!crate::authority::living(app.pidfd())?
                && !Path::new(&format!("/proc/{}", app.metadata.pid)).exists())
        })?;
        case.observations.push(json!({
            "actual_app_retired_after_Source_expiry": true,
            "app_pidfd_dead": true,
            "app_proc_absent": true,
            "later_event_restored_no_read_or_disk": true,
            "fresh_event_snapshot": snapshot
        }));
        Ok(())
    })();
    let closed = case.close();
    case.report("actual-source-expiry-late-event-denial", &body, &closed);
    assert!(closed.is_ok(), "actual expiry fixture closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
}

#[test]
#[ignore = "requires disposable root fixture, installed daemon/launcher, UID11010 app and explicit capabilities"]
fn old_completion_after_actual_idle_uid1000_lock_cannot_arm_service() {
    let mut case = Case::new(None, false).expect("ordinary old completion setup");
    let body = (|| {
        let before = case.policy()?;
        check(submit(&case, false)?, "initial actual-loop begin refused")?;
        actual_idle_lock(&mut case)?;
        let accepted = submit(&case, true)?;
        case.observations
            .push(json!({"old_completion_after_actual_idle_lock_armed":accepted}));
        check(!accepted, "old completion armed after actual new idle lock")?;
        publish_later_event(&mut case)?;
        retained_denial(&mut case, &before)
    })();
    let closed = case.close();
    case.report("old-completion-new-idle-lock-denial", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual old completion fixture closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
}
