//! Native retained-history restoration tracer. The initial authored Provider
//! cannot serve these native queries; actual auth/Source remain fixture inputs.
use super::*;

fn seed_history() -> io::Result<(RetainedHistory, Value)> {
    let install = PathBuf::from(
        std::env::var_os("NATIVE_LIFECYCLE_RESTORE_INSTALL").ok_or(io::ErrorKind::NotFound)?,
    );
    let seed = install.join("messages-seed-fixture");
    super::super::trusted_executable(&seed)?;
    let directory =
        PathBuf::from("/var/lib").join(format!("restore-history-{}", &crate::fresh_id()?[..10]));
    fs::create_dir(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let paths = RetainedHistory {
        raw: directory.join("raw"),
        inbox: directory.join("inbox"),
    };
    for path in [&paths.raw, &paths.inbox] {
        fs::create_dir(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    let mut child = Command::new(seed)
        .arg(&paths.raw)
        .env_clear()
        .env("NATIVE_MESSAGES_ISOLATED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let status = match child_wait(&mut child, boottime_ns() + WAIT_NS) {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill();
            if child_wait(&mut child, boottime_ns() + WAIT_NS).is_err() {
                std::process::abort();
            }
            return Err(error);
        }
    };
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut output)?;
    let mut errors = Vec::new();
    child
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut errors)?;
    check(
        status.success()
            && output.is_empty()
            && errors.is_empty()
            && !Path::new(&format!("/proc/{}", child.id())).exists(),
        "actual isolated receipt seed did not close normally",
    )?;
    Ok((
        paths,
        json!({"actual_seed_parent_wait":status.code(),"stdout_stderr_eof":true,
        "seed_proc_absent":true,"authored_deliver_occurrences":2,"seeded_once":true}),
    ))
}

fn read_native(case: &mut Case, query: Value) -> io::Result<Value> {
    let reply = rpc(
        &case.runtime.join("app/driver.sock"),
        json!({"op":"read","query":query}),
        11010,
    )?;
    case.observations
        .push(json!({"actual_native_query_reply":reply}));
    Ok(reply)
}

fn read_pages(case: &mut Case) -> io::Result<Vec<String>> {
    let page = read_native(
        case,
        json!({"query":"conversations","after":null,"limit":8}),
    )?;
    check(
        page["page"] == "conversations",
        "behavioral RED: restored authored Provider cannot serve retained native Messages conversations",
    )?;
    let items = page["items"].as_array().ok_or(io::ErrorKind::InvalidData)?;
    check(
        items.len() == 1 && items[0]["messages"] == 2,
        "expected one real conversation containing two retained occurrences",
    )?;
    let conversation = items[0]["id"].as_str().ok_or(io::ErrorKind::InvalidData)?;
    let page = read_native(
        case,
        json!({"query":"messages","conversation":conversation,"after_ordinal":0,"limit":2}),
    )?;
    check(page["page"] == "messages", "actual Messages page missing")?;
    let items = page["items"].as_array().ok_or(io::ErrorKind::InvalidData)?;
    check(
        items.len() == 2 && items.iter().all(|message| message["text"] == "hello"),
        "actual retained hello occurrences missing",
    )?;
    let receipts: Vec<String> = items
        .iter()
        .map(|item| {
            item["receipt"]
                .as_str()
                .map(str::to_owned)
                .ok_or(io::ErrorKind::InvalidData)
        })
        .collect::<Result<_, _>>()?;
    check(
        receipts[0] != receipts[1] && receipts.iter().all(|value| value.len() == 32),
        "distinct bounded real receipt IDs required",
    )?;
    Ok(receipts)
}

