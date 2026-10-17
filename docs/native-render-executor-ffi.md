# Retained render executor C boundary

The current factory/table is now mandatory ABI2; see
[Rust session reconciliation](native-render-session-reconciliation.md). The
`create_v1` factory is removed, without compatibility aliases. The earlier
checkpoint below retains its original source/archive and is historical evidence.

The current [`pokelinux_render_executor.h`](../native-render/include/pokelinux_render_executor.h)
and Rust [`executor_ffi`](../native-render/src/executor_ffi.rs) connect actual C++
callers to the retained Rust worker. Current native Linux x86 execution passes
both a lifecycle test and five independent authored-owner groups, alongside62
Rust cases, format-check, strict Clippy and release compilation. This is an
ownership/ABI component, not the installed native AQ/GL compositor. The earlier
real GL checkpoint still selects the old Frame-only archive. [Hash-only evidence](evidence/native-render-executor-ffi-provenance.json)
keeps these selections and failed attempts distinct. Independent review matches
the frozen sources, actual logs and closure receipts, without another runtime
run; it checks archive selection/size and retains the primary pinned digest.

## Construction and native custody

`OwnerFactory` now takes one typed, by-value, move-only `OwnerFactoryConfig`.
There is no second factory/compatibility path. Its only contents are a validated
copied current function table and bounded scalar creation options. No userdata,
control-created native pointer, captured graph or borrowed table crosses it.
Unsafe construction requires callbacks to obey the native custody, no-unwind and
code-lifetime contract; shape validation cannot authenticate a backend.

The existing worker retains a deferred Rust adapter, then retains the owned
bootstrap pack and the worker-created empty C++ holder before native bootstrap.
The C callback borrows duplicate FDs; an API that consumes FDs needs explicit new
duplicates. The Rust pack stays owned until after verified native shutdown and
worker-side holder destruction. The supplying library must remain loaded through
that destruction and actual join. No successful callback, close or fixture
process reaping proves real GPU/vendor cleanup.

Bootstrap capture copies metadata and duplicates borrowed capabilities CLOEXEC;
original caller FDs remain caller-owned. Invalid table/options and invalid handle
slots preserve the bootstrap. After constructor shape validation, creation
consumes/nulls the owned pack even if cutoff/spawn fails. The public header states
this separately from Frame admission.

## Responsive control and historical facts

An immutable creating-thread guard precedes access to the inner `UnsafeCell`
executor, ownership changes and output writes. Release first reads its outer
handle slot to locate that guard. Valid calls are serialized and nonreentrant.
No graphics work, callback, native cleanup or blocking Drop runs on that control
path. Busy join/release retains the exact owner slot until actual join.

The one shared admission path validates/reserves before taking a C Frame Box.
Accepted admission consumes/nulls it; every refusal preserves the exact original
handle and output buffer. Ordinary refusal also preserves earlier authority;
terminal serial exhaustion deliberately fail-closes authority. Callback Frame,
view and attempt-token pointers are synchronous borrows, never retained handles.
Only an explicit Snapshot clone owns a longer pixel lifetime.

The attempt token has its own immutable worker guard. Complete bounded scope
values are copied before one-shot sealing; invalid scope consumes the attempt,
and another use refuses. Native adapters still must authenticate the actual
complete old/new routing. A copied grant is historical observation, not reusable
permission. Separate C structs and fieldwise conversions preserve result,
scope and completion values without exposing Rust enum/Option layouts.
Linux64 sizes/critical offsets are asserted independently in Rust and C++.
An authored successful outcome proves neither real display submission nor
presentation. Real GL offscreen fences cannot be relabelled as display facts.

Register both borrowed completion and startup timer FDs. On startup readability,
call status/discovery/admission/join to observe the original CLOCK_BOOTTIME
cutoff; merely draining the timer is insufficient. Stop or expiry cannot accept a
late bootstrap success. BOOTTIME includes suspend time but neither wakes a phone
nor interrupts a blocked callback. Native work and all custody stay on the
retained worker while snapshots, Stop and Busy join/release remain available.

## Executed tests and preserved failures

