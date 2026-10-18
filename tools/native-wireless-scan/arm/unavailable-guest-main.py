"""One unavailable control on observed loopback; no AP, radios or package installation."""
from pathlib import Path
import ctypes, hashlib, json, os, stat, sys, traceback
sys.dont_write_bytecode = True
from owned import Scope, now
import consumer, worker_caps
R = Path("/results")
I = Path("/run/hwsim-original-viability")
S = json.loads((R / "selection.json").read_text())
T = int(os.environ["POKELINUX_HWSIM_T0_NS"])
W = T + S["guest_work_seconds"] * 1_000_000_000
C = T + S["guest_close_seconds"] * 1_000_000_000
scope = Scope(R)
r = {"schema": 1, "phase": S["phase"], "passed": False,
     "original_work_until_ns": W, "original_close_until_ns": C,
     "normal_app_graphics_Source_auth_executed": False,
     "native_wireless_worker_executed": False, "phone_accessed": False,
     "native_scan_worker_commands_admitted": 0, "original_iw_scan_commands_admitted": 0,
     "AP_or_radio_creation_commands_admitted": 0, "package_install_commands_admitted": 0,
     "desired_wireless_scan_completed": False}
def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size,
                "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
def run(argv, label, seconds=20, limit=1048576):
    assert all(v["wait"] is not None and v["eof"] for v in scope.children)
    end = min(W, now() + seconds * 1_000_000_000)
    try:
        code, body = scope.run(argv, f"{len(scope.children)+1:03d}-{label}.log", end, limit=limit)
    except BaseException:
        scope.close(C)
        raise
    assert code == 0 and now() < end, (label, code, body[-1200:])
    assert all(v["wait"] is not None and v["eof"] for v in scope.children)
    return body.decode()
def expected_command_exit(receipt):
    if receipt["role"] == r.get("native_worker_role"):
        return (receipt["returncode"] == 2 and receipt["argv"] == r.get("native_worker_exact_argv")
                and receipt.get("expected_returncode") == 2
                and r.get("exact_unsupported_response_qualified") is True)
    return receipt["returncode"] == 0
def namespace():
    p = Path("/proc/self/ns/net")
    st = p.stat()
    return {"dev": st.st_dev, "ino": st.st_ino,
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}
def substrate():
    phys = Path("/sys/class/ieee80211")
    assert not phys.exists() or not list(phys.iterdir()), "wireless phy exists"
    interfaces = sorted(p.name for p in Path("/sys/class/net").iterdir())
    assert interfaces == ["lo"], interfaces
    loop = Path("/sys/class/net/lo")
    assert not (loop / "phy80211").exists() and not (loop / "wireless").exists()
    raw = (loop / "ifindex").read_text().strip()
    assert raw.isdigit() and int(raw) > 0
    assert (loop / "type").read_text().strip() == "772"
    st = loop.stat()
    return {"loopback": {"name": "lo", "ifindex": int(raw), "type": 772,
            "dev": st.st_dev, "ino": st.st_ino, "real_path": str(loop.resolve())},
            "interfaces": interfaces, "wireless_phys": [], "namespace": namespace(),
            "modules": sorted(p.name for p in Path("/sys/module").iterdir()),
            "observed_boottime_ns": now()}
