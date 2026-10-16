# Native owner SHM textures and fences

Two actual native x86 KVM RED/GREEN pairs now qualify worker-owned SHM textures
and native fences against a real EGL/GLES context. The final sampler successor
passes all eight cases, including six successful `sync_file` observations before
six exact 192-byte readbacks. Rust retains the immutable pixel/frame storage;
the C++ helpers issue native GL/EGL calls. The production Rust executor is not
selected in these runs. [Hash-only provenance](evidence/native-render-owner-resources-provenance.json)
keeps these component results separate from Source, ARM and phone evidence.

## Resource ownership

The [native helper sources](../tools/native-render-owner/native/OwnerFrameResources.hpp)
create empty holders on the graphics worker. Upload retains one explicit Rust
snapshot clone before GL and checks its full description, byte length and live
allocation identity. A different allocation with identical metadata cannot be
adopted as that texture. AR24/XR24 uploads preserve the supplied stride and padding;
GLES core swizzles map the little-endian B/G/R/A-or-X layout. XR24 forces alpha one.
The upload restores the active unit's texture binding, unpack PBO and pixel-store
state. An unaligned byte stride has a row-upload path, which is source-reviewed
but not exercised by the current stride32 runtime fixture.

Every operation checks its constructing thread and exact current display/context
before touching mutable resources or output arguments. Refused operations retain
custody. Partial upload/export failures retain acquired snapshot, texture, sync
and FD state for owner cleanup. Destructors require prior explicit local
retirement; they perform no GL work or implicit cross-thread cleanup.

Fence issue preserves native-sync creation with null attributes, GL flush, then
FD export. The holder immediately retains each acquisition, checks CLOEXEC and
binds the borrowed descriptor to six copied correlation fields. Borrowing proves
issue/export only. The independent actor requires actual `POLLIN` and positive
`SYNC_IOC_FILE_INFO` status before any SHM readback or local retirement. Already
signaled fences are valid. No `glFinish`, eventfd or readback substitutes for that
fact. This follows the [EGL native fence contract](https://registry.khronos.org/EGL/extensions/ANDROID/EGL_ANDROID_native_fence_sync.txt).

## Actual test chronology

The frozen behavior actor precedes implementation. The first incremental native
source implements upload/draw/owner guards and retains a created/flushed sync,
while export/ready publication is still unfinished. Its actual RED qualifies the
real backend's sync-file capability, retained 96 bytes, CLEAR/RECT readback and
SHM draw commands, then observes NGL-03 failure at the missing export boundary.
The worker remains retained. A fresh Source sample follows that failure before
controlled process disposal; there is no successful worker join or native cleanup.

Adding owned export/CLOEXEC/ready publication changes only `OwnerFrameResources.cpp`.
The identical actor, controller, classifier, shaders and Source then pass all eight
cases with 41 authenticated packets. The tests cover padded 5x3 AR24/XR24, nonzero
origin and cropped scissors, wrong thread, a genuinely different nonshared current
context, same-allocation clone acceptance, different-allocation and commit-sequence
refusal, plus a separately duplicated completed fence surviving helper retirement.

A separate sampler fixture then binds an actual mipmap sampler on texture unit0
while the uploaded texture has only level0. The unchanged pixel oracle observes
eight wrong bytes after genuine fence completion: a texture's filter parameters
can be overridden by the bound sampler. Explicit sampler-zero selection changes
only `OwnerFramePrimitives.cpp`. The same sampler actor/oracles then pass all eight
cases with 42 packets and zero mismatches in all six SHM readbacks. This second
pair is distinct from the earlier missing-export pair.

GREEN closure observes local resource retirement and context cleanup, DONE,
socket EOF, public plugin unload, actual thread join and worker-task absence.
Each of the four epochs retains 50 direct waits and 57 PIPE EOFs under the original
cutoffs, same-peer QMP/KVM guest shutdown, QEMU/ordinary bootstrap/foreground CLI0,
exact container absence and unchanged locked inputs. Authenticated live Source
samples qualify the requested before/after windows only. Later terminal Observer
records remain preserved; their exact time and cause are not inferred.

## Selected boundary and remaining work

These runs keep the earlier Frame ELF91848cff, matching 507-file SDKc1775 and
Rust3cf0a142. Four local native sources/headers compile explicitly into the test
plugin, with actual compiler and relocation checks. The installed SDK and main
ELF are unchanged; this is not a rebuilt production compositor cohort. The actual
renderer is virgl with an explicitly selected LLVMPIPE software host. Pixel10
metadata qualifies 1080x2424 and target-window DPR2.625; the 8x6 FBO is an offscreen
oracle, not a phone preview or physical GPU result.

Next expose the Rust executor through a thin C ABI and worker-created native
holder, then connect this real GL operation. Frame submission must consume the caller's handle
only on accepted admission; refusal retains it. Move a typed factory record
containing only copied functions and bounded scalar options to the worker; keep
owned bootstrap separate and retain the supplying library through actual join.
No control-created native pointer or userdata crosses that factory boundary.
Build one matching AQ15, Hyprland, Rust, SDK and plugin cohort before selecting
the new Rust archive in the native GL case. Then connect ordered session ingress,
revocation, genuine output/routing authority and distinct startup/StopRetire
permissions. AQ14/AQ15 SONAME separation alone does not qualify a mixed runtime.

Actual output commit, scanout, presentation, GPU-wide/device retirement, secure
lock output, blocked-driver recovery, the unchanged seven-case Source GREEN,
cold normal app/PCM and the full smartphone matrix remain open. Odd-stride and
fractional-alpha rendering need separate evidence. No ARM, matched Google kernel
or physical-phone result follows; no phone was accessed.
