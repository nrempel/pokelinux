#!/usr/bin/env python3
"""Count actual procfs opens by the compiled pidfd_read_probe executable."""
import pathlib
import re
import subprocess
import sys
import tempfile

probe = pathlib.Path(sys.argv[1]).resolve(strict=True)
results = []
with tempfile.TemporaryDirectory(prefix="pokelinux-pidfd-reads-") as temporary:
    for mode, expected in [("same", 1), ("distinct", 2)]:
        trace = pathlib.Path(temporary) / (mode + ".strace")
        run = subprocess.run(
            ["strace", "-f", "-yy", "-e", "trace=openat", "-o", str(trace),
             str(probe), mode], check=True, capture_output=True, text=True,
        )
        pid = re.fullmatch(r"probe_pid=(\d+)\n", run.stdout).group(1)
        # Match the directory argument, not the fdinfo directory's own open
        # whose returned descriptor also receives a strace path annotation.
        pattern = re.compile(r"openat\(\d+</proc/" + pid + r"/fdinfo>,")
        reads = [line for line in trace.read_text().splitlines() if pattern.search(line)]
        results.append((mode, len(reads), expected))
        print(f"{mode}: actual fdinfo opens={len(reads)}, expected={expected}", flush=True)
        print("\n".join(reads), flush=True)
assert all(actual == expected for _, actual, expected in results), results
