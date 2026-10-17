# Exact ordinary-scanout resource operations — frozen independent tests

SOURCE ONLY / UNRUN. Root approved the concrete signatures in
sdk-declarations.proposed.patch for independent test finalization. That patch
and Exact.hpp.proposed are review artifacts, never structural RED inputs. No
production source, SDK, compiler, stage, VM or runtime is changed by this package.
Actual GNU16/SDK/runner selections remain pending Root review.

The strict operation path reuses AQ swapchain, GBM buffer and CDRMFB ownership.
It adds primitive dispatch/raw copied receipts, not portable format policy or a
buffer-manager framework. Root's current Rust selector still refuses implicit;
these fixture requests do not grant production eligibility. A buffer_id names a
retained native allocation occurrence, never a Rust epoch, grant or owner proof.
All exact options/request equality includes the live weak output identity.
Native allocator and KMS descriptors must name the same device by successful
actual drmGetDevice2 plus drmDevicesEqual, not equal rdev. Both rdevs are raw
separate facts. Query failure/mismatch refuses, never selects another device.

Initial contract is ordinary scanout: concrete positive integral size/format,
scanout=true, cursor=false, multigpu=false, live matching DRM output. No format
inference, option rewriting, output substitution, retry or fallback. Implicit
uses one gbm_bo_create; Explicit uses one gbm_bo_create_with_modifiers2 with one
exact modifier. cap0 Explicit scanout is an operation precondition refusal with
zero allocation/import calls. Always preserve the raw modifier getter separately
from exported attrs. A concrete implicit getter does not change request mode.
Explicit raw-getter discrepancy is a factual failure; retain its new buffer in
swapchain cleanup custody even when the previous swapchain remains unchanged.
Do not normalize INVALID to LINEAR. Failure of reconfigureExact preserves the
old committed buffers/options; partially acquired candidates remain owned until
explicit cleanup/process disposal. Reuse is allowed only for whole exact
request/options identity and reports zero new native allocation attempts.

Import requires this retained exact buffer/request/device. Implicit performs
AddFB2 flags0; Explicit uses the modifier entrypoint with MODIFIERS, no inference
from exported modifier. Same exact attachment reuse reports zero new calls;
legacy/mismatched attachments refuse. The explicit cap0 operation refusal is not
permission for implicit or another modifier. No implicit eligibility bool exists.

Caller keeps SP<CDRMFB> alive through closeExact and receipt inspection. At most
one real CLOSEFB per registration; subsequent closeExact copies the prior result.
Never retry uncertain CLOSEFB or silently RMFB, including strict ordinary
object destruction. A failed/unknown close remains visible and retained in this
consumer until bounded owned-process disposal; destruction cannot claim to keep
a shared pointer alive. This narrow never-attached case can qualify registration
release; it cannot qualify active scanout retirement or all AQ/GBM descendants.

## Structural missing-interface RED

current-sdk-control.cpp and missing-interface.cpp include only genuine existing
AQ headers. No Exact.hpp.proposed/header overlay, archive, link or method body.
Run actual GNU16 version first, then exactly two syntax-only commands using the
original strict flags in structural-commands.json, LC_ALL=C and actual -MD full
header closure. The source manifest's genuine header pins must match selected
SDK leaves; all transitive dependencies must lie in the authenticated SDK/system
closure. A proposed header is forbidden even if it would make syntax pass.

Require actual control wait0 and zero stdout/stderr bytes. Then target wait1
with exactly six missing names at frozen lines4–9: SExactBufferRequest,
reconfigureExact, exactAllocation, createExact, exactImport, closeExact. All six
mandatory, no fmax-errors limit. Classifier rejects setup diagnostics, warnings,
other errors, truncation and unknown suggestions. A qualified missing-interface
RED remains aggregate FAILED and proves no allocation/import behavior.

## Actual kernel consumer, after reviewed implementation/build selection

