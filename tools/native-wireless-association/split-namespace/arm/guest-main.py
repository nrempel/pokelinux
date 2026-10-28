"""One normal Rust open-association completion, independent kernel state and explicit Disconnect; assert after owned cleanup."""
from pathlib import Path
import array, ctypes, gzip, hashlib, json, os, re, select, signal, socket, stat, sys, time, traceback
sys.dont_write_bytecode = True
from owned import Scope, now
import arm_tools, consumer, worker_caps, disconnect_consumer
R = Path("/results")
I = Path("/run/hwsim-original-viability")
S = json.loads((R / "selection.json").read_text())
T = int(os.environ["POKELINUX_HWSIM_T0_NS"])
W = T + S["guest_work_seconds"] * 1_000_000_000
C = T + S["guest_close_seconds"] * 1_000_000_000
scope = Scope(R)
ap = None
namespace_fd = None
ap_namespace_fd = None
namespace_keeper = None
namespace_control = None
namespace_other = None
ap_prefix = None
received_namespace_fds = []
r = {"schema": 1, "phase": S["phase"], "passed": False,
     "original_work_until_ns": W, "original_close_until_ns": C,
     "normal_app_graphics_Source_auth_executed": False,
     "native_wireless_worker_executed": False,
     "phone_accessed": False, "scan_commands_admitted": 0,
     "native_association_worker_commands_admitted": 0}
def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size,
                "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
def frequency_mhz_zero_offset(value):
    match = re.fullmatch(r"([0-9]+)(?:\.([0-9]+))?", value)
    assert match and (match[2] is None or int(match[2]) == 0), value
    return int(match[1])
def complete_except_ap():
    return all(v is ap or v is namespace_keeper or (v["wait"] is not None and v["eof"]) for v in scope.children)
def run(argv, label, seconds=20, limit=1048576, expected=0):
    assert expected == 0 or (expected == 1 and label == "hostapd-version"
        and argv == ["/usr/bin/hostapd", "-v"]), "unexpected nonzero command allowance"
    assert complete_except_ap(), "next command fenced by unresolved prior wait/EOF"
    if namespace_keeper is not None and namespace_keeper["wait"] is None:
        assert scope.live(namespace_keeper), "retained AP namespace keeper exited"
    if ap is not None and ap["wait"] is None:
        assert scope.live(ap), "owned AP exited before scan completed"
    end = min(W, now() + seconds * 1_000_000_000)
    try:
        code, body = scope.run(argv, f"{len(scope.children)+1:03d}-{label}.log", end, limit=limit)
    except BaseException:
        scope.close(C)
        raise
    assert complete_except_ap() and now() < end
    if expected is not None:
        assert code == expected, (label, code, body[-1200:])
    return body.decode()
def expected_command_exit(receipt):
    version = r.get("original_hostapd_version_exit", {})
    if receipt["role"] == version.get("role"):
        return (receipt["argv"] == ["/usr/bin/hostapd", "-v"]
                and receipt.get("expected_returncode") == 1
                and receipt["returncode"] == 1)
    if receipt["role"] == r.get("native_worker_role") and receipt["returncode"] == 64:
        observation = r.get("native_worker_observation", {})
        return (receipt["argv"] == r.get("native_worker_exact_argv")
                and receipt.get("expected_returncode") == 64
                and observation.get("known_existing_cli_refusal") is True
                and observation.get("outcome_confirmed") is True)
    return receipt["returncode"] == 0
def devices(body, expected_count=2):
    result = []
    phy = None
    current = None
    for line in body.splitlines():
        match = re.fullmatch(r"phy#([0-9]+)", line.strip())
        if match:
            phy = int(match[1])
        match = re.fullmatch(r"Interface ([A-Za-z0-9_.-]{1,15})", line.strip())
        if match:
            assert phy is not None
            current = {"phy": phy, "name": match[1]}
            result.append(current)
        elif current is not None:
            match = re.fullmatch(r"(ifindex|wdev|addr|type) (.+)", line.strip())
            if match:
                assert match[1] not in current
                current[match[1]] = match[2]
    assert len(result) == expected_count and len({v["phy"] for v in result}) == expected_count
    assert len({v["name"] for v in result}) == expected_count
    for value in result:
        assert all(k in value for k in ["phy", "name", "ifindex", "wdev", "addr", "type"])
        assert value["ifindex"].isdigit() and int(value["ifindex"]) > 0
        assert re.fullmatch(r"0x[0-9a-f]+", value["wdev"])
        assert re.fullmatch(r"[0-9a-f]{2}(?::[0-9a-f]{2}){5}", value["addr"])
    return sorted(result, key=lambda v: v["phy"])
def identity(value):
    return {k: value[k] for k in ["phy", "name", "ifindex", "wdev", "addr"]}
def no_addresses(name, label):
    execute = run_ap if ap_prefix is not None and name == ap_role["name"] else run
    value = json.loads(execute(["/usr/bin/ip", "-j", "address", "show", "dev", name], label))
    assert len(value) == 1 and value[0]["ifname"] == name and value[0]["addr_info"] == []
