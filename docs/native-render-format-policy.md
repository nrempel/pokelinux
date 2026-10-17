# Rust format operation planning

Rust now chooses an exact implicit or explicit buffer request from raw native
observations. The [implementation](../native-render/src/format.rs) replaces the
old format API with `pr_format_select_v2`; there is no compatibility endpoint.
The [genuine C ABI consumer](../native-render/tests/format_ffi.cpp) passes six
frozen groups against the new archive. All64 existing Rust tests, the five earlier
C++ programs, formatting, strict Clippy and Release compilation pass in the same
selection. These are authored CPU facts, not new DRM observations or a working
compositor. The common native startup/CHANGE collector is authored separately;
its matching full build and runtime remain pending.

## Ownership and language boundary

Rust owns phone services, permissions, lifecycle, deadlines, recovery, frame
admission and portable selection policy. Retained Hyprland, Aquamarine and Qt
provide scene machinery, native graphics operations and object lifetimes. Custom
C++ adapters copy observations, invoke exact primitives and translate callbacks.
They must not introduce alternative format, scheduling or recovery decisions.
A whole-engine language rewrite is not required to finish the smartphone.

The planner is stateless and synchronous. It retains no caller pointer, FD,
native object or executor handle and makes no allocation or native call. Its
copied context identifies supplied observations without authenticating freshness.
The executor retains epoch, availability, deadlines and one-shot display authority.
A positive plan requests one primitive attempt; it proves neither native success,
Ready, admission authority nor presentation.

## Current contract

The [header](../native-render/include/pokelinux_render_format.h) defines312-byte
borrowed facts and a112-byte owned result on the tested x86-64 ABI. Seven bounded
flat arrays preserve all native format groups, modifiers, planes, legacy formats,
complete decoded pairs, CRTC IDs and encoder masks. Limits remain128,4096,128,
4096,4096,32 and128. Excess observations must be refused without truncation.

Validation precedes policy over every record, including unused later groups and
modifier/pair data ignored by implicit selection. Counts, checked spans, alignment,
null consistency and output overlap are checked before borrowing or writing.
Reserved fields, dimensions1..16384, option booleans and query payload consistency
are validated. Invalid/capacity results leave all input and output bytes unchanged.
Callers still guarantee live initialized memory; numeric checks cannot authenticate
arbitrary addresses.

Facts include actual allocator/backend kinds, dimensions/options, both device-query
results, independent rdevs and the actual device-comparison invocation count/raw
return. An unperformed comparison remains unknown. A successful bus comparison,
not numeric render/primary rdev equality, supplies the same-device observation.
Raw rdev0 remains representable; the KMS rdev must match the supplied context.
Unknown/error observations and valid unsupported options produce structured
refusals. Malformed state/count combinations return atomic INVALID.

Known cap1 considers only the first native format and its first modifier. Empty
first modifier lists and INVALID are refused; Rust requires raw route/primary/
legacy corroboration and the exact pair in a completely decoded IN_FORMATS blob.
Known cap0 plans an implicit request for the first fourcc, including an empty
first modifier list. Raw modifier0, INVALID or another value never chooses this
mode or implies LINEAR. Known absent IN_FORMATS is allowed for implicit; unknown
or failed property reads are refused. A present blob must be completely decoded
in both modes, although implicit needs no explicit pair match. No later-tuple
retry, forced LINEAR or fallback after a failed native operation is added.

Success overwrites the whole canonical owned plan or refusal, including initialized
output storage. Refusals zero choice/mode/dimensions, preserve context and may name
a plane witness. The copied result survives input disposal. A plane witness does
not prove which plane Aquamarine actually selects. Exact allocation/import and
authorized atomic submission remain separate boundaries.

## Executed boundary evidence

The independent V2 contract/header/consumer were frozen before implementation.
Actual GCC14.2/C++20 first compiled the four genuine current headers with control
exit0 and zero diagnostics. The target exited1 on exactly three missing names:
PrFormatFactsV2, PrFormatSelectionV2 and pr_format_select_v2. Both actual dependency
files authenticated all four selected headers. This is declaration-only RED; it
does not claim a Rust body, linked function or native semantic failure. Its six
actual outer waits and PIPE EOFs closed, with exact container absence, unchanged
inputs and the same daemon.

After implementation, all six real C++ programs link one fresh Rust archive. The
six replacement format groups cover explicit first-tuple/owned results; recorded
cap0 and absent/unknown/error distinctions; raw route/primary/legacy/pair controls;
whole-input bounds and atomic refusal; implicit modifier/property controls; and
raw backend/device/context/options refusals. The copied cap0 fixture remains an
authored input, not a fresh kernel observation. All other19 compiled source files
and all five previous C++ programs retain their bytes.

The actual format PID51 matches its pre-exec shell PID and foreground exit0. Its
dependency file selects the production V2 header; its link map selects only the
new archive. Actual nm defines exactly31 strong public functions, including
pr_format_select_v2 with select_v1 and the obsolete executor factory absent.
All10 outer waits/PIPE EOFs close; both exact containers are absent on the same
daemon, with unchanged source inputs. Embedded producer/thread closure remains
separate from outer container closure.

| Artifact | SHA-256 |
| --- | --- |
| Frozen V2 consumer | d130c73f1230ee0f41997be8c61b5c2345ebfe14e4355bc071e7fabb653a794d |
| Rust format module | 89956ee4bb8df2b1d0917b5d56f23e29bd5576587089412160f3011831a5c254 |
| Production V2 header | a58e9396a2a1f144041a4beb45d06e9c79f9f2d4df9515df2538ffedd6cbcf97 |
| New Rust archive | 425dd6fa9d604054b2ab15beeb488c79f1dbcac5efc0779b213bed5bd6de9f83 |
| Structural V2 RED result | c34a0fa96095e2ba564d8b74559346c335cf8b4c84bc69a95eea9e9f9ad8a6a3 |
| Full component QA result | 1ad4bfbfab4a1731f83e34176515ec5eee0a0c70c20db63ad909b35fe45b0fbe |

The historical V1 checkpoint used224/104-byte views, four authored groups and
archive d9a7ad56. It refused implicit selection and was not connected to native
startup. That result remains historical; its API is removed by this change.

## Next integrated slice

Connect one bounded raw collector to this Rust planner after native enumeration
at both bootstrap and CHANGE. Preserve real events and existing Rust reconciliation;
do not ignore revocation to make a fixture pass. Retain native output/mode/device
identity and the copied plan without choosing a second tuple in C++.

The [exact native allocation/import operations](native-render-exact-buffer.md)
pass their separate normal source/SDK build. Their real-driver runs observe five
passing resource groups, while both aggregates fail on a queued CHANGE/revocation.
Use the new plan in owned prepare/execute, then connect real client content to two
successive normal presented frames at the Pixel10 display profile. Presentation,
replacement and retirement are distinct; the executor already supports three
bounded admissions. Keep display authorization immediately before actual atomic
mutation. CLOSEFB and process reaping do not establish scanout release.

Clean Stop still needs narrow Rust-owned authorization for actual output disable,
independent of occupied frame credits. Cleanup observation grants no disable
permission. Continue the same session through Sounds/PCM, permissions, live
Omarchy themes, Source and honest native drain, then ARM. Normal readiness,
client frames, Source, ARM and all Pixel hardware gates remain unqualified.
No physical phone has been accessed.