#[test]
#[ignore = "requires disposable root fixture, installed Messages seed/writer/reader, lifecycle helpers and explicit capabilities"]
fn exact_restore_releases_real_retained_messages_pages() {
    let (paths, seeded) = seed_history().expect("actual once-only retained history seed");
    let mut case = Case::new_with_history(None, false, Some(paths))
        .expect("retained history restoration setup");
    case.observations.push(seeded);
    let body = (|| {
        let before = case.policy()?;
        check(
            read_native(
                &mut case,
                json!({"query":"conversations","after":null,"limit":8}),
            )? == json!({"result":"denied"}),
            "native page read escaped initial lock",
        )?;
        case.arm()?;
        case.unlock()?;
        wait_until(boottime_ns() + 900_000_000, || {
            let snapshot = case.admin(json!({"op":"snapshot"}))?;
            Ok(case.policy()?["lock_pending"] == false
                && snapshot["ready"] == true
                && snapshot["locked"] == false
                && snapshot["storage_busy"] == false)
        })?;
        let after = case.policy()?;
        check(
            after["grants"] == before["grants"] && after["receipts"] == before["receipts"],
            "policy clear changed retained grants/operation receipts",
        )?;
        let ids = read_pages(&mut case)?;
        case.retain_messages_children()?;
        case.observations.push(
            json!({"actual_message_receipt_ids":ids,"policy_operation_receipts_are_separate":true}),
        );
        Ok(())
    })();
    let closed = case.close();
    case.report("real-retained-history-read", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual retained-history fixture closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
}

pub(super) struct RealHistory(native_session_authority_fixture::History);
impl RealHistory {
    pub(super) fn spawn(runtime: &Path, raw: &Path, inbox: &Path, exe: &Path) -> io::Result<Self> {
        Ok(Self(native_session_authority_fixture::History::spawn(
            runtime, raw, inbox, exe,
        )?))
    }
}
impl messages::Provider for RealHistory {
    fn incarnation(&self) -> &str {
        self.0.incarnation()
    }
    fn occupied(&self) -> bool {
        self.0.occupied()
    }
    fn read_occupied(&self) -> bool {
        self.0.read_occupied()
    }
    fn busy(&self) -> bool {
        self.0.busy()
    }
    fn available(&self) -> bool {
        self.0.available()
    }
    fn incoming_progress(&self) -> (u64, u64, u64) {
        self.0.incoming_progress()
    }
    fn receiving(&self) -> bool {
        self.0.receiving()
    }
    fn incoming_failure(&self) -> Option<&str> {
        self.0.incoming_failure()
    }
    fn start(&mut self, query: Value, until: u64) -> io::Result<u64> {
        self.0.start(&serde_json::to_vec(&query)?, until)
    }
    fn poll(&mut self) -> io::Result<Option<messages::Event>> {
        use native_session_authority_fixture::ReadEvent;
        Ok(self.0.poll()?.map(|event| match event {
            ReadEvent::Complete { token, packet } => messages::Event::Complete { token, packet },
            ReadEvent::Failed { token } => messages::Event::Failed { token },
        }))
    }
    fn cancel_read(&mut self, token: u64) {
        self.0.cancel_read(token);
    }
    fn stop(&mut self) {
        self.0.stop();
    }
    fn restart(&mut self) -> io::Result<()> {
        self.0.restart()
    }
    fn drain_until(&mut self, until: u64) -> bool {
        self.0.drain_until(until)
    }
}

impl Case {
    fn retain_messages_children(&mut self) -> io::Result<()> {
        check(
            self.real_history && self.messages_children.is_empty(),
            "Messages child observation must occur once per real provider",
        )?;
        let mut children = std::collections::BTreeSet::new();
        for task in fs::read_dir("/proc/self/task")?.flatten() {
            if let Ok(text) = fs::read_to_string(task.path().join("children")) {
                children.extend(
                    text.split_whitespace()
                        .filter_map(|pid| pid.parse::<u32>().ok()),
                );
            }
        }
        for (flag, socket) in [
            ("--messages-worker", "messages-storage.sock"),
            ("--messages-read-worker", "messages-read.sock"),
        ] {
            let expected = self.runtime.join(socket);
            let mut matched = None;
            for pid in &children {
                if let Ok(bytes) = fs::read(format!("/proc/{pid}/cmdline")) {
                    let args: Vec<_> = bytes.split(|b| *b == 0).collect();
                    if args.contains(&flag.as_bytes())
                        && args.contains(&expected.as_os_str().as_encoded_bytes())
                    {
                        check(
                            matched.is_none(),
                            "more than one exact Messages child for role",
                        )?;
                        let alive = rustix::process::pidfd_open(
                            Pid::from_raw(*pid as i32).unwrap(),
                            PidfdFlags::NONBLOCK,
                        )?;
                        check(
                            crate::authority::living(&alive)?,
                            "Messages child already died during observation",
                        )?;
                        matched = Some((*pid, alive));
                    }
                }
            }
            self.messages_children
                .push(matched.ok_or(io::ErrorKind::NotFound)?);
        }
        self.observations.push(json!({"actual_retained_Messages_children":2,"roles":["writer","readonly"],"separate_from_policy_worker":true,"driver_independently_stops_or_reaps_them":false}));
        Ok(())
    }
}

fn restore_native_pages(case: &mut Case) -> io::Result<Vec<String>> {
    let before = case.policy()?;
    check(
        read_native(
            case,
            json!({"query":"conversations","after":null,"limit":8}),
        )? == json!({"result":"denied"}),
        "native history escaped pending lock",
    )?;
    case.arm()?;
    case.unlock()?;
    wait_until(boottime_ns() + 900_000_000, || {
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        Ok(case.policy()?["lock_pending"] == false
            && snapshot["ready"] == true
            && snapshot["locked"] == false
            && snapshot["storage_busy"] == false)
    })?;
    let after = case.policy()?;
    check(
        before["grants"] == after["grants"] && before["receipts"] == after["receipts"],
        "fresh clear changed policy grants/history",
    )?;
    let ids = read_pages(case)?;
    case.retain_messages_children()?;
    check_policy_owner(case)?;
    Ok(ids)
}

fn check_policy_owner(case: &mut Case) -> io::Result<()> {
    let (original_pid, original) = case.worker.take().ok_or(io::ErrorKind::NotFound)?;
    case.find_worker()?;
    let (current_pid, current) = case.worker.as_ref().unwrap();
    check(
        *current_pid == original_pid
            && pokelinux_kernel_identity::ProcessView::open()?.matches(
                &original,
                current,
                original_pid,
            )?,
        "clear replaced the original sole policy worker",
    )?;
    case.observations.push(
        json!({"same_retained_policy_worker_after_clear":true,"actual_kernel_identity_check":true}),
    );
    Ok(())
}

#[test]
#[ignore = "requires disposable root fixture, installed Messages seed/writer/reader, lifecycle helpers and explicit capabilities"]
fn retained_message_ids_survive_relock_clean_restart_and_fresh_restore() {
    let (paths, seeded) = seed_history().expect("actual once-only retained history seed");
    let mut first = Case::new_with_history(None, false, Some(paths.clone()))
        .expect("first retained history owner");
    first.observations.push(seeded);
    let policy = first.policy.clone();
    let mut first_ids = None;
    let body = (|| {
        let ids = restore_native_pages(&mut first)?;
        super::negative_tests::actual_idle_lock(&mut first)?;
        check(
            first.policy()?["lock_pending"] == true,
            "actual relock did not sync pending true",
        )?;
        check(
            read_native(
                &mut first,
                json!({"query":"conversations","after":null,"limit":8}),
            )? == json!({"result":"denied"}),
            "native history escaped actual relock",
        )?;
        first
            .observations
            .push(json!({"message_receipt_ids_before_restart":ids,"actual_relock_synced":true}));
        first_ids = Some(ids);
        Ok(())
    })();
    let closed = first.close();
    first.report("retained-history-relock-before-restart", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual first history owner closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
    assert!(
        policy.join("owner.clean").is_file(),
        "clean actor restart prerequisite missing"
    );
    let mut reopened = Case::new_with_history(Some(policy), false, Some(paths))
        .expect("same retained data and policy reopen");
    let body = (|| {
        check(
            reopened.policy()?["lock_pending"] == true,
            "actor startup lost synced relock",
        )?;
        // Fresh actual admission and independent private completion/Source are
        // required. No cached old completion or unlocked Source shortcut.
        super::negative_tests::actual_idle_lock(&mut reopened)?;
        let ids = restore_native_pages(&mut reopened)?;
        check(
            Some(&ids) == first_ids.as_ref(),
            "reopen changed retained message receipt IDs or reseeded history",
        )?;
        reopened.observations.push(json!({"same_message_receipt_ids_after_restart":ids,
            "raw_and_inbox_paths_retained":true,"reseeded":false,"fresh_actual_lock_and_private_attempt":true}));
        Ok(())
    })();
    let closed = reopened.close();
    reopened.report("retained-history-reopen-fresh-restore", &body, &closed);
    assert!(
        closed.is_ok(),
        "actual reopened history owner closure: {closed:?}"
    );
    assert!(body.is_ok(), "{body:?}");
}

// The actual production client is linked unchanged into a private QCore host.
// The normal GUI, QML and original compositor are separate evidence boundaries.
fn qt_case(policy: Option<PathBuf>, paths: RetainedHistory) -> io::Result<Case> {
    Case::new_with_app(policy, false, Some(paths), true)
}

fn qt_model(case: &mut Case) -> io::Result<Value> {
    let model = rpc(
        &case.runtime.join("app/driver.sock"),
        json!({"op":"model"}),
        11010,
    )?;
    check(
        model["client_kind"] == "actual_production_MessagesClient",
        "composition RED: current app driver has no actual production Qt client/model",
    )?;
    Ok(model)
}

fn qt_denied(case: &mut Case, phase: &str) -> io::Result<()> {
    let mut observed = None;
    wait_until(boottime_ns() + WAIT_NS, || {
        let model = qt_model(case)?;
        let denied = model["status"] == "Messages access denied"
            && model["conversations"] == json!([])
            && model["messages"] == json!([]);
        observed = Some(model);
        Ok(denied)
    })?;
    case.observations
        .push(json!({"actual_Qt_denied_model":observed,"phase":phase,
        "poll_observed_denial_not_immediate_pixel_withdrawal":true}));
    Ok(())
}

fn qt_restore(case: &mut Case) -> io::Result<Vec<String>> {
    qt_denied(case, "initial pending lock")?;
    case.arm_only()?;
    qt_denied(case, "armed before later Source event")?;
    case.unlock()?;
    wait_until(boottime_ns() + 900_000_000, || {
        let snapshot = case.admin(json!({"op":"snapshot"}))?;
        Ok(case.policy()?["lock_pending"] == false
            && snapshot["ready"] == true
            && snapshot["locked"] == false
            && snapshot["storage_busy"] == false)
    })?;
    let mut admitted = None;
    wait_until(boottime_ns() + WAIT_NS, || {
        let model = qt_model(case)?;
        let ready = model["status"] == "Local receipt history · modem receive and send unavailable"
            && model["conversations"]
                .as_array()
                .is_some_and(|v| v.len() == 1)
            && model["messages"].as_array().is_some_and(|v| v.len() == 2);
        admitted = Some(model);
        Ok(ready)
    })?;
    let model = admitted.ok_or(io::ErrorKind::InvalidData)?;
    let items = model["messages"]
        .as_array()
        .ok_or(io::ErrorKind::InvalidData)?;
    check(
        items.iter().all(|v| v["text"] == "hello"),
        "actual Qt decoded bodies differ",
    )?;
    let ids = items
        .iter()
        .map(|v| {
            v["receipt"]
                .as_str()
                .map(str::to_owned)
                .ok_or(io::ErrorKind::InvalidData)
        })
        .collect::<Result<Vec<_>, _>>()?;
    check(
        ids[0] != ids[1] && ids.iter().all(|v| v.len() == 32),
        "actual Qt distinct receipt IDs required",
    )?;
    case.observations
        .push(json!({"actual_Qt_admitted_model":model,
        "production_client_source_linked_unchanged":true,"GUI_or_QML_executed":false}));
    case.retain_messages_children()?;
    check_policy_owner(case)?;
    Ok(ids)
}

#[test]
#[ignore = "requires disposable root fixture, actual Qt Core client, retained Messages helpers and explicit capabilities"]
fn actual_qt_client_models_deny_restore_relock_and_same_id_reopen() {
    let (paths, seeded) = seed_history().expect("actual once-only retained history seed");
    let mut first = qt_case(None, paths.clone()).expect("first actual Qt client owner");
    first.observations.push(seeded);
    let policy = first.policy.clone();
    let mut first_ids = None;
    let body = (|| {
        let ids = qt_restore(&mut first)?;
        super::negative_tests::actual_idle_lock(&mut first)?;
        check(
            first.policy()?["lock_pending"] == true,
            "Qt relock did not sync true",
        )?;
        qt_denied(&mut first, "after actual relock")?;
        first_ids = Some(ids);
        Ok(())
    })();
    let closed = first.close();
    first.report("actual-qt-retained-history-relock", &body, &closed);
    assert!(closed.is_ok(), "first Qt owner closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
    assert!(
        policy.join("owner.clean").is_file(),
        "clean actor seal missing"
    );
    let mut reopened = qt_case(Some(policy), paths).expect("fresh Qt over same retained data");
    let body = (|| {
        qt_denied(&mut reopened, "clean reopen before fresh admission")?;
        super::negative_tests::actual_idle_lock(&mut reopened)?;
        let ids = qt_restore(&mut reopened)?;
        check(
            Some(&ids) == first_ids.as_ref(),
            "actual Qt reopen changed retained IDs",
        )?;
        reopened.observations.push(json!({"same_Qt_message_receipt_ids_after_restart":ids,
            "raw_and_inbox_paths_retained":true,"reseeded":false,"fresh_actual_lock_and_private_attempt":true}));
        Ok(())
    })();
    let closed = reopened.close();
    reopened.report("actual-qt-retained-history-reopen", &body, &closed);
    assert!(closed.is_ok(), "reopened Qt owner closure: {closed:?}");
    assert!(body.is_ok(), "{body:?}");
}
