"""Frozen desired one-shot scan oracle; caller supplies actual owned-process evidence.

No process, socket, AP, namespace, clock or filesystem operation occurs here.
Persist the observation, perform owned cleanup, then qualify and assert once.
"""
import json
import re

ACTION_NS = 5_000_000_000
MAX_WIRE = 32768
MAX_BSS = 64
U32_MAX = (1 << 32) - 1
U64_MAX = (1 << 64) - 1
DESIRED_MARKER = "WIRELESS_SCAN_MUST_COMPLETE_ACTUAL_AP_DISCOVERY"


def need(condition, detail):
    if not condition:
        raise ValueError(detail)


def integer(value, low, high, label):
    need(type(value) is int and low <= value <= high, label)
    return value


def unique(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def no_constant(value):
    raise ValueError("non-finite JSON constant")


def observe_worker(returncode, stdout, receipt, *, armed_at_ns,
                   pre_spawn_admission_ns, scan_role, ap):
    """Classify only one actual post-wait/EOF worker result; never assert desired yet.

    scan_role: actual trusted {ifindex, wiphy, wdev, frequency_mhz}, all integers.
    ap: actual foreground AP {bssid_hex, ssid_hex, frequency_mhz}.
    receipt: canonical owned.Scope.finish receipt for this exact worker only.
    """
    result = {"schema": 1, "desired_passed": False,
              "known_refusing_baseline": False, "outcome_confirmed": False,
              "classification": "UNCONFIRMED_WORKER_OUTCOME"}
    try:
        integer(returncode, -255, 255, "actual worker returncode")
        need(type(stdout) is bytes and 0 < len(stdout) <= MAX_WIRE, "wire byte bound")
        armed = integer(armed_at_ns, 1, U64_MAX - ACTION_NS, "original armed BOOTTIME")
        until = armed + ACTION_NS
        admission = integer(pre_spawn_admission_ns, armed, until - 1, "pre-spawn admission")
        need(type(scan_role) is dict and set(scan_role) ==
             {"ifindex", "wiphy", "wdev", "frequency_mhz"}, "actual scan-role fields")
        ifindex = integer(scan_role["ifindex"], 1, U32_MAX, "actual ifindex")
        phy = integer(scan_role["wiphy"], 0, U32_MAX, "actual wiphy")
        wdev = integer(scan_role["wdev"], 1, U64_MAX, "actual wdev")
        frequency = integer(scan_role["frequency_mhz"], 1, U32_MAX, "actual requested frequency")
        need(type(ap) is dict and set(ap) ==
             {"bssid_hex", "ssid_hex", "frequency_mhz"}, "actual AP fields")
        need(type(ap["bssid_hex"]) is str and
             re.fullmatch("[0-9a-f]{12}", ap["bssid_hex"]) is not None, "actual AP BSSID")
        need(type(ap["ssid_hex"]) is str and
             re.fullmatch("(?:[0-9a-f]{2}){1,32}", ap["ssid_hex"]) is not None, "actual AP nonce SSID")
        need(type(ap["frequency_mhz"]) is int and ap["frequency_mhz"] == frequency,
             "AP and selected scan frequency differ")
        need(type(receipt) is dict, "actual owned worker receipt required")
        for field in ["actual_direct_parent_wait", "stdout_EOF",
                      "wait_before_original_cutoff", "EOF_before_original_cutoff"]:
            need(receipt.get(field) is True, "unqualified actual custody: " + field)
        need(type(receipt.get("returncode")) is int and receipt["returncode"] == returncode,
             "returncode/receipt mismatch")
        need(type(receipt.get("original_cutoff_ns")) is int and
             receipt["original_cutoff_ns"] == until, "worker cutoff renewed or mismatched")
        need(type(receipt.get("spawn_admission_cutoff_ns")) is int and
             receipt["spawn_admission_cutoff_ns"] == until, "missing original pre-spawn fence")
        waited = integer(receipt.get("waited_at_boottime_ns"), admission, until - 1,
                         "actual wait BOOTTIME")
        eof = integer(receipt.get("stdout_EOF_at_ns"), admission, until - 1,
                      "actual EOF BOOTTIME")
        need(type(receipt.get("stdout_bytes")) is int and
             receipt["stdout_bytes"] == len(stdout), "wire/receipt bytes mismatch")
        need(receipt.get("output_overflow") is False, "owned output overflow")
        need(type(receipt.get("stdout_limit")) is int and
             0 < receipt["stdout_limit"] <= MAX_WIRE, "owned capture bound")
        value = json.loads(stdout.decode("utf-8"), object_pairs_hook=unique,
                           parse_constant=no_constant)
        need(type(value) is dict, "one JSON object required")
        common = {"schema", "status", "ifindex", "frequency_mhz", "original_until_ns"}
        need(type(value.get("schema")) is int and value["schema"] == 1, "schema1")
        need(type(value.get("ifindex")) is int and value["ifindex"] == ifindex,
             "wrong scan ifindex")
        need(type(value.get("frequency_mhz")) is int and
             value["frequency_mhz"] == frequency, "wrong requested frequency")
        need(type(value.get("original_until_ns")) is int and
             value["original_until_ns"] == until, "wrong original cutoff")
        result.update(actual_returncode=returncode, wire_bytes=len(stdout),
                      original_armed_at_ns=armed, original_until_ns=until,
                      pre_spawn_admission_ns=admission, actual_wait_ns=waited,
                      actual_EOF_ns=eof, actual_scan_role=dict(scan_role), actual_AP=dict(ap))
        if returncode == 2:
            need(set(value) == common | {"reason"} and value["status"] == "unavailable"
                 and value["reason"] == "unsupported", "unexpected nonzero worker outcome")
            result.update(known_refusing_baseline=True, outcome_confirmed=True,
                          classification="CONFIRMED_REFUSING_BASELINE", actual_result=value)
            return result
        need(returncode == 0, "unexpected actual worker exit")
        need(set(value) == common | {"wiphy", "wdev", "started_boottime_ns",
             "completed_boottime_ns", "bss"} and value["status"] == "completed",
             "completed result shape/status")
        need(type(value["wiphy"]) is int and value["wiphy"] == phy, "wrong wiphy")
        need(type(value["wdev"]) is int and value["wdev"] == wdev, "wrong wdev")
        started = integer(value["started_boottime_ns"], admission, until - 1,
                          "actual operation start BOOTTIME")
        completed = integer(value["completed_boottime_ns"], started, until - 1,
                            "actual operation completed BOOTTIME")
        need(completed <= waited and completed <= eof, "result timestamp after actual wait/EOF")
        need(type(value["bss"]) is list and len(value["bss"]) <= MAX_BSS, "BSS bound")
        need(len(value["bss"]) == 1, "exactly one actual AP BSS required")
        bss = value["bss"][0]
        need(type(bss) is dict and set(bss) ==
             {"bssid_hex", "ssid_hex", "frequency_mhz", "last_seen_boottime_ns"},
             "exact BSS fields")
        need(type(bss["bssid_hex"]) is str and bss["bssid_hex"] == ap["bssid_hex"],
             "actual AP BSSID mismatch")
        need(type(bss["ssid_hex"]) is str and bss["ssid_hex"] == ap["ssid_hex"],
             "actual nonce SSID hex mismatch")
        need(type(bss["frequency_mhz"]) is int and bss["frequency_mhz"] == frequency,
             "actual BSS frequency mismatch")
        integer(bss["last_seen_boottime_ns"], started, completed,
                "kernel BSS last-seen outside original operation span")
        result.update(desired_passed=True, outcome_confirmed=True,
                      classification="CONFIRMED_COMPLETED_ACTUAL_AP_SCAN", actual_result=value)
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError) as error:
        result["unconfirmed_detail"] = str(error)
    return result


