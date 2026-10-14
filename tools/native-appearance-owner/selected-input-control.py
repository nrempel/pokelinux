#!/usr/bin/env python3
"""Own held-colors consistency control, with real ptrace/SEQPACKET boundaries.

Run only in the selected network-none x86 fixture with SYS_PTRACE. Imports the
SHA-selected public process-control.py. No attach, syscall/register rewriting,
replacement helpers or injected replies. The fixture root atomically exchanges
two UID1000-owned synthetic input files; it does not claim a UID1000 mutator.
"""
import argparse
import ctypes
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import queue
import signal
import socket
import struct
import threading
import time
import traceback


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def boot():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


BASE = Path(__file__).with_name("process-control.py")
spec = importlib.util.spec_from_file_location("appearance_owned_process_control", BASE)
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)


class HeldColors(original.Realm):
    def __init__(self, binary, result):
        self.key = "appearance"
        self.job = None
        self.sent_candidate = None
        self.received_candidate = None
        self.ack = None
        self.pidfd = None
        self.successor = None
        self.freeze_successors = False
        self.held_pids = set()
        self.successor_event = threading.Event()
        self.candidate_event = threading.Event()
        super().__init__(binary, "held-colors", result)

    def background(self, payload):
        value = payload[self.key]
        if value is None:
            return None
        return value["palette"]["background"]

    def launch(self):
        super().launch()
        # The owner's exec event is pending before the event loop resumes it.
        # Trace its syscalls from startup so every warmup child has an actual
        # parent wait4 observation, rather than just the selected target.
        self.active[self.owner]["syscalls"] = True

    def memory(self, pid, address, count):
        assert 0 <= count <= 4096
        return b"".join(self.read_word(pid, address + off).to_bytes(8, "little")
                        for off in range(0, count, 8))[:count]

    def syscall_info(self, pid):
        info = ctypes.create_string_buffer(88)
        original.ptrace(original.GET_SYSCALL_INFO, pid, 88,
                        ctypes.cast(info, ctypes.c_void_p))
        assert struct.unpack_from("I", info.raw, 4)[0] == 0xc000003e
        return info.raw

    def packet_after_recvmsg(self, pid, address, count):
        header = self.memory(pid, address, 56)
        iov, iov_count, control, control_length = struct.unpack_from("4Q", header, 16)
        flags = struct.unpack_from("I", header, 48)[0]
        assert iov_count == 1 and not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC)
        assert 0 < count <= 4096 and control_length <= 512
        buf, capacity = struct.unpack("2Q", self.memory(pid, iov, 16))
        assert count <= capacity
        body = json.loads(self.memory(pid, buf, count))
        ancillary = self.memory(pid, control, control_length)
        credentials = []
        offset = 0
        while offset + 16 <= len(ancillary):
            length, level, kind = struct.unpack_from("Qii", ancillary, offset)
            assert length >= 16 and offset + length <= len(ancillary)
            assert level == socket.SOL_SOCKET and kind == socket.SCM_CREDENTIALS and length == 28
            credentials.append(list(struct.unpack_from("3i", ancillary, offset + 16)))
            offset += (length + 7) & ~7
        assert len(credentials) == 1
        return {"pid": pid, "wire": body, "credentials": credentials[0],
                "count": count, "flags": flags, "boottime_ns": boot()}

    def on_syscall(self, pid, state):
        raw = self.syscall_info(pid)
        op = raw[0]
        if op == 1:
            nr = struct.unpack_from("Q", raw, 24)[0]
            args = list(struct.unpack_from("6Q", raw, 32))
            state["entry_args"] = args
            if nr == 44 and 0 < args[2] <= 4096:
                state["send_packet"] = json.loads(self.memory(pid, args[1], args[2]))
            else:
                state["send_packet"] = None
            if (state["role"] == "collector" and pid != self.target
                    and self.freeze_successors and nr in [59, 322]):
                assert self.successor is None, "multiple successor collectors"
                self.successor = pid
                state["last_nr"] = nr
                identity = self.identity(pid)
                assert int(identity["PPid"]) == self.owner
                self.result["successor_held_before_exec"] = {
                    "pid": pid, "syscall": nr, "identity": identity,
                    "fd_table": self.fd_table(pid), "boottime_ns": boot()}
                self.held_pids.add(pid)
                self.record("held_successor_before_exec", pid=pid, syscall=nr)
                self.successor_event.set()
                return True
        elif op == 2:
            nr = state["last_nr"]
            rval = struct.unpack_from("q", raw, 24)[0]
            args = state.get("entry_args", [])
            if nr == 434 and pid == self.owner and rval >= 0 and args[0] == self.target:
                info = Path(f"/proc/{pid}/fdinfo/{rval}").read_text()
                assert f"Pid:\t{self.target}\n" in info, info
                self.pidfd = {"owner_pid": pid, "child_pid": self.target,
                              "fd": rval, "fdinfo": info, "syscall": "pidfd_open",
                              "return": rval, "boottime_ns": boot()}
                self.result["actual_target_pidfd"] = self.pidfd
            if nr == 47 and rval > 0 and pid in [self.owner, self.target]:
                packet = self.packet_after_recvmsg(pid, args[1], rval)
                body = packet["wire"]
                if pid == self.target and set(body) == {"owner", "token", "until_ns", "config"}:
                    assert packet["credentials"] == [self.owner, 0, 0]
                    assert self.job is None
                    self.job = packet
                    self.result["actual_target_job"] = packet
                    self.record("actual_target_job_received", pid=pid, token=body["token"])
                if pid == self.owner and set(body) == {"owner", "token", "until_ns", self.key}:
                    if self.job and body["token"] == self.job["wire"]["token"]:
                        assert packet["credentials"] == [self.target, 1000, 1000]
                        assert body["owner"] == self.job["wire"]["owner"]
                        assert body["until_ns"] == self.job["wire"]["until_ns"]
                        self.received_candidate = packet
                        self.result["actual_owner_candidate_recvmsg"] = packet
                        self.candidate_event.set()
                        self.record("actual_owner_candidate_received", pid=pid, child=self.target)
            if nr == 44 and state.get("send_packet") is not None:
                body = state["send_packet"]
                if pid == self.target and set(body) == {"owner", "token", "until_ns", self.key}:
                    if rval == args[2]:
                        assert self.job and body["owner"] == self.job["wire"]["owner"]
                        assert body["token"] == self.job["wire"]["token"]
                        assert body["until_ns"] == self.job["wire"]["until_ns"]
                        self.sent_candidate = {"pid": pid, "wire": body, "bytes": rval,
                                               "syscall": "sendto", "boottime_ns": boot()}
                        self.result["actual_target_candidate_sendto"] = self.sent_candidate
                if pid == self.owner and self.job and body == {
                        "owner": self.job["wire"]["owner"], "token": self.job["wire"]["token"]}:
                    if rval == args[2]:
                        self.ack = {"pid": pid, "wire": body, "bytes": rval,
                                    "syscall": "sendto", "boottime_ns": boot()}
                        self.result["actual_owner_ack_sendto"] = self.ack
        held = super().on_syscall(pid, state)
        if pid == self.owner or pid == self.target or pid == self.successor:
            state["syscalls"] = True
        if op == 2 and state["last_nr"] == 317 and struct.unpack_from("q", raw, 24)[0] == 0:
            if pid == self.target:
                assert not self.held_event.is_set() and self.job and self.pidfd
                identity = self.identity(pid)
                assert identity["Uid"].split() == ["1000"] * 4
                assert identity["Gid"].split() == ["1000"] * 4
                assert not identity["Groups"]
                assert identity["NoNewPrivs"] == "1" and identity["Seccomp"] == "2"
                assert int(original.proc_status(pid)["CapEff"].strip(), 16) == 0
                selected = self.colors.stat()
                entries = self.fd_table(pid)
                colors = [e for e in entries if (e["dev"], e["ino"]) == (selected.st_dev, selected.st_ino)]
                assert len(colors) == 1 and "pos:\t0\n" in colors[0]["fdinfo"]
                self.result["actual_acquired_input_boundary"] = {
                    "pid": pid, "identity": identity, "CapEff": "0",
                    "successful_seccomp_return": True, "fd_table": entries,
                    "colors_fd": colors[0], "selected_colors": original.fingerprint(self.colors),
                    "generation": self.dir_identity(self.colors.parent), "boottime_ns": boot()}
                self.held_pids.add(pid)
                self.record("held_after_input_acquisition_before_retained_reads", pid=pid)
                self.held_event.set()
                return True
        return held

    def dir_identity(self, path):
        m = path.stat()
        return {"dev": m.st_dev, "ino": m.st_ino, "uid": m.st_uid, "gid": m.st_gid,
                "mode": oct(m.st_mode & 0o777)}

    def exchange(self):
        staged = self.colors.with_name("colors-retained-after-exchange.toml")
        body = ("background = '#faf4e8'\nforeground = '#292522'\naccent = '#8a5b1f'\n"
                "color1 = '#a83246'\ngreen = '#406524'\n")
        with staged.open("x") as f:
            f.write(body)
        os.chown(staged, 1000, 1000)
        staged.chmod(0o600)
        before = original.fingerprint(self.colors)
        new = original.fingerprint(staged)
        lock = self.runtime / "omarchy-theme-set.lock"
        lock_before = self.dir_identity(lock)
        assert self.colors.stat().st_nlink == 1 and staged.stat().st_nlink == 1
        fn = original.libc.renameat2
        fn.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        fn.restype = ctypes.c_int
        start = boot()
        if fn(-100, os.fsencode(staged), -100, os.fsencode(self.colors), 2) != 0:
            raise OSError(ctypes.get_errno(), "actual renameat2(RENAME_EXCHANGE)")
        end = boot()
        after = original.fingerprint(self.colors)
        retained = original.fingerprint(staged)
        assert (after["dev"], after["ino"]) == (new["dev"], new["ino"])
        assert (retained["dev"], retained["ino"]) == (before["dev"], before["ino"])
        assert after["ino"] != before["ino"] and after["sha256"] != before["sha256"]
        assert self.colors.stat().st_nlink == 1 and staged.stat().st_nlink == 1
        assert self.dir_identity(self.colors.parent) == self.result["actual_acquired_input_boundary"]["generation"]
        assert self.dir_identity(lock) == lock_before
        self.result["actual_replacement"] = {"syscall": "renameat2", "flags": "RENAME_EXCHANGE",
            "mutator_uid": os.getuid(), "mutator_gid": os.getgid(),
            "scope": "owned synthetic fixture-root mutation of UID1000-owned0600 files; no writer lock cooperation",
            "before": before, "new_selected": after, "old_retained_at": retained,
            "both_link_counts": [1, 1], "unchanged_generation": self.dir_identity(self.colors.parent),
            "unchanged_lock": lock_before, "started_boottime_ns": start, "finished_boottime_ns": end}
        assert end < self.job["wire"]["until_ns"], "fixture held input past original deadline"

    def tester(self):
        try:
            end = time.monotonic() + 5
            healthy = None
            while time.monotonic() < end:
                if self.socket.exists() and self.control.exists():
                    s = self.request(self.control, "status")
                    p = self.request(self.socket, "snapshot")
                    if (s["received"] and p["received"] and s["reply"]["completed"] > 0
                            and p["reply"]["schema"] == 3 and p["reply"]["available"]
                            and self.background(p["reply"]) == {"r": 6682, "g": 6939, "b": 9766, "a": 65535}
                            and self.warmup_uid_transitions and self.warmup_seccomp):
                        healthy = {"status": s, "snapshot": p}
                        break
                time.sleep(.003)
            assert healthy, "real selected-schema healthy prerequisite failed"
            self.result["healthy_prerequisites"] = healthy
            self.commands.put("arm")
            assert self.arm_event.wait(1) and self.held_event.wait(3)
            assert self.target and self.job and self.pidfd
            old_status = self.request(self.control, "status")
            old_snapshot = self.request(self.socket, "snapshot")
            assert old_status["received"] and old_snapshot["received"]
            assert old_status["reply"]["worker_pid"] == self.target
            assert old_snapshot["reply"]["available"]
            assert old_status["reply"]["worker_until_ns"] == str(self.job["wire"]["until_ns"])
            assert old_snapshot["reply"]["until_ns"] != str(self.job["wire"]["until_ns"])
            self.result["before_replacement"] = {"status": old_status, "snapshot": old_snapshot}
            self.exchange()
            self.freeze_successors = True
            self.commands.put(("release_pid", self.target))
            observations = []
            observed_target = None
            end = time.monotonic() + 2
            while time.monotonic() < end:
                status = self.request(self.control, "status")
                snapshot = self.request(self.socket, "snapshot")
                observations.append({"status": status, "snapshot": snapshot})
                if (status["received"] and snapshot["received"] and self.received_candidate
                        and self.sent_candidate and self.ack
                        and status["reply"]["completed"] == old_status["reply"]["completed"] + 1):
                    body = snapshot["reply"]
                    if body["available"] and body["until_ns"] == str(self.job["wire"]["until_ns"]):
                        observed_target = observations[-1]
                        break
                    if self.background(self.received_candidate["wire"]) is None and not body["available"]:
                        observed_target = observations[-1]
                        break
                time.sleep(.002)
            self.result["post_release_observations"] = observations
            assert observed_target is not None, "exact target completion publication/withdrawal was not observed before expiry"
            self.result["exact_target_outcome"] = observed_target
            assert self.received_candidate["wire"] == self.sent_candidate["wire"]
            assert self.sent_candidate["boottime_ns"] < self.job["wire"]["until_ns"]
            assert self.received_candidate["boottime_ns"] < self.job["wire"]["until_ns"]
            obsolete = (observed_target["snapshot"]["reply"]["available"]
                        and self.background(observed_target["snapshot"]["reply"]) == self.background(old_snapshot["reply"])
                        and observed_target["snapshot"]["reply"]["until_ns"] == str(self.job["wire"]["until_ns"]))
            self.result["obsolete_acquired_input_accepted_and_renewed"] = obsolete
            self.result["behavior_passed"] = not obsolete
            self.result["acceptance_assertion"] = "obsolete held acquired colors must not be accepted or renew publication"
            assert self.successor_event.wait(2), "successor before-exec stop not observed"
            assert any(r["pid"] == self.target for r in self.actual_parent_reaps)
            self.freeze_successors = False
            self.commands.put(("release_pid", self.successor))
            latest = None
            end = time.monotonic() + 3
            while time.monotonic() < end:
                s = self.request(self.control, "status")
                p = self.request(self.socket, "snapshot")
                if (s["received"] and p["received"] and p["reply"]["available"]
                        and self.background(p["reply"]) == {"r": 64250, "g": 62708, "b": 59624, "a": 65535}
                        and s["reply"]["completed"] >= old_status["reply"]["completed"] + 2
                        and p["reply"]["revision"] > old_snapshot["reply"]["revision"]):
                    latest = {"status": s, "snapshot": p}
                    break
                time.sleep(.003)
            assert latest, "fresh actual collection did not expose replacement colors"
            self.result["fresh_current_collection"] = latest
            stopped = self.request(self.control, "stop")
            assert stopped["received"] and stopped["reply"] == {"result": "stopping"}
            self.result["actual_stop"] = stopped
            assert self.exited_event.wait(4), "owner did not exit after actual Stop"
            self.result["setup_passed"] = True
        except BaseException:
            self.error = traceback.format_exc()
            self.result["setup_passed"] = False
            self.result["error"] = self.error
            self.commands.put("cleanup")

    def run(self):
        self.launch()
        tester = threading.Thread(target=self.tester, daemon=True)
        tester.start()
        deadline = time.monotonic() + 18
        while self.active and time.monotonic() < deadline:
            try:
                command = self.commands.get_nowait()
            except queue.Empty:
                command = None
            if command == "arm":
                self.arm = True
                self.record("armed_after_real_healthy_prerequisites")
                self.arm_event.set()
            if isinstance(command, tuple) and command[0] == "release_pid":
                pid = command[1]
                assert pid in self.held_pids and pid in self.active
                self.held_pids.remove(pid)
                self.record("released_exact_held_child", pid=pid, boottime_ns=boot())
                self.resume(pid)
            if command == "cleanup":
                self.freeze_successors = False
                for pid in list(self.held_pids):
                    if pid in self.active:
                        self.resume(pid)
                self.held_pids.clear()
                if self.owner in self.active:
                    os.kill(self.owner, signal.SIGTERM)
            try:
                pid, status = os.waitpid(-1, os.WNOHANG | original.WAIT_ALL)
            except ChildProcessError:
                break
            if pid:
                self.event(pid, status)
            else:
                time.sleep(.0002)
        if self.active:
            self.result["harness_deadline_cleanup"] = list(self.active)
            for pid in list(self.active):
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            end = time.monotonic() + 3
            while self.active and time.monotonic() < end:
                try:
                    pid, status = os.waitpid(-1, os.WNOHANG | original.WAIT_ALL)
                except ChildProcessError:
                    break
                if pid:
                    self.event(pid, status)
                else:
                    time.sleep(.001)
        tester.join(1)
        owned = [pid for pid in self.traced_exits if pid != self.owner]
        reaped = {entry["pid"] for entry in self.actual_parent_reaps}
        cleanup = {"owner": self.owner, "target": self.target, "successor": self.successor,
                   "tracer_wait_exits": self.traced_exits, "actual_owner_wait4_reaps": self.actual_parent_reaps,
                   "all_traced_children_reaped_by_actual_parent": set(owned) <= reaped,
                   "all_owned_proc_absent": all(not Path(f"/proc/{pid}").exists() for pid in self.traced_exits),
                   "owner_exit_zero": self.traced_exits.get(self.owner, {}).get("exit_code") == 0,
                   "both_sockets_absent": not self.socket.exists() and not self.control.exists(),
                   "active_tracees": list(self.active)}
        self.result["cleanup"] = cleanup
        self.result["cleanup_passed"] = (self.target in reaped and cleanup["all_traced_children_reaped_by_actual_parent"]
            and cleanup["all_owned_proc_absent"] and cleanup["owner_exit_zero"]
            and cleanup["both_sockets_absent"] and not cleanup["active_tracees"])
        self.result["events"] = self.events
        self.result["service_stderr"] = (self.root / "service.log").read_text()
        self.result["passed"] = (self.result.get("setup_passed", False)
            and self.result.get("behavior_passed", False) and self.result["cleanup_passed"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--expected-sha256", required=True)
    parser.add_argument("--expected-base-sha256", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    assert sha(args.binary) == args.expected_sha256
    assert sha(BASE) == args.expected_base_sha256
    output = Path(args.output)
    output.mkdir(exist_ok=True)
    assert not (output / "result.json").exists(), "result label already used"
    result = {"schema": 1, "wire_schema": 3,
        "scope": "actual synthetic x86 owned held-colors consistency control; no Original desktop/Google kernel/phone",
        "input_binary": original.fingerprint(args.binary), "base_control": original.fingerprint(BASE),
        "control": original.fingerprint(__file__), "started_ns": time.monotonic_ns(),
        "isolation": {"uid": os.getuid(), "gid": os.getgid(), "tracer_cap_eff": original.proc_status(os.getpid())["CapEff"].strip(),
            "ptrace_scope": "only directly launched TRACEME owner and its forked descendants; read-only syscall/memory observation",
            "registers_or_syscalls_rewritten": False, "injected_replies": False,
            "fixture_capabilities": "Docker defaults plus test-only SYS_PTRACE; no production capability claim"}}
    try:
        HeldColors(args.binary, result).run()
    except BaseException:
        result["setup_passed"] = False
        result["passed"] = False
        result["error"] = traceback.format_exc()
    result["finished_ns"] = time.monotonic_ns()
    result["selected_binary_unchanged"] = sha(args.binary) == args.expected_sha256
    result["base_control_unchanged"] = sha(BASE) == args.expected_base_sha256
    result["passed"] = bool(result.get("passed") and result["selected_binary_unchanged"] and result["base_control_unchanged"])
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result.get(key) for key in ["setup_passed", "behavior_passed", "cleanup_passed", "passed",
        "obsolete_acquired_input_accepted_and_renewed", "error"]}), flush=True)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
