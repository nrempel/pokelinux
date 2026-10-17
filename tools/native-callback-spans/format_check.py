#!/usr/bin/env python3
"""Authored binary-format controls only; no real callback or Source claims."""
import importlib.util
import json
from pathlib import Path
import struct
import sys
sys.dont_write_bytecode = True
H = struct.Struct("<8s6I4Q2i7Q")
S = struct.Struct("<6Q4I2i2I")
PID, TID, CAP = 4242, 4242, 1024


def row(q, span, edge, kind=1, identity=100, boot=None, cpu=None, be=0, ce=0):
    return S.pack(q, span, q if boot is None else boot, q if cpu is None else cpu,
                  identity, 7 if kind == 1 else 0, kind, edge, PID, TID, be, ce, 0, 0)


def ring(rows, last=None, arm_errors=(0, 0)):
    last = len(rows) if last is None else last
    out = bytearray(128 + CAP * 80)
    out[:128] = H.pack(b"PLCBSP01", 1, 128, 80, CAP, PID, TID, last,
                      1 | (2 if last > CAP else 0), 0, 0, *arm_errors, *([0] * 7))
    for q, value in rows.items():
        slot = (q - 1) % CAP
        out[128 + slot * 80:128 + (slot + 1) * 80] = value
    return bytes(out)


def main():
    path = Path(sys.argv[1])
    spec = importlib.util.spec_from_file_location("selected_reader", path)
    selected = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(selected)
    completed = []

    def control(name, raw, check=None, reject=None):
        try:
            try:
                result = selected.assess(raw, PID, TID, [0, 2])
            except ValueError as error:
                if reject is None or str(error) != reject:
                    raise AssertionError("unexpected rejection: " + str(error))
                completed.append({"control": name, "passed": True, "actual_rejection": str(error)})
                return
            if reject is not None:
                raise AssertionError("invalid format was accepted: " + reject)
            assert result["diagnostic_protocol_valid"] and result["runtime_qualified"] is False
            assert check(result)
            completed.append({"control": name, "passed": True})
        except BaseException as error:
            completed.append({"control": name, "passed": False,
                              "error": type(error).__name__ + ": " + str(error)})
            print(json.dumps({"schema": 1, "authored_format_controls_only": True,
                              "passed": False, "controls": completed}, sort_keys=True))
            raise SystemExit(1)

    # Frozen first check proves the actual v1 acceptance defect before v2 exists.
    control("crossing_completed_spans_rejected", ring({
        1: row(1, 1, 1), 2: row(2, 2, 1, 2, 200),
        3: row(3, 1, 2), 4: row(4, 2, 2, 2, 200)}),
        reject="crossing completed spans")
    control("normal_disjoint_returns", ring({
        1: row(1, 1, 1), 2: row(2, 1, 2),
        3: row(3, 3, 1, 3, 8), 4: row(4, 3, 2, 3, 8)}),
        lambda v: len(v["completed_spans"]) == 2 and not v["unmatched_entries"])
    control("nested_returns", ring({
        1: row(1, 1, 1), 2: row(2, 2, 1, 2, 200),
        3: row(3, 2, 2, 2, 200), 4: row(4, 1, 2)}),
        lambda v: [x["span_id"] for x in v["completed_spans"]] == [2, 1])
    control("unmatched_inner_entry_allowed", ring({
        1: row(1, 1, 1), 2: row(2, 2, 1, 2, 200), 3: row(3, 1, 2)}),
        lambda v: len(v["completed_spans"]) == 1 and [x["span_id"] for x in v["unmatched_entries"]] == [2])
    control("unmatched_entries_allowed", ring({1: row(1, 1, 1), 2: row(2, 2, 1, 2, 200)}),
        lambda v: not v["completed_spans"] and len(v["unmatched_entries"]) == 2)
    control("interrupted_tail_not_accepted", ring({1: row(1, 1, 1)}, last=2),
        lambda v: v["partial_final_reservation"] == {"sequence": 2, "slot": 1, "observed_commit": 0}
        and v["committed_record_count"] == 1 and len(v["unmatched_entries"]) == 1)
    wrapped = {q: row(q, q if q % 2 else q - 1, 1 if q % 2 else 2) for q in range(1, 1026)}
    control("rolling_wrap_and_overwritten_entry", ring(wrapped, last=1025),
        lambda v: v["overflow_observed"] and v["overwritten_prefix_reservations"] == 1
        and v["committed_record_count"] == 1024 and len(v["completed_spans"]) == 511
        and v["normal_returns_whose_entries_were_overwritten"] == [1]
        and [x["span_id"] for x in v["unmatched_entries"]] == [1025])
    control("unknown_clocks_remain_unknown", ring({
        1: row(1, 1, 1, boot=0, cpu=0, be=5, ce=13),
        2: row(2, 1, 2, boot=0, cpu=0, be=5, ce=13)}, arm_errors=(5, 13)),
        lambda v: v["retained_known_boottime_range_ns"] is None
        and v["arm_boottime_ns"] is None and v["arm_thread_cpu_ns"] is None
        and v["completed_spans"][0]["boottime_elapsed_ns"] is None
        and v["completed_spans"][0]["thread_cpu_elapsed_ns"] is None)
    control("successful_zero_clocks_are_valid", ring({
        1: row(1, 1, 1, boot=0, cpu=0), 2: row(2, 1, 2, boot=0, cpu=0)}),
        lambda v: v["retained_known_boottime_range_ns"] == [0, 0]
        and v["completed_spans"][0]["boottime_elapsed_ns"] == 0)
    control("backwards_known_clock_rejected", ring({
        1: row(1, 1, 1, boot=10, cpu=10), 2: row(2, 1, 2, boot=9, cpu=11)}),
        reject="clock moved backwards")
    print(json.dumps({"schema": 1, "authored_format_controls_only": True,
                      "passed": True, "controls": completed}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
