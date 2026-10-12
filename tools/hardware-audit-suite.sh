#!/bin/sh
# Run in the audit image with --network none; no phone or host devices.
set -eu
cd "$(dirname "$0")/.."
python3 -m unittest discover -s tests -p test_hardware_audit.py
python3 tools/hardware_audit.py artifacts/telephony-audit/input artifacts/telephony-audit/results
python3 tools/audit-resources.py
python3 tools/native-graph.py
python3 tools/verify-hardware-evidence.py
