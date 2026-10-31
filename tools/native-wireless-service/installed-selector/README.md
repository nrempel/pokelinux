# Frozen static-selector ARM acceptance adapter

Apply `consumer.patch` only to `tools/native-wireless-service/consumer.py` at
[c511741](https://github.com/nrempel/pokelinux/blob/c51174102adc838dba8a9f9f541a198ab7be0072/tools/native-wireless-service/consumer.py).
Verify both the base and resulting hashes in `source-pins.json`, then apply
with `git apply --unidiff-zero consumer.patch`. This reproduces
the exact consumer selected for the actual static-name ARM baseline and fresh
candidate, without copying historical sources or changing maintained follow-up
scenarios. It is not a patch for the current maintained consumer.

The adapter passes four static Main paths and an interface-only protected
profile. The exact provisioned name is `wlan1`; namespace/index/wiphy/wdev remain
independent observer data. A narrowly pinned pre-worker unknown-interface
schema refusal can qualify RED only after actual Main1, independent absence,
peer0 and complete original fixture/namespace/radio/process/QEMU/container
closure. It cannot claim a worker/App wait or Maincap0 when not observed.
The one original in-process desired assertion still fails on that qualified RED.
The unchanged successful branch still requires the actual Settings SDK actor,
authorization, Connect, two cap0 packets with requester close/reopen and the
same worker, Disconnect and quiet, then full original closure.

`contract.json` retains its exact frozen pre-execution metadata and baseline
ELF eligibility. Candidate failures cannot acquire RED by replacing those pins.
Actual outcomes are curated separately in
[the evidence](../../../docs/evidence/native-connectivity-installed-selector.json).
The maintained parent `consumer.py` separately migrates the static profile for
its additional follow-up scenarios; that current public variant is unexecuted.
No file here runs a build, VM or phone, or establishes installed PID1 startup.
