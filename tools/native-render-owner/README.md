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
binding of the current Rust executor C ABI to native GL remains separate work.

The [executor lifecycle checkpoint](../../docs/native-render-executor-gl.md)
adds `executor-handshake-oracle.py`, the exact classifier frozen before the new
worker actor. Its actual RED follows successful GL work with an unknown Stop
request. The GREEN branch requires authenticated Stop/finalize, actual join,
watch removal and release before unload; that branch remains unexecuted here.


## Exact AQ15 and compile-only Hyprland successor sources

aquamarine.patch applies to public Aquamarine
[f31c47a1b9d300847d8fc3108ad959448103dfc1](https://github.com/hyprwm/aquamarine/tree/f31c47a1b9d300847d8fc3108ad959448103dfc1).
aquamarine-cohort.json binds the actual83-file/16-change candidate and six
compiled CPP units. The public patch reconstructs82 files with14 build changes:
it omits a non-build private planning note and retains the upstream README rather
than private preimplementation notes. Both exceptions are explicit hash-only
records; all compiled source, headers, CMake and pkg-config bytes match the actual
build. Preserve the upstream executable data/hwdata.sh mode. The actual standalone AQ build/SDK and ten historical CPU groups qualify
real library methods and character-FD handling, not libseat/DRM/atomic display
execution, Root Rust authority or complete device retirement. The source follows
upstream licensing; no binary or raw private receipt is supplied.

hyprland-executor-aq15.patch is incremental after the exact patch in
native-render-import; its cohort binds the parent patch, sole CMake delta and
all 33 final source hashes. It retains/exports all 23 current Rust C entrypoints
explicitly, requires all three matching public headers and checks the actual
AQ15 imported target/version/include prefix. Supply the explicit absolute
POKELINUX_RENDER_STATIC_LIBRARY, POKELINUX_RENDER_INCLUDE_DIR,
POKELINUX_RENDER_NATIVE_LIBRARIES and POKELINUX_AQ15_ROOT from a matched build.
CMake performs no Cargo or network work. AQ15 has no ABI14 compatibility alias.

Build this independent patched AQ source into a selected prefix first. During
Hyprland configure, put its actual lib/pkgconfig directory first in
PKG_CONFIG_PATH and verify the selected pcfiledir, libdir, includes and version.
The qualified SDK packages AQ15 under usr/lib/pokelinux-aq15 and puts its derived
relocatable pc in usr/lib/pokelinux-aq15/pkgconfig. Later plugin commands must
explicitly prepend that directory to PKG_CONFIG_PATH; system AQ14 pc search paths
must not take priority. Main uses $ORIGIN/../lib/pokelinux-aq15 RUNPATH. Require
actual DT_NEEDED/loader closure with AQ15 only. Resolve Rust C functions from
matching main; never link a second Rust archive into the plugin. Keep the four
native resource helper sources as explicit matching plugin inputs.

The [actual matching build and subsequent startup failure](../../docs/native-render-executor-gl.md#separate-aq15current-abi-build-and-startup-failure)
qualify compilation, CPU reruns and loader closure only. The new selected main
aborts before Source/actor, with zero GL cases; its exact abort branch/cause is
unresolved. This separate successor does not replace the frozen Frame507 import
cohort or provide a fallback. Actual broker/render startup, native worker
handshake/cleanup, Source7 and every full-runtime/phone gate remain required.


executor-actor.cpp is the exact new Rust-executor offscreen fixture. Both later
attempts compile it; the second passes actual main/DSO relocation checks after
adding explicit Wayland-server linkage. Compile with the matching main/SDK and
both native CPP helpers, retaining authored -Wall/-Wextra/-Werror flags and SDK
system includes. Link GBM/EGL/GLESv2, Hyprutils, pthread and explicitly
pkg-config --libs wayland-server. Resolve the current Rust C functions from the
matching main, with no second archive. The plugin has no direct AQ DT_NEEDED;
AQ15 is checked in the main's actual loader closure.

This actor creates no output or connector identity: its owned bootstrap has zero
outputs and tests offscreen native work inside the Rust worker only. It exercises
no frame-admission/native-display grant here. Controlled pre-unload calls Stop,
then nonblocking finalize until actual Rust join, removes both borrowed watches,
releases the owner, and only then unloads. Unexpected unload aborts instead of
unloading live supplier code. The matched compositor aborts before actor startup,
so these runtime GL/handshake contracts and the frozen GREEN oracle are still
unreached. No source change or compiler success is relabelled execution evidence.

## Initial broker batch successor

`aquamarine-initial-batch.patch` is incremental after `aquamarine.patch`; its
cohort pins exactly three changed files. The const peek supplies borrowed values
for Rust capture before queue admission; refusals preserve output bytes and
queue/descriptor custody. `initial-batch-consumer.cpp` and its runtime contract
are the exact frozen inputs, and `initial-batch-compile-oracle.py` is the executed
structural-RED classifier. The [report](../../docs/native-render-initial-batch.md)
records actual missing-API RED and matching native source/CPU build separately.
The unchanged real-broker consumer now passes with one virtual DRM graphics lease,
three events and two pending duplicates closed after Rust capture. C++ compilation
force-includes `initial-batch-c-headers.hpp` for real libseat C linkage; this adds
no function wrapper/interposer. `initial-batch-runtime-oracle.py` records the
executed separate runtime predicate. Preserve the historical AQ15
and Frame fixtures; no compatibility or runtime fallback is added.

The [owned startup checkpoint](../../docs/native-render-owned-startup.md) adds
`main-startup-existing-header-control.cpp`, `main-startup-public-member.cpp` and
`main-startup-compile-oracle.py`, the exact executed original-source structural
RED inputs/classifier. `main-startup-runtime-oracle.py` is separately frozen
source only; no execution or implementation GREEN is claimed. Actual deferred
factory acquisition, failure custody and main worker startup remain required.

The same report separately qualifies `aq-retained-initialization-header-control.cpp`,
`aq-retained-initialization-members.cpp` and
`aq-retained-initialization-compile-oracle.py` against the actual historical SDK.
Both missing API names are mandatory. `main-startup-runtime-oracle.py` also binds
retained-before-initialize ordering and bounded EOF-framed startup control; it
remains source only. Neither structural compile RED proves native failure custody.

`aquamarine-retained-initialization.patch` applies after `aquamarine.patch` and
`aquamarine-initial-batch.patch`; its cohort pins the exact two changed files,
compiled library/SDK and unchanged frozen interface consumers. It removes the
old factory without an alias and splits empty outer-holder creation from native
initialization. The matching build passes the ten existing CPU groups and public
API syntax checks. Actual partial-child, mandatory-start and subordinate-factory
failure custody remain deliberately unqualified for the next behavioral RED.

`aquamarine-startup-custody-test-hooks.patch` applies next, after the retained
initialization patch. Its matching cohort pins the dedicated
`POKELINUX_STARTUP_CUSTODY_TEST` build, seven affected C++ objects, nine interfaces
(11 function symbols), the generated SDK and the address probe's actual AQ15
relocation closure. `aq-startup-custody-hook-link-probe.cpp` is the exact compiled
address-only probe; its functions and the hook callbacks were not invoked. Ten
unchanged CPU groups and both frozen API syntax contracts pass, with 57 original
waits/PIPE EOFs. This publishes test infrastructure and build evidence only;
actual native failure RED, child/start/subordinate-factory custody and matching
main/Rust worker execution remain open. Keep the inherited non-build source
exceptions in `aquamarine-cohort.json`; no private planning note is needed to
reconstruct the compiled sources or SDK headers.
