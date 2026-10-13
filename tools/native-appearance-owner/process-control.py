#!/usr/bin/env python3
"""Actual owned-process x86_64 appearance boundaries; no injected service replies.

Run as root in a network-none disposable container with test-only SYS_PTRACE.
Only PTRACE_TRACEME of the service launched here and its traced descendants is
used. The tracer never attaches to pre-existing/host processes, rewrites registers,
changes syscalls or changes service inputs after launch.
"""
import argparse
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import signal
import socket
import struct
import threading
import time
import traceback

TRACEME, PEEKDATA, CONT, GETREGS, SYSCALL = 0, 2, 7, 12, 24
SETOPTIONS, GETEVENTMSG, GET_SYSCALL_INFO = 0x4200, 0x4201, 0x420e
FORK, VFORK, CLONE, EXEC, EXIT = 1, 2, 3, 4, 6
OPTIONS = 1 | (1 << FORK) | (1 << VFORK) | (1 << CLONE) | (1 << EXEC) | (1 << EXIT)
WAIT_ALL = 0x40000000
SENTINEL_FD = 198
libc = ctypes.CDLL(None, use_errno=True)
libc.ptrace.restype = ctypes.c_long
libc.ptrace.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_void_p, ctypes.c_void_p]


def ptrace(req, pid, addr=0, data=0):
    ctypes.set_errno(0)
    ret = libc.ptrace(req, pid, addr, data)
    if ret == -1 and ctypes.get_errno():
        raise OSError(ctypes.get_errno(), f"ptrace request {req} pid {pid}")
    return ret


def proc_status(pid):
    p = Path(f"/proc/{pid}/status")
    if not p.exists():
        return None
    return dict(line.split(":", 1) for line in p.read_text().splitlines())


def fingerprint(path):
    p = Path(path)
    m = p.stat()
    return {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
            "uid": m.st_uid, "gid": m.st_gid, "mode": oct(m.st_mode & 0o777),
            "dev": m.st_dev, "ino": m.st_ino, "size": m.st_size}


class Regs(ctypes.Structure):
    _fields_ = [(name, ctypes.c_ulonglong) for name in
                "r15 r14 r13 r12 rbp rbx r11 r10 r9 r8 rax rcx rdx rsi rdi orig_rax rip cs eflags rsp ss fs_base gs_base ds es fs gs".split()]


class Pending:
    def __init__(self, owner, path, op):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        self.sock.settimeout(.2)
        self.sock.connect(str(path))
        self.peer = struct.unpack("3i", self.sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        assert self.peer == (owner, 0, 0), (self.peer, owner)
        self.op = op
        self.sent = time.monotonic_ns()
        self.sock.send(json.dumps({"op": op}).encode())

    def receive(self, timeout=.2):
        start = time.monotonic_ns()
        self.sock.settimeout(timeout)
        try:
            packet, ancillary, flags, _ = self.sock.recvmsg(4096, socket.CMSG_SPACE(12))
        except socket.timeout:
            return {"op": self.op, "received": False, "timeout_ms": timeout * 1000,
                    "elapsed_ms": (time.monotonic_ns() - start) / 1e6, "peer": self.peer}
        creds = [struct.unpack("3i", data) for level, kind, data in ancillary
                 if level == socket.SOL_SOCKET and kind == socket.SCM_CREDENTIALS]
        assert not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC)
        assert creds == [self.peer], creds
        reply = json.loads(packet)
        return {"op": self.op, "received": True, "elapsed_ms": (time.monotonic_ns() - start) / 1e6,
                "since_send_ms": (time.monotonic_ns() - self.sent) / 1e6,
                "peer": self.peer, "packet_credentials": creds[0], "reply": reply}

    def close(self):
        self.sock.close()


