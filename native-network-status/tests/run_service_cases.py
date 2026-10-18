#!/usr/bin/env python3
"""Run the four network-status boundary cases in an isolated root namespace."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import sys
import traceback

sys.dont_write_bytecode = True
REPOSITORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY / "tools/native-installed-services"))
from owned import Scope, now
from service_cases import one_case


def pin(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--normal", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    normal = args.normal.resolve(strict=True)
    fixture = args.fixture.resolve(strict=True)
    assert normal.is_file() and fixture.is_file()
    output = args.output.resolve()
    output.mkdir(mode=0o700)  # A fresh output directory; never overwrite evidence.
    commands = output / "commands"
    commands.mkdir(mode=0o700)
    cases = output / "cases"
    cases.mkdir(mode=0o700)
    started = now()
    work_until = started + 30_000_000_000
    close_until = started + 36_000_000_000
    scope = Scope(commands)
    result = {
        "schema": 1, "passed": False, "groups": [],
        "scope": "Isolated onlyloopback normal service plus controlled Reader lifecycle; no TUN/Internet/installed-service/physical claim.",
        "original_work_until_ns": work_until,
        "original_close_until_ns": close_until,
    }
    inputs = {"normal": normal, "fixture": fixture,
              "service_cases": Path(__file__).with_name("service_cases.py"),
              "owned_helper": REPOSITORY / "tools/native-installed-services/owned.py"}
    try:
        assert os.getuid() == os.geteuid() == os.getgid() == os.getegid() == 0
        assert "CapEff:\t0000000000000000" in Path("/proc/self/status").read_text()
        names = socket.if_nameindex()
        proc_names = [line.split(":", 1)[0].strip()
                      for line in Path("/proc/net/dev").read_text().splitlines()
                      if ":" in line]
        assert names == [(1, "lo")] and proc_names == ["lo"], "isolated onlyloopback namespace required"
        namespace = os.stat("/proc/thread-self/ns/net")
        result["independent_kernel_prerequisites"] = {
            "if_nameindex": names, "proc_net_dev_names": proc_names,
            "namespace_dev": namespace.st_dev, "namespace_ino": namespace.st_ino,
            "effective_capabilities": 0, "network_mutations": False,
        }
        result["input_pre"] = {name: pin(path) for name, path in inputs.items()}
        for name in ("normal_kernel_empty", "wait_and_EOF_before_publication",
                     "held_collector_Stop", "queued_success_after_original_deadline"):
            # The unchanged case has its own original six-second cap.
            assert now() + 7_000_000_000 < work_until, "original global case-admission cutoff"
            row = one_case(scope, normal, fixture, cases, name)
            result["groups"].append(row)
            assert "setup_or_custody_failure" not in row and "closure_failure" not in row, row
        result["all_four_cases_passed"] = len(result["groups"]) == 4 and all(row["passed"] for row in result["groups"])
        assert result["all_four_cases_passed"], "frozen service requirements failed"
    except BaseException as error:
        result["error"] = type(error).__name__ + ": " + str(error)
        result["traceback"] = traceback.format_exc()
    finally:
        result["cleanup_errors"] = scope.close(close_until)
        result["owned_process_receipts"] = scope.receipts
        result["input_post"] = {name: pin(path) for name, path in inputs.items()}
        result["inputs_unchanged"] = result.get("input_pre") == result["input_post"]
        result["completed_ns"] = now()
        result["passed"] = bool(
            result.get("all_four_cases_passed") and not result["cleanup_errors"]
            and result["inputs_unchanged"] and len(scope.children) == len(scope.receipts)
            and all(row["actual_direct_parent_wait"] and row["stdout_EOF"]
                    and row["wait_before_original_cutoff"] and row["EOF_before_original_cutoff"]
                    and not row["output_overflow"] for row in scope.receipts)
            and now() < close_until)
        (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"passed": result["passed"], "groups": len(result["groups"]),
                      "error": result.get("error")}), flush=True)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
