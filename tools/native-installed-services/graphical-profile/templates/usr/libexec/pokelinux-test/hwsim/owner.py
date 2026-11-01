"""TEST ONLY installed hwsim/AP provisioning; no service actions or authored authority."""
import time
T = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
from pathlib import Path
import array, ctypes, gzip, hashlib, json, os, re, resource, select, signal, socket, stat, sys, traceback
sys.dont_write_bytecode = True
I = Path(__file__).resolve().parent
sys.path.insert(0, str(I / "source"))
from owned import Scope, now
import consumer

def protected_bytes(path, maximum, expected=None):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        assert stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
        assert stat.S_IMODE(info.st_mode) == 0o600 and info.st_nlink == 1
        data = stream.read(maximum + 1)
    assert 0 < len(data) <= maximum
    if expected is not None:
        assert expected == {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    return data

assert len(sys.argv) == 2 and sys.argv[1] == "/etc/pokelinux/hwsim-provision.json"
S = json.loads(protected_bytes(Path(sys.argv[1]), 262144), object_pairs_hook=consumer.unique,
               parse_constant=consumer.no_constant)
assert S["schema"] == 1 and S["guest_work_seconds"] == 660 and S["guest_close_seconds"] == 700
assert S["adopted_terminal_closure_policy"]["max_terminal_waits"] == 512
assert I == Path("/usr/libexec/pokelinux-test/hwsim")
R = Path("/run/pokelinux-hwsim")
runtime = R.lstat()
assert stat.S_ISDIR(runtime.st_mode) and runtime.st_uid == runtime.st_gid == 0
assert stat.S_IMODE(runtime.st_mode) == 0o700
assert not any(os.path.lexists(R / name) for name in ["result.json", "ready.json", "fixture-bootstrap.json", "ap-control"])
W = T + 660_000_000_000
C = T + 700_000_000_000
scope = Scope(R)
ap = namespace_fd = ap_namespace_fd = namespace_keeper = None
namespace_control = namespace_other = ap_prefix = None
received_namespace_fds = []
stop_requested = False
provisioning = True
CHILD_ENV = dict(os.environ)
CHILD_ENV.pop("NOTIFY_SOCKET", None)
CHILD_ENV.pop("INVOCATION_ID", None)
r = {"schema":1,"phase":"TEST_ONLY_INSTALLED_HWSIM_AP_PROVISION","passed":False,
     "original_work_until_ns":W,"original_close_until_ns":C,"phone_accessed":False,
     "scan_commands_admitted":0,"native_association_worker_commands_admitted":0,
     "actor_grant_Connect_or_main_launch_admitted":False,"protected_profile_rewritten":False,
     "READY_sent":False,"native_worker_or_App_wait_claimed":False}
def request_stop(number, frame):
    global stop_requested
    stop_requested = True
    r.setdefault("SIGTERM_observed_at_ns", now())
signal.signal(signal.SIGTERM, request_stop)
def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size,
                "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
def publish_record(path, value, until):
    """One bounded local publication; no shutdown, timer or effect retry."""
    assert now() < until
    body = (json.dumps(value, indent=2) + "\n").encode()
    soft, hard = resource.getrlimit(resource.RLIMIT_FSIZE)
    assert len(body) <= 1_048_576 and (soft == resource.RLIM_INFINITY or len(body) <= soft), "selected publication exceeds inherited FSIZE; no limit override"
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("xb") as stream:
        stream.write(body)
        stream.flush()
        os.fsync(stream.fileno())
    assert now() < until
    os.replace(temporary, path)
    descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    assert now() < until

def frequency_mhz_zero_offset(value):
    match = re.fullmatch(r"([0-9]+)(?:\.([0-9]+))?", value)
    assert match and (match[2] is None or int(match[2]) == 0), value
    return int(match[1])
def complete_except_ap():
    return all(v is ap or v is namespace_keeper or (v["wait"] is not None and v["eof"]) for v in scope.children)
def run(argv, label, seconds=20, limit=1048576, expected=0):
    assert expected == 0 or (expected == 1 and label == "hostapd-version"
        and argv == ["/usr/bin/hostapd", "-v"]), "unexpected nonzero command allowance"
    assert not provisioning or not stop_requested, "Stop during provisioning; no READY or new effect"
    assert complete_except_ap(), "next command fenced by unresolved prior wait/EOF"
    if namespace_keeper is not None and namespace_keeper["wait"] is None:
        assert scope.live(namespace_keeper), "retained AP namespace keeper exited"
    if ap is not None and ap["wait"] is None:
        assert scope.live(ap), "owned AP exited before scan completed"
    end = min(W, now() + seconds * 1_000_000_000)
    try:
        code, body = scope.run(argv, f"{len(scope.children)+1:03d}-{label}.log", end, limit=limit, env=CHILD_ENV)
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
def unit_properties(name, label, cleanup_scope=None):
    argv = ["/usr/bin/systemctl", "show", "--all", name,
        "--property=Id,Type,MainPID,ControlPID,ActiveState,SubState,InvocationID,ControlGroup,NotifyAccess,KillMode,KillSignal,Restart,TimeoutStartUSec,TimeoutStopUSec,LimitFSIZE,LimitFSIZESoft,FragmentPath,ExecStart,ExecStopPost"]
    if cleanup_scope is None:
        text = run(argv, label)
    else:
        code, body = cleanup_scope.run(argv, "failure-"+label+".log", C, limit=1048576, env=CHILD_ENV)
        assert code == 0 and now() < C
        text = body.decode()
    return dict(line.split("=", 1) for line in text.splitlines())

def installed_dependents_stopped(cleanup_scope=None):
    states = {}
    for name in ["pokelinux-connectivity.service", "pokelinux-session-owner.service"]:
        value = unit_properties(name, "dependent-stopped-" + name, cleanup_scope)
        assert value["Id"] == name and value["MainPID"] == value["ControlPID"] == "0"
        assert value["ActiveState"] in ["inactive", "failed"]
        group = value["ControlGroup"] or "/system.slice/" + name
        assert group == "/system.slice/" + name
        path = Path("/sys/fs/cgroup") / group.lstrip("/")
        if path.exists():
            assert not (path / "cgroup.procs").read_text().split()
            events = dict(line.split() for line in (path / "cgroup.events").read_text().splitlines())
            assert events["populated"] == "0"
        states[name] = value
    r["dependent_units_stopped_before_radio_cleanup"] = states
    r["dependent_unit_state_is_not_native_reap_evidence"] = True

def notify_ready():
    assert not stop_requested and now() < W
    value = unit_properties("pokelinux-hwsim-provision.service", "actual-owner-before-READY")
    assert value["MainPID"] == str(os.getpid()) and os.getppid() == 1
    assert value["InvocationID"] == os.environ["INVOCATION_ID"] == r["InvocationID"]
    assert value["Type"] == "notify" and value["NotifyAccess"] == "main"
    assert value["ActiveState"] == "activating" and value["SubState"] == "start"
    station_identity = {"name":scan_role["name"],"ifindex":int(scan_role["ifindex"]),
        "wiphy":scan_role["phy"],"wdev":int(scan_role["wdev"],16),
        "mac_hex":scan_role["addr"].replace(":", "")}
    AP_identity = {"name":ap_role["name"],"ifindex":int(ap_role["ifindex"]),
        "wiphy":ap_role["phy"],"wdev":int(ap_role["wdev"],16),
        "bssid_hex":ap_role["addr"].replace(":", ""),"ssid_hex":ssid.encode().hex(),
        "frequency_mhz":2412}
    record = {"PID":os.getpid(),"PPID":os.getppid(),"InvocationID":r["InvocationID"],
        "station":station_identity,"AP":AP_identity,"observed_radio_roles":current,
        "namespace_binding":namespace_binding(),
        "AP_namespace_fd_path":"/proc/"+str(os.getpid())+"/fd/"+str(ap_namespace_fd),
        "config_pins":r["protected_config_pins"],"AP_fixed_peer":r["AP_fixed_peer"],
        "station_unassociated":True,"station_addresses_empty":True,"actual_owner_unit":value,
        "original_work_until_ns":W,"original_close_until_ns":C,"published_at_ns":now(),
        "scope":"Provisioning only; not Main/network/session/permission readiness"}
    publish_record(R / "ready.json", record, W)
    assert not stop_requested
    address = os.environ["NOTIFY_SOCKET"]
    assert address.startswith("/") or address.startswith("@")
    if address.startswith("@"):
        address = "\0" + address[1:]
    message = b"READY=1\nSTATUS=Owned fixed hwsim station and AP provisioned; no connection requested"
    with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM | socket.SOCK_NONBLOCK | socket.SOCK_CLOEXEC) as notify:
        assert notify.sendto(message, address) == len(message)
    r["READY_sent"] = True
    r["READY_sender_actual_PID1_Main"] = os.getpid()
    r["READY_sent_at_ns"] = now()