The initial frozen lifecycle test preceded the C implementation. The exact old
fcc5673 Rust archive fails the actual C++ link on the missing executor symbols.
That is a compilation RED, not a production semantic RED. The first two attempted
runs instead fail fixture setup: absent C++ in the Rust image, then absent C++ on
the host. A retained native Qt build image supplies the actual C++ compiler in
subsequent offline containers; no compiler was installed or downloaded.

The larger suite's first selection fails `-Werror=comment` on comment continuation
backslashes. The corrected suite then fails actual Frame capture because its SHM
draw omitted the already-required nonzero surface ID; a later aggregate retains
that same invalid-fixture failure. A one-field fixture correction leaves the
production validator unchanged. Its next aggregate passes the first three groups
but fails an immediate procfs disappearance assertion after actual join.

That observation cannot be inferred from join alone. Primary [glibc join code](https://github.com/bminor/glibc/blob/glibc-2.41/nptl/pthread_join_common.c)
waits on clear-child-TID; Linux6.8 [memory release](https://github.com/torvalds/linux/blob/v6.8/kernel/fork.c)
can clear/wake it before later [task removal](https://github.com/torvalds/linux/blob/v6.8/kernel/exit.c).
This ordering makes the immediate assertion unsound; it does not establish the
particular instruction/cause of the failed observation. The corrected fixture
observes exact-TID absence within its existing pre-join cutoff, without extending
it, and records that fact separately. All failed selections stay failed; none of
these fixture changes supplies a production bug-fix RED/GREEN claim.

Final QA3 passes the lifecycle test plus these five groups:

1. Invalid current ABI validation preserves the original owned bootstrap.
2. Two concurrent distinct static factories receive their own copied options;
   held bootstrap and wrong-thread calls preserve exact Frame/output/owner slots.
3. Real Frame/Snapshot bytes survive original storage destruction; copied-table
   dispatch, one-shot authored scope, copied outcome/retirement, worker-side
   destruction and actual Rust joins complete without using overwritten tables.
4. Explicit Stop refuses a held bootstrap's late successful discovery.
5. An actual150ms BOOTTIME watch expires held bootstrap, keeping native holder
   and worker custody until explicit fixture release and verified cleanup.

Four separate post-join task-absence observations accompany the larger suite.
Its original work/close bounds are5s/8s, with existing2s observation cutoffs.
Final Rust QA passes36 library/14 Frame/12 bootstrap cases, with two ignored
owned-child entry modes invoked by parent tests. The current release archive is
246a6680,22,901,236 bytes. Eleven finite outer epochs retain90 direct waits and90
merged PIPE EOFs, original240s/270s work/close bounds, readonly-source continuity,
exact same-daemon container absence and no forced outer container disposal.
The passing Rust panic test separately SIGKILLs/actually reaps four authored
quarantine children; their process disposal proves no native retirement. Failed
functional aggregates are not made successful by outer closure receipts.

The [executor GL lifecycle checkpoint](native-render-executor-gl.md) now qualifies an actual
missing-handshake RED after all eight GL cases, 42 packets and six sync-file
facts. Stop returns `unknown request`; no finalize, unload or Rust join/release
occurs. A separate [AQ15/current-ABI native build](native-render-executor-gl.md#separate-aq15current-abi-build-and-startup-failure)
now passes CPU/loader checks, but the matched runtime aborts during backend
startup before Source/actor; its exact cause remains unresolved. Source GREEN
and all phone gates remain open.

## Next native and phone gates

The separate matching AQ15/main/SDK build now passes CPU/loader checks. Its
runtime startup still fails before Source/actor; integrate the real broker/
render owner before claiming the offscreen worker/handshake result. Do not link a
second Rust archive into the earlier compositor or mix AQ14/AQ15 symbols.
Ordered later session ingress/OOB revocation, genuine output discovery/updates,
complete native routing and distinct startup/StopRetire authority remain open.

Real display commit/scanout/presentation, device-wide retirement and supervised
blocked-driver recovery remain unqualified. The unchanged seven-case Source
rendering-stall acceptance still fails three entered-render controls; no cold
normal app/PCM GREEN follows here. No ARM, matched Google kernel or physical
phone executed in this checkpoint. Every Pixel/carrier/camera/power/suspend gate
and the full smartphone completion matrix remain required; no phone was accessed.