class Realm:
    def __init__(self, binary, mode, result):
        assert os.getuid() == 0 and os.getgid() == 0
        assert os.uname().machine == "x86_64"
        self.mode = mode
        self.result = result
        self.root = Path(f"/run/appearance-boundary-{os.getpid()}-{mode}")
        self.root.mkdir(mode=0o755)
        self.binary = self.root / "installed-appearance"
        shutil.copyfile(binary, self.binary)
        self.binary.chmod(0o755)
        self.home, self.runtime = self.root / "home", self.root / "runtime"
        (self.home / ".local/state/omarchy/current/theme").mkdir(parents=True)
        self.runtime.mkdir()
        for directory in [self.home, *self.home.rglob("*"), self.runtime]:
            os.chown(directory, 1000, 1000)
            directory.chmod(0o700)
        self.colors = self.home / ".local/state/omarchy/current/theme/colors.toml"
        self.colors.write_text("background = '#1a1b26'\nforeground = '#a9b1d6'\naccent = '#7aa2f7'\ncolor1 = '#f7768e'\ngreen = '#9ece6a'\n")
        os.chown(self.colors, 1000, 1000)
        self.colors.chmod(0o600)
        self.config = self.root / "config.json"
        self.config.write_text(json.dumps({"home": str(self.home), "runtime": str(self.runtime)}))
        self.config.chmod(0o600)
        self.sentinel = self.root / "unrelated-root-input"
        self.sentinel.write_text("synthetic unrelated root-opened file; no host/private input\n")
        self.sentinel.chmod(0o600)
        self.socket = self.root / "appearance.sock"
        self.control = self.root / "appearance-control.sock"
        self.commands = queue.Queue()
        self.events = []
        self.held_event = threading.Event()
        self.arm_event = threading.Event()
        self.exited_event = threading.Event()
        self.active = {}
        self.owner = None
        self.target = None
        self.arm = False
        self.held = False
        self.traced_exits = {}
        self.actual_parent_reaps = []
        self.warmup_uid_transitions = []
        self.warmup_seccomp = []
        self.error = None
        self.result["setup"] = {"binary": fingerprint(self.binary), "config": fingerprint(self.config),
                                "colors": fingerprint(self.colors), "sentinel": fingerprint(self.sentinel),
                                "home": {"uid": self.home.stat().st_uid, "mode": oct(self.home.stat().st_mode & 0o777)},
                                "runtime": {"uid": self.runtime.stat().st_uid, "mode": oct(self.runtime.stat().st_mode & 0o777)}}
        self.probe_private_path()

    def record(self, what, **kw):
        self.events.append({"ns": time.monotonic_ns(), "event": what, **kw})

    def probe_private_path(self):
        probe = os.fork()
        if probe == 0:
            os.setgroups([])
            os.setgid(1000)
            os.setuid(1000)
            try:
                os.open(self.sentinel, os.O_RDONLY)
            except OSError as error:
                os._exit(0 if error.errno == errno.EACCES else 2)
            os._exit(1)
        waited, status = os.waitpid(probe, 0)
        assert waited == probe and os.waitstatus_to_exitcode(status) == 0
        self.result["setup"]["actual_uid1000_sentinel_open_eacces"] = True
        self.result["setup"]["uid_probe_wait"] = {"pid": probe, "raw_status": status}

    def launch(self):
        fd = os.open(self.sentinel, os.O_RDONLY)
        self.owner = os.fork()
        if self.owner == 0:
            try:
                os.dup2(fd, SENTINEL_FD, inheritable=True)
                if fd != SENTINEL_FD:
                    os.close(fd)
                log = os.open(self.root / "service.log", os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                os.dup2(log, 2)
                os.close(log)
                ptrace(TRACEME, 0)
                os.kill(os.getpid(), signal.SIGSTOP)
                os.execv(self.binary, [str(self.binary), "--serve", str(self.socket), str(self.config)])
            except BaseException:
                traceback.print_exc()
                os._exit(120)
        os.close(fd)
        pid, status = os.waitpid(self.owner, 0)
        assert pid == self.owner and os.WIFSTOPPED(status) and os.WSTOPSIG(status) == signal.SIGSTOP
        ptrace(SETOPTIONS, self.owner, data=OPTIONS)
        self.active[self.owner] = {"role": "owner", "syscalls": False, "last_nr": None}
        self.record("owner_direct_traceme_initial_stop", pid=pid, raw_status=status)
        ptrace(CONT, self.owner)

    def read_word(self, pid, address):
        return ptrace(PEEKDATA, pid, address) & ((1 << 64) - 1)

    def read_string(self, pid, address):
        out = bytearray()
        for off in range(0, 4096, 8):
            chunk = self.read_word(pid, address + off).to_bytes(8, "little")
            if 0 in chunk:
                out += chunk[:chunk.index(0)]
                return out.decode()
            out += chunk
        raise AssertionError("unbounded trace string")

    def identity(self, pid):
        s = proc_status(pid)
        assert s is not None
        return {key: s[key].strip() for key in ["Name", "State", "PPid", "TracerPid", "Uid", "Gid", "Groups", "NoNewPrivs", "Seccomp", "Seccomp_filters"] if key in s}

    def fd_table(self, pid):
        entries = []
        for path in sorted(Path(f"/proc/{pid}/fd").iterdir(), key=lambda p: int(p.name)):
            meta = path.stat()
            info = Path(f"/proc/{pid}/fdinfo/{path.name}").read_text()
            entries.append({"fd": int(path.name), "link": os.readlink(path), "dev": meta.st_dev,
                            "ino": meta.st_ino, "uid": meta.st_uid, "gid": meta.st_gid,
                            "mode": oct(meta.st_mode & 0o777), "fdinfo": info})
        return entries

    def stop_at_exec(self, pid):
        regs = Regs()
        ptrace(GETREGS, pid, data=ctypes.cast(ctypes.byref(regs), ctypes.c_void_p))
        assert regs.orig_rax in [59, 322]
        at = regs.orig_rax == 322
        path = self.read_string(pid, regs.rsi if at else regs.rdi)
        argv_pointer = regs.rdx if at else regs.rsi
        args = []
        for index in range(5):
            pointer = self.read_word(pid, argv_pointer + index * 8)
            if pointer == 0:
                break
            args.append(self.read_string(pid, pointer))
        expected_argv0 = "pokelinux-appearance" if at else str(self.binary)
        assert args == [expected_argv0, "--collector", str(self.owner)], (path, args)
        exec_fd = None
        if at:
            assert path == "" and regs.r8 == 0x1000, (path, regs.r8)
            exec_fd = regs.rdi
            selected = Path(f"/proc/{pid}/fd/{exec_fd}").stat()
            installed = self.binary.stat()
            assert (selected.st_dev, selected.st_ino) == (installed.st_dev, installed.st_ino)
        else:
            assert path == str(self.binary), path
        identity = self.identity(pid)
        assert int(identity["PPid"]) == self.owner
        self.result["actual_preexec_boundary"] = {"pid": pid, "syscall": "execveat" if at else "execve", "number": regs.orig_rax,
                                                  "phase": "kernel syscall-entry ptrace stop before exec",
                                                  "path": path, "argv": args, "argv0_contract": expected_argv0, "executable_fd": exec_fd, "identity": identity,
                                                  "proc_exe_before_exec": os.readlink(f"/proc/{pid}/exe"),
                                                  "fd_table": self.fd_table(pid)}
        self.record("exact_owned_collector_exec_entry", pid=pid, syscall="execveat" if at else "execve", path=path, argv=args)
        if self.mode == "preexec":
            self.held = True
            self.record("held_before_exec", pid=pid)
            self.held_event.set()
            return True
        return False

    def on_syscall(self, pid, state):
        info = ctypes.create_string_buffer(88)
        ptrace(GET_SYSCALL_INFO, pid, 88, ctypes.cast(info, ctypes.c_void_p))
        raw = info.raw
        op = raw[0]
        if op == 1:
            nr = struct.unpack_from("Q", raw, 24)[0]
            state["last_nr"] = nr
            if pid == self.target and nr in [59, 322]:
                return self.stop_at_exec(pid)
        elif op == 2:
            rval = struct.unpack_from("q", raw, 24)[0]
            nr = state["last_nr"]
            if pid == self.owner and nr == 61 and rval > 0:
                self.actual_parent_reaps.append({"pid": rval, "syscall": "wait4", "return": rval, "ns": time.monotonic_ns()})
                self.record("actual_owner_wait4_reap_return", pid=pid, child=rval)
            if state["role"] == "collector" and nr == 117 and rval == 0:
                identity = self.identity(pid)
                assert identity["Uid"].split() == ["1000"] * 4
                assert identity["Gid"].split() == ["1000"] * 4
                assert not identity["Groups"]
                self.warmup_uid_transitions.append({"pid": pid, "identity": identity})
                self.record("actual_setresuid1000_return", pid=pid)
            if state["role"] == "collector" and nr == 317 and rval == 0:
                identity = self.identity(pid)
                self.warmup_seccomp.append({"pid": pid, "identity": identity})
                if pid == self.target and self.mode == "inherited-fd":
                    assert identity["Uid"].split() == ["1000"] * 4
                    assert identity["Gid"].split() == ["1000"] * 4
                    assert identity["NoNewPrivs"] == "1" and identity["Seccomp"] == "2"
                    entries = self.fd_table(pid)
                    m = self.sentinel.stat()
                    hits = [entry for entry in entries if (entry["dev"], entry["ino"]) == (m.st_dev, m.st_ino)]
                    self.result["actual_uid1000_fd_boundary"] = {"pid": pid, "identity": identity,
                                                                "after_successful_collector_seccomp_install": True,
                                                                "fd_table": entries, "unrelated_sentinel_fds": hits}
                    self.held = True
                    self.record("held_uid1000_after_collector_seccomp", pid=pid, sentinel_fds=[entry["fd"] for entry in hits])
                    self.held_event.set()
                    return True
                state["syscalls"] = False
        return False

    def resume(self, pid, sig=0):
        state = self.active[pid]
        try:
            ptrace(SYSCALL if state["syscalls"] else CONT, pid, data=sig)
        except OSError as error:
            # An owned parent SIGKILL can overtake a queued syscall stop. ESRCH
            # alone is not reap evidence: retain the tracee until actual waitpid.
            if error.errno != errno.ESRCH:
                raise
            self.record("resume_esrch_awaiting_actual_wait_exit", pid=pid)

    def event(self, pid, status):
        if os.WIFEXITED(status) or os.WIFSIGNALED(status):
            self.traced_exits[pid] = {"raw_wait_status": status, "exit_code": os.waitstatus_to_exitcode(status)}
            self.record("tracer_actual_wait_exit", pid=pid, raw_status=status, exit_code=os.waitstatus_to_exitcode(status))
            self.active.pop(pid, None)
            if pid == self.owner:
                self.exited_event.set()
            return
        assert os.WIFSTOPPED(status)
        event = status >> 16
        sig = os.WSTOPSIG(status)
        if pid not in self.active:
            self.active[pid] = {"role": "collector", "syscalls": True, "last_nr": None}
        state = self.active[pid]
        if event in [FORK, VFORK, CLONE]:
            child = ctypes.c_ulong()
            ptrace(GETEVENTMSG, pid, data=ctypes.cast(ctypes.byref(child), ctypes.c_void_p))
            assert pid == self.owner, "collector unexpectedly spawned"
            self.active[child.value] = {"role": "collector", "syscalls": True, "last_nr": None}
            self.record("actual_owned_child_event", pid=pid, child=child.value, kind=event)
            if self.arm and self.target is None:
                self.target = child.value
                state["syscalls"] = True
                self.record("selected_fresh_owned_child", pid=pid, child=self.target)
            self.resume(pid)
            return
        if event == EXEC:
            self.record("actual_exec_event", pid=pid, exe=os.readlink(f"/proc/{pid}/exe"))
            self.resume(pid)
            return
        if event == EXIT:
            self.record("actual_exit_event", pid=pid)
            self.resume(pid)
            return
        if sig == signal.SIGTRAP | 0x80:
            if self.on_syscall(pid, state):
                return
            self.resume(pid)
            return
        if sig in [signal.SIGSTOP, signal.SIGTRAP]:
            self.resume(pid)
            return
        self.record("actual_signal_delivery", pid=pid, signal=sig)
        self.resume(pid, sig)

    def request(self, path, op, timeout=.2):
        pending = Pending(self.owner, path, op)
        try:
            return pending.receive(timeout)
        finally:
            pending.close()

    def tester(self):
        pending = []
        try:
            end = time.monotonic() + 5
            healthy = False
            while time.monotonic() < end:
                if self.socket.exists() and self.control.exists():
                    s = self.request(self.control, "status")
                    p = self.request(self.socket, "snapshot")
                    if (s["received"] and p["received"] and s["reply"]["completed"] > 0
                            and p["reply"]["available"] and p["reply"]["palette"]["background"] == "#1a1b26"
                            and self.warmup_uid_transitions and self.warmup_seccomp):
                        self.result["healthy_prerequisites"] = {"status": s, "snapshot": p,
                                                                 "uid_transitions": self.warmup_uid_transitions[:],
                                                                 "seccomp_installs": self.warmup_seccomp[:]}
                        healthy = True
                        break
                time.sleep(.003)
            assert healthy, "normal original collector palette/credentials/filter prerequisites failed"
            self.commands.put("arm")
            assert self.arm_event.wait(1)
            assert self.held_event.wait(3), "actual owned target boundary not reached"
            assert self.target is not None
            self.result["owner_during_hold"] = {"pid": self.owner, "identity": self.identity(self.owner),
                                                "wchan": Path(f"/proc/{self.owner}/wchan").read_text().strip(),
                                                "children": Path(f"/proc/{self.owner}/task/{self.owner}/children").read_text().strip()}
            status = Pending(self.owner, self.control, "status")
            pending.append(status)
            status_result = status.receive()
            snapshot = Pending(self.owner, self.socket, "snapshot")
            pending.append(snapshot)
            snapshot_result = snapshot.receive()
            stop = Pending(self.owner, self.control, "stop")
            pending.append(stop)
            stop_result = stop.receive()
            self.result["while_actual_child_held"] = {"status": status_result, "snapshot": snapshot_result, "stop": stop_result}
            if self.mode == "preexec":
                self.result["behavior_passed"] = (status_result["received"] and snapshot_result["received"]
                                                   and stop_result["received"] and status_result["reply"]["worker_pid"] == self.target
                                                   and stop_result["reply"] == {"result": "stopping"})
            else:
                self.result["behavior_passed"] = not self.result["actual_uid1000_fd_boundary"]["unrelated_sentinel_fds"]
                assert status_result["received"] and snapshot_result["received"] and stop_result["received"]
                assert status_result["reply"]["worker_pid"] == self.target
                assert stop_result["reply"] == {"result": "stopping"}
            if not stop_result["received"]:
                self.commands.put("release")
                self.result["after_release_original_queued_stop"] = stop.receive(3)
                assert self.result["after_release_original_queued_stop"]["received"]
                assert self.result["after_release_original_queued_stop"]["reply"] == {"result": "stopping"}
            assert self.exited_event.wait(4), "owner did not exit after Stop"
            self.result["setup_passed"] = True
        except BaseException:
            self.error = traceback.format_exc()
            self.result["setup_passed"] = False
            self.result["error"] = self.error
            self.commands.put("cleanup")
        finally:
            for p in pending:
                p.close()

    def run(self):
        self.launch()
        tester = threading.Thread(target=self.tester, daemon=True)
        tester.start()
        deadline = time.monotonic() + 15
        while self.active and time.monotonic() < deadline:
            try:
                command = self.commands.get_nowait()
            except queue.Empty:
                command = None
            if command == "arm":
                self.arm = True
                self.record("tester_armed_after_healthy_prerequisites")
                self.arm_event.set()
            if command in ["release", "cleanup"]:
                if self.held and self.target in self.active:
                    self.record("tracer_release_actual_held_child", pid=self.target)
                    try:
                        self.resume(self.target)
                    except OSError as error:
                        if error.errno != errno.ESRCH:
                            raise
                    self.held = False
                if command == "cleanup" and self.owner in self.active:
                    os.kill(self.owner, signal.SIGTERM)
            try:
                pid, status = os.waitpid(-1, os.WNOHANG | WAIT_ALL)
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
                    pid, status = os.waitpid(-1, os.WNOHANG | WAIT_ALL)
                except ChildProcessError:
                    break
                if pid:
                    self.event(pid, status)
                else:
                    time.sleep(.001)
        tester.join(1)
        child_reaped = any(entry["pid"] == self.target for entry in self.actual_parent_reaps)
        self.result["cleanup"] = {"owner": self.owner, "target": self.target,
                                  "tracer_wait_exits": self.traced_exits,
                                  "actual_owner_wait4_reaps": self.actual_parent_reaps,
                                  "target_actual_parent_reap": child_reaped,
                                  "owner_exit_zero": self.traced_exits.get(self.owner, {}).get("exit_code") == 0,
                                  "owner_proc_absent": not Path(f"/proc/{self.owner}").exists(),
                                  "target_proc_absent": self.target is not None and not Path(f"/proc/{self.target}").exists(),
                                  "both_sockets_absent": not self.socket.exists() and not self.control.exists(),
                                  "active_tracees": list(self.active)}
        self.result["events"] = self.events
        self.result["service_stderr"] = (self.root / "service.log").read_text()
        cleanup = self.result["cleanup"]
        self.result["cleanup_passed"] = (cleanup["target_actual_parent_reap"] and cleanup["owner_exit_zero"]
                                          and cleanup["owner_proc_absent"] and cleanup["target_proc_absent"]
                                          and cleanup["both_sockets_absent"] and not cleanup["active_tracees"])
        self.result["passed"] = (self.result.get("setup_passed", False)
                                  and self.result.get("behavior_passed", False) and self.result["cleanup_passed"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--expected-sha256", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    source_hash = fingerprint(args.binary)["sha256"]
    assert source_hash == args.expected_sha256
    output = Path(args.output)
    output.mkdir(exist_ok=True)
    aggregate = {"scope": "actual synthetic x86_64 Linux service/owned child controls; no original desktop, phone or proprietary inputs",
                 "input_binary": fingerprint(args.binary), "control": fingerprint(__file__),
                 "expected_binary_sha256": args.expected_sha256, "started_ns": time.monotonic_ns(),
                 "isolation": {"uid": os.getuid(), "gid": os.getgid(), "test_only_ptrace": True,
                               "ptrace_scope": "directly launched TRACEME service and its own fork/clone descendants",
                               "capability_profile": "Docker default fixture capabilities plus test-only SYS_PTRACE; not production profile",
                               "tracer_cap_eff": proc_status(os.getpid())["CapEff"].strip()}, "cases": {}}
    for mode in ["preexec", "inherited-fd"]:
        result = {}
        aggregate["cases"][mode] = result
        try:
            Realm(args.binary, mode, result).run()
        except BaseException:
            result["setup_passed"] = False
            result["passed"] = False
            result["error"] = traceback.format_exc()
        (output / f"{mode}.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps({"case": mode, "setup_passed": result.get("setup_passed"),
                          "behavior_passed": result.get("behavior_passed"), "cleanup_passed": result.get("cleanup_passed"),
                          "passed": result.get("passed"), "error": result.get("error")}), flush=True)
    aggregate["finished_ns"] = time.monotonic_ns()
    aggregate["setup_passed"] = all(c.get("setup_passed") and c.get("cleanup_passed") for c in aggregate["cases"].values())
    aggregate["passed"] = all(c.get("passed") for c in aggregate["cases"].values())
    aggregate["input_binary_unchanged"] = fingerprint(args.binary)["sha256"] == source_hash
    (output / "result.json").write_text(json.dumps(aggregate, indent=2) + "\n")
    return 0 if aggregate["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
