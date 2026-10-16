# Immutable frame CPU custody

The original [`frame` module](../native-render/src/frame.rs) captures bounded
immutable frame values and clones existing Rust snapshot ownership without
copying pixels again. The [public C header](../native-render/include/pokelinux_render_frame.h)
declares only the implemented capture, release and synchronous visitor functions.
This is a CPU custody prerequisite for the [rendering ownership work](native-render-authority.md).
It grants no native output authority. [Hash-only provenance](evidence/native-render-frame-provenance.json)
keeps the Rust, historical native build and new public-header selection separate.

## Implemented boundary

A frame owns copied target, draw, material, color-management, matrix and rectangle
values plus independently owned `Snapshot` values. Snapshot clones share their
immutable pixel allocation and its existing byte reservation. No client memory,
C++ scene pointer or callback survives capture. The codec flattens an authored
`SFramePlan` synchronously; refusal preserves the caller's prior captured result.

Admission checks the version, exact structure size, known flags, nonzero identities,
finite values, supported value ranges, exact resource descriptors and immutable
import sequences. Each SHM draw selects its exact indexed resource. The limits are
256 draws,256 resources,4,096 rectangles globally across damage/final damage/all
scissors, and1MiB checked metadata. Rectangle ranges must be valid and disjoint.
Frame flags are copied values, not commit permission; a future native owner must
separately authenticate its real operation and capabilities.

`pr_frame_with_view_v1` provides pointers only inside its immediate callback.
The callback/token are never queued; callers must not retain pointers, unwind or
release/transfer that frame during the call. An explicit snapshot clone obtains
independent ownership. `CSHMSnapshot::takeTransferred` consumes an incoming owned
handle on entry, including refusal; callers must not reuse or release that handle.

The new public header uses `PR_FRAME_ABI_V1` and four `PR_FRAME_*` status names.
It contains no executor/session/native-owner declarations or old-name aliases.
Its frame C type declarations exactly preserve the previously tested layout.
Existing snapshot `Box`/`Arc` metadata allocation can still abort on exhaustion;
this is not complete process out-of-memory recovery.

## Actual Rust qualification and chronology

The14 boundary tests were authored after the frame implementation draft.
The first actual run compiles all14 and passes13; one test expected incorrect
Draw-field offsets. Correcting that test expectation and formatting the source
produces14 passes, strict Clippy, format-check and x86 release-build success.
No production behavior was repaired and no strict TDD RED is claimed.

A later structural expectation is written before extracting the public header;
its initial missing-file failure is structural, not rendering behavior RED.
The new nominal header/ABI-name selection then reruns the same14 behavioral cases,
format-check, strict Clippy and release compilation successfully. It changes no
frame behavior or layout. Each CPU epoch retains six direct waits/six merged PIPE
EOFs, exact same-daemon container absence, unchanged readonly sources and original
cutoffs. Historical permit8 tests remain a separate result.

## Historical native FULL33 CPU boundary

A separate actual x86 KVM guest builds full Hyprland with33 replacement payloads,
all16 affected C++ units,454 configured command entries and its complete507-file
SDK. This historical selection uses the private proposed95339 header, whose
unused executor declarations are not published by this checkpoint. Five existing
snapshot C++/Rust groups and four real codec groups pass separately. The latter
check refusal preserving output; every value/matrix/global range after original
plan destruction; padded pixel custody after frame replacement and actual C++
thread join; and consume-on-entry refused transfer preserving prior output.

The codec actor supplies authored CPU plan values and calls the real linked
capture/visitor/snapshot/clone/adoption functions. Its thread is actually joined;
it exports no separate codec-thread TID/proc receipt. The original snapshot helper
retains its separate producer wait/pidfd/proc/ECHILD/EOF and thread custody records.
The native epoch closes35 Scope waits/35 merged PIPE EOFs, including early once,
with original cutoffs and same-peer QMP/KVM/guest shutdown/QEMU0/CLI0/exact-CID/
locked-input closure. No compositor executes in this build epoch.

## Actual native public-header selection

The [complete public patch](../tools/native-render-import/README.md) now has33
payloads and the [CPU actor](../tools/native-render-import/frame-codec-integration.cpp)
uses the implemented-only header. Only CMake and the two codec files differ from
historical FULL33; the other30 payloads and all four actor expectations are
unchanged. Source comparisons and patch applicability pass. The new header has
its own actual Rust CPU QA above and a separate successful native33 build:
all16 affected units,454 command entries and507 matching SDK files, including
the exact snapshot and public frame headers with no private executor header.
The same five FFI and four codec CPU groups pass. The codec process has actual
wait0/EOF under its original15s cutoff and takes7,993,913ns; its thread is joined,
without a separate codec-thread TID/proc receipt. This165.943350216s native epoch
closes35 waits/35 merged PIPE EOFs with early once and original QMP/KVM/shutdown/
QEMU0/CLI0/exact-CID/same-daemon/locked-input closure. Root and independent
qualification agree. These are fresh build/CPU results, not a compositor run.

The live scene producer and GL primitives compile but have not executed here.
No production render worker, native authorization, GPU upload/completion, fence,
output commit, presentation or secure lock frame is qualified. The unchanged
Source7 rendering RED, original cold full-composition failure and full smartphone,
ARM and phone gates remain open. The earlier no-attach/Cairo oracles were not
rerun in these frame build/CPU epochs.
