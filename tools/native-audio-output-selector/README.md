# Native audio logical-route and current-observation tests

These fixed original-code controllers test actual IPC and original host
PipeWire/WirePlumber graphs. `media-routes.py` covers the logical catalog and
private-target mapping; `current-observation.py` destroys/substitutes/restores a
real link and checks fresh read-only application status without replay.

Each takes `--build SEALED_BUILD --output FRESH_OUTPUT --image sha256:IMAGE`.
The selected build directory must contain `source/`, `target/` and a verified
`build-review.json` with complete source hashes and compiled ELF hashes. It must
include `tools/media-route-fixture.py`. The disposable runtime needs the original
PipeWire tools, WirePlumber, dbus-daemon, Python and fixture UIDs1000/1001/1002.
The controllers select the explicit local Docker socket, run without network or
hardware devices, retain actual client/fixture/container waits and write raw
private results beneath the selected output. Run under a bounded owning driver;
qualify the exact same-daemon CID absence separately rather than accepting an
arbitrary inspect failure. Preserve failed outputs.

The fixed tests are source artifacts; the hash-only public evidence and detailed
limits are in [the checkpoint](../../docs/native-audio-output-selector.md).
The current host proof is separate from Qt, PCM, ARM execution and hardware.