def inactive(units, label):
    observed = {}
    for unit in units:
        if "@." in unit:
            assert unit.count("@.") == 1
            pattern = unit.replace("@.", "@*.")
            instances = run(["/usr/bin/systemctl", "list-units", "--all", "--plain",
                             "--no-legend", "--no-pager", pattern],
                            label + "-" + unit + "-concrete-instances").strip()
            assert instances == "", (unit, instances)
            observed[unit] = {"bare_template": True, "pattern": pattern,
                              "listed_concrete_instances": []}
        else:
            state = run(["/usr/bin/systemctl", "show", unit,
                         "--property=ActiveState", "--value"], label + "-" + unit).strip()
            assert state == "inactive", (unit, state)
            observed[unit] = state
    return observed
def module_files():
    for relative, row in S["module_files"].items():
        path = Path("/usr/lib/modules") / S["kernel_release"] / relative
        assert stat.S_ISREG(path.lstat().st_mode) and pin(path) == row
def namespace():
    p = Path("/proc/self/ns/net")
    st = p.stat()
    return {"dev": st.st_dev, "ino": st.st_ino,
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}
def namespace_at(path):
    st = Path(path).stat()
    return {"dev": st.st_dev, "ino": st.st_ino,
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}
def retained_AP_namespace():
    assert ap_namespace_fd is not None and namespace_keeper is not None
    st = os.fstat(ap_namespace_fd)
    expected = r["retained_AP_namespace_epoch"]
    assert (st.st_dev, st.st_ino) == (expected["dev"], expected["ino"])
    assert scope.live(namespace_keeper)
    actual = namespace_at("/proc/" + str(namespace_keeper["child"].pid) + "/ns/net")
    assert actual == expected and namespace() == r["retained_namespace_epoch"]
    return actual
def in_ap(argv):
    retained_AP_namespace()
    return ap_prefix + argv
def run_ap(argv, label, **options):
    value = run(in_ap(argv), label, **options)
    retained_AP_namespace()
    return value
def radio_inventory(label):
    station_rows = devices(run(["/usr/bin/iw", "dev"], label + "-STA-namespace"), 1)
    ap_rows = devices(run_ap(["/usr/bin/iw", "dev"], label + "-AP-namespace"), 1)
    assert station_rows[0]["phy"] == scan_role["phy"] and ap_rows[0]["phy"] == ap_role["phy"]
    return sorted(station_rows + ap_rows, key=lambda row: row["phy"])
def namespace_binding():
    peer = retained_AP_namespace()
    assert ap is not None and scope.live(ap)
    actual_ap = namespace_at("/proc/" + str(ap["child"].pid) + "/ns/net")
    assert actual_ap == peer
    return {"station_namespace": namespace(), "AP_namespace": peer,
            "keeper_namespace": namespace_at("/proc/" + str(namespace_keeper["child"].pid) + "/ns/net"),
            "AP_process_namespace": actual_ap, "keeper_pid": namespace_keeper["child"].pid,
            "keeper_pidfd_live": scope.live(namespace_keeper), "AP_pidfd_live": scope.live(ap),
            "observed_at_ns": now()}
