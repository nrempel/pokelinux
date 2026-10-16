# Actual native render-owner GL baseline

One prepared native x86 KVM epoch now executes the existing production CLEAR/RECT
leaves on a separate real GBM/EGL worker. Both offscreen8x6 RGBA8 comparisons check
all192 bytes and report zero mismatches. A real Rust Frame retains all96 source
pixel/padding bytes after producer poisoning/unmapping and original handle/store
release. These are actual GL/CPU controls; owned SHM upload and native-fence
behavior remain missing source interfaces. [Hash-only provenance](evidence/native-render-gl-baseline-provenance.json)
keeps this scope separate from Rust worker tests and the original Source RED.

## Selected native boundary

The fixture uses the exact earlier public Frame native33 ELF91848cff, its507-file
SDKc1775 and Rust Frame archive3cf0a142. It explicitly compiles the matching
production `OwnerFramePrimitives.cpp` into the test plugin because the main ELF
exports none of those methods. Actual compiler/linker and main-export relocation
checks pass. No newer Rust worker archive is selected and no second archive is
linked. The [published fixtures](../tools/native-render-owner/README.md) preserve
the executed actor and actual-header DRM metadata probe.

The actual renderer is virgl with an explicitly selected LLVMPIPE host simulation;
the guest reports OpenGL ES3.2 Mesa26.2.4. No fallback is attempted. The exact
UID/GID1000/no-extra-group probe opens renderD128 without changing permissions.
It separately observes the virtio-pci transport and libdrm's actual virtio_gpu
function driver. Pixel metadata qualifies1080x2424,65x146mm and target-window
DPR2.625. The8x6 FBO is an offscreen test target, not a resized phone preview or
physical Pixel GPU evidence.

The actor consumes one actual immutable PrFrame on its retained worker and calls
the real CLEAR/RECT leaves with matched shaders and copied final scissors. All15
bounded packets authenticate the exact compositor PID/UID/GID and one worker TID.
After worker cleanup/DONE, the controller observes socket EOF, requests public
plugin unload, obtains successful actual std::thread join and verifies task
absence. Context closure/program deletion is not asserted as GPU/device drain.
EGL native-fence extension/procedures are available, but the actor issues or
observes no native fence FD. Cases03–08 are structural interface absence, not semantic
GL behavior RED/GREEN or successful SHM/fence tests.

## Result-channel correction and retained attempts

Five earlier attempts stay failed. The first stops at32 unchanged SDK-header
warnings promoted to errors; the next preserves strict source warnings while
using system SDK include paths, then fails a missing Hyprutils link dependency.
Explicit installed Hyprutils linking repairs that setup. The third distinguishes
a wrong sysfs transport-level expectation from DRM driver identity; it fails
before open and proves no permission denial. The actual-header libdrm query then
qualifies the unchanged original-UID open.

The fourth and fifth overflow the unchanged262144-byte Observer output bound
while waiting for actor EOF. The fifth adds bounded pre-cleanup observation only:
it records all15 authenticated packets, exact96 bytes, CLEAR/RECT PASS, cleanup
and DONE, plus an absent finished worker task. A watched owned Xwayland process
still holds the exact sender socket FD. This directly identifies inherited test
channel custody, rather than a blocked GL worker. Full case/unload/join/normal
closure remain unqualified in those failed attempts.

The sixth creates the actor connection only at explicit plugin load, with atomic
CLOEXEC/nonblocking flags, exact root-controller peer credentials and one accepted
compositor-authenticated connection. No result sender FD exists at compositor
exec. Xwayland stays enabled. Rendering/capture/packet assertions, Source, output
cap,35/45s epoch bounds and the structural classifier stay unchanged. No claim is
made that CLOEXEC prevents a transient later fork without exec. The fresh complete
epoch now qualifies in24.805s, with actual EOF/unload/join/task absence.

## Source and closure scope

The original Source adapter, sampler, Observer and real Wayland client bytes are
retained. Live authenticated owned focus remains fresh before the actor and after
EOF/unload, sequence7→10, against the unchanged100ms lease. A later Observer
`failed: invalid data` record during deliberate client/compositor teardown is
preserved separately; this is not a claim that Source never fails or that the
cause of that teardown event is established.

The successful outer selection retains50 direct waits and57 PIPE EOFs within
original numeric cutoffs. Exact same-peer QMP confirms KVM and observes guest
SHUTDOWN, QMP EOF and QEMU0; normal cloud bootstrap and foreground CLI0 follow.
Exact container absence, same daemon and original locked input hashes qualify.
The private seatd and watched Xwayland helper processes are killed/reaped after
the completed live boundary; this is fixture disposal, not vendor/GPU cleanup.
Earlier output-overflow/failed-path Xvfb disposals remain failed evidence.

## Next connected work

The later [owner resource checkpoint](native-render-owner-resources.md) now
qualifies the small SHM/fence helpers and a separate sampler-state repair through
two actual RED/GREEN pairs. Next bind Rust bootstrap/ingress, copied discovery and frame
execution to one rebuilt AQ15/native compositor/SDK/plugin cohort, with complete
actual native routing authority and distinct startup/StopRetire permissions.
No competing AQ14/AQ15 C++ runtime is qualified by SONAME separation alone.

This baseline does not select the production Rust worker, commit/scanout,
presentation or secure lock output. The unchanged seven-case Source rendering
acceptance still has four passing controls and three rendering-hold failures.
Cold normal app/PCM, full service/session composition, ARM/matched Google kernel,
phone and full smartphone gates remain open. No physical phone was accessed.
