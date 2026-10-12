#!/usr/bin/env python3
"""Fetch and verify pinned public sources into ignored artifacts only."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = json.loads((ROOT / "tools/suspend-reference/fetch-manifest.json").read_text())
DESTINATION = ROOT / "artifacts/native-suspend/public"
parser = argparse.ArgumentParser()
parser.add_argument("--verify-only", action="store_true")
args = parser.parse_args()
for item in MANIFEST["files"]:
    path = DESTINATION / item["file"]
    if path.exists():
        data = path.read_bytes()
    elif args.verify_only:
        raise RuntimeError(f"Missing pinned source: {item['file']}")
    else:
        with urllib.request.urlopen(item["url"], timeout=30) as response:
            encoded = response.read(2 * 1024 * 1024)
        data = base64.b64decode(encoded, validate=True)
        if len(data) != item["bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise RuntimeError(f"Source identity mismatch: {item['file']}")
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".partial")
        with temporary.open("xb") as output:
            output.write(data)
        temporary.rename(path)
    if len(data) != item["bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
        raise RuntimeError(f"Source identity mismatch: {item['file']}")
print(json.dumps({"verified_public_files": len(MANIFEST["files"]), "tag": MANIFEST["tag"]}))
