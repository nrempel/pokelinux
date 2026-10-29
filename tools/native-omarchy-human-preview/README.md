# Bounded normal-entry Omarchy preview

Original execution sources for the [preview checkpoint](../../docs/native-omarchy-human-preview.md).
These are fixture components, not a standalone installer or a qualified human connection recipe.

Apply the three patches with `git apply --unidiff-zero` to the reconstructed
`native-omarchy-viewer` Sol6 consumers.
`source-pins.json` records exact before/after hashes. Copy the two human helpers alongside
those consumers. Retain the existing viewer protocol/device/viewport helpers and owned
process support. The selected image, SDK, genuine compositor/shell, installation
manifest and matching Rust services remain required. The profile selects the normal
Sounds executable directly, preserving UID 11042 and all six arguments.

`profile_contract.py PROFILE` is the unchanged configuration RED/GREEN consumer.
`human_viewer_boundary.py` tests real Xvfb/x11vnc after explicitly synthetic readiness;
it qualifies no VM, QMP, native window or human client. The separate `viewer_smoke.py CASE`
connects to the running full VM, sends three fixed menu actions once, observes the
normal app process, receives a frame and disconnects. Its coordinates come from the
previous qualified menu layout; it does not remeasure layout before each click and
must not be treated as the earlier seven-action viewport-qualified consumer.

The guest and host each cap their local dwell by their original work deadline and
300 seconds. The server permits 60 seconds for a first connection. Both successful
client disconnect and timeout retain actual owned process waits/EOF before teardown.
A connected-client dwell-expiry case and the human SSH/VNC workflow remain untested.
