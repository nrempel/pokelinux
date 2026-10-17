# Same-owner format facts: source-only test proposal

No production source edit, compiler, staging or runtime is authorized by this
package. Root reviews/freezes the actual test before its fresh chronological
missing-diagnostic RED. Completed GREEN1 remains FAILED/SETUP_UNQUALIFIED; it is
not retrospectively classified by this new test. The existing native wire target
and all of its assertions remain unchanged and failed while a separate diagnostic
target may eventually pass. No Ready/frame/Source/hardware/phone claim.

Actual retained GREEN1: normal main reports primary-format-modifier-unavailable,
errno0, connector40. First authenticated snapshot is healthy stage8 at
8,580,422,302ns. Second is UNAVAILABLE/stage5/error9 at8,844,077,541ns; actual
firstStop origin11/error9 was8,668,834,183ns, before28,560,329,122ns startup cutoff.
No selected tuple, primary plane, IN_FORMATS state or kernel cap was recorded.
AQ's retained source seeds each noncursor format's modifiers with LINEAR then
INVALID and returns its native primary format vector. Thus source predicts first
LINEAR0; it is NOT an observation of this failed run. AQ_NO_MODIFIERS=1 changes AQ's
internal path flag, not the kernel cap. These are distinct facts to retain.

Minimal proposed observation addition, only after a genuine diagnostic RED:
use the existing normal logger inside the same actual worker callback and same
owned primary DRM FD, under the existing denyDisplay scope. No callback/API/table
change, extra observer process, FD substitution or different-device drm open.
Collect fixed numeric/bool metadata during the existing query and add the one
read-only drmGetCap(DRM_CAP_ADDFB2_MODIFIERS). Emit one bounded single-line JSON
packet with marker `owned-device-change-format-v1: ` immediately before the
UNCHANGED aggregate predicate refusal. No retry, alternate tuple selection,
implicit modifier fallback, setter, KMS/display/FB operation, or recovery policy.
If a primary-format query unexpectedly succeeds, the target is unqualified rather
than inventing the expected refusal. The original failure packet remains too.

Packet exact schema (oracle file defines exact keys/types): schema1; actual worker
TID, event sequence/session generation, connector ID and fd fstat rdev; selected
fourcc/modifier as unmodified u32/u64; DRM cap actual return/errno/value (null when
not returned); CRTC count, current connector encoder/current CRTC IDs, connector
compatible CRTC mask; total plane count and bounded list of each returned plane's
ID/current CRTC/mask/selected-format membership/count. For route-compatible planes
record actual `type` property presence/value, IN_FORMATS property presence and
blob ID, blob query errno/length and exact selected-pair membership (null when no
readable blob); counts only, no full format/modifier/blob dump. Non-compatible
planes carry null for unqueried property/blob fields. Record actual no-display
checks before/after query. Completion=true requires the bounded enumeration and
all applicable queries to have completed; a real query error is retained as setup
failure, never represented as absence. Do not call cap0 or property absence an
error. No packet or truncated/incomplete packet is not capability proof.

Use existing limits: <=128 planes/properties, <=4096 formats, <=1MiB blob, <=32KiB
one diagnostic packet within existing2MiB normal-main log; no budget increase.
The original35s work/45s close/2s public request deadlines,20s startup, guest6CPU/
6144MiB, container8CPU/10GiB, QEMU/devices/image/caps/locks and exact owned cleanup
remain unchanged. Emit only at the terminal format predicate refusal, naturally
once because native uncertain apply is not retried. The diagnostic is independent
of actual native readiness. Do not infer AQ's private chosen primary plane ID
from a compatible kernel-plane list; no AQ API expansion is needed for these facts.

Frozen test wrapper calls the unchanged actual native wire case, retains its full
result, and reads the owned normal-main log only after original disposal/closure.
It requires healthy authenticated bootstrap, the observed error9/aggregate refusal
before original20s, original bound direct waits/EOF and descendant closure. With
those prerequisites, absent packet is fresh DIAGNOSTIC_FACTS_MISSING_RED. A present
complete coherent packet yields DIAGNOSTIC_FACTS_PASS only when the unchanged
format predicate is still false: no compatible primary plane has the selected
format AND a readable blob matching the selected pair. Counterexamples are test
failures, not reasons to relax the predicate. Failed/error/oversize/incomplete
queries remain unqualified setup. A later Root repair requires a separately
frozen actual boundary test and new actual implementation/qualification.

Primary contracts: IN_FORMATS enumerates explicit format/modifier pairs. Without
it explicit modifier buffers are not supported; cap ADDFB2_MODIFIERS=1 requires the
property on all planes, including linear-only ones. Explicit LINEAR0 does not mean
implicit/unspecified layout; INVALID is a sentinel, not a claim of linear support.
- https://www.kernel.org/doc/html/v6.12/gpu/drm-kms.html#plane-composition-properties
- https://cdn.kernel.org/doc/html/latest/gpu/drm-kms.html#plane-composition-properties
- https://github.com/torvalds/linux/blob/v6.12/include/uapi/drm/drm_fourcc.h
These public contracts are not runtime facts about this guest's7.2.9 kernel cap.
