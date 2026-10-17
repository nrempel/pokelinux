# Rust format operation planning

Rust now chooses an exact implicit or explicit buffer request from raw native
observations. The [implementation](../native-render/src/format.rs) replaces the
old format API with `pr_format_select_v2`; there is no compatibility endpoint.
The [genuine C ABI consumer](../native-render/tests/format_ffi.cpp) passes six
frozen groups against the new archive. All64 existing Rust tests, the five earlier
C++ programs, formatting, strict Clippy and Release compilation pass in the same
selection. These are authored CPU facts, not new DRM observations or a working
compositor. The common native startup/CHANGE collector now passes its matching
full normal compositor/SDK build and the unchanged actual native startup/CHANGE
test on generic x86 KVM virtio. This accepts metadata, without client frames or
native retirement.

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

## Matching native build

The [common native collector](../tools/native-render-owner/format-plan/README.md)
removes both C++ first-tuple choices and the separate C++ format corroboration.
Bootstrap and CHANGE call the same Rust planner with each output's own retained
device identity and actual sequence. Native pointers and the copied request mode
remain worker-owned; existing mode corroboration stays unchanged.

Its complete normal main build passes against exact AQ9512cc02/SDK93150843 and
Rust425dd6fa. All458 configured C++/PCH entries omit the test macro,31 Rust symbols
are retained/exported, and the matching SDK fully reads back537 regular files and
two links. The original15 CPU groups, texture5, first-Stop5, session6 and seven
strict SDK checks pass; the format6 result is the separately selected component
QA, not a new guest execution. All82 owned waits/PIPE EOFs close, with QMP socket
closure separately verified, QEMU/CLI0, exact container absence, the same daemon
and unchanged inputs. No compositor main/native runtime was invoked by this build.

The first full source build compiled and linked, then failed because the fixture's
manual SDK builder omitted the fourth Rust header. Its failed aggregate is
preserved. The repair adds four staging lines only; production code, resources,
all assertions and oracles stay unchanged. The second full selection above passes.

| Native build artifact | SHA-256 |
| --- | --- |
| Main ELF | 4ffa7ae10fdbbf2c42eac3e53a0e7a827a2a86857bc65e777d14cfc8e3a6c182 |
| Matching SDK | cf58cbf5a946ec700a81cb72344e995cd4f56f404d234c9934ecec131b0e8603 |
| Closed execution | 84d635a0279dc5190e39d8f5bbb96ad152ee598ae1756bab2c85ad644ac9b93a |
| Root build qualification | f1b3fa2e2d996fc5f43af98ef0d9125176b6fb4ba069ad9e8c8bb70e680cda63 |

## Actual native bootstrap and CHANGE

The unchanged genuine recovery success target now passes with that exact main,
SDK, Rust and AQ cohort. The normal broker/session and Rust worker observe actual
bootstrap stage8, then accept one refreshed output after the queued real CHANGE.
All ten Rust watermarks agree: bootstrap ends at sequence3; invalidation,
admission, application and acceptance epochs are1; all latest sequence witnesses
are4. The initial session generation and original20s startup deadline are unchanged.
There is no first-Stop record, no Stop request and no startup retry.

Native logs show the same V2 planner called at sequence3 and4. Both request
implicit XR24 with known cap0, actual device-query returns0 and one actual device
comparison returning1. Modifier0 is the reserved implicit request value, not a
LINEAR layout claim. The plane35 witness does not establish actual plane selection.
Two authenticated public socket replies preserve real peer PID/UID/GID and EOF.
This run observes metadata Ready282,850,187ns after the session start; it is one
startup observation, not a general latency benchmark.

All28 owned direct waits and35 PIPE EOFs close; QMP socket and two public-response
EOFs are separate. QEMU and outer CLI exit0, the exact container is absent on the
same daemon and selected inputs stay unchanged. Main and seatd are disposed with
SIGKILL/wait-9. That disposal supplies no native cleanup, retirement or Rust join.
The accepted snapshot explicitly retains shell_ready=false and
native_retirement=false. No client, plugin or diagnostic substitute was launched.

| Native runtime artifact | SHA-256 |
| --- | --- |
| Closed execution | 07f82c618063abac6e99c17e811138cccfe203fb4685e02efc63d6efc8a524c0 |
| Frozen-case result | d33e7ce449edfc92e0e4352027f58a65144e3ffd361e5fe96096dd2917cb51da |
| Root runtime qualification | ce1ee0970484642bebeaf19ceb7ba18b2678d4a23ec1e1ab8faa197301829d98 |

This is modified open-source native compositor/Rust integration on a generic
x86-64 KVM kernel and virtio driver. It is separate from Android emulator,
ARM64/QEMU, the matched Google/Pixel kernel and physical-device evidence.
The earlier failed raw allocation/import fixtures remain failed; this run does
not invoke those production operations or qualify rendering/presentation.

## Next integrated slice

Continue the accepted native session through real normal Wayland globals, client
content and Rust frame admission. Preserve actual event reconciliation and the
retained output/mode/device identity; use the copied Rust request mode directly.

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
