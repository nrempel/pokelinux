#!/usr/bin/env python3
"""Publish only outcomes of our synthetic descriptor/Unix-lifetime scenarios."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
source = ROOT / "artifacts/native-suspend/reference/reference.tsv"
expected = {"ordering", "idempotent-release", "destructor-release", "concurrent-acquire",
            "count-reject", "state-write-failure", "dead-controller", "dead-client"}
seen = set()
for line in source.read_text().splitlines():
    name, result = line.split("\t")
    if name not in expected or name in seen or result != "PASS":
        raise RuntimeError(f"Unexpected synthetic outcome: {name}/{result}")
    seen.add(name)
if seen != expected:
    raise RuntimeError("Original-source cases incomplete")
fixture = ROOT / "native-suspend/tests/fixtures/public-suspend-reference.tsv"
fixture.parent.mkdir(parents=True, exist_ok=True)
fixture.write_bytes(source.read_bytes())
print(json.dumps({"original_source_cases": len(seen), "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
                  "binder_qualified": False, "kernel_suspend_qualified": False}))
