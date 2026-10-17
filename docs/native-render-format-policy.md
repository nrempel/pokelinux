# Rust format capability policy

The original Rust [format module](../native-render/src/format.rs) now implements
first-format/first-modifier selection and capability corroboration. Its real
[C ABI consumer](../native-render/tests/format_ffi.cpp) passes all four frozen
groups with the matching new Rust archive. This is authored CPU evidence, not a
new DRM observation. The native compositor adapter has not been connected to the
helper yet; its existing C++ decisions still execute. Native startup still fails,
frame preparation/execution remain unimplemented, and no phone has been accessed.

## Ownership and language boundary

Rust owns phone services, permissions, lifecycle, deadlines, recovery, frame
admission and portable selection policy. Retained Hyprland, Aquamarine and Qt
provide scene machinery, graphics operations and native object lifetimes. Custom
C++ should copy native observations, invoke exact operations and translate
callbacks. It should not add alternative selection, scheduling or recovery policy.
The project does not require a wholesale Rust rewrite of these upstream engines.

The helper is synchronous and stateless. It retains no pointer, FD, native object
or executor handle and performs no allocation or native call. Copied context
identifies the supplied observations; it does not authenticate their freshness.
The existing executor remains responsible for epoch, availability and deadline
admission. A capability-positive result grants no allocation, import or display
operation and cannot establish Ready or a presented frame.

## Input and decision contract

The [public header](../native-render/include/pokelinux_render_format.h) defines
224-byte borrowed facts and a104-byte owned result on the tested Linux x86-64
ABI. Seven flat arrays preserve native format groups, modifiers, planes, legacy
formats, decoded explicit pairs, CRTC IDs and encoder masks. Their respective
limits are128,4096,128,4096,4096,32 and128. Native collectors must refuse excess
rather than truncate or erase an empty first modifier group.

Rust checks all records and spans before selection, including unused later
records. Counts precede slice creation; checked arithmetic, alignment, null and
output-overlap checks precede borrowing or writing. Zero counts require null.
Reserved fields and state/error/value consistency are validated. Unknown,
absent, failed and successful queries remain distinct. C callers must still
provide live initialized input memory and writable output storage; these checks
cannot authenticate arbitrary addresses.

Only the first native format and its first modifier are considered. Empty first
lists and the implicit modifier sentinel are refused. For an explicit tuple,
Rust requires known cap1, a compatible route computed from raw encoder/CRTC
facts, a primary plane with the selected legacy format, and a known nonzero
IN_FORMATS property whose completely decoded blob contains that exact pair.
Known cap0, absent properties and legacy LINEAR do not imply explicit support.
There is no sorting, later-tuple retry, implicit positive or LINEAR synthesis.

Each successful call writes one canonical owned choice or structured refusal,
including when replacing an initialized result. Malformed/capacity failures leave
all output bytes unchanged. Borrowed inputs remain unchanged on every outcome.
The result survives disposal or overwrite of the input arrays. A reported plane
is a corroborating witness, not proof that Aquamarine selected that KMS plane.

## Executed boundary evidence

The independent consumer and contract were frozen before implementation. Actual
GNU C++14.2 first compiled the unchanged current three-header ABI2 control with
exit0 and zero diagnostics; the new target exited1 on exactly the absent format
header. This structural RED does not claim execution reached the missing function
or establish a semantic failure. Its six actual outer waits and PIPE EOFs closed,
with the exact container absent on the same daemon.

After Rust implementation, formatting, strict Clippy, all64 previous named Rust
tests with two ignored child modes and Release compilation pass. Six real C++
programs link the same fresh archive: the five previous programs plus the
unchanged four-group format consumer. The new groups exercise owned explicit
results and first-tuple-only behavior; recorded cap0 and missing-fact refusals;
raw route/type/legacy/pair controls; and whole-input malformed-shape atomicity.
These records are authored fixtures, including the copy of the prior measured
cap0 case. They do not become fresh kernel or graphics evidence.

The new consumer's actual PID51 matches its pre-exec shell PID and foreground
exit0. Its dependency file selects the actual new header; its link map selects
only the matching new Rust archive. Exactly31 strong public functions are defined,
including pr_format_select_v1 and excluding the obsolete executor factory. All10
outer waits/PIPE EOFs close; both exact containers are absent on the same daemon.
The five earlier programs retain their original assertions and source bytes.

Compact identities for this checkpoint:

| Artifact | SHA-256 |
| --- | --- |
| Frozen C ABI consumer | 34140a2057fb9a11c1f0342829b6409177a11eb38844b92289b2010f0ccc1426 |
| Rust format module | ee71d7285f9b46c8aeb58800acf1226061863fd38dd8d8900c32b715a15b8962 |
| Public format header | 96ea4097bf9bbb6e3f554c70bc9780091df7ca96e56bfa041ba73251543ff250 |
| New Rust archive | d9a7ad56a45cce137f1ef6858e400b595d9a5c39363f6d0fef8cf9ad16f697ca |
| Structural RED result | 8b6c31522b39a9c1cd1b796ed07a9c47209b2a62ad84562f61b39e2419f4a3a1 |
| Component QA result | 619d12e11c5456ff82f1d32f6a516a68bf5a99fa73fd40810328f735883416b8 |

## Next integrated slice

Connect one common native collector to this Rust helper after enumeration at
both bootstrap and recovery. Registration should retain native objects and
identity without choosing a tuple. Strict missing-query handling can add or move
refusals; do not describe all old behavior as unchanged.

The [exact native allocation/import operations](native-render-exact-buffer.md)
now pass their matching normal source/SDK build and strict API checks. Their
real-driver tests observe five passing resource groups but fail the control-session
check on a queued CHANGE/revocation. Connect production Rust reconciliation and
prepare/execute, then connect real client content to a normal presented frame at
the Pixel10 display profile. Allocation and framebuffer registration are owned
preparation; the one-shot display seal stays immediately before the actual atomic
commit or lease mutation. Resource creation, CLOSEFB success and process reaping
do not prove complete native retirement.

Continue that same session through the existing Sounds app, Rust session and
permission services, real virtual audio output and live Omarchy theme changes.
Exercise Source timing and clean shutdown on the integrated selection, then ARM.
This component does not qualify native adapter wiring, allocation/import,
normal readiness, client frames, Source, ARM or physical hardware. The matched
Google kernel/module/firmware and remaining Pixel hardware gates stay separate.