def unique_object(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, "duplicate JSON key"
        result[key] = value
    return result
try:
    assert os.geteuid() == os.getegid() == 0 and now() < W
    assert "ipv6.disable=1" in Path("/proc/cmdline").read_text().split()
    libc = ctypes.CDLL(None, use_errno=True)
    assert libc.prctl(36, 1, 0, 0, 0) == 0
    observed = ctypes.c_int(-1)
    assert libc.prctl(37, ctypes.byref(observed), 0, 0, 0) == 0 and observed.value == 1
    r["actual_child_subreaper"] = True
    expected = (I / "source/payload/expected-installed393.txt").read_text().splitlines()
    actual = run(["/usr/bin/pacman", "-Q"], "original393-before").splitlines()
    assert len(actual) == len(expected) == 393 and sorted(actual) == sorted(expected)
    before = substrate()
    r["before_worker_substrate"] = before
    worker_source = I / S["wireless_worker"]["relative_path"]
    assert pin(worker_source) == S["wireless_worker"]["ELF"]
    fixture = Path("/usr/libexec/pokelinux-fixture")
    fixture.mkdir(mode=0o755, exist_ok=True)
    protected = []
    for directory in [Path("/"), Path("/usr"), Path("/usr/libexec"), fixture]:
        info = directory.lstat()
        assert stat.S_ISDIR(info.st_mode) and info.st_uid == info.st_gid == 0
        assert not info.st_mode & 0o022
        protected.append({"path": str(directory), "uid": info.st_uid, "gid": info.st_gid,
                          "mode": stat.S_IMODE(info.st_mode)})
    assert stat.S_IMODE(fixture.stat().st_mode) == 0o755
    worker_path = fixture / "pokelinux-wireless-scan"
    assert not os.path.lexists(worker_path)
    with worker_source.open("rb") as src, worker_path.open("xb") as dst:
        while block := src.read(65536):
            assert now() < W
            dst.write(block)
        dst.flush()
        os.fsync(dst.fileno())
    worker_path.chmod(0o755)
    os.chown(worker_path, 0, 0)
    info = worker_path.lstat()
    assert stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
    assert stat.S_IMODE(info.st_mode) == 0o755 and info.st_nlink == 1
    assert "security.capability" not in os.listxattr(worker_path)
    assert pin(worker_path) == S["wireless_worker"]["ELF"]
    with worker_path.open("rb") as stream:
        head = stream.read(20)
    assert head[:6] == b"\x7fELF\x02\x01" and int.from_bytes(head[18:20], "little") == 183
    loader = run(["/usr/bin/ldd", "-r", str(worker_path)], "native-wireless-worker-loader")
    assert loader.strip() and "not found" not in loader.lower() and "undefined symbol" not in loader.lower()
    r["native_worker_loader_qualified"] = True
    r["native_worker_installed"] = dict(pin(worker_path), path=str(worker_path),
        protected_parents=protected, uid=0, gid=0, mode=0o755, ELF_machine=183,
        loader_sha256=hashlib.sha256(loader.encode()).hexdigest())
    actual_scan_role = {"ifindex": before["loopback"]["ifindex"]}
    evidence_path = I / "wireless-worker-same-PID-preexec.json"
    evidence_fd = os.open(evidence_path, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
    parent_pid = os.getpid()
    def reduce_worker_before_exec():
        values = worker_caps.reduce_caps(1 << worker_caps.NET_ADMIN)
        os.set_inheritable(evidence_fd, False)
        data = json.dumps({"schema": 1, "stage": "same-PID pre-exec after reduction",
            "pid": os.getpid(), "ppid": os.getppid(), "caps": values,
            "observed_boottime_ns": now(), "evidence_fd_closes_on_exec":
                not os.get_inheritable(evidence_fd)}, sort_keys=True).encode() + b"\n"
        assert len(data) <= 4096 and os.write(evidence_fd, data) == len(data)
    try:
        armed = now()
        until = armed + consumer.ACTION_NS
        assert until < W and consumer.ACTION_NS == 5_000_000_000 and consumer.MAX_WIRE == 32768
        worker_argv = [str(worker_path), str(actual_scan_role["ifindex"]), "2412", str(until)]
        admission = now()
        assert armed <= admission < until
        r["native_scan_worker_commands_admitted"] += 1
        native = scope.start(worker_argv, "one-actual-native-unavailable-control.log",
            consumer.MAX_WIRE, cutoff=until, wait_cutoff=until,
            identity={"preexec_fn": reduce_worker_before_exec, "pass_fds": (evidence_fd,)})
        code = scope.finish(native, until)
    finally:
        os.close(evidence_fd)
    wire = (R / native["name"]).read_bytes()
    receipt = native["wait"]
    r["native_wireless_worker_executed"] = bool(receipt["actual_direct_parent_wait"]
        and receipt["stdout_EOF"] and receipt["argv"] == worker_argv)
    evidence_info = evidence_path.lstat()
    assert stat.S_ISREG(evidence_info.st_mode) and evidence_info.st_uid == evidence_info.st_gid == 0
    assert stat.S_IMODE(evidence_info.st_mode) == 0o600 and 0 < evidence_info.st_size <= 4096
    evidence = json.loads(evidence_path.read_text())
    assert evidence["stage"] == "same-PID pre-exec after reduction"
    assert evidence["pid"] == native["child"].pid and evidence["ppid"] == parent_pid
    assert admission <= evidence["observed_boottime_ns"] < until and evidence["evidence_fd_closes_on_exec"]
    values = evidence["caps"]
    assert all(int(values[k], 16) == 1 << worker_caps.NET_ADMIN for k in ["CapEff", "CapPrm", "CapBnd"])
    assert int(values["CapInh"], 16) == int(values["CapAmb"], 16) == 0 and values["NoNewPrivs"] == "1"
    r["native_worker_same_PID_preexec_evidence"] = {"record": evidence, "pin": pin(evidence_path),
        "after_exec_capability_sample_claimed": False}
    r["native_worker_role"] = receipt["role"]
    r["native_worker_exact_argv"] = worker_argv
    assert r["native_wireless_worker_executed"] and code == 2
    assert receipt["actual_direct_parent_wait"] and receipt["stdout_EOF"]
    assert receipt["wait_before_original_cutoff"] and receipt["EOF_before_original_cutoff"]
    assert receipt["original_cutoff_ns"] == receipt["original_wait_cutoff_ns"] == until
    assert receipt["waited_at_boottime_ns"] < until and receipt["stdout_EOF_at_ns"] < until
    assert not receipt["output_overflow"] and 0 < len(wire) <= consumer.MAX_WIRE
    assert wire.startswith(b"{") and wire.endswith(b"\n") and wire.count(b"\n") == 1
    decoded = json.loads(wire.decode("utf-8"), object_pairs_hook=unique_object)
    assert type(decoded) is dict
    for key in ["schema", "ifindex", "frequency_mhz", "original_until_ns"]:
        assert type(decoded.get(key)) is int
    expected_wire = {"schema": 1, "status": "unavailable", "ifindex": before["loopback"]["ifindex"],
        "frequency_mhz": 2412, "original_until_ns": until, "reason": "unsupported"}
    assert decoded == expected_wire, decoded
    r["exact_unsupported_response_qualified"] = True
    receipt["expected_returncode"] = 2
    receipt["exit_classification"] = "exact same completing ELF returned unsupported on observed non-wireless loopback"
    r["actual_worker_response"] = decoded
    r["actual_worker_wire"] = pin(R / native["name"])
    after = substrate()
    r["after_worker_substrate"] = after
    assert after["loopback"] == before["loopback"] and after["namespace"] == before["namespace"]
    r["module_inventory_delta"] = {"added": sorted(set(after["modules"]) - set(before["modules"])),
        "removed": sorted(set(before["modules"]) - set(after["modules"]))}
    r["zero_kernel_effect_or_all_modules_absent_claimed"] = False
    actual = run(["/usr/bin/pacman", "-Q"], "original393-after").splitlines()
    assert len(actual) == 393 and sorted(actual) == sorted(expected)
    r["original393_versions_unchanged"] = True
    assert r["native_scan_worker_commands_admitted"] == 1
    for relative, row in S["direct_inputs"].items():
        assert pin(I / relative) == {key: row[key] for key in ["bytes", "sha256"]}
    r["unavailable_control_qualified"] = True
    r["fixture_contract_passed"] = True
except BaseException as error:
    r.update(error=repr(error), traceback=traceback.format_exc())
finally:
    r["cleanup_errors"] = scope.close(C)
    r["receipts"] = scope.receipts
    r["all_commands_actual_wait_EOF"] = len(scope.children) == len(scope.receipts) and all(
        v["actual_direct_parent_wait"] and v["stdout_EOF"] and v["wait_before_original_cutoff"]
        and v["EOF_before_original_cutoff"] and expected_command_exit(v) for v in scope.receipts)
    r["all_command_handles_closed"] = all(
        v["log"].closed and v["child"].stdout.closed and v["pidfd"] is None for v in scope.children)
    adopted = []
    r["ECHILD"] = False
    r["adopted_live_child_observed"] = False
    terminal_limit = S["adopted_terminal_closure_policy"]["max_terminal_waits"]
    r["adopted_terminal_wait_limit"] = terminal_limit
    r["adopted_wait_scope"] = "Actual adopted parent waits only; origin not sampled. No PIPE EOF, producer, vendor or radio-cleanup claim."
    try:
        # A final probe can establish ECHILD after the last allowed terminal wait.
        # Never wait after the original C cutoff or signal an unknown child.
        for attempt in range(terminal_limit + 1):
            if now() >= C:
                r["adopted_reap_original_deadline_exhausted"] = True
                break
            pid, status = os.waitpid(-1, os.WNOHANG)
            observed = now()
            if pid == 0:
                r["adopted_live_child_observed"] = True
                r["adopted_live_child_observed_boottime_ns"] = observed
                break
            adopted.append({"pid": pid, "raw_status": status,
                "returncode": os.waitstatus_to_exitcode(status),
                "actual_adopted_parent_wait": True,
                "observed_boottime_ns": observed,
                "before_original_close_cutoff": observed < C})
            if len(adopted) > terminal_limit:
                r["adopted_terminal_wait_limit_exceeded"] = True
                break
    except ChildProcessError:
        r["ECHILD"] = True
        r["ECHILD_observed_boottime_ns"] = now()
    except BaseException as error:
        r["adopted_reap_error"] = repr(error)
    r["adopted_waits"] = adopted
    try:
        remaining = Path("/proc/self/task/" + str(os.getpid()) + "/children").read_text().split()
        r["remaining_direct_child_pids_after_terminal_reap"] = [int(v) for v in remaining]
        r["remaining_children_observed_boottime_ns"] = now()
    except BaseException as error:
        r["remaining_children_observation_error"] = repr(error)
    r["adopted_terminal_closure_qualified"] = bool(
        r["ECHILD"] and r.get("ECHILD_observed_boottime_ns", C) < C
        and not r["adopted_live_child_observed"] and len(adopted) <= terminal_limit
        and not r.get("adopted_reap_error") and not r.get("adopted_terminal_wait_limit_exceeded")
        and not r.get("adopted_reap_original_deadline_exhausted")
        and r.get("remaining_direct_child_pids_after_terminal_reap") == []
        and r.get("remaining_children_observed_boottime_ns", C) < C
        and all(v["actual_adopted_parent_wait"] and v["raw_status"] == 0
                and v["returncode"] == 0 and v["before_original_close_cutoff"]
                and v["observed_boottime_ns"] < C for v in adopted)
        and now() < C)
    r["passed"] = bool(r.get("fixture_contract_passed") and r["all_commands_actual_wait_EOF"]
        and r["all_command_handles_closed"] and not r["cleanup_errors"]
        and r["ECHILD"] and r["adopted_terminal_closure_qualified"] and now() < C)
    shutdown = Scope(R)
    try:
        code, _ = shutdown.run(["/usr/bin/systemctl", "--no-block", "poweroff"],
                               "poweroff.log", C, limit=65536)
        assert code == 0
    except BaseException as error:
        r["poweroff_error"] = repr(error)
    r["shutdown_cleanup_errors"] = shutdown.close(C)
    r["shutdown_receipts"] = shutdown.receipts
    r["normal_poweroff_command_closed"] = not r.get("poweroff_error") and not r["shutdown_cleanup_errors"] and len(shutdown.receipts) == 1 and all(
        v["actual_direct_parent_wait"] and v["stdout_EOF"] and v["wait_before_original_cutoff"]
        and v["EOF_before_original_cutoff"] and v["returncode"] == 0 for v in shutdown.receipts)
    r["passed"] = bool(r["passed"] and r["normal_poweroff_command_closed"])
    r["completed_ns"] = now()
    (R / "result.json").write_text(json.dumps(r, indent=2) + "\n")
    print(json.dumps({"passed": r["passed"], "error": r.get("error")}), flush=True)

raise SystemExit(0 if r["passed"] else 2)