exact-buffer-kernel.cpp is independent test infrastructure. It obtains genuine
broker events, uses actual Rust bootstrap capture before broker admission, and
runs real AQ mirror/backend/allocator objects on the genuine Rust-owned worker.
The operations execute in this TEST BOOTSTRAP callback, not production prepare
or execute. It returns NATIVE deliberately, publishing no discovery/Ready/frame.
This is not a replacement worker or a passing normal startup surrogate.

Five distinct operation groups, in frozen order:

1. Actual bound KMS/render native device, cap0; one length1 64x64 XR24 ordinary
   scanout Implicit allocation. One gbm_bo_create, exact flags, raw getter/export
   separately, live export FDs and retained native buffer receipt.
2. Explicit LINEAR request on that SAME positive swapchain with identical
   dimensions/format refuses at actual cap0, with no GBM/import fallback. The
   old buffer remains identical; this catches the old size/format-only no-op.
3. With the positive real buffer, mismatched request ID/mode/reserved implicit
   modifier refuse before PRIME/ADDFB, preserving raw export metadata.
4. Same-buffer Implicit import uses actual AddFB2 flags0. Independent GETFB2
   matches size/fourcc/pitches/offsets. Preserve returned flags/modifier raw:
   GETFB2 may disclose a concrete modifier even after input flags0. Close every
   unique fresh GETFB2 GEM handle exactly once; zero handles on nonmaster is
   recorded honestly. Same exact attachment reuse has zero new import calls.
5. Explicit owned CLOSEFB while both pointers remain retained, result copy with
   no retry, real FBID absent, original raw export unchanged, then buffer release
   and actual exported FDs EBADF. No plane attachment/atomic/pageflip is issued.

Every resource call runs under existing denyDisplay, requiring bound scope and
zero display requests. Allocation/export/PRIME/ADDFB2/GETFB2 acquire resources;
they do not consume PrAttempt or add an import permission kind. The one-shot real
seal remains at actual ATOMIC/external-lease display mutation. No EGL import,
rendering, scanout, presentation, full nativeReady, Source, normal app or phone
claim follows. Explicit allocation success/raw-discrepancy branches are not
exercised by this cap0 label; source enforcement remains a separate future gate.

Native GBM/import/driver/getter/environment unsupported failures are preserved
as setup failures, never tuple substitution or feature RED. Raw attempted
allocation/import/close results are logged before success assertions. Typed
refusal/no-call/mismatch violations after a real positive baseline are contract
failures. Pure classifier preserves the distinction; no expected result is
invented by static checks. Source-bound receipts are not independently traced
GBM/ioctl entrypoints. Actual GETFB2 is the independent kernel metadata readback.

## Bounds, custody and unfinished production gates

Keep original envelope: 35s work / 45s external close / 2s request / 20s startup,
original owned KVM/QMP/CID/direct-wait/EOF/input-closure resources and no extra
capabilities. Consumer requires completion inside that original startup cutoff.
Actual VM/compiler command pins, binaries, SDK and runtime are not selected here.
No library/callback execution occurs during this source preparation.

The consumer honestly reports whole_native_retirement=false and joined=false:
it retains the real AQ graph and ends by bounded process disposal rather than
claiming that the current unimplemented shutdown aggregate became complete.
The external owned parent must directly wait this process, retain stdout/stderr
EOF and PID/cgroup/VM/CID closure. Process exit never upgrades native retirement.
No detached worker/IO or blocking Rust Drop is used; source _Exit is intent until
actual parent wait. Partial operation groups remain counted/raw even on failure.
Production prepare/execute wiring, full retirement, actual frame and the first
integrated normal app session remain separate required next steps.

Primary contracts:
- https://www.kernel.org/doc/html/v6.12/gpu/drm-kms.html
- https://kernel.org/doc/html/v6.16/gpu/drm-uapi.html
- https://github.com/torvalds/linux/blob/master/drivers/gpu/drm/drm_framebuffer.c
- https://android.googlesource.com/platform/external/libdrm/+/master/xf86drm.c
  drmDevicesEqual compares native bus identities; rdev equality is insufficient.
