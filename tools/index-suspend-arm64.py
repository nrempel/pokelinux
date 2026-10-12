#!/usr/bin/env python3
"""Index exact Cargo ARM64 executables for an owned guest-kernel runner."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
results = ROOT / "artifacts/native-suspend"
executables = {}
for filename in ["arm64-helper-cargo.jsonl", "arm64-tests-cargo.jsonl"]:
    for line in (results / filename).read_text().splitlines():
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or not message.get("executable"):
            continue
        name = message["target"]["name"]
        if name not in {"lifetime-peer", "pokelinux_suspend", "coordination", "descriptors", "deadlines", "process_lifetime"}:
            continue
        if message["profile"]["test"] and name == "lifetime-peer":
            continue
        container_path = message["executable"]
        suffix = Path(container_path).relative_to("/target")
        path = results / "target" / suffix
        data = path.read_bytes()
        executables[name] = {"cargo_executable": container_path,
                             "project_path": str(path.relative_to(ROOT)),
                             "guest_path": "/workspace/" + str(path.relative_to(ROOT)),
                             "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
if set(executables) != {"lifetime-peer", "pokelinux_suspend", "coordination", "descriptors", "deadlines", "process_lifetime"}:
    raise RuntimeError(f"Incomplete exact executable set: {sorted(executables)}")
output = {"schema": 1, "target": "aarch64-unknown-linux-gnu", "executables": executables,
          "process_lifetime_arguments": ["--ignored"],
          "helper_override_env": "POKELINUX_SUSPEND_LIFETIME_PEER",
          "actual_arm_kernel_executed": False}
(results / "arm64-executables.json").write_text(json.dumps(output, indent=2) + "\n")
print(json.dumps({"indexed_executables": len(executables), "actual_arm_kernel_executed": False}))
