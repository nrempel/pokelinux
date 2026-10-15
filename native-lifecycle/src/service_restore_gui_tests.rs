//! Normal, uninstrumented Qt GUI downstream of the actual actor and Messages.
//! Authentication and typed Source inputs remain private cfgtest authorship.
//! The send witness records publication, never client decoding or rendered text.
use super::*;

const GUI_PHASE_BUDGET_NS: u64 = 60_000_000_000;

fn packets(case: &mut Case, after: u64) -> io::Result<Vec<Value>> {
    let app = case.app.as_ref().ok_or(io::ErrorKind::NotFound)?;
    let pid = app.metadata.pid;
    check(
        crate::authority::living(app.pidfd())?,
        "exact normal Qt app died",
    )?;
    let mut values = vec![];
    for reply in case.drain_gui_replies()? {
        check(
            reply.pid == pid && reply.uid == 11010,
            "publication peer is not the retained normal Qt app",
        )?;
        check(
            reply.bytes.len() <= 4096,
            "normal Qt publication exceeds its actual receive bound",
        )?;
        let value: Value = serde_json::from_slice(&reply.bytes)?;
        case.observations
            .push(json!({"actual_normal_Qt_socket_publication":value,
            "connection_id":reply.connection_id,"peer_pid":reply.pid,"peer_uid":reply.uid,"sent_at_ns":reply.sent_at_ns,
            "actual_successful_actor_send":true,"client_consumption_or_pixels_proved":false}));
        if reply.sent_at_ns >= after {
            values.push(value);
        }
    }
    Ok(values)
}

fn await_packet(case: &mut Case, after: u64, accept: impl Fn(&Value) -> bool) -> io::Result<Value> {
    let mut found = None;
    wait_until(boottime_ns() + WAIT_NS, || {
        for packet in packets(case, after)? {
            if accept(&packet) {
                found = Some(packet);
            }
        }
        Ok(found.is_some())
    })?;
    found.ok_or(io::ErrorKind::NotFound.into())
}

fn phase(case: &mut Case, sequence: u64, name: &str) -> io::Result<()> {
    let root = PathBuf::from(
        std::env::var_os("NATIVE_LIFECYCLE_RESTORE_RESULTS").ok_or(io::ErrorKind::NotFound)?,
    );
    let value = json!({"schema":1,"sequence":sequence,"phase":name,
        "app_pid":case.app.as_ref().unwrap().metadata.pid,
        "app_runtime":case.runtime.join("app"),
        "actual_socket_publication_observed":true,"model_getters_or_fake_packets_used":false});
    let temporary = root.join("GUI-phase.new");
    fs::write(&temporary, serde_json::to_vec_pretty(&value)?)?;
    fs::rename(&temporary, root.join("GUI-phase.json"))?;
    // This is only an external observation budget. It never becomes an actor,
    // Source, authentication, app initialization or cleanup deadline.
    let until = boottime_ns().saturating_add(GUI_PHASE_BUDGET_NS);
    wait_until(until, || {
        let _ = packets(case, 0)?;
        let path = root.join("GUI-ack.json");
        let Ok(bytes) = fs::read(path) else {
            return Ok(false);
        };
        check(
            bytes.len() <= 4096,
            "oversized private GUI observation acknowledgement",
        )?;
        let ack: Value = serde_json::from_slice(&bytes)?;
        Ok(ack["schema"] == 1
            && ack["sequence"] == sequence
            && ack["phase"] == name
            && ack["qualified"] == true)
    })?;
    case.observations
        .push(json!({"external_GUI_observation_ack":value,
        "external_observation_budget_ns":GUI_PHASE_BUDGET_NS,
        "output_qualification_is_separate_from_normal_QWindow_DPR":true}));
    Ok(())
}

#[test]
#[ignore = "requires disposable ARM graphics guest, original Hyprland, normal Qt GUI and real Messages/display helpers; auth and Source are cfgtest-authored"]
fn normal_gui_restores_history_and_stops_gracefully() {
    let (paths, seeded) = super::retained_history_tests::seed_history()
        .expect("once-only actual retained Messages seed");
    let mut case = Case::new_with_mode(None, false, Some(paths), TestApp::NormalGui)
        .expect("normal Qt GUI actor/display setup; setup errors are not behavioral REDs");
    case.observations.push(seeded);
    let app_pid = case.app.as_ref().unwrap().metadata.pid;
    let body = (|| {
        await_packet(&mut case, 0, |packet| *packet == json!({"result":"denied"}))?;
        phase(&mut case, 1, "initial-denied")?;
        let before = case.policy()?;
        case.arm_only()?;
        let armed_at = boottime_ns();
        await_packet(&mut case, armed_at, |packet| {
            *packet == json!({"result":"denied"})
        })?;
        check(
            case.policy()?["lock_pending"] == true,
            "arming released GUI before later Source",
        )?;
        case.unlock()?;
        wait_until(boottime_ns() + 900_000_000, || {
            let snapshot = case.admin(json!({"op":"snapshot"}))?;
            Ok(case.policy()?["lock_pending"] == false
                && snapshot["ready"] == true
                && snapshot["locked"] == false
                && snapshot["storage_busy"] == false)
        })?;
        let page = await_packet(&mut case, armed_at, |packet| {
            packet["page"] == "messages" && packet["items"].as_array().is_some_and(|v| v.len() == 2)
        })?;
        let items = page["items"].as_array().ok_or(io::ErrorKind::InvalidData)?;
        check(
            items.iter().all(|item| item["text"] == "hello"),
            "actual normal Qt socket bodies differ",
        )?;
        let ids: Vec<_> = items
            .iter()
            .map(|item| {
                item["receipt"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(io::ErrorKind::InvalidData)
            })
            .collect::<Result<_, _>>()?;
        check(
            ids[0] != ids[1] && ids.iter().all(|id| id.len() == 32),
            "distinct retained receipt IDs missing",
        )?;
        let after = case.policy()?;
        check(
            after["grants"] == before["grants"] && after["receipts"] == before["receipts"],
            "GUI clear changed retained policy grants or operation receipts",
        )?;
        case.retain_messages_children()?;
        case.observations
            .push(json!({"actual_normal_Qt_socket_receipt_ids":ids,
            "no_test_model_RPC_or_observer_binary":true}));
        phase(&mut case, 2, "restored")?;
        super::negative_tests::actual_idle_lock(&mut case)?;
        check(
            case.policy()?["lock_pending"] == true,
            "normal GUI relock did not sync true",
        )?;
        let relocked_at = boottime_ns();
        await_packet(&mut case, relocked_at, |packet| {
            *packet == json!({"result":"denied"})
        })?;
        phase(&mut case, 3, "relocked")?;
        let app = case.app.as_ref().unwrap();
        check(
            crate::authority::living(app.pidfd())?,
            "normal GUI must be alive before Stop",
        )?;
        case.observations
            .push(json!({"normal_GUI_live_immediately_before_Stop":true,
            "app_pid":app_pid,"all_three_external_GUI_phases_acknowledged":true}));
        Ok(())
    })();
    let closed = case.close();
    // Overflow/disconnection during retirement is sticky even after the app dies.
    let witness_closed = case.drain_gui_replies().map(|_| ());
    let body = body.and(witness_closed).and_then(|_| {
        check(
            case.observed_app_waits.len() == 1
                && case.observed_app_waits[0].0 == app_pid
                && case.observed_app_waits[0].1.success(),
            "behavioral RED: normal actor Stop did not reach normal Qt graceful exit0",
        )
    });
    case.report("normal-gui-retained-history-stop", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual normal GUI actor/helper owner closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
}
