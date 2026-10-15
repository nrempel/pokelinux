#!/usr/bin/env python3
"""Two fixed natural-terminal acceptance cases; same GREEN oracles for every source."""
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import tempfile
import time
import controller as base

MODES = ("natural-before-commit", "terminal-after-commit")


def run_case(build, out, mode, contract, request, response, expected):
    until = base.MONO() + base.CASE_NS
    work_until = until - base.CLEANUP_NS
    case = out / mode
    case.mkdir(mode=0o700)
    runtime = Path(tempfile.mkdtemp(prefix="sit-natural-", dir="/tmp"))
    os.chmod(runtime, 0o700)
    peer = runtime / "peer.sock"
    exe, before, binding = base.native_inputs(build, contract)
    uid, gid, controller = os.getuid(), os.getgid(), os.getpid()
    argv = [str(exe), "--runtime", str(runtime), "--peer", str(peer), "--case", mode,
            "--peer-pid", str(controller)]
    rec = {"case": mode, "passed": False, "failure": None,
           "classification": "SETUP_OR_SCHEDULE_FAILURE", "argv": argv,
           "inputs_before": before, "artifact_binding": binding,
           "original_outer_until_ns": until}
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    listener.setblocking(False)
    listener.bind(str(peer))
    listener.listen(1)
    p = pipes = identity = None
    try:
        base.require(base.child_probe() == {"ECHILD": True}, "fresh controller child state")
        p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, start_new_session=True,
                             env={"PATH": "/usr/bin:/bin", "LANG": "C"})
        pipes = base.Pipes(p, listener)
        pipes.until(lambda: pipes.request is not None, work_until)
        base.require(pipes.request == request, "exact original983-byte request")
        worker = base.proc_capture(pipes.cred["pid"], with_maps=True)
        identity = worker["stat_fields"]
        base.require(identity["ppid"] == p.pid and identity["pgrp"] == identity["session"] == p.pid,
                     "worker is actual owned child/group/session")
        base.require(pipes.cred["uid"] == uid and pipes.cred["gid"] == gid,
                     "fixture credentials match")
        rec["worker_before_response"] = worker
        journal = runtime / "admission"
        journal_before = base.pin(journal)
        rec["journal_before"] = journal_before
        while True:
            try:
                base.require(pipes.conn.send(response) == 144, "single144-byte response send")
                break
            except BlockingIOError:
                pipes.pump(work_until)
        pipes.until(lambda: any(x["event"] == "snapshots" for x in pipes.rows), work_until)
        base.require([x["event"] for x in pipes.rows] == ["dispatch-unknown", "prepared", "snapshots"],
                     "fixed precommit event order")
        dispatch, prepared, snapshots = pipes.rows
        source, deadline = dispatch["incarnation"], dispatch["until_ns"]
        base.require(re.fullmatch("[0-9a-f]{32}", source) is not None, "source incarnation")
        base.require(dispatch["worker_pid"] == pipes.cred["pid"] and dispatch["request_bytes"] == 983,
                     "actual source and request binding")
        worker_argv = [str(exe), "--pokelinux-process-worker", "--peer", str(peer),
                       "--packet", request.hex(), "--peer-pid", str(controller),
                       "--until-ns", str(deadline)]
        if mode == "natural-before-commit":
            worker_argv.append("--return-after-response")
        base.require_proc(worker, worker_argv, p.pid, p.pid, uid, gid)
        base.require(prepared["actual_source"] == source and bytes.fromhex(prepared["record_hex"]) == response
                     and prepared["report"] == expected and prepared["peer_pid_observed"] == controller,
                     "full source/144-byte record/14-field report comparison")
        base.require(prepared["prepared_at_ns"] < prepared["original_until_ns"] == deadline,
                     "prepared before original deadline")
        base.require(snapshots["count"] == 100 and snapshots["elapsed_ns"] < 200_000_000,
                     "100 responsive precommit snapshots")
        if mode == "natural-before-commit":
            cutoff = min(work_until, base.MONO() + 1_000_000_000)
            while True:
                status = (Path("/proc") / str(pipes.cred["pid"]) / "status").read_text()
                if base.status_map(status)["State"].startswith("Z"):
                    break
                base.require(base.MONO() < cutoff, "worker did not naturally exit before commit")
                time.sleep(.001)
            rec["actual_worker_zombie_before_release"] = status
            pipes.conn.settimeout(base.remaining(cutoff))
            base.require(pipes.conn.recv(1) == b"", "natural worker peer EOF before Main release")
            rec["peer_EOF_before_commit"] = True
        else:
            cutoff = min(work_until, base.MONO() + 200_000_000)
            while True:
                held = base.proc_capture(pipes.cred["pid"])
                if base.status_map(held["status"])["State"].startswith("S") and re.search("skb|unix|sock", held["wchan"]):
                    break
                base.require(base.MONO() < cutoff, "worker not in actual postresponse receive")
                time.sleep(.001)
            rec["actual_worker_blocked_before_commit"] = held
        base.require(base.BOOT() < deadline, "schedule consumed original deadline")
        p.stdin.write(b"G")
        p.stdin.flush()
        p.stdin.close()
        p.stdin = None
        if mode == "terminal-after-commit":
            pipes.until(lambda: any(x["event"] == "commit" for x in pipes.rows), work_until)
            commit = next(x for x in pipes.rows if x["event"] == "commit")
            base.require(commit["completion_present"] and commit["history_present"]
                         and commit["withdrawal"] == "None", "actual live commit before terminal trigger")
            pipes.conn.shutdown(socket.SHUT_WR)
            rec["actual_terminal_trigger"] = "peer SHUT_WR after live commit"
            rec["terminal_trigger_boottime_ns"] = base.BOOT()
        pipes.finish(work_until)
        code = p.wait(timeout=base.remaining(work_until))
        rec.update(actual_parent_wait_exit=code, stdout_EOF=pipes.eof["stdout"], stderr_EOF=pipes.eof["stderr"])
        rec["journal_after"] = base.pin(journal)
        rec["journal_bytes_hex"] = journal.read_bytes().hex()
        base.require(rec["journal_after"] == journal_before, "same durable journal unchanged")
        rows = pipes.rows
        observed = [x for x in rows if x["event"] == "natural-wait"]
        withdrawn = [x for x in rows if x["event"] == "withdrawn"]
        want_code = 0 if mode == "natural-before-commit" else 70
        base.require(len(observed) == len(withdrawn) == 1, "actual natural wait and withdrawal events")
        wait, loss = observed[0], withdrawn[0]
        base.require(wait["actual_wait"] and wait["worker_code"] == want_code and wait["worker_signal"] is None
                     and not wait["occupied"] and not wait["admitted"], "exact natural worker wait/status/admission")
        base.require(loss["disposition"] == "OutcomeUnknown" and loss["withdrawal"] == "Some(SourceLost)"
                     and not loss["current_present"] and loss["same_history"]
                     and loss["snapshot_count"] == 100 and loss["snapshot_elapsed_ns"] < 200_000_000,
                     "withdrawal and responsive historical snapshots")
        if mode == "natural-before-commit":
            base.require(not loss["history_present"] and loss["history_source"] is None
                         and loss["history_record_hex"] is None and loss["history_report"] is None,
                         "no history from uncommitted candidate")
        else:
            base.require(loss["history_present"] and loss["history_source"] == source
                         and bytes.fromhex(loss["history_record_hex"]) == response and loss["history_report"] == expected,
                         "exact committed history retained")
        rec["natural_wait_and_withdrawal_oracles_passed"] = True
        rec["final_children"] = base.child_probe()
        base.require(rec["final_children"] == {"ECHILD": True}, "Main reaped its worker without adoption")
        base.require(rows[-1].get("event") != "cleanup-failure", "Main discarded natural wait then cleanup timed out")
        base.require(code == 0, "Main must finish0 after accepting actual natural wait")
        wanted = ["dispatch-unknown", "prepared", "snapshots"]
        wanted += ["natural-wait", "commit"] if mode == "natural-before-commit" else ["commit", "natural-wait"]
        wanted += ["withdrawn", "worker-wait", "successor-refused"]
        base.require([x["event"] for x in rows] == wanted, "fixed complete natural event order")
        receipt = rows[-2]
        base.require(receipt["already_reaped"] and receipt["worker_code"] == want_code and receipt["worker_signal"] is None
                     and receipt["waited_at_ns"] == wait["waited_at_ns"]
                     and receipt["waited_at_ns"] <= receipt["stop_started_ns"]
                     and receipt["stop_elapsed_ns"] < 200_000_000
                     and not receipt["occupied"] and not receipt["admitted"], "same actual wait receipt and responsive Stop")
        base.require(rows[-1] == {"event":"successor-refused", "reason":"AdmissionUncertain", "occupied":False},
                     "same-journal successor refusal")
        expected_stderr = b"" if want_code == 0 else b"fixture worker: ZeroRead\n"
        base.require(bytes(pipes.raw["stderr"]) == expected_stderr, "exact natural terminal diagnostic")
        if mode == "terminal-after-commit":
            pipes.conn.settimeout(base.remaining(work_until))
            base.require(pipes.conn.recv(1) == b"", "terminal peer EOF/no extra request")
        try:
            other, _ = listener.accept()
        except BlockingIOError:
            pass
        else:
            other.close()
            raise RuntimeError("unexpected successor connection")
        rec.update(passed=True, classification="QUALIFIED_NATURAL_TERMINAL_CASE")
    except BaseException as error:
        rec["failure"] = type(error).__name__ + ": " + str(error)
        if rec.get("natural_wait_and_withdrawal_oracles_passed") and rec.get("actual_parent_wait_exit") == 70:
            if pipes and pipes.rows[-1].get("event") == "cleanup-failure" and "TimedOut" in pipes.rows[-1].get("error", ""):
                rec["classification"] = "BEHAVIORAL_MAIN_NATURAL_WAIT_DISCARDED"
        if p and p.poll() is None:
            rec["failure_cleanup"] = base.emergency_cleanup(p, pipes, until, identity)
    finally:
        if pipes:
            rec["events"] = pipes.rows
            rec["pipe_EOF_actual"] = pipes.eof
            for name, data in pipes.raw.items():
                (case / (name + ".raw")).write_bytes(bytes(data))
            pipes.close()
        else:
            listener.close()
        if p and p.stdin:
            p.stdin.close()
        rec["inputs_after"] = [base.pin(x["path"]) for x in before]
        rec["inputs_unchanged"] = rec["inputs_after"] == before
        rec["completed_ns"] = base.MONO()
        base.require(rec["inputs_unchanged"] and rec["completed_ns"] < until, "input pins and original outer cutoff")
        base.write_json(case / "result.json", rec)
    return rec


def main():
    base.require(len(sys.argv) == 3, "usage: natural_controller.py BUILD ABS_NEW_OUTPUT")
    build, out = Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).absolute()
    out.mkdir(mode=0o700)
    contract = json.loads((base.HERE / "contract.json").read_text())
    request, response, expected = base.reference(contract)
    sub = base.subreaper_enable()
    results = [run_case(build, out, mode, contract, request, response, expected) for mode in MODES]
    final = base.child_probe()
    passed = all(r["passed"] for r in results) and final == {"ECHILD": True}
    base.write_json(out / "result.json", {"scope":"Fixture natural-terminal integration; no production CPIF authority",
                    "passed":passed, "cases":results, "final_children":final, "subreaper":sub,
                    "controller_pin":base.pin(__file__)})
    print(json.dumps({"passed":passed, "cases":[{"case":r["case"], "passed":r["passed"],
          "classification":r["classification"], "failure":r["failure"]} for r in results]}), flush=True)
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
