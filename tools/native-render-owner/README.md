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

## Later owned-resource component

The [owner-resource report](../../docs/native-render-owner-resources.md) records two
separate actual RED/GREEN pairs. `behavior-actor.cpp` is the exact first pair's
actor; `sampler-actor.cpp` adds one genuine nondefault sampler control while
preserving its eight oracles. The corresponding Python classifiers consume the
bounded authenticated controller receipts; they do not fabricate runtime facts.
The original baseline actor still selects its historical SDK and is separate.
Actor comments retain their original preimplementation wording as exact tested
inputs; the linked report records subsequent implementation and execution.

`native/` contains the four exact final tested native helper sources/headers.
Compile BOTH CPP files explicitly with the chosen actor. Put this directory at
`hyprland/src/render/gl/` in a separate ordinary include root and place that root
plus its local GL directory before the matching Frame507 system SDK includes.
Relative FramePlan/OpenGL/Rust headers resolve only to that exact SDK. Never
mutate the installed SDK, link another Rust archive, or claim that these local
headers make a rebuilt native cohort. Keep authored files under -Wall/-Wextra/
-Werror and the actual main-export/DSO relocation check. These native glue files
follow the pinned Hyprland source's licensing.

The holders are worker-only, noncopyable/nonmovable, and retain partial native
acquisitions. Failed workers stay held; an actual RED's owned-process reaping is
not native cleanup. GREEN needs real sync_file/POLLIN success before SHM readback,
local retirement then context cleanup/DONE/EOF/unload/join/task absence. Do not
unload the supplying plugin/library before actual worker join. The remaining
Rust executor C ABI and matched AQ15/native cohort are separate required work.

The [executor lifecycle checkpoint](../../docs/native-render-executor-gl.md)
adds `executor-handshake-oracle.py`, the exact classifier frozen before the new
worker actor. Its actual RED follows successful GL work with an unknown Stop
request. The GREEN branch requires authenticated Stop/finalize, actual join,
watch removal and release before unload; that branch remains unexecuted here.
