# Native render-owner boundary fixtures

These original C++ fixtures are test infrastructure for the prepared native x86
KVM environment, not production plugins or phone qualification. The actual
[baseline report](../../docs/native-render-gl-baseline.md) records their exact
matching native compositor/SDK and finite owned guest/controller selection.

`baseline-actor.cpp` captures one real immutable Rust Frame, drops/poisons its
producer and original handles, then transfers it to a joined worker. That worker
uses real GBM/EGL and the existing production CLEAR/RECT leaves for exact8x6
RGBA8 readback. It checks96 retained snapshot bytes, including padding, then
reports missing owner SHM/fence interfaces as structural absence. It refuses an
SDK that declares future resource helpers. No repaired-helper behavior is tested.

Compile against the exact507-file public Frame SDK selected by the report. Link
the exact production `src/render/gl/OwnerFramePrimitives.cpp` from the same33-file
source cohort into the plugin: the selected main ELF exports none of its methods.
Use GNU++26, shared/PIC/O2, `-Wall -Wextra -Werror`, the actual pkg-config Hyprland
flags, GBM/EGL/GLESv2 and explicit Hyprutils libraries plus pthread. SDK directories
are system search paths; this preserves their original bytes and retains strict
warnings in the actor/production CPP. Actual relocation output must resolve
against that plugin's DSOs and the matching main ELF before loading. Never link
a second Rust archive or overlay another SDK. This is not a standalone load test.

The test controller creates a private UNIX SEQPACKET listener and supplies
`POKELINUX_OWNER_GL_RESULT_SOCKET` and `POKELINUX_OWNER_GL_RESULT_PID`. At explicit
plugin load the actor opens its own CLOEXEC/nonblocking connection and verifies
the exact controller PID with root UID/GID. Controller acceptance and every
packet authenticate the actual compositor PID/UID/GID. No actor result FD passes
through compositor startup. The finite controller must drain packets during
public IPC, then require DONE, actual socket EOF, public unload/worker join and
worker-task absence. A failed worker must never be detached or unloaded alive.

`render-node-inventory.cpp` is a separate strict C++23/libdrm metadata probe run
as UID/GID1000 with no extra groups. It opens only selected guest renderD128 with
O_RDWR/CLOEXEC/NOFOLLOW. It records sysfs PCI transport separately from the actual
`drmGetVersion` function driver. Neither descriptor identity nor a successful
metadata query authenticates real native output authority or physical hardware.

The original Source adapter/sampler/Observer/client,100ms Source lease,35/45s
per-epoch cutoffs, Pixel10 profile and outer guest/QMP/KVM/process/input bounds
remain mandatory. Shader inputs are the four exact pinned original files in the
report. Environment/path/UID/capability failures are setup failures, not target
RED or a fallback. Native-fence extension/procedure availability alone qualifies
no native FD/completion. The8x6 target is an offscreen oracle, not a UI preview.
