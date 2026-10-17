#!/usr/bin/env python3
"""Pure, bounded diagnostic-ring reader. It does not authenticate a Source stream."""
import argparse
import json
import os
import stat
import struct

HEADER = struct.Struct("<8s6I4Q2i7Q")
RECORD = struct.Struct("<6Q4I2i2I")
CAPACITY = 1024
FILE_BYTES = 128 + CAPACITY * 80
MAGIC = b"PLCBSP01"
MAX_U64 = (1 << 64) - 1
KINDS = {1: "AQ_onSignal", 2: "timer_call", 3: "deferred_call"}


def require(ok, text):
    if not ok:
        raise ValueError(text)


def clock_value(value, error, label):
    require(error >= 0, label + ": negative errno")
    require(error == 0 or value == 0, label + ": failed clock has nonzero value")
    return value if error == 0 else None


def assess(raw, expected_pid, expected_tid, source_stamps=()):
    """Only a quiescent exact-file snapshot is valid input; custody is external."""
    require(len(raw) == FILE_BYTES, "wrong fixed file size")
    require(0 < expected_pid < (1 << 31) and 0 < expected_tid < (1 << 31), "bad expected identity")
    require(len(source_stamps) <= 16, "too many Source stamps")
    require(all(type(v) is int and 0 <= v <= MAX_U64 for v in source_stamps), "bad Source stamp")
    h = HEADER.unpack_from(raw)
    require(h[:5] == (MAGIC, 1, 128, 80, CAPACITY), "unarmed or incompatible header")
    pid, tid, last, flags, arm_boot, arm_cpu, arm_be, arm_ce = h[5:13]
    require((pid, tid) == (expected_pid, expected_tid), "writer identity mismatch")
    require(flags & 1 and not flags & ~31, "bad status flags")
    require(not flags & 16, "producer observed a foreign writer")
    require(not flags & 8 or last == MAX_U64, "bad saturation state")
    require(not any(h[13:]), "nonzero header reserved fields")
    arm_boot = clock_value(arm_boot, arm_be, "arm BOOTTIME")
    arm_cpu = clock_value(arm_cpu, arm_ce, "arm THREAD_CPUTIME")
    first = max(1, last - CAPACITY + 1)
    rows, partial = [], None
    if last < CAPACITY:
        for slot in range(last, CAPACITY):
            require(not any(raw[128 + slot * 80:128 + (slot + 1) * 80]), "dirty unused slot")
    for q in range(first, last + 1):
        slot = (q - 1) % CAPACITY
        v = RECORD.unpack_from(raw, 128 + slot * 80)
        committed = v[0]
        if committed != q:
            # Reserve is published before touching a slot. A killed final writer may
            # leave zero or the previous cycle's commit; its payload is not accepted.
            require(q == last and committed in (0, q - CAPACITY if q > CAPACITY else 0),
                    "missing or inconsistent non-tail commit")
            partial = {"sequence": q, "slot": slot, "observed_commit": committed}
            continue
        seq, span, boot, cpu, identity, aux, kind, edge, rp, rt, be, ce, rf, reserved = v
        require(kind in KINDS and edge in (1, 2), "unknown kind or edge")
        require((rp, rt) == (pid, tid), "record identity mismatch")
        require(rf == 0 and reserved == 0, "nonzero record reserved fields")
        require((edge == 1 and span == seq) or (edge == 2 and 0 < span < seq), "bad span reference")
        require(aux <= 0xffffffff if kind == 1 else aux == 0, "bad auxiliary identity")
        boot = clock_value(boot, be, "record BOOTTIME")
        cpu = clock_value(cpu, ce, "record THREAD_CPUTIME")
        row = {"sequence": seq, "span_id": span, "kind": KINDS[kind], "edge": "entry" if edge == 1 else "normal_return",
               "callback_identity": identity, "fd": (aux if aux < (1 << 31) else aux - (1 << 32)) if kind == 1 else None,
               "boottime_ns": boot, "thread_cpu_ns": cpu, "boottime_errno": be, "thread_cpu_errno": ce}
        rows.append(row)
    previous_boot, previous_cpu = arm_boot, arm_cpu
    entries, paired, outside_exits = {}, [], []
    returned_intervals = []
    for row in rows:
        for key, previous in (("boottime_ns", previous_boot), ("thread_cpu_ns", previous_cpu)):
            value = row[key]
            require(value is None or previous is None or value >= previous, "clock moved backwards")
        if row["boottime_ns"] is not None:
            previous_boot = row["boottime_ns"]
        if row["thread_cpu_ns"] is not None:
            previous_cpu = row["thread_cpu_ns"]
        span = row["span_id"]
        if row["edge"] == "normal_return":
            require(not any(start < span < end for start, end in returned_intervals),
                    "crossing completed spans")
            returned_intervals.append((span, row["sequence"]))
        if row["edge"] == "entry":
            require(span not in entries, "duplicate span entry")
            entries[span] = row
        elif span not in entries:
            require(first > 1 and span < first, "return lacks a retained or overwritten entry")
            require(span not in outside_exits, "duplicate overwritten-entry return")
            outside_exits.append(span)
        else:
            entry = entries.pop(span)
            require((entry["kind"], entry["callback_identity"], entry["fd"]) ==
                    (row["kind"], row["callback_identity"], row["fd"]), "return identity changed")
            paired.append({"span_id": span, "entry": entry, "normal_return": row,
                           "boottime_elapsed_ns": None if entry["boottime_ns"] is None or row["boottime_ns"] is None else row["boottime_ns"] - entry["boottime_ns"],
                           "thread_cpu_elapsed_ns": None if entry["thread_cpu_ns"] is None or row["thread_cpu_ns"] is None else row["thread_cpu_ns"] - entry["thread_cpu_ns"]})
    known = [r["boottime_ns"] for r in rows if r["boottime_ns"] is not None]
    correlations = []
    for stamp in source_stamps:
        correlations.append({"source_boottime_ns": stamp,
            "within_retained_known_record_time_range": bool(known) and min(known) <= stamp <= max(known),
            "completed_span_ids_bracketing_stamp": [p["span_id"] for p in paired if p["entry"]["boottime_ns"] is not None and p["normal_return"]["boottime_ns"] is not None and p["entry"]["boottime_ns"] <= stamp <= p["normal_return"]["boottime_ns"]],
            "unmatched_entry_ids_observed_by_stamp": [span for span, r in entries.items() if r["boottime_ns"] is not None and r["boottime_ns"] <= stamp]})
    return {"schema": 1, "diagnostic_protocol_valid": True, "runtime_qualified": False,
        "external_custody_and_authenticated_Source_case_required": True,
        "writer_pid": pid, "writer_tid": tid, "arm_boottime_ns": arm_boot, "arm_thread_cpu_ns": arm_cpu,
        "arm_boottime_errno": arm_be, "arm_thread_cpu_errno": arm_ce, "status_flags": flags,
        "highest_reserved_sequence": last, "retained_logical_first_sequence": first if last else None,
        "committed_record_count": len(rows), "overwritten_prefix_reservations": max(0, last - CAPACITY),
        "overflow_observed": bool(flags & 2) or last > CAPACITY, "sequence_saturated": bool(flags & 8),
        "partial_final_reservation": partial,
        "retained_known_boottime_range_ns": [min(known), max(known)] if known else None,
        "completed_spans": paired, "unmatched_entries": list(entries.values()),
        "normal_returns_whose_entries_were_overwritten": outside_exits,
        "source_correlations": correlations,
        "interpretation": "Only the three instrumented callback brackets are covered. Unmatched entry means no normal-return record in the retained committed window; it does not establish an internal cause."}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("ring")
    p.add_argument("--expected-pid", required=True, type=int)
    p.add_argument("--expected-tid", required=True, type=int)
    p.add_argument("--source-stamp", action="append", default=[], type=int)
    a = p.parse_args()
    fd = None
    try:
        fd = os.open(a.ring, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size == FILE_BYTES, "not an exact single-link regular ring")
        raw = os.read(fd, FILE_BYTES + 1)
        after = os.fstat(fd)
        require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) ==
                (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), "ring changed during read")
        result = assess(raw, a.expected_pid, a.expected_tid, a.source_stamp)
        result["read_file_metadata"] = {"device": before.st_dev, "inode": before.st_ino,
            "uid": before.st_uid, "gid": before.st_gid, "mode": stat.S_IMODE(before.st_mode), "bytes": before.st_size}
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        return 0
    except (ValueError, OSError, struct.error) as e:
        print(json.dumps({"schema": 1, "diagnostic_protocol_valid": False,
            "runtime_qualified": False, "error": str(e)}, sort_keys=True))
        return 2
    finally:
        if fd is not None:
            os.close(fd)


if __name__ == "__main__":
    raise SystemExit(main())
