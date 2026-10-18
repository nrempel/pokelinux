//! Test-only final assertion over the closed real Qt/actor/PCM case. No app IPC path.
use serde_json::Value;
use std::{env, fs, io};
fn ns(value: &Value) -> u64 { value.as_str().expect("actual decimal clock").parse().expect("u64 clock") }
fn wire(value: &Value) -> Value { serde_json::from_str(value["wire_utf8"].as_str().expect("verbatim actual UTF8 send bytes")).expect("actual JSON packet") }
fn public(value: &Value, text: &str) {
    let s = &value["state"];
    assert_eq!(s["status"], text); assert_eq!(s["qml_status"], text);
    assert_eq!(s["requested"], "media-a7");
}
fn level(value: &Value, muted: bool, enabled: bool) {
    let s = &value["state"];
    assert_eq!(s["previewLevelPercent"], 50); assert_eq!(s["previewMuted"], muted);
    assert_eq!(s["qml_level"], "50%"); assert_eq!(s["qml_muted"], muted);
    for field in ["canAdjustPreview", "qml_level50_enabled", "qml_level100_enabled", "qml_mute_enabled"] { assert_eq!(s[field], enabled); }
}
fn main() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 { return Err(io::ErrorKind::InvalidInput.into()); }
    let r: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    if r["setup_qualified"] != true || r["cleanup_qualified"] != true || r["ready_for_compiled_assertion"] != true { return Err(io::ErrorKind::InvalidData.into()); }
    assert_eq!(r["passed"], true, "actual QML actions and independent complete PCM must pass after actual cleanup");
    assert_eq!(r["same_Main_actor_Qt_app_live"], true);
    assert_eq!(r["effect_counts"], serde_json::json!({"route":1,"plays":2,"stops":1}));
    let clicks = r["qml"]["clicks"].as_array().expect("actual native dispatch records");
    assert_eq!(clicks.len(), 6);
    for (i, (operation, button, choice)) in [("click_route","route_media-a7",0),("click_level50","previewLevel50",0),("click_play","previewPlay",1),("click_stop","previewStop",1),("click_mute","previewMute",1),("click_play","previewPlay",2)].iter().enumerate() {
        let click = &clicks[i]; let d = &click["dispatch"];
        assert_eq!(click["operation"], *operation); assert_eq!(d["button"], *button); assert_eq!(d["play_choice"], *choice);
        assert_eq!(d["clicked_signals"], 1);
        for field in ["press_delivered", "release_delivered", "all_quick_windows_hidden"] { assert_eq!(d[field], true); }
        assert!(ns(&d["before_boottime_ns"]) <= ns(&d["after_boottime_ns"]));
    }
    let sends = r["qml"]["actor_sends"].as_array().expect("actual unchanged-forward sends");
    assert!(!sends.is_empty()); assert_eq!(wire(&sends[0])["op"], "media_status");
    let peer = sends[0]["record"]["peer_pid"].as_i64().expect("actual actor peer");
    let fd = sends[0]["record"]["fd"].as_i64().expect("actual actor FD");
    let mut complete = [0_u32; 4];
    let mut original = [None, None]; let mut ttl = [1001_u64; 2]; let mut plays = [0_u32; 2];
    for sent in sends {
        let record = &sent["record"]; let packet = wire(sent);
        assert_eq!(record["actor_peer"], true); assert_eq!(record["peer_valid"], true);
        assert_eq!(record["peer_pid"], peer); assert_eq!(record["peer_uid"], 0); assert_eq!(record["peer_gid"], 0);
        assert_eq!(record["fd"], fd); assert_ne!(record["fd"], record["controller_fd_at_send"]);
        assert_eq!(record["clocks_valid"], true); assert!(ns(&record["before_boottime_ns"]) <= ns(&record["after_boottime_ns"]));
        let kind = match packet["op"].as_str().expect("actual operation") {
            "media_status" => 0, "media_route" => 1, "media_preview_play" => 2, "media_preview_stop" => 3,
            _ => panic!("no Query/reconnect/alternate app command allowed"),
        };
        if kind == 2 {
            let choice = record["play_choice"].as_u64().expect("actual explicit choice"); assert!((1..=2).contains(&choice)); let i = (choice-1) as usize;
            let request = &packet["request"]; let until = request["until_boottime_ns"].as_u64().expect("actual absolute Play cutoff"); let remaining = request["ttl_ms"].as_u64().expect("actual remaining TTL");
            assert_eq!(request.as_object().expect("mandatory Play fields").len(), 5); assert_eq!(request["level_percent"], 50); assert_eq!(request["muted"], i == 1);
            assert_eq!(request["route"], "media-a7"); assert!((1..=1000).contains(&remaining) && remaining <= ttl[i]); ttl[i] = remaining;
            let before = ns(&record["before_boottime_ns"]); assert!(until > before && until - before <= 1_000_000_000);
            if let Some(previous) = original[i] { assert_eq!(until, previous); } original[i] = Some(until);
            if record["complete_send"] == true { plays[i] += 1; assert!(ns(&record["after_boottime_ns"]) < until); }
        }
        if record["complete_send"] == true {
            assert_eq!(record["length"], record["result"]); complete[kind] += 1;
            if kind == 1 { assert_eq!(packet["request"]["route"], "media-a7"); }
            if kind == 3 { assert_eq!(packet, serde_json::json!({"op":"media_preview_stop"})); }
        }
    }
    assert!(complete[0] >= 1); assert_eq!(&complete[1..], &[1,2,1]); assert_eq!(plays, [1,1]);
    public(&r["selected_idle"], "Output selected; preview idle"); assert_eq!(r["selected_idle"]["state"]["canPlay"], true);
    assert_eq!(r["selected_idle"]["state"]["previewLevelPercent"], 100); assert_eq!(r["selected_idle"]["state"]["previewMuted"], false);
    for name in ["local_level_50", "local_mute"] { assert_eq!(r[name]["effect_attempts_before"], r[name]["effect_attempts_after"]); }
    assert_eq!(r["local_level_50"]["effect_attempts_after"], serde_json::json!({"media_route":1,"media_preview_play":0,"media_preview_stop":0}));
    assert_eq!(r["local_mute"]["effect_attempts_after"], serde_json::json!({"media_route":1,"media_preview_play":1,"media_preview_stop":1}));
    level(&r["local_level_50"], false, true); level(&r["local_mute"], true, true);
    public(&r["first_QML_Starting"], "Preview starting"); level(&r["first_QML_Starting"], false, false);
    public(&r["new_QML_Starting"], "Preview starting"); level(&r["new_QML_Starting"], true, false);
    level(&r["first_QML_Playing"], false, false); level(&r["QML_Playing_before_Stop"], false, false);
    level(&r["QML_Retiring"], false, false); level(&r["QML_idle_before_new_Play"], false, true); level(&r["new_QML_Playing"], true, false);
    public(&r["first_QML_Playing"], "Preview playing"); public(&r["QML_Playing_before_Stop"], "Preview playing");
    assert_eq!(r["QML_Playing_before_Stop"]["state"]["applied"], "media-a7"); assert_eq!(r["QML_Playing_before_Stop"]["state"]["canStop"], true);
    public(&r["QML_Retiring"], "Preview retiring"); public(&r["QML_idle_before_new_Play"], "Output selected; preview idle");
    assert_eq!(r["QML_idle_before_new_Play"]["state"]["applied"], ""); assert_eq!(r["QML_idle_before_new_Play"]["state"]["canPlay"], true);
    public(&r["new_QML_Playing"], "Preview playing"); assert_eq!(r["producer_live_before_Stop"], true); assert_eq!(r["new_producer_live_before_cleanup"], true);
    let pcm = &r["PCM"]; assert_eq!(pcm["passed"], true); assert_eq!(pcm["actual_QML_Retiring_fence"], true);
    assert_eq!(pcm["old_retained_pidfd_still_terminal"], true); assert_eq!(pcm["new_explicit_producer_distinct_same_Main_cookie"], true);
    assert!(pcm["captured_at_monotonic_ns"].as_u64().expect("actual final observation") < pcm["original_capture_until_monotonic_ns"].as_u64().expect("one original cutoff"));
    for name in ["before_Stop_half_tone", "after_Stop_quiet", "new_explicit_muted_Play"] {
        let stage = &pcm["stages"][name]; assert_eq!(stage["passed"], true); assert_eq!(stage["complete_paired_windows"], true);
        for sink in ["speaker", "earpiece"] { assert_eq!(stage["stats"][sink]["frames"], 24000); assert_eq!(stage["stats"][sink]["finite_all"], true); }
    }
    let half = &pcm["stages"]["before_Stop_half_tone"]["stats"]["speaker"];
    assert!((0.08..=0.1).contains(&half["rms"].as_f64().expect("half-tone RMS")));
    assert!(half["selected_tone_energy_fraction"].as_f64().expect("half-tone energy") >= 0.9);
    for name in ["after_Stop_quiet", "new_explicit_muted_Play"] {
        for sink in ["speaker", "earpiece"] {
            let stat = &pcm["stages"][name]["stats"][sink];
            assert!(stat["rms"].as_f64().expect("full quiet RMS") <= 1e-6);
            assert!(stat["peak"].as_f64().expect("full quiet peak") <= 1e-5);
        }
    }
    assert_eq!(pcm["stages"]["new_explicit_muted_Play"]["live_attributed_producer_and_active_selected_link"], true);
    assert!(pcm["stages"]["new_explicit_muted_Play"]["producer_after_capture"].is_object());
    for field in ["old_source_client_links_absent", "retained_pidfd_terminal", "old_proc_absent"] { assert_eq!(pcm["actual_retirement"][field], true); }
    assert_eq!(pcm["stages"]["after_Stop_quiet"]["old_source_client_links_absent_pidfd_dead_proc_absent"], true);
    assert!(r["producer_cleanup_facts"].as_array().expect("retained producer closure").iter().all(|v|v["retained_pidfd_dead"]==true&&v["proc_absent"]==true));
    Ok(())
}
