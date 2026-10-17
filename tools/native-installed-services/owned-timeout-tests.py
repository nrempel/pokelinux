#!/usr/bin/env python3
"""Focused real-child cleanup regression; host-safe, with no services or VM.

Run with a fresh --output-dir. Original command timeout remains a failure even
when the distinct, preselected cleanup budget obtains complete process custody.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import sys
import traceback

sys.dont_write_bytecode = True
from owned import Scope, now

STDOUT = b"stdout-ready\n"
STDERR = b"stderr-ready\n"
LEAF = (
    "import os,time; "
    "os.write(1,b'stdout-ready\\n'); "
    "os.write(2,b'stderr-ready\\n'); "
    "time.sleep(30)"  # The timeout cases must be killed by their retained owner.
)


def exercise(root, regular, overdue, overall_until, shorter_finish=False):
    label = ("regular" if regular else "merged") + ("-timeout" if overdue else "-timely")
    if shorter_finish:
        label = "regular-shorter-finish-timeout"
    output = root / label
    output.mkdir(mode=0o700)
    started = now()
    original = started + (80_000_000 if overdue else 2_000_000_000)
    wait_until = started + 2_000_000_000 if shorter_finish else original
    cleanup_until = min(overall_until, started + 3_000_000_000)
    scope = Scope(output)
    item = None
    result = {"case": label, "passed": False, "started_boottime_ns": started,
              "original_command_until_ns": original, "original_start_wait_until_ns": wait_until,
              "original_cleanup_until_ns": cleanup_until,
              "overdue_case_functional_success_claimed": False}
    failures = []
    try:
        code = LEAF if overdue else LEAF.replace("time.sleep(30)", "time.sleep(0)")
        item = scope.start([sys.executable, "-c", code], "stdout.log", 4096, cutoff=original, wait_cutoff=wait_until,
                           stderr_regular=output / "stderr.log" if regular else None)
        result["exact_child_pid"] = item["child"].pid
        primary = None
        try:
            returned = scope.finish(item, original)
            if overdue:
                failures.append("overdue command incorrectly returned success")
            else:
                assert returned == 0
        except AssertionError as error:
            primary = str(error)
            if not overdue:
                failures.append("timely command refused: " + primary)
        result["original_finish_error"] = primary
        if overdue:
            assert primary and now() >= original, "actual original command timeout required"
        result["close_errors"] = scope.close(cleanup_until)
        result["Scope_receipts_before_any_rescue"] = list(scope.receipts)
        result["stdout_handle_closed"] = item["child"].stdout.closed
        result["stdout_log_closed"] = item["log"].closed
        result["stderr_handle_closed"] = regular and item["stderr_file"].closed
        rows = [row for row in scope.receipts if row["pid"] == item["child"].pid]
        assert len(rows) == 1, "cleanup must record exactly one actual same-parent wait"
        row = rows[0]
        assert row["actual_direct_parent_wait"] and row["stdout_EOF"], "cleanup must actually wait and drain PIPE EOF"
        assert row["original_cutoff_ns"] == original
        assert row["original_wait_cutoff_ns"] == wait_until
        assert row["wait_before_original_cutoff"] is (not overdue), "cleanup renewed original wait qualification"
        assert row["EOF_before_original_cutoff"] is (not overdue), "cleanup renewed original EOF qualification"
        assert row["waited_at_boottime_ns"] < cleanup_until and row["stdout_EOF_at_ns"] < cleanup_until
        assert row["returncode"] == (-signal.SIGKILL if overdue else 0)
        assert result["stdout_handle_closed"] and result["stdout_log_closed"]
        assert (output / "stdout.log").read_bytes() == (STDOUT if regular else STDOUT + STDERR)
        if overdue:
            assert result["close_errors"], "cleanup must preserve the original timeout failure"
        else:
            assert not result["close_errors"]
        if regular:
            stderr = (output / "stderr.log").read_bytes()
            assert stderr == STDERR and row["stderr_mode"] == "regular_file"
            assert row["stderr_regular_sha256"] == hashlib.sha256(STDERR).hexdigest()
            assert row["stderr_regular_bytes"] == len(STDERR)
            assert row["stderr_regular_descriptor_closed"] and result["stderr_handle_closed"]
            assert row["stderr_regular_closed_after_direct_wait"]
            assert row["stderr_regular_closed_at_ns"] < cleanup_until
            assert row["stderr_regular_close_before_original_cutoff"] is (not overdue)
            assert row["stderr_regular_is_PIPE"] is False
        else:
            assert row["stderr_mode"] == "merged_stdout_PIPE"
    except BaseException as error:
        failures.append(type(error).__name__ + ": " + str(error))
        result["failure_traceback"] = traceback.format_exc()
    finally:
        if item is not None:
            # RED-only disposal retains the exact Popen. It never supplies a
            # missing Scope receipt or claims EOF after a helper closed the PIPE.
            child = item["child"]
            if child.returncode is None:
                if child.poll() is None:
                    child.kill()
                returned = child.wait(timeout=max(.001, (cleanup_until - now()) / 1e9))
                result["RED_only_rescue_wait"] = {"pid": child.pid, "returncode": returned,
                                                "observed_ns": now()}
            try:
                os.waitpid(child.pid, os.WNOHANG)
            except ChildProcessError:
                result["same_parent_ECHILD"] = True
            else:
                result["same_parent_ECHILD"] = False
            result["proc_absent"] = not Path("/proc", str(child.pid)).exists()
            if not result["same_parent_ECHILD"] or not result["proc_absent"]:
                failures.append("retained child was not actually reaped")
            for stream in [item["log"], child.stdout, item["stderr_file"]]:
                if stream is not None and not stream.closed:
                    stream.close()
            if "RED_only_rescue_wait" in result:
                failures.append("Scope required a RED-only exact-Popen rescue wait")
        result["failures"] = failures
        result["completed_boottime_ns"] = now()
        result["passed"] = not failures and result["completed_boottime_ns"] < cleanup_until
        (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    args.output_dir.mkdir(mode=0o700)
    overall_until = now() + 12_000_000_000
    cases = [exercise(args.output_dir, regular, overdue, overall_until)
             for overdue in [True, False] for regular in [False, True]]
    cases.append(exercise(args.output_dir, True, True, overall_until, shorter_finish=True))
    summary = {"schema": 1, "passed": all(row["passed"] for row in cases),
               "scope": "Actual real-child public Scope cleanup only; no VM/service/power-supply-driver/phone execution",
               "original_overall_until_ns": overall_until, "completed_boottime_ns": now(),
               "helper_sha256": hashlib.sha256(Path(__file__).with_name("owned.py").read_bytes()).hexdigest(),
               "test_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "cases": cases}
    (args.output_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"passed": summary["passed"], "cases": {row["case"]: row["passed"] for row in cases}}), flush=True)
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