try:
    inherited_fsize = list(resource.getrlimit(resource.RLIMIT_FSIZE))
    for name, row in S["installed_files"].items():
        path = Path("/") / name
        resolved = path.resolve(strict=True)
        assert str(resolved) == row.get("resolved_path", str(path)), "preselected canonical installed tool identity"
        info = resolved.lstat()
        assert stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
        assert not stat.S_IMODE(info.st_mode) & 0o022
        assert pin(path) == {key:row[key] for key in ["bytes","sha256"]}
    properties = unit_properties("pokelinux-hwsim-provision.service", "effective-fixture-unit-properties")
    assert properties["Id"] == "pokelinux-hwsim-provision.service" and properties["Type"] == "notify"
    assert properties["MainPID"] == str(os.getpid()) and os.getppid() == 1
    assert properties["NotifyAccess"] == "main" and properties["KillMode"] == "mixed"
    assert properties["KillSignal"] == "15" and properties["Restart"] == "no"
    assert properties["TimeoutStartUSec"] == "11min" and properties["TimeoutStopUSec"] == "1min 30s"
    typed_stop_post = json.loads(run([
        "/usr/bin/busctl", "--system", "--json=short", "get-property", "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1/unit/pokelinux_2dhwsim_2dprovision_2eservice",
        "org.freedesktop.systemd1.Service", "ExecStopPost"], "typed-empty-ExecStopPost", limit=65536),
        object_pairs_hook=consumer.unique, parse_constant=consumer.no_constant)
    assert typed_stop_post == {"type":"a(sasbttttuii)", "data":[]}
    r["typed_ExecStopPost"] = typed_stop_post
    assert re.fullmatch(r"[0-9a-f]{32}", os.environ["INVOCATION_ID"])
    assert properties["InvocationID"] == os.environ["INVOCATION_ID"]
    assert Path("/proc/self/cgroup").read_text().strip() == "0::" + properties["ControlGroup"]
    r["InvocationID"] = properties["InvocationID"]
    r["effective_fixture_bootstrap"] = {"PID":os.getpid(),"PPID":os.getppid(),
        "effective_unit_properties":properties,"inherited_RLIMIT_FSIZE_soft_hard":inherited_fsize}
    publish_record(R / "fixture-bootstrap.json", r["effective_fixture_bootstrap"], W)
    profile_path = Path("/etc/pokelinux/connectivity.json")
    config_path = Path("/etc/pokelinux/two-boot-hostapd.conf")
    profile_bytes = protected_bytes(profile_path, 4096, S["protected_config_pins"][str(profile_path)])
    ap_bytes = protected_bytes(config_path, 4096, S["protected_config_pins"][str(config_path)])
    expected_profile = {"backend":"wireless_open","interface":"wlan1","frequency_mhz":2412,
        "bssid":[2,0,0,0,0,0],"ssid":list(b"pkl-two-boot-sol1")}
    assert json.loads(profile_bytes, object_pairs_hook=consumer.unique, parse_constant=consumer.no_constant) == expected_profile
    expected_ap = {"driver":"nl80211","interface":"wlan0","bssid":"02:00:00:00:00:00",
        "ssid":"pkl-two-boot-sol1","hw_mode":"g","channel":"1","auth_algs":"1","wpa":"0",
        "ctrl_interface":"/run/pokelinux-hwsim/ap-control","logger_stdout":"-1","logger_stdout_level":"2"}
    pairs = [line.split("=",1) for line in ap_bytes.decode().splitlines() if line.strip() and not line.lstrip().startswith("#")]
    assert len(pairs) == len(dict(pairs)) and dict(pairs) == expected_ap
    r["protected_config_pins"] = {str(profile_path):pin(profile_path),str(config_path):pin(config_path)}
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
    r["inactive_before"] = inactive(S["conflicting_service_units"], "inactive-before")
    r["preinstalled_frozen_closure_verified"] = True
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
    assert [(role["name"],role["addr"]) for role in initial] == [("wlan0","02:00:00:00:00:00"),("wlan1","02:00:00:00:01:00")]
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
            cutoff=keeper_ready_until, wait_cutoff=C, env=CHILD_ENV, identity={"pass_fds": (namespace_other.fileno(),)})
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
    assert [(role["name"],role["addr"]) for role in initial] == [("wlan0","02:00:00:00:00:00"),("wlan1","02:00:00:00:01:00")]
    r["roles_before"] = initial
    r["AP_namespace_move_qualified"] = True
    r["AP_namespace_entry_argv_prefix"] = ap_prefix
    run(["/usr/bin/ip", "link", "set", "dev", scan_role["name"], "up"], "scan-interface-up")
    ssid = "pkl-two-boot-sol1"
    private = R / "ap-control"
    private.mkdir(mode=0o700)
    assert config_path == Path("/etc/pokelinux/two-boot-hostapd.conf")
    r["AP_configuration"] = {"SSID":ssid,"BSSID":ap_role["addr"],"frequency_MHz":2412,
        "config":pin(config_path),"open_auth_only":True,"fixed_protected_config_not_rewritten":True}
    ap = scope.start(in_ap(["/usr/bin/hostapd", str(config_path)]), "owned-foreground-hostapd.log",
                     262144, cutoff=W, wait_cutoff=W, env=CHILD_ENV)
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
    assert not stop_requested
    provision_until = min(W, now() + 1_000_000_000)
    ap_ns = retained_AP_namespace()
    peer_path = "/proc/" + str(namespace_keeper["child"].pid) + "/ns/net"
    actual = json.loads(run(["/usr/bin/python3","-B",str(I/"source/wifi_peer.py"),"provision",
        peer_path,str(provision_until),ap_role["name"],ap_role["ifindex"],str(ap_ns["dev"]),str(ap_ns["ino"])],
        "owned-AP-fixed-peer-provisioner", seconds=1, limit=32768),
        object_pairs_hook=consumer.unique, parse_constant=consumer.no_constant)
    assert actual["passed"] and actual["peer_index"] == int(ap_role["ifindex"])
    assert actual["addresses"] == [{"family":2,"prefix":30,"ifindex":int(ap_role["ifindex"]),"local":"192.0.2.2"}]
    r["AP_fixed_peer"] = actual
    status_body = run_ap(["/usr/bin/hostapd_cli","-p",str(private),"-i","wlan0","status"],"actual-fixed-AP-status")
    pairs = [line.split("=",1) for line in status_body.splitlines() if "=" in line]
    assert len(pairs) == len(dict(pairs))
    status = dict(pairs)
    assert status["state"] == "ENABLED" and status["freq"] == "2412"
    assert status["ssid[0]"] == ssid and status["bssid[0]"] == "02:00:00:00:00:00" and status["num_sta[0]"] == "0"
    assert run(["/usr/bin/iw","dev","wlan1","link"],"station-disconnected-before-READY").strip() == "Not connected."
    assert run_ap(["/usr/bin/iw","dev","wlan0","station","dump"],"AP-no-stations-before-READY").strip() == ""
    no_addresses("wlan1","station-unconfigured-before-READY")
    loopback = json.loads(run(["/usr/bin/ip","-j","address","show","dev","lo"],"STA-loopback-before-installed-services"))
    assert len(loopback) == 1 and loopback[0]["ifname"] == "lo"
    r["STA_loopback_before_service"] = {"ifindex":loopback[0]["ifindex"],"ifname":"lo",
        "addr_info":sorted(loopback[0]["addr_info"],key=lambda value:json.dumps(value,sort_keys=True))}
    notify_ready()
    provisioning = False
    while not stop_requested:
        assert now() < W, "original work cutoff; no automatic Stop/timeout extension"
        scope.pump(); scope.check()
        assert scope.live(ap) and scope.live(namespace_keeper)
        namespace_binding()
        time.sleep(.005)
    properties = unit_properties("pokelinux-hwsim-provision.service","actual-PID1-normal-Stop")
    assert properties["MainPID"] == str(os.getpid()) and properties["InvocationID"] == r["InvocationID"]
    assert properties["ActiveState"] == "deactivating"
    r["normal_PID1_SIGTERM_Stop_observed"] = properties
    installed_dependents_stopped()
    assert run(["/usr/bin/iw","dev","wlan1","link"],"station-disconnected-after-installed-Stop").strip() == "Not connected."
    after = radio_inventory("same-split-radio-epoch-after-installed-Stop")
    assert [identity(value) for value in after] == [identity(value) for value in initial]
    assert namespace() == r["retained_namespace_epoch"]
    no_addresses("wlan1","station-unconfigured-after-installed-Stop")
    body = json.loads(run_ap(["/usr/bin/ip","-j","address","show","dev","wlan0"],"AP-profile-retained-until-owned-teardown"))
    assert len(body) == 1 and body[0]["ifindex"] == int(ap_role["ifindex"])
    assert len(body[0]["addr_info"]) == 1 and body[0]["addr_info"][0]["local"] == "192.0.2.2" and body[0]["addr_info"][0]["prefixlen"] == 30
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
    station_after = json.loads(run(["/usr/bin/ip", "-j", "address", "show"], "STA-lo-only-after-owned-module-removal"))
    assert len(station_after) == 1 and station_after[0]["ifname"] == "lo"
    loopback_after = {"ifindex": station_after[0]["ifindex"], "ifname": "lo",
        "addr_info": sorted(station_after[0]["addr_info"], key=lambda value: json.dumps(value, sort_keys=True))}
    assert loopback_after == r["STA_loopback_before_service"]
    assert all(value["local"] not in ["192.0.2.1", "192.0.2.2"] for value in loopback_after["addr_info"])
    r["STA_loopback_preserved_after_radio_removal"] = True
    r["STA_namespace_lo_only_after_radio_removal"] = station_after
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
    r["inactive_after"] = inactive(S["conflicting_service_units"],"inactive-after")
    for path, row in r["protected_config_pins"].items():
        assert pin(Path(path)) == row
    r["fixture_contract_passed"] = True

