# Retained Rust worker CPU component

The original [`executor` module](../native-render/src/executor.rs) implements a
retained Rust worker, bounded frame admission and copied completion records.
Actual Linux x86 CPU tests use real threads, kernel eventfds and `poll`, with
authored `NativeOwner` adapters. This is a worker component, not production
AQ/GL/DRM integration or Source GREEN. [Hash-only provenance](evidence/native-render-worker-provenance.json)
separates its results from the earlier [Frame native build](native-render-frame.md).

## Implemented ownership

The control owner keeps the `JoinHandle` and is restricted to its creating thread.
The worker creates, uses and destroys its adapter on that worker; the factory is
a plain function pointer. Control admission, status, Stop and completion draining
hold no mutex over adapter work. Stop fences unused tickets independently of the
frame/completion queues. Normal `try_join` returns Busy while the thread is live
and retains its handle; release requires an actual join and caller removal of
the borrowed completion FD. Fallback Rust `Drop` stops and joins, can block
indefinitely, and never detaches. It must not run in the responsive Source path.

At most three admitted frames share live credits, including queued and retiring
frames. Each admission first reserves five records from a separate128-record
pool. Delivering an outcome does not release native frame custody, and retiring a
frame does not erase undelivered facts. One actual CPU case preserves125 records
from25 completed frames and refuses the next frame without consuming it.

`SubmissionOutcome` records an immediate result, including refusal or uncertainty;
it is not proof of successful submission. A grant is historical ordering evidence,
not native entry/completion. A sealed attempt whose adapter fails to report entry
remains Unknown. Full bounded object values are copied before granting, but the
adapter must still authenticate genuine device/output/routing scope. These
shape checks supply no real multi-output authority.

Pending retirement retains frames. Stop drives shutdown progress even with live
entries; every entry and the aggregate need verified drain before owner destruction.
The final-retirement repair rechecks aggregate shutdown before sleeping after the
last native event is consumed. Native readiness uses real `poll`; one ingress job
is the budget before another readiness dispatch. The full FD/token/generation/
events tuple is revalidated before each dispatch. That revalidation is source-
reviewed, without a real multi-source generation-change runtime qualification.

Presentation records preserve raw sequence/timestamp zero and an explicit clock
domain. No monotonic timestamp is relabeled BOOTTIME. Ordinary native failures
remain recorded while custody waits for verified retirement. Adapter panics,
protocol failures or failed drain reports quarantine the worker and retained
custody. The tests actually reap whole owned child fixtures after quarantine;
this is not vendor-resource drain, production recovery or permission to detach.

## Actual CPU evidence

The retained regression series includes three real RED/GREEN pairs with identical
frozen test modules within each pair: pending retirement/shutdown and delivery
custody; ready-FD starvation/shutdown progress with a live entry; and rejection of
valid raw-zero presentation fields. Its historical QA has35 active passes.
Earlier fixture and supervisor-count failures remain recorded separately; fixing
a verifier against retained output is not a test rerun or behavioral repair.

A later frozen regression fails one actual test: final retirement consumes the
last readiness source while the old Pending shutdown observation leaves the
worker waiting for a nonexistent next event. The RED fixture sends another Stop
wake and actually joins before asserting failure. The targeted shutdown recheck
passes that same test, with the full unformatted test module unchanged between
RED and GREEN. Formatting follows separately.

Final actual QA passes22 library cases:8 permit and14 worker, with two ignored
child-entry modes invoked explicitly by parent tests. The same epoch also passes
all14 Frame integration cases:36 active passes total, not a sum with the earlier
35-case epoch. Format-check, strict Clippy and the x86 release build pass. Each
outer epoch closes six direct waits/six merged PIPE EOFs under original cutoffs,
with unchanged readonly inputs and exact same-daemon container absence. Owned
fixture child waits/EOFs remain separate from those six outer receipts. Frame's
original14-case postimplementation chronology remains unchanged. The QA tree's
README is historical; publication retains current documentation. All compiled
Frame/permit/SHM, Cargo, header and test dependencies match the public base; only
the worker source and module declaration are added.

## Integration still required

The new worker archive is `8067436b…`. The independently qualified native Frame
ELF/SDK selected earlier `3cf0a142…`; it did not link this worker archive, and its
five FFI/four codec CPU groups are not new worker evidence. No worker C ABI or
private proposed executor header is published. Genuine session/FD transport,
bootstrap/discovery, a worker-local AQ/GL implementation, complete actual native
object scope and distinct Stop-retirement authority/ledger remain required. The
current configuration-free factory has no session input. A real binding needs a
retained deferred holder, then explicit session/bootstrap/output discovery before
constructing the real FrameTarget; those operations are not implemented here.

No live scene producer, real render owner, GPU/fence/output/presentation, secure
lock frame or blocked-driver recovery is qualified. The unchanged
[seven-case rendering RED](native-render-authority.md), cold full composition,
normal app/PCM, ARM, Google-kernel, phone and full smartphone gates remain open.