try:
    assert os.getresuid() == os.getresgid() == (0, 0, 0) and os.uname().machine == "aarch64"
    assert os.uname().release == S["kernel_release"]
    assert Path("/proc/1/comm").read_text().strip() == "systemd"
    assert sorted(p.name for p in Path("/sys/class/net").iterdir()) == ["lo"]
    assert not list(Path("/sys/class/ieee80211").glob("*"))
    assert "ipv6.disable=1" in Path("/proc/cmdline").read_text().split()
    libc = ctypes.CDLL(None, use_errno=True)
    assert libc.prctl(36, 1, 0, 0, 0) == 0
    observed = ctypes.c_int(-1)
    assert libc.prctl(37, ctypes.byref(observed), 0, 0, 0) == 0 and observed.value == 1
    r["actual_child_subreaper"] = True
    r["declared_exclusive_fixture_contract"] = S["exclusive_fixture_contract"]
    r["retained_namespace_epoch"] = namespace()
    namespace_fd = os.open("/proc/self/ns/net", os.O_RDONLY | os.O_CLOEXEC)
    retained_ns = os.fstat(namespace_fd)
    assert (retained_ns.st_dev, retained_ns.st_ino) == (r["retained_namespace_epoch"]["dev"], r["retained_namespace_epoch"]["ino"])
    r["inactive_before"] = inactive(S["masked_service_units"], "inactive-before")
    for relative, row in S["direct_inputs"].items():
        assert pin(I / relative) == {k: row[k] for k in ["bytes", "sha256"]}
    r["package_install"] = arm_tools.prepare(run, I, S)
    r["hostapd_units_inactive"] = inactive(
        ["hostapd.service", "hostapd@.service"], "hostapd-installed-inactive")
    assert run(["/usr/bin/systemctl", "list-units", "--all", "--type=service",
                "--state=active", "--plain", "--no-legend", "--no-pager", "hostapd*"],
               "no-active-hostapd-instances").strip() == ""
    for relative, row in S["wireless_policy_files"].items():
        assert pin(Path("/") / relative) == row
    policy = Path("/etc/conf.d/wireless-regdom").read_text()
    assert not any(line.strip() and not line.lstrip().startswith("#") for line in policy.splitlines())
    r["packaged_udev_regdom_helper_has_no_configured_domain"] = True
    r["native_loaders"] = {}
    for name in ["hostapd", "hostapd_cli", "iw"]:
        path = Path("/usr/bin") / name
        with path.open("rb") as stream:
            head = stream.read(20)
        assert head[:6] == b"\x7fELF\x02\x01" and int.from_bytes(head[18:20], "little") == 183
        body = run(["/usr/bin/ldd", "-r", str(path)], "native-loader-" + name)
        assert body.strip() and "not found" not in body.lower() and "undefined symbol" not in body.lower()
        r["native_loaders"][name] = dict(pin(path), loader_sha256=hashlib.sha256(body.encode()).hexdigest())
    hostapd_version = run(["/usr/bin/hostapd", "-v"], "hostapd-version", expected=1)
    version_receipt = scope.receipts[-1]
    assert version_receipt["argv"] == ["/usr/bin/hostapd", "-v"]
    version_receipt["expected_returncode"] = 1
    version_receipt["exit_classification"] = "original hostapd2.12 version-report exit1; expected CLI behavior, not execution failure"
    first_line = hostapd_version.splitlines()[0] if hostapd_version.splitlines() else ""
    r["original_hostapd_version_exit"] = {
        "role": version_receipt["role"], "expected_returncode": 1,
        "actual_returncode": version_receipt["returncode"], "first_line": first_line,
        "classification": version_receipt["exit_classification"],
        "source_reference": S["original_hostapd_version_source_reference"]}
    assert first_line == "hostapd v2.12-hostap_2_12+", first_line
    r["original_tool_versions"] = {
        "hostapd": hostapd_version,
        "iw": run(["/usr/bin/iw", "--version"], "iw-version")}
    config = gzip.decompress(Path("/proc/config.gz").read_bytes())
    assert len(config) <= 524288 and hashlib.sha256(config).hexdigest() == S["kernel_config"]["sha256"]
    for setting in ["CONFIG_WIRELESS=y", "CONFIG_CFG80211=m", "CONFIG_MAC80211=m",
                    "CONFIG_MAC80211_HWSIM=m", "CONFIG_RFKILL=m"]:
        assert setting in config.decode().splitlines()
    module_files()
    loaded_before = Path("/proc/modules").read_text()
    assert all(not re.search(r"^" + name + r" ", loaded_before, re.M)
               for name in ["mac80211_hwsim", "mac80211", "cfg80211", "rfkill", "libarc4"])
    dependency_body = run(["/usr/bin/modprobe", "--show-depends", "mac80211_hwsim", "radios=2"],
                          "actual-hwsim-dependency-plan")
    dependency_paths = []
    for line in dependency_body.splitlines():
        words = line.split()
        assert words and words[0] == "insmod" and len(words) >= 2
        dependency_paths.append(str(Path(words[1]).resolve()))
    expected_paths = {str((Path("/usr/lib/modules") / S["kernel_release"] / v).resolve())
                      for v in S["module_files"]}
    assert len(dependency_paths) == len(expected_paths) == 5 and set(dependency_paths) == expected_paths
    for name in ["mac80211_hwsim", "mac80211", "cfg80211", "rfkill", "libarc4"]:
        version = run(["/usr/bin/modinfo", "-F", "vermagic", name], "matching-module-" + name).strip()
        assert version == S["module_vermagic"], (name, version)
    for relative, row in S["regdb_files"].items():
        assert pin(Path("/") / relative) == row
    r["signed_regdb_files_installed"] = S["regdb_files"]
    r["radio_create_command_admitted"] = True
    run(["/usr/bin/modprobe", "mac80211_hwsim", "radios=2"], "create-two-guest-simulated-radios")
    run(["/usr/bin/udevadm", "settle", "--timeout=10"], "guest-radio-udev-settle", seconds=15)
    assert Path("/sys/module/mac80211_hwsim/parameters/radios").read_text().strip() == "2"
    initial = devices(run(["/usr/bin/iw", "dev"], "two-radio-inventory"))
    assert all(v["type"] == "managed" for v in initial)
    extras = set(p.name for p in Path("/sys/class/net").iterdir()) - {"lo", *(v["name"] for v in initial)}
    assert len(extras) <= 1
    for monitor in extras:
        assert (Path("/sys/class/net") / monitor / "type").read_text().strip() == "803"
    r["global_simulated_radiotap_monitors"] = sorted(extras)
    ap_role, scan_role = initial
    r["roles_before"] = initial
    r["regulatory_report"] = run(["/usr/bin/iw", "reg", "get"], "kernel-regulatory-report")
    assert r["regulatory_report"].strip()
    r["actual_channel_capabilities"] = {}
    for role in initial:
        body = run(["/usr/bin/iw", "phy", "phy" + str(role["phy"]), "info"],
                   "actual-capabilities-phy" + str(role["phy"]))
        channel = [line.strip() for line in body.splitlines()
                   if re.fullmatch(r"\*\s+2412(?:\.0+)? MHz \[1\](?:\s.*)?", line.strip())]
        assert len(channel) == 1 and "disabled" not in channel[0].lower() and "no ir" not in channel[0].lower()
        required = "AP" if role is ap_role else "managed"
        assert re.search(r"^\s+\*\s+" + re.escape(required) + r"\s*$", body, re.M)
        r["actual_channel_capabilities"][str(role["phy"])] = {"frequency_MHz": 2412,
            "channel": 1, "original_channel_line": channel[0], "required_mode": required,
            "report_sha256": hashlib.sha256(body.encode()).hexdigest()}
    kernel_log = run(["/usr/bin/dmesg"], "kernel-after-wireless-registration")
    assert not any(text in kernel_log.lower() for text in [
        "failed to load regulatory.db", "regulatory.db is malformed",
        "signature is missing/invalid", "direct firmware load for regulatory.db failed"])
    r["regdb_acceptance_claim"] = "Signed files pinned and rejection diagnostics checked; actual reported regulatory/channel capabilities retained, not an independent signature-loader trace."
    assert namespace() == r["retained_namespace_epoch"]
    for role in initial:
        no_addresses(role["name"], "no-addresses-before-" + role["name"])
    # Create/retain a second namespace before moving the owned, still-down AP phy.
    nsenter = Path("/usr/bin/nsenter")
    assert nsenter.is_file() and os.access(nsenter, os.X_OK)
    with nsenter.open("rb") as stream:
        header = stream.read(20)
    assert header[:6] == b"\x7fELF\x02\x01" and int.from_bytes(header[18:20], "little") == 183
    r["native_nsenter_version"] = run([str(nsenter), "--version"], "native-nsenter-version")
    ns_loader = run(["/usr/bin/ldd", "-r", str(nsenter)], "native-nsenter-loader")
    assert ns_loader.strip() and "not found" not in ns_loader.lower() and "undefined symbol" not in ns_loader.lower()
    namespace_control, namespace_other = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC)
    namespace_control.setblocking(False)
    keeper_armed = now()
    keeper_ready_until = min(W, keeper_armed + 5_000_000_000)
    try:
        namespace_keeper = scope.start(["/usr/bin/python3", "-B", str(I / "source/namespace_peer.py"),
            "keeper", str(namespace_other.fileno()), str(C)], "owned-AP-namespace-keeper.log", 65536,
            cutoff=keeper_ready_until, wait_cutoff=C, identity={"pass_fds": (namespace_other.fileno(),)})
    finally:
        namespace_other.close()
        namespace_other = None
    while not select.select([namespace_control], [], [], .005)[0]:
        scope.pump(); scope.check()
        assert now() < keeper_ready_until and scope.live(namespace_keeper)
    data, ancillary, flags, _ = namespace_control.recvmsg(4096, socket.CMSG_SPACE(array.array("i").itemsize), socket.MSG_CMSG_CLOEXEC)
    for level, kind, payload in ancillary:
        assert level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS
        values = array.array("i"); values.frombytes(payload); received_namespace_fds.extend(values)
    assert not flags & ~socket.MSG_CMSG_CLOEXEC and 0 < len(data) <= 4096
    assert len(received_namespace_fds) == 1
    ap_namespace_fd = received_namespace_fds[0]
    assert not os.get_inheritable(ap_namespace_fd)
    kept = json.loads(data, object_pairs_hook=consumer.unique, parse_constant=consumer.no_constant)
    assert set(kept) == {"namespace", "caps", "uid"} and kept["uid"] == 0
    assert all(int(v, 16) == 0 for k, v in kept["caps"].items() if k.startswith("Cap")) and kept["caps"]["NoNewPrivs"] == "1"
    ap_stat = os.fstat(ap_namespace_fd)
    assert kept["namespace"] == {"dev": ap_stat.st_dev, "ino": ap_stat.st_ino}
    r["retained_AP_namespace_epoch"] = dict(kept["namespace"], boot_id=r["retained_namespace_epoch"]["boot_id"])
    assert r["retained_AP_namespace_epoch"] != r["retained_namespace_epoch"]
    r["AP_namespace_keeper_ready"] = {"actual_record": kept, "pid": namespace_keeper["child"].pid,
        "armed_at_ns": keeper_armed, "original_ready_until_ns": keeper_ready_until, "observed_at_ns": now()}
    assert now() < keeper_ready_until and scope.live(namespace_keeper)
    ap_prefix = [str(nsenter), "--no-fork", "--net=/proc/" + str(namespace_keeper["child"].pid) + "/ns/net", "--"]
    empty_peer = json.loads(run_ap(["/usr/bin/ip", "-j", "address", "show"], "peer-namespace-initially-lo-only"))
    assert len(empty_peer) == 1 and empty_peer[0]["ifname"] == "lo" and empty_peer[0]["addr_info"] == []
    r["roles_before_namespace_move"] = initial
    r["AP_namespace_move_commands_admitted"] = 1
    run(["/usr/bin/iw", "phy", "phy" + str(ap_role["phy"]), "set", "netns", str(namespace_keeper["child"].pid)],
        "move-owned-AP-phy-to-retained-peer-namespace")
    moved = radio_inventory("post-move-radio-inventory")
    for before_role, after_role in zip(initial, moved):
        assert all(before_role[key] == after_role[key] for key in ["phy", "name", "wdev", "addr"])
        if before_role["phy"] == scan_role["phy"]:
            assert before_role["ifindex"] == after_role["ifindex"]
    ap_role, scan_role = moved
    initial = moved
    r["roles_before"] = initial
    r["AP_namespace_move_qualified"] = True
    r["AP_namespace_entry_argv_prefix"] = ap_prefix
    run(["/usr/bin/ip", "link", "set", "dev", scan_role["name"], "up"], "scan-interface-up")
    nonce = os.getrandom(12).hex()
    ssid = "pkl-hwsim-" + nonce[:20]
    assert len(ssid.encode()) <= 32
    private = I / "ap-control"
    private.mkdir(mode=0o700)
    config_path = I / "hostapd.conf"
    config_path.write_text("\n".join([
        "driver=nl80211", "interface=" + ap_role["name"], "ssid=" + ssid,
        "hw_mode=g", "channel=1", "auth_algs=1", "wpa=0",
        "ctrl_interface=" + str(private), "logger_stdout=-1",
        "logger_stdout_level=2", ""]) )
    config_path.chmod(0o600)
    r["AP_configuration"] = {"SSID": ssid, "BSSID": ap_role["addr"],
        "frequency_MHz": 2412, "config": pin(config_path), "open_auth_only": True}
    ap = scope.start(in_ap(["/usr/bin/hostapd", str(config_path)]), "owned-foreground-hostapd.log",
                     262144, cutoff=W, wait_cutoff=W)
    ready_until = min(W, now() + 15_000_000_000)
    while True:
        scope.pump()
        scope.check()
        assert scope.live(ap), "hostapd exited before AP-ENABLED"
        if "AP-ENABLED" in (R / ap["name"]).read_text():
            assert now() < ready_until, "original AP startup bound before readiness acceptance"
            break
        assert now() < ready_until, "original AP startup bound"
        time.sleep(.005)
    r["AP_process_namespace_after_ready"] = namespace_binding()
    assert Path("/proc/" + str(ap["child"].pid) + "/comm").read_text().strip() == "hostapd"
    current = radio_inventory("AP-ready-split-radio-inventory")
    assert [identity(v) for v in current] == [identity(v) for v in initial]
    assert current[0]["type"] == "AP" and current[1]["type"] == "managed"
    for role in current:
        no_addresses(role["name"], "no-addresses-before-scan-" + role["name"])
    assert namespace() == r["retained_namespace_epoch"]
    # BEGIN EXPLICIT_OPEN_ASSOCIATION_CONSUMER: unchanged normal ARM worker, one original5s request.
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
    actual_station = {"name": scan_role["name"], "ifindex": int(scan_role["ifindex"]),
        "wiphy": scan_role["phy"], "wdev": int(scan_role["wdev"], 16),
        "mac_hex": scan_role["addr"].replace(":", "")}
    actual_ap = {"name": ap_role["name"], "ifindex": int(ap_role["ifindex"]),
        "wiphy": ap_role["phy"], "wdev": int(ap_role["wdev"], 16),
        "bssid_hex": ap_role["addr"].replace(":", ""),
        "ssid_hex": ssid.encode().hex(), "frequency_mhz": 2412}
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
        worker_argv = [str(worker_path), "associate-open", str(actual_station["ifindex"]), "2412",
                       actual_ap["bssid_hex"], actual_ap["ssid_hex"], str(until)]
        admission = now()
        assert armed <= admission < until
        r["native_association_worker_commands_admitted"] += 1
        native = scope.start(worker_argv, "one-actual-explicit-association-request-worker.log",
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
    observation = consumer.observe_worker(code, wire, receipt,
        executable=str(worker_path), armed_at_ns=armed, pre_spawn_admission_ns=admission,
        station=actual_station, ap=actual_ap)
    r["native_worker_observation"] = observation
    if observation.get("known_existing_cli_refusal") is True and observation.get("outcome_confirmed") is True:
        receipt["expected_returncode"] = 64
        receipt["exit_classification"] = "exact frozen consumer confirmed existing normal CLI invalid_arguments refusal64"
    (R / "native-worker-observation.json").write_text(json.dumps(observation, indent=2) + "\n")
    # Independent kernel observations are captured before teardown under the
    # very same original action cutoff, without renewal or another mutation.
    kernel_receipts = {}
    kernel_outputs = {}
    for label, argv in [
        ("link", ["/usr/bin/iw", "dev", scan_role["name"], "link"]),
        ("AP_station_dump", in_ap(["/usr/bin/iw", "dev", ap_role["name"], "station", "dump"])),
        ("station_roles", ["/usr/bin/iw", "dev"]),
        ("AP_roles", in_ap(["/usr/bin/iw", "dev"]))]:
        assert complete_except_ap() and scope.live(ap) and now() < until
        status, body = scope.run(argv, f"{len(scope.children)+1:03d}-independent-{label}.log",
                                 until, limit=262144)
        assert status == 0 and complete_except_ap() and now() < until
        kernel_outputs[label] = body
        kernel_receipts[label] = scope.receipts[-1]
    kernel = consumer.observe_kernel(station=actual_station, ap=actual_ap,
        armed_at_ns=armed, until_ns=until,
        link_stdout=kernel_outputs["link"], station_dump_stdout=kernel_outputs["AP_station_dump"],
        roles_stdout=kernel_outputs["station_roles"], ap_roles_stdout=kernel_outputs["AP_roles"],
        receipts=kernel_receipts, observed_at_ns=now(), ap_prefix=ap_prefix, namespace_binding=namespace_binding())
    assert now() < until, "original association cutoff after namespace/kernel decoding"
    r["independent_kernel_observation"] = kernel
    r["independent_kernel_receipts"] = kernel_receipts
    r["no_reconnect_or_retry_or_implicit_disconnect"] = True
    (R / "kernel-observation-before-cleanup.json").write_text(json.dumps({
        "observation": kernel, "receipts": kernel_receipts,
        "raw_output_sha256": {key: hashlib.sha256(body).hexdigest() for key, body in kernel_outputs.items()},
        "worker_observation": observation}, indent=2) + "\n")
    r["explicit_disconnect_and_readback_qualified"] = False
    r["explicit_disconnect_commands_admitted"] = 0
    # Only a confirmed association admits this separate explicit cleanup action.
    # Uncertain Connect outcomes never resubmit, reconnect or implicitly Disconnect.
    if observation.get("requested_association_completed") is True and kernel.get("exact_selected_AP_association") is True:
        evidence_path_disconnect = I / "wireless-disconnect-same-PID-preexec.json"
        evidence_fd = os.open(evidence_path_disconnect, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
        try:
            disconnect_armed = now()
            disconnect_until = disconnect_armed + consumer.ACTION_NS
            assert disconnect_until < W
            disconnect_argv = [str(worker_path), "disconnect", str(actual_station["ifindex"]), str(disconnect_until)]
            r["explicit_disconnect_commands_admitted"] += 1
            disconnected_child = scope.start(disconnect_argv, "one-explicit-normal-Rust-disconnect.log",
                consumer.MAX_WIRE, cutoff=disconnect_until, wait_cutoff=disconnect_until,
                identity={"preexec_fn": reduce_worker_before_exec, "pass_fds": (evidence_fd,)})
            disconnect_code = scope.finish(disconnected_child, disconnect_until)
        finally:
            os.close(evidence_fd)
        evidence_disconnect = json.loads(evidence_path_disconnect.read_text())
        assert evidence_disconnect["pid"] == disconnected_child["child"].pid and evidence_disconnect["ppid"] == parent_pid
        assert disconnect_armed <= evidence_disconnect["observed_boottime_ns"] < disconnect_until
        assert evidence_disconnect["evidence_fd_closes_on_exec"]
        caps_disconnect = evidence_disconnect["caps"]
        assert all(int(caps_disconnect[k], 16) == 1 << worker_caps.NET_ADMIN for k in ["CapEff", "CapPrm", "CapBnd"])
        assert int(caps_disconnect["CapInh"],16) == int(caps_disconnect["CapAmb"],16) == 0 and caps_disconnect["NoNewPrivs"] == "1"
        r["disconnect_same_PID_preexec_evidence"] = {"record":evidence_disconnect,"pin":pin(evidence_path_disconnect),"after_exec_capabilities_claimed":False}
        disconnect_receipts = {}
        disconnect_outputs = {}
        for label, argv in [
            ("link", ["/usr/bin/iw", "dev", scan_role["name"], "link"]),
            ("AP_station_dump", in_ap(["/usr/bin/iw", "dev", ap_role["name"], "station", "dump"])),
            ("station_roles", ["/usr/bin/iw", "dev"]),
            ("AP_roles", in_ap(["/usr/bin/iw", "dev"]))]:
            assert complete_except_ap() and scope.live(ap) and now() < disconnect_until
            status, body = scope.run(argv, f"{len(scope.children)+1:03d}-independent-disconnect-{label}.log",
                                     disconnect_until, limit=262144)
            assert status == 0 and complete_except_ap() and now() < disconnect_until
            disconnect_outputs[label] = body
            disconnect_receipts[label] = scope.receipts[-1]
        disconnect_kernel = consumer.observe_kernel(station=actual_station,ap=actual_ap,
            armed_at_ns=disconnect_armed,until_ns=disconnect_until,
            link_stdout=disconnect_outputs["link"],station_dump_stdout=disconnect_outputs["AP_station_dump"],
            roles_stdout=disconnect_outputs["station_roles"],ap_roles_stdout=disconnect_outputs["AP_roles"],
            receipts=disconnect_receipts,observed_at_ns=now(),ap_prefix=ap_prefix,namespace_binding=namespace_binding())
        disconnect_observation = disconnect_consumer.observe(disconnect_code,
            (R / disconnected_child["name"]).read_bytes(),disconnected_child["wait"],
            executable=str(worker_path),armed_at_ns=disconnect_armed,station=actual_station,ap=actual_ap,kernel=disconnect_kernel)
        assert now() < disconnect_until, "original Disconnect cutoff after namespace/kernel decoding"
        r["explicit_disconnect_observation"] = disconnect_observation
        r["explicit_disconnect_and_readback_qualified"] = disconnect_observation["qualified"]
        (R / "explicit-disconnect-observation.json").write_text(json.dumps({"observation":disconnect_observation,
            "worker_receipt":disconnected_child["wait"],"kernel_receipts":disconnect_receipts},indent=2)+"\n")
    # END EXPLICIT_OPEN_ASSOCIATION_CONSUMER. Existing owned fixture teardown follows.
    after = radio_inventory("same-split-radio-epoch-after-actions")
    assert [identity(v) for v in after] == [identity(v) for v in initial]
    assert namespace() == r["retained_namespace_epoch"] and scope.live(ap)
    for role in after:
        no_addresses(role["name"], "no-addresses-after-scan-" + role["name"])
    r["original_iw_scan_commands_admitted"] = r["scan_commands_admitted"]
    assert r["original_iw_scan_commands_admitted"] == 0
    r["one_native_worker_same_exclusive_epoch"] = r["native_association_worker_commands_admitted"] == 1
    assert r["one_native_worker_same_exclusive_epoch"]
    signal.pidfd_send_signal(ap["pidfd"], signal.SIGTERM)
    assert scope.finish(ap, min(W, now() + 10_000_000_000)) == 0
    assert ap["eof"]
    r["foreground_AP_SIGTERM_actual_wait0_EOF"] = True
    control_endpoint = private / ap_role["name"]
    assert not os.path.lexists(control_endpoint), "owned AP control endpoint remains after actual wait0/EOF"
    r["owned_AP_control_endpoint_absent_after_actual_wait0_EOF"] = str(control_endpoint)
    run(["/usr/bin/modprobe", "-r", "mac80211_hwsim"], "remove-owned-simulated-radios")
    assert not list(Path("/sys/class/ieee80211").glob("*"))
    assert sorted(p.name for p in Path("/sys/class/net").iterdir()) == ["lo"]
    assert not Path("/sys/module/mac80211_hwsim").exists()
    r["owned_simulated_radios_removed"] = True
    assert run_ap(["/usr/bin/iw", "dev"], "peer-no-wireless-interfaces-after-owned-module-removal").strip() == ""
    peer_after = json.loads(run_ap(["/usr/bin/ip", "-j", "address", "show"], "peer-lo-only-after-owned-module-removal"))
    assert len(peer_after) == 1 and peer_after[0]["ifname"] == "lo" and peer_after[0]["addr_info"] == []
    r["AP_namespace_lo_only_after_radio_removal"] = peer_after
    assert namespace_control.send(b"STOP", socket.MSG_DONTWAIT) == 4
    assert scope.finish(namespace_keeper, W) == 0 and namespace_keeper["eof"]
    keeper_result = json.loads((R / namespace_keeper["name"]).read_text())
    assert keeper_result == {"keeper_stopped": True}
    r["AP_namespace_keeper_STOP_actual_wait0_EOF"] = True
    namespace_control.close(); namespace_control = None
    module_files()
    arm_tools.verify_after(run, I, S, r["package_install"])
    r["inactive_after"] = inactive(S["masked_service_units"], "inactive-after")
    for relative, row in S["direct_inputs"].items():
        assert pin(I / relative) == {k: row[k] for k in ["bytes", "sha256"]}
    r["original393_versions_unchanged_after_exact3"] = True
    r["fixture_contract_passed"] = True
except BaseException as error:
    r.update(error=repr(error), traceback=traceback.format_exc())
finally:
    r["cleanup_errors"] = scope.close(C)
    failure_cleanup = Scope(R)
    try:
        if r.get("radio_create_command_admitted") and Path("/sys/module/mac80211_hwsim").exists():
            assert all(v["wait"] is not None and v["eof"] and v["log"].closed and v["child"].stdout.closed and v["pidfd"] is None for v in scope.children), "radio cleanup fenced by unretired worker"
            r["failure_owned_radio_cleanup_attempted"] = True
            code, _ = failure_cleanup.run(["/usr/bin/modprobe", "-r", "mac80211_hwsim"], "failure-owned-radio-removal.log", C, limit=65536)
            assert code == 0 and not Path("/sys/module/mac80211_hwsim").exists()
            assert not list(Path("/sys/class/ieee80211").glob("*")) and sorted(p.name for p in Path("/sys/class/net").iterdir()) == ["lo"]
            if ap_namespace_fd is not None:
                cleanup_argv = ["/usr/bin/nsenter", "--no-fork", "--net=/proc/self/fd/" + str(ap_namespace_fd), "--", "/usr/bin/ip", "-j", "address", "show"]
                code, body = failure_cleanup.run(cleanup_argv, "failure-peer-lo-only-after-radio-removal.log", C, limit=65536, identity={"pass_fds": (ap_namespace_fd,)})
                values = json.loads(body); assert code == 0 and len(values) == 1 and values[0]["ifname"] == "lo" and values[0]["addr_info"] == []
            r["failure_owned_radio_cleanup_confirmed"] = True
    except BaseException as error:
        r["failure_owned_radio_cleanup_error"] = repr(error)
    r["failure_radio_cleanup_errors"] = failure_cleanup.close(C)
    r["failure_radio_cleanup_receipts"] = failure_cleanup.receipts
    r["receipts"] = scope.receipts
    r["all_commands_actual_wait_EOF"] = len(scope.children) == len(scope.receipts) and all(
        v["actual_direct_parent_wait"] and v["stdout_EOF"] and v["wait_before_original_cutoff"]
        and v["EOF_before_original_cutoff"] and expected_command_exit(v) for v in scope.receipts)
    r["all_command_handles_closed"] = all(
        v["log"].closed and v["child"].stdout.closed and v["pidfd"] is None for v in scope.children)
    if namespace_control is not None:
        namespace_control.close(); namespace_control = None
    if namespace_other is not None:
        namespace_other.close(); namespace_other = None
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
    try:
        assert namespace_fd is not None
        retained_ns = os.fstat(namespace_fd)
        assert (retained_ns.st_dev, retained_ns.st_ino) == (r["retained_namespace_epoch"]["dev"], r["retained_namespace_epoch"]["ino"])
        assert namespace() == r["retained_namespace_epoch"]
        os.close(namespace_fd)
        namespace_fd = None
        r["retained_namespace_custody_qualified"] = True
        r["retained_namespace_descriptor_closed"] = True
    except BaseException as error:
        r["namespace_closure_error"] = repr(error)
        r["retained_namespace_custody_qualified"] = False
        if namespace_fd is not None:
            os.close(namespace_fd)
            namespace_fd = None
    try:
        assert ap_namespace_fd is not None and len(received_namespace_fds) == 1
        st = os.fstat(ap_namespace_fd)
        assert (st.st_dev, st.st_ino) == (r["retained_AP_namespace_epoch"]["dev"], r["retained_AP_namespace_epoch"]["ino"])
        assert r.get("AP_namespace_keeper_STOP_actual_wait0_EOF") is True
        assert r.get("AP_namespace_lo_only_after_radio_removal") is not None
        r["retained_AP_namespace_custody_qualified"] = True
    except BaseException as error:
        r["AP_namespace_closure_error"] = repr(error)
        r["retained_AP_namespace_custody_qualified"] = False
    finally:
        for descriptor in received_namespace_fds:
            os.close(descriptor)
        received_namespace_fds.clear(); ap_namespace_fd = None
        r["retained_AP_namespace_descriptors_closed"] = True
    r["passed"] = bool(r.get("fixture_contract_passed") and r.get("retained_AP_namespace_custody_qualified") and r["all_commands_actual_wait_EOF"]
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
    r["fixture_custody_passed"] = bool(r["passed"] and r["normal_poweroff_command_closed"])
    observation = r.get("native_worker_observation", {
        "schema": 1, "desired_passed": False, "known_refusing_baseline": False,
        "outcome_confirmed": False, "classification": "WORKER_NOT_QUALIFIED"})
    review = consumer.qualify_after_cleanup(observation,
        r.get("independent_kernel_observation", {}),
        fixture_qualified=r.get("fixture_contract_passed") is True,
        worker_loader_qualified=r.get("native_worker_loader_qualified") is True,
        retained_namespace_qualified=(r.get("retained_namespace_custody_qualified") is True
            and r.get("retained_AP_namespace_custody_qualified") is True),
        cleanup_qualified=r["fixture_custody_passed"],
        explicit_disconnect_and_readback_qualified=r.get("explicit_disconnect_and_readback_qualified") is True)
    r["consumer_review"] = review
    # Persist the actual worker/fixture/cleanup evidence before the one desired assertion.
    (R / "consumer-review-before-desired-assert.json").write_text(json.dumps({
        "review": review, "receipts": r["receipts"], "adopted_waits": r["adopted_waits"],
        "adopted_terminal_closure_qualified": r["adopted_terminal_closure_qualified"],
        "shutdown_receipts": r["shutdown_receipts"],
        "consumer": S["desired_consumer"]}, indent=2) + "\n")
    r["desired_assertion_calls"] = 1
    try:
        consumer.assert_desired_after_cleanup(review)
        r["desired_assertion"] = {"passed": True}
    except BaseException as error:
        r["desired_assertion"] = {"passed": False, "error_kind": type(error).__name__,
                                  "error": str(error)}
    r["desired_passed"] = bool(review["passed"] and r["desired_assertion"]["passed"])
    r["behavior_RED_qualified"] = bool(review["behavior_RED_qualified"]
        and r["desired_assertion"].get("error_kind") == "AssertionError"
        and r["desired_assertion"].get("error") == consumer.DESIRED_MARKER)
    r["classification"] = ("QUALIFIED_EXISTING_NORMAL_CLI_ASSOCIATION_REFUSAL_RED"
        if r["behavior_RED_qualified"] else "DESIRED_NATIVE_ASSOCIATION_PASSED"
        if r["desired_passed"] else "PREREQUISITE_CLEANUP_OR_WORKER_OUTCOME_UNQUALIFIED")
    r["split_namespace_association_prerequisite_qualified"] = bool(r["desired_passed"] and r.get("AP_namespace_move_qualified")
        and r.get("retained_AP_namespace_custody_qualified") and r.get("AP_namespace_keeper_STOP_actual_wait0_EOF"))
    r["passed"] = r["desired_passed"]
    r["completed_ns"] = now()
    (R / "result.json").write_text(json.dumps(r, indent=2) + "\n")
    print(json.dumps({"desired_passed": r["desired_passed"],
        "behavior_RED_qualified": r["behavior_RED_qualified"],
        "classification": r["classification"], "error": r.get("error")}), flush=True)
raise SystemExit(0 if r["desired_passed"] else 1 if r["behavior_RED_qualified"] else 2)