except BaseException as error:
    r.update(error=repr(error), traceback=traceback.format_exc())
finally:
    r["cleanup_errors"] = scope.close(C)
    failure_cleanup = Scope(R)
    try:
        if r.get("radio_create_command_admitted") and Path("/sys/module/mac80211_hwsim").exists():
            assert all(v["wait"] is not None and v["eof"] and v["log"].closed and v["child"].stdout.closed and v["pidfd"] is None for v in scope.children), "radio cleanup fenced by unretired worker"
            # Do not remove owned radios underneath a still-running installed provider.
            installed_dependents_stopped(failure_cleanup)
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
    r["passed"] = bool(r.get("fixture_contract_passed") and r.get("retained_AP_namespace_custody_qualified") and r.get("READY_sent") and r.get("normal_PID1_SIGTERM_Stop_observed") and r.get("retained_namespace_custody_qualified") and not r.get("failure_owned_radio_cleanup_error") and not r["failure_radio_cleanup_errors"] and r["all_commands_actual_wait_EOF"]
        and r["all_command_handles_closed"] and not r["cleanup_errors"]
        and r["ECHILD"] and r["adopted_terminal_closure_qualified"] and now() < C)
    r["completed_ns"] = now()
    publish_record(R / "result.json", r, C)
    print(json.dumps({"provisioning_only_closed":r["passed"],"error":r.get("error")}),flush=True)

raise SystemExit(0 if r["passed"] else 1)
