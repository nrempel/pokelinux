# Actual compositor rendering-stall regression actors

These are the exact authored actors and result oracle used by the
[prepared native x86 rendering-stall RED](../../docs/native-render-authority.md).
The actual original Hyprland compositor and current Rust Observer execute the
boundary. The test hook holds a genuine native-fence EGL call; actual Wayland
clients issue focus, disconnect and session-lock operations. Python never writes
or relays Source packets.

- `render-stall-hook.cpp` builds as a test-only Hyprland plugin against the exact
  selected compositor SDK. Its Root-owned private SEQPACKET protocol arms one
  finite hold and records real entry, release and delegated return. Installation
  requires serialized/quiescent render ownership. RETURN does not prove wrapper
  quiescence; an active plugin cannot be unloaded safely.
- `real-wayland-client.c` uses generated XDG-shell and ext-session-lock client
  headers/code plus libwayland-client. It maps a real SHM toplevel and accepts
  D (damage), L (native lock) and Q (disconnect) on its owned stdin. Its arguments
  are app ID and the original absolute CLOCK_BOOTTIME cutoff in nanoseconds.
- `send-focus-request.py` issues one real current-Lua Hyprland public focus
  dispatch, verifies the exact compositor peer and records both issuance and the
  actual reply. The result oracle requires that same reply to be `ok`.
- `render_epoch.py` checks all seven ordered cases, actual preparation and
  operation validity, the full unchanged 100ms post-operation interval inside the
  rendering hold, terminal expiry under control-owner stall, and owned cleanup.
  Expected-RED qualification requires actual behavioral failure; setup and
  cleanup errors cannot qualify it.

Run these only under a finite owned integration driver that provisions the
selected compositor/SDK, current Rust Observer, unchanged Source adapter/sampler,
real Pixel 10 virtual metadata, private control socket and explicit child/fd
custody. The recorded VM driver, selection and closure receipts are pinned by
hash in the report; this directory supplies the boundary actors and oracle.
No binary, VM disk, raw process log or private runtime receipt is published here.

The baseline has three no-stall passes, three entered-rendering-hold failures and
one expected control-owner-expiry pass. It supplies the RED before the ownership
refactor. A future candidate must retain these exact actor/oracle bytes and also
qualify real GL/context/buffer/fence/output lifetime, late completion and shutdown,
then the unchanged cold full-owner/Sounds/Pixel/graph/PCM runtime. A valid EGLSync
return does not establish GPU completion; native protocol lock events do not
establish secure presentation. All ARM, Google-kernel and phone gates remain
separate. No physical phone was accessed.
