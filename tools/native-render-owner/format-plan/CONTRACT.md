# Format operation plan ABI2 — frozen independent contract

Root accepted this contract before independent test freeze. No production edit, compile, stage or runtime. Replace select_v1 and
FactsV1/SelectionV1 entirely: one current select_v2, FactsV2, SelectionV2, ABI2.
The six raw V1 context/native/pair/plane/property/cap types remain byte-identical.
No compatibility alias, old symbol, new framework or eligibility bit.

LP64 layout (x86_64/AArch64; final tests assert every field): CallV1 size16/align4;
FactsV2 size312/align8, all old fact-prefix offsets unchanged through reserved at
208; allocator_kind224, output_backend_kind228, width232, height236,
scanout240, cursor244, multigpu248, ordinary_reserved252,
allocator_device_query256, kms_device_query272, allocator_rdev288, kms_rdev296,
devices_equal304, devices_equal_count308. Count is factual0/1, not eligibility. SelectionV2 size112/align8:
context16, plane_id80, drm_format84, refusal_source88, buffer_mode92,
modifier96, width104, height108. Neither LP64 size is an invocation witness.

Output decision REFUSED1 or PREPARE_ATTEMPT2; modes NONE0/IMPLICIT1/EXPLICIT2.
Plan copies context exactly and carries the first fourcc, an observed corroborating plane witness,
input dimensions and exact mode. Implicit modifier0 is RESERVED, never LINEAR
proof. Explicit uses the exact first modifier, including explicit LINEAR0 when
actually advertised. Refusal has modeNONE/dimensions/format/modifier0 and may
retain the plane refusal witness. PREPARE_ATTEMPT requests the selected
primitive once; it is not permission, native success, admission authority, Ready or presentation.
The outer Rust executor retains all epoch/deadline/availability/grant policy.

Validation always precedes policy over ALL input arrays, including later unused
native groups and modifier/pair arrays ignored by the implicit decision. Preserve
existing bounded counts, spans, pointer/null/alignment/overflow/nonaliasing,
unique IDs, state/payload consistency and zero reserved checks. ABI/size mismatch,
malformed shape/state, reserved, devices_equal_count outside0/1, option booleans outside
0/1, width/height outside1..16384 => INVALID atomic unchanged output. Count above
existing max => CAPACITY atomic unchanged output. Inputs always unchanged.
A CallV1 UNKNOWN or OK has ret0/errno0; ERROR has nonzero ret/errno>=0.
Count0 requires raw devices_equal0; count1 requires both device queries READ_OK
and one actual drmDevicesEqual invocation. These consistency errors are atomic
INVALID, not unknown observations. Successful drmGetDevice2 with a null returned
pointer cannot cause a comparison: preserve count0, never invent mismatch.
Any representable devices_equal integer is accepted with count1, then non0/1
becomes structured INCONSISTENT. Old Context validation, including rdev0, stays.

Deterministic policy order after full validation:

1. Backend kind0 => QUERY_UNKNOWN/BACKEND; observed kind other than1 =>
   BACKEND_UNSUPPORTED/BACKEND. No fallback to another backend.
2. Valid boolean options other than scanout1/cursor0/multigpu0 =>
   OPTIONS_UNSUPPORTED/OPTIONS. This is structured refusal, not malformed memory.
3. Allocator then KMS device query UNKNOWN/ERROR => QUERY_UNKNOWN/QUERY_ERROR
   with its respective source. After successful queries, comparison count0 =>
   QUERY_UNKNOWN/DEVICE. Count1 rawreturn0 => DEVICE_MISMATCH/DEVICE; rawreturn1
   is the observed match; any other rawreturn => INCONSISTENT/DEVICE. Require
   kms_rdev == context.rdev; mismatch => INCONSISTENT/CONTEXT. Numeric rdev0
   remains representable; successful queries and actual comparison establish
   the device fact. Render/primary rdev equality is NOT required or sufficient.
4. Empty native groups => EMPTY_NATIVE. Require observed cap success; unknown/
   error retain corresponding refusal. cap not0/1 => INCONSISTENT/CAP.
5. cap1: first group's first modifier mandatory; empty => EMPTY_FIRST_MODIFIERS;
   INVALID sentinel => EXPLICIT_MODIFIER_INVALID. Do not scan a later modifier
   or format for success. cap0: first fourcc only, first modifier list may be
   empty; raw mod0/INVALID/other values remain untouched and do not choose mode.
6. Reuse exact old route/current-encoder/compatible CRTC computation and ordered
   plane scan. Unknown/error plane query refuses; nonroute planes skip. First
   fourcc must be in the raw legacy list; type must be observed primary1.
   Missing/unknown/error type refuses; known nonprimary skips.
7. IN_FORMATS UNKNOWN/ERROR refuses in BOTH modes. Known absent is allowed only
   for implicit. PRESENT requires nonzero blob ID and complete bounded decoded
   blob in BOTH modes; incomplete/unknown/error decoding refuses. Implicit does
   not require an explicit pair match; explicit requires the exact first tuple.
   No matching candidate => NO_IMPLICIT_MATCH or original NO_EXPLICIT_MATCH.

No synthetic implicit_supported flag, forced LINEAR or native automatic fallback.
A cap0 plan is chosen before GBM, not after a failed explicit operation. Production
uses the pinned exact SDK/library and verifies actual same-request allocation/
import receipts before preparation succeeds. This authored CPU policy contract
is distinct from the separate actual kernel GBM/AddFB2 consumer.

Collector overflow cannot truncate observations or advertise memory it does not
own: return existing callback CAPACITY before selector call. Existing raw blob
state already covers complete bounded decoding. A decoding error with no native
errno can be ERROR/errno0/zero length+pair span; never invent an API errno or
turn failed property enumeration into known absence. No extra collector state.

After Root accepts, migrate the meaningful existing four C++ groups to ABI2:
explicit first-tuple/owned-result; missing-vs-unknown-vs-error; raw routes/primary/
legacy/pairs; whole-input bounds/alignment/atomic refusal. Replace obsolete cap0
refusal assertions with implicit plans. Add two focused groups: ordinary implicit
(including empty modifiers, raw INVALID/nonlinear, absent/present-complete blob,
no pair requirement/no forced LINEAR) and backend/device/context/options refusal.
All successful output bytes, unchanged caller storage and prior-result overwrite
remain exact. Keep original5s work/8s close, all other five C++ programs and64 Rust
regression tests. Current pending consumer has no actual native/kernel claim.

Structural RED uses current genuine four headers only. Existing raw-view control
must compile0/zero diagnostics; tiny target independently names exactly
PrFormatFactsV2, PrFormatSelectionV2 and pr_format_select_v2, so actual current
header must fail with exactly those three missing names. Proposed header is
forbidden in RED. Keep existing GCC14.2/C++20/profile/owned runner; this does not
freeze or authorize the run. Header, final tests/classifier freeze follows Root
signature/contract approval; no implementation until actual qualified RED.

Native kind mapping is explicit: AQ GBM raw0 maps plan-kindGBM1; observed
DRM_DUMB maps a non1 unsupported value, never unknown. Likewise observed
WAYLAND0 must map to a known unsupported non0/non1 kind, not UNKNOWN0. Unknown
is reserved for unobserved type. The raw device query/comparison are facts, not
a precomputed backend eligibility boolean. Context lease/gen/rdev comes from
the matched retained mirror ADD identity; kms/allocator rdevs are independent
fstat observations. CHANGE sequence is actual event sequence; unrelated output
contexts must not copy that event's device identity. Plane witness is not proof
AQ selected that route or plane. No authentication/lifecycle guarantee follows.