def qualify_after_cleanup(observation, *, fixture_qualified,
                          worker_loader_qualified, cleanup_qualified):
    """Caller sets these only from actual fixture/loader/owned cleanup records.

    Cleanup includes worker wait+EOF, foreground AP wait+EOF, module/radio and
    control-endpoint cleanup, and retained namespace custody/closure. Never use
    inventories/names alone as exclusive-authority proof. No cleanup runs here.
    """
    ready = (fixture_qualified is True and worker_loader_qualified is True
             and cleanup_qualified is True)
    confirmed = observation.get("outcome_confirmed") is True
    return {"schema": 1, "passed": ready and confirmed and observation.get("desired_passed") is True,
            "behavior_RED_qualified": ready and confirmed
                and observation.get("known_refusing_baseline") is True,
            "prerequisites_and_cleanup_qualified": ready,
            "worker_outcome_confirmed": confirmed, "observation": observation}


def assert_desired_after_cleanup(review):
    """Call only after persisting review and actual cleanup receipts; same for GREEN."""
    if review.get("prerequisites_and_cleanup_qualified") is not True:
        raise RuntimeError("WIRELESS_SCAN_PREREQUISITE_OR_CLEANUP_UNQUALIFIED")
    if review.get("worker_outcome_confirmed") is not True:
        raise RuntimeError("WIRELESS_SCAN_UNCONFIRMED_WORKER_OUTCOME")
    if review.get("passed") is not True:
        raise AssertionError(DESIRED_MARKER)
