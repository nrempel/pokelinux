# Actual no-attach import identity boundary

These are the exact authored actors and oracle selected by the
[qualified native x86 RED](../../../docs/native-render-import.md#actual-no-attach-identity-red).
The baseline is one actual unassigned-surface case, separate from the five C++
FFI groups and seven Source/rendering-stall cases. Its stable-identity oracle is
frozen for baseline and repaired candidate.

- `real-wayland-client.snapshot.c` is an opt-in Wayland client. Invoke it with
  app ID, original absolute CLOCK_BOOTTIME cutoff and `--snapshot-profile` under
  an owned driver. The first stdin `U` creates an unassigned64×64 SHM surface,
  attaches XRGB8888/stride256 data and commits, then emits flush and actual
  `wl_display_sync` completion events carrying its surface ID. After the first
  sync, a second `U` issues damage/frame/commit without attach and another sync.
  `Q` requests a genuine disconnect; the driver must retain actual parent wait,
  pidfd/process disappearance and output EOF before its original cutoffs.
- `shm-snapshot-witness.cpp` is a test-only plugin built against the exact rebuilt
  compositor SDK and C header. Its `retain unassigned` request requires the actual
  client PID, a finite BOOTTIME deadline and real surface ID; it verifies the live
  UID1000 peer and retains one Rust clone. `verify-live` compares retained/current
  bytes and import descriptions; `verify-retained` checks the clone without a
  live-object query. `close` releases the clone. Its command prefix is
  `shm_snapshot_witness`; the no-attach epoch uses only the unassigned branch.
- `unassigned_identity_oracle.py` checks real ordered U/flush/sync/witness facts,
  same peer/surface, retained client buffer, unchanged bytes and stable import
  identity. It refuses setup/closure failures and never fabricates a RED.
- `noattach_epoch.py` joins that semantic result to actual owned epoch, client,
  compositor and descendant closure, selected Pixel metadata and RED/GREEN exit
  contracts. Its tuple/list normalization changes no identity expectation.

Use these only with a finite owned driver that provisions actual Hyprland, its
matching SDK, the exact Rust ABI and real Wayland clients. The C client needs
libwayland-client plus generated XDG-shell and ext-session-lock client headers
and protocol code. The plugin needs matching SDK/runtime hashes. This directory
supplies boundary actors/oracles; it does not supply the recorded private VM
controller/stager or a complete one-command runtime reproduction.

The actual RED changes import identity7→8 with the same16,384 bytes. The one-file
incoming-buffer repair passes its matching full native rebuild/five FFI groups and
actual unchanged-oracle GREEN: identity stays7→7 with the same bytes. Client,
witness, controller, semantic oracle and epoch join remain byte-identical to RED4.
A retained clone after actual client exit does not prove server-resource
retirement. Texture `ok()` is object validity, not upload or GPU
completion. Assigned window/cursor branches in these files are not qualified by
this one case. The separate [padded-cursor consumer case](../consumer/README.md)
qualifies CPU-consumer RED→GREEN separately; complete cursor output, real output readback/
presentation and secure lock display remain open. Source/Observer are not selected here, and the separate
seven-case rendering-stall RED, cold full owner and ARM/Google-kernel/phone matrix
remain unchanged. No binary, raw private receipt, process log or proprietary input
is published here.
