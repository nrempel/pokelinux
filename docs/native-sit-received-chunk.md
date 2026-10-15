# Native SIT receive ownership

The Rust backend now owns raw receive bytes before their storage can be reused.
`ReceivedChunk::copy_from` returns an opaque, non-Clone owned value with a readonly
slice. It refuses empty input and more than65,568 bytes, reserves fallibly, then
copies the entire supplied slice. The bound is native ingress policy selected
from the observed factory manager buffer; it is not a protocol maximum or a
matched-kernel claim. Allocation failure is returned explicitly; no forced-OOM
experiment is claimed.

Copying performs no header, token, declared-length, status or complete-record
validation. The existing exact144-byte decoder and source/attempt gate retain
those separate responsibilities. No null pointer, signed count, C++ layout or
boolean ABI enters the Rust API. Empty refusal is not a transport EOF/reopen rule.

## Original-code comparison

A separately authored guarded C helper calls the exact original named C1 and
nondeleting D1 from the matched protocol provider. One load control and eleven
constructor cases run under the real matched Bionic loader and ARM64 user-QEMU,
inside an isolated Harbour container. No parser, getter, SendMessage, queue,
handler or modem transport is called by the helper.

Each constructor case captures the full source before/after C1 and after XOR0x5a,
and the full owned payload before/after that overwrite and after C-free of the
helper's source allocation. Actual ranges establish that positive owned payloads
are disjoint from the entire live guarded source allocation. Named D1 releases
only the original payload; the helper then checks receiver guards/cleared fields
and frees its own receiver allocation. The false boolean still copies. Null,
zero and negative-count controls retain empty payload state.

| Native comparison | Supplied bytes | What copying does not establish |
|---|---:|---|
| Base IPv4 response |144| Acceptance, correlation or APN readiness |
| Complete initial request |983| Response validity |
| Mismatched inner declared length |144| Header consistency |
| Prefix of a144-byte source |16| A complete response |
| Two coalesced responses |288| Record boundaries or reassembly |
| Opaque pattern |32| Any RCM meaning |
| Entire native policy bound |65,568| A modem/driver maximum |

These seven nonempty false-flag cases supply14 public hex files. Input and
expected owned bytes are independently rendered from their actual captured files;
decoding each rendering reproduces every captured byte. The positive true-flag
case and three empty controls remain original-only observations. The helper
prezeros padding, so this is not complete24-byte C++ ABI equivalence.

Root and a separate reviewer qualify all twelve actual QEMU waits0, both output
PIPE EOFs, final ECHILD, live process/task credentials, full regular mapped-file
identity, five provider symbol/RVA bindings per constructor case and original
T0-derived20/60/90-second cutoffs. Each child maps35 ARM files plus the x86 QEMU
executable. Actual receiver vptr/D1-slot checks are assertions in the frozen helper;
the controller does not read child memory. Supplementary groups are recorded as
1000/1000, with semantic set{1000}; all UID/GID fields are1000, all capability
sets0 and NoNewPrivs1.

Natural dependency constructors attempt missing properties/sysfs and a UNIX
socket connection. Their signatures match the actual load control; no absence
of ambient attempts is claimed. `dlclose` returning0 proves neither complete
finalization nor unmapping. The container has no network or attached host devices
and an empty readonly sysfs. Proprietary libraries and raw execution/analysis
artifacts remain private.

Two earlier prerequisites remain failed setup: an overly strict supplementary
group list check stopped before any child, and a copied loader without execution
permission failed the load-only case. They supply no Rust RED or constructor
result. The original fixed cases, helper and byte oracle were unchanged.

## Native RED and GREEN

The compiled, formatted and Clippy-clean missing-copy baseline passes42 codec
tests and fails eight specifically with `MISSING_OWNED_RECEIVED_CHUNK_COPY:
Unsupported`, actual test exit101. All15 attempt tests pass. The seven comparisons
and source-overwrite/drop test fail; the two native policy guards already pass.
Only receive-copy implementation source changes for GREEN; the other51 staged
files, all tests, fixtures, previous40 codec cases and attempt prerequisites stay
fixed. The placeholder Unsupported error is removed from the new API.

GREEN passes all50 codec and15 attempt tests on x86 and, separately, all50/15
through explicit ARM64 user-QEMU runners. Seven actual ARM test executables are
qualified as ELF machine183. These component executions are not ARM kernel or
physical-device evidence. Before the valid RED, test-source formatting changed
only layout; the hex fixtures preserve all qualified original bytes.

## Supervised integration

The explicit Unix-record worker copies its actual accepted144-byte receive slice,
poisons its entire145-byte reusable buffer with0xa5 and checks it before emitting
hex from the owned chunk. The same complete response reaches the source/attempt
checks and all14 report fields. This is source/ELF-bound behavior, not a separate
heap observer or a claim about compiler machine stores.

The additional actual host process run passes completion, expiry and cancellation
with exits0/0/0, the fixed983-byte request, full144-byte reply and report oracle.
It retains100 held-worker snapshots below200ms, original deadlines, actual Stop9
and worker wait within2s, unchanged-journal successor refusal, both Main output
PIPE EOFs and final controller ECHILD. The controller bytes and behavioral oracle
are unchanged; a new contract pins the receive-copy source and qualified component
RED/GREEN. This is additional integration after the component repair; the old
worker already retained its selected reply, so no second missing-capability RED
is manufactured.

The additional actual generic ARM kernel run passes the same0/0/0 controls on
Arch7.1.6, independently of Pixel's matched Google kernel. All17 owned waits close:
seven host waits with both PIPE EOFs, two boot and two guest waits with merged
PIPE EOFs, three controller→Main waits with both EOFs, and three separately
recorded Main→worker waits9. No separate worker-output EOF claim follows. Actual
UID/GID1000, groups[1000], all capability sets0 and NoNewPrivs1 are qualified.
Root rehashes103 selected and106 external inputs, including the readonly backing,
and reviews the actual case/proc/maps/credential/Stop records and original cutoffs.
Guest mapped files are rehashed by the controller and guest, not freshly read by
Root after guest destruction. Fresh private runtimes and distinct source
incarnations share one disposable guest; no per-case new namespace is claimed.
Process reaping and unchanged local uncertainty establish no vendor cleanup.

A staging permission failure and a fixture formatting failure remain setup-only;
neither changes the accepted component RED/GREEN or the fixed process oracle.

## Remaining boundary

[Factory static findings](native-sit-transport.md) identify a candidate wrapper
callback that copies bytes, but its normal runtime creation chain is still open.
Selected scalar classifiers have pointer checks and unchecked byte/word reads,
not length/completeness checks. Native code keeps strict response admission;
there is no inferred split/coalesced assembler, production CPIF worker, descriptor
or subscription binding, provider/APN acquisition, remote cleanup, safe retry,
installed phone service, carrier or Pixel proof. No physical phone was accessed.

[Hash-only provenance](evidence/native-sit-received-chunk-provenance.json) separates
original execution, component tests and actual process integration.
