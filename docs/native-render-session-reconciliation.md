# Rust session reconciliation

The qualified baseline stops the normal compositor at a genuine Aquamarine
device-change callback. Recoverable same-session CHANGE handling is now implemented
in Rust; its native acceptance remains unqualified. This is an unfinished graphics integration path; no
normal client frame, Source acceptance or phone result follows from its tests.

## Language and ownership boundary

Rust owns authority invalidation, copied queue custody, ordered application,
readiness and frame admission, stale-candidate rejection and BOOTTIME deadlines.
C++ translates actual AQ callbacks, invokes worker-local native operations and
copies device/output facts. Hyprland, Aquamarine and Qt remain substantial C++
dependencies. Their engines have not been ported to Rust by this change.

Invalidation immediately revokes old unsealed tickets and suspends availability.
It does not substitute the AQ session identity for the security generation.
Already-sealed work retains its historical grant, frame and retirement duties.
A successful native callback alone cannot establish Ready: its copied refreshed
discovery must cover the latest Rust invalidation. Unsupported REMOVE, disable,
transport failure or uncertain native application remain terminal. Process
reaping proves no vendor cleanup or native retirement.

The frozen boundary has a mandatory120-byte ABI2 native callback table and sole
`create_v2` factory. Its required CHANGE callback applies a copied metadata event
on the existing worker, then returns refreshed native discovery. Unchanged value
layouts keep their existing version. There is no legacy factory or optional
callback. The32-entry Rust queue accepts contiguous CHANGE sequences seeded by
the real captured bootstrap end. Only successful copying permits acknowledging
the actual AQ head. Busy and capacity refusals retain that head.

Ten copied Rust watermarks distinguish invalidation, admission, application and
acceptance. Dirty means invalidation has not yet been accepted, including a
revocation whose event has not been enqueued. Initial recovery keeps the original
startup cutoff. After initial Ready, the first change of a new burst arms a fixed
real BOOTTIME20s deadline; later changes do not renew it. Fresh discovery, submit,
snapshot and late callback acceptance share that gate.

## Actual failing baselines

Before production recovery changes, an actual native x86 KVM compiler run uses
the three current public Rust headers. The existing API control exits0 without
diagnostics; the target exits1 on exactly four missing ABI2 functions. All23
actual direct waits and30 PIPE EOFs close. This is structural RED, without linked
worker execution or a native startup result.

A separate actual NORMAL compositor run authenticates two public socket replies:
STARTING/stage8 followed by clean STOPPING/stage5. The first Stop is actual
AQ_REVOKE3/reason DEVICE_CHANGE5 for the same session, observed105,952,907ns after
that session identity and before the original20s cutoff. The transport is healthy
and no public Stop was sent. All28 direct waits/35 PIPE EOFs close; QMP and two
public-response EOFs remain separate. Main disposal is actual SIGKILL/wait-9,
without a Rust join or native cleanup claim. The particular device/change subtype
remains unobserved.

The independent C++ to Rust consumer was frozen before implementation. Its six
groups require immediate old-ticket refusal before another control observation,
32-entry capacity with atomic refusal, three queued changes draining without an
extra wake, full408-byte copied metadata, latest-only acceptance, real kernel
timer arm/nonrenewal/disarm, late callback refusal and retained sealed custody.
Its native callbacks are authored CPU fixtures. Original work5s/close8s bounds
remain. A passing component cannot substitute for real native reconciliation.

The unchanged native GREEN target requires actual nonempty accepted discovery,
no Stop and all ten consistent Rust watermarks before the original startup
cutoff. Ready alone is insufficient. The [source package](../tools/native-render-owner/session-reconciliation/README.md)
contains frozen original tests and compact hash-only provenance.

## Actual Rust component GREEN

The ABI2 implementation now passes formatting, strict Clippy, all64 original
named Rust tests with two ignored child modes, and Release compilation. Five
actual C++ consumers link the same fresh archive: snapshot5, lifecycle,
executor5, first-Stop5 and the frozen reconciliation6 groups pass. The latter
retains nine actual worker joins, one foreign-thread join and11 ordered CHANGE
callbacks. Real timer arm/nonrenewal/disarm, original-cutoff expiry, immediate
old-ticket refusal and retained sealed work are exercised with authored native
callbacks. These are CPU integration facts, without actual AQ reconciliation.

The archive has exactly30 unique strong public functions. The old factory is
absent. Raw symbol output, matching header dependencies and the consumer link map
bind the same new archive. Both exact containers are absent on the same daemon;
all10 outer direct waits/PIPE EOFs close under their original bounds. The
snapshot producer wait and two embedded EOFs remain separate.

Only serialized control expires the active deadline. A worker sample can straddle
acceptance and a new burst; it may refuse late publication, but cannot stop that
new burst using an older cutoff. Control accepts only the latest copied candidate.
A stale final seal also preserves no successful native-entry scope; the scope is
fully copied before sealing and installed only on success.

## Matching build and actual native refusal

The matching NORMAL compositor and SDK now pass in one actual x86 KVM build.
All458 configured C++/PCH entries omit the test definition. The main exports30
current Rust functions; seven strict SDK probes and the original15 CPU, texture5,
first-Stop5 and reconciliation6 groups pass with that same provider. All535
regular SDK files and two links match the retained manifest. The82 Scope waits/
PIPE EOFs close, including the early receipt once. The snapshot producer's one
wait/two EOFs and QMP EOF remain separate. The build executes no compositor main.

The first build attempt remains failed before any translation-unit compilation.
After complete base/overlay/final source authentication, its fixture tries to
record a missing `exact_delta_paths` field. The corrected assembler emits the
seven paths that actually differ from the earlier1153-file base. No production
source, frozen assertion or bound changes. The failed run's26 waits/PIPE EOFs,
CLI1/QEMU0 and locked input/CID closure remain separate.

The first actual native recovery attempt remains failed. Two authenticated
replies contain raw stages8 then5. The second reports UNAVAILABLE, native error9,
Rust failed/stopping, outputs0 and first-Stop origin11/RUST_FAILED. The unchanged
oracle rejects the required zero-error prerequisite before entering its target
branch, so its retained `observed_stages` contains only8. This is neither original
device-change target RED nor recovery GREEN.

The matching main log records `primary-format-modifier-unavailable`, errno0,
connector40. This establishes the failed corroboration branch, without recording
the selected format/modifier, actual primary-plane properties or kernel modifier
capability. It does not establish which fact is missing or which repair is valid.
The next integration gate is actual same-worker capability evidence before any
change to that predicate. Readiness remains unavailable; the worker and both
watches stay retained. Actual SIGKILL/wait-9 disposal proves no native join or
retirement.

Normal frames, presentation, native retirement, Source7, new ARM execution and
all physical-phone gates remain open. No physical phone was accessed.
