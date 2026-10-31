# Native worker error context

## Complete early pipe diagnostic

A failed graphical ARM wireless run lost its `RefreshBackend` error during
worker retirement. `worker_step` now attempts a complete error record before
error propagation releases any caller-owned parent connection. It formats into
a fixed4096-byte buffer and performs one nonblocking write through a separately
reopened, identity-checked FIFO handle. It does not change inherited stderr flags.
Full/closed pipes, oversized records and unsupported sinks are skipped; the
original error, control protocol, cutoffs and immediate retirement stay intact.
Delivery is best effort, including when another failure already caused SIGKILL.

The frozen normal-worker CLI check fails against the preceding worker and passes
against the candidate: actual UID1000/cap0/NNP1 execution retains the original
EPERM and natural exit1, now with the complete early record matching the later
Termination context. Both cases qualify actual wait/PIPE EOF and closed custody
under the unchanged1s work/3s close bounds. This executes ReduceCapabilities,
not the graphical RefreshBackend failure.

Four actual FIFO/sink controls and the full selected host suite pass:42 library
tests and8 owner tests, with1 ignored library test. Formatting, strict all-target
Clippy, host/ARM normal Release builds and ARM libtest compilation pass. ARM
tests remain unexecuted. An initial test run used UID1000 and failed11 existing
root-fixture assertions; the unchanged source and tests pass with isolated UID0
plus CHOWN/DAC_OVERRIDE, without network authority.

The new normal ARM GUI diagnostic has now executed as root3 and failed after
Enable/Completed, association/fixed IPv4, one cap0 packet and a normal close
request. Its combined app-wait0/SDK-retirement record is published after the
retained-worker guard and cancellation, without a separate wait observation time.
Main's complete output is one closure JSON reporting
worker signal9, with no early worker diagnostic or error stage. This establishes
neither root2's `RefreshBackend` path nor a retirement cause; the successful host
reporting control does not qualify graphical recovery. Controller custody passes;
guest-case closure remains false. [Partial graphical evidence](evidence/native-wireless-settings-window-partial.json)
and [host diagnostic evidence](evidence/native-connectivity-early-diagnostic.json).

Main now latches its first retirement reason in the existing closure JSON. The
unchanged13728-byte normal-Main consumer records an actual native x86_64
missing-reason RED, then candidate GREEN: `startup_failure`, `poll_worker` and
`InvalidData` precede external Stop and retain the original startup cutoff.
Initial UID0/SETPCAP-only authority lets Main reduce itself to cap0; NET_ADMIN
is always absent and the worker fails ReduceCapabilities with errno1 before
backend access. Main exits0, worker exits1 and both cases qualify actual
wait/PIPE EOF, closed handles and ECHILD. The candidate passes45 library plus8
owner tests,1 ignored, formatting, strict Clippy and host/ARM compilation. ARM connectivity component tests remain compile-only.
[Curated host evidence](evidence/native-connectivity-first-retirement.json) and
[exact consumer/reproduction](../tools/native-connectivity-worker/first-retirement/README.md).

The graphical first-retirement candidate later executes as failed root4. Its
Main closure records `receive_failure` / `receive_message` / `TimedOut`, detected
at `135610806603` against original cutoff `135570450012`, followed by worker
signal9. The 40,356,591 ns difference is detection past the cutoff, not a measured
syscall duration or evidence of empty/consumed input or a specific pre/post guard.
The last published lease extends beyond detection, while the transport cutoff
has passed; no exact internal pending state or receive stage is established.
No CPU, scheduling or IPv6 attribution follows. This identifies only root4's
first Main retirement branch; root2/root3 and the underlying timing cause remain
unexplained. One packet and brief post-close background readback qualify, but
complete graphical success remains failed.
[Graphical evidence](evidence/native-wireless-settings-window-partial.json).

## Earlier context-wrapper checkpoint

The later [idle receive investigation](native-connectivity-idle-receive.md)
separately reproduces empty-idle expiry on the host and adds a non-consuming
readiness hint. It preserves the 50 ms authenticated receive envelope and the
failed ARM observations below; their unique cause remains unknown.

The normal connectivity worker now preserves operation-stage context on errors
after an actual missing-context CLI RED. The unchanged frozen consumer passes
with `ReduceCapabilities`, `original_until_ns: None`, an error-only BOOTTIME
detection sample and the original source errno1/`PermissionDenied`. The worker
exits naturally with code1 in that host control; the GREEN consumer exits0.

## What changed

The original checkpoint changed only `native-connectivity-control/src/service.rs` at
error reporting: a private `WorkerFailure` wraps the original `io::Error`, keeps
its outer kind and exposes the source chain. The outer custom error's
`raw_os_error()` is not claimed unchanged. The detection clock is sampled only
when a selected call fails. Seven compiled contexts are `ReduceCapabilities`,
`BindParent`, `PublishInitial`, `ReceiveCommand`, `PublishRetirement`,
`PublishCompletion` and `PublishObservation`. Only the first error context
executes in the host CLI control. The separate failed ARM observation below
executes the ReceiveCommand error context. Kernel error mappings, validation,
47/19-byte records, control flow,
cutoffs, retries and retirement remain unchanged.

## Actual evidence

| Epoch | Qualified result |
| --- | --- |
| Baseline build |35 active host tests:10 connectivity,10 process-owner and15 kernel-identity;7 ignored. Network-state compiles only. Four normal host/ARM Release ELFs build without execution. |
| CLI RED | Actual UID1000 parent/child, all capability sets0 and NNP1. Normal worker returns exact unwrapped EPERM and naturally exits1. The consumer's desired assertion fails after closure. |
| Candidate build |10 connectivity tests pass,3 ignored; unchanged shared QA is inherited from baseline. Four current normal host/ARM Release ELFs build without execution. |
| CLI GREEN | Same8290-byte consumer and actual cap0 boundary. Context contains `ReduceCapabilities`, no cutoff yet and detection within actual spawn/wait. Original EPERM and natural worker exit1 remain. |

Both CLI controls qualify one actual direct-child wait and merged stdout/stderr
PIPE EOF under the original1s cutoff,4096-byte output bound, closed namespace
handle, closed Scope handles and ECHILD. The separate3s custody cutoff never
renews functional qualification. Child process/task metadata is sampled before
parent poll/wait; a zombie sample is not a running claim. The helper, raw result
and normal artifact hashes are recorded in the
[curated provenance](evidence/native-connectivity-worker-error-context-provenance.json).
The two build envelopes close16 Scope waits/PIPE EOFs each;31 baseline and22
candidate foreground-shell regular-output waits are recorded separately.

An extra candidate validation assertion initially omitted generated
`source.sha256`; that tool assertion fault is preserved. Frozen pins were
verified before launch, actual build pins passed, and the independent source
comparison was corrected after the build. It supplies no product RED.

## Reuse and limits

The [exact public consumer](../tools/native-connectivity-worker/error-context/consumer.py)
uses the existing cooperative selector and four trusted arguments. Its
[invocation contract](../tools/native-connectivity-worker/README.md#normal-cli-error-context-consumer)
requires the original owned/closure helpers beside it and the qualified normal
Release provider. No special worker flag, capability or network setup is added.

The CLI evidence remains native x86_64 ReduceCapabilities reporting only. It
qualifies no virtual-link effect or other error-context branch. The fourth
[ARM window trace](native-settings-connectivity-window.md#fourth-attempt-partial-touch-and-packets)
retains its original failure and unknown timeout stage; the separate fifth and
sixth attempts below do not retroactively qualify it or the third attempt.

## Failed ARM ReceiveCommand observation

A separate actual generic ARM window cohort verifies the full old image and all
providers, then installs only the qualified 660,848-byte contextual worker at the
same protected fixture path and passes its native loader. It reaches normal
Enable/Completed, independent `192.0.2.1/30` and one nonce packet before failure.
The actual context is `ReceiveCommand`, source `TimedOut`, original cutoff
`44296794456` and error-only detection sample `44323767175` BOOTTIME nanoseconds.
Their difference is 26,972,719 ns. Subtracting the recorded 50,000,000 ns envelope
from the original cutoff gives a derived origin-to-detection span of 76,972,719 ns.
These values measure neither a syscall duration nor which pre/post guard fired;
no scheduling or CPU-throttling cause is established.

The aggregate ARM case still fails. First app wait 0 is observed after cancel and
proves no background persistence. Its final actual worker wait is signal 9/code
null, not the natural exit 1 of either host CLI control or attempt four. Main
exits 0 and the driver 101. Controller custody passes, guest-case closure does
not; eventual ECHILD/one adopted exit 0 and all 53 scope waits/PIPE EOFs remain
separate. The [fifth-attempt report](native-settings-connectivity-window.md#fifth-attempt-receivecommand-context)
records QEMU/Xvfb/container closure and the curated hashes.

The fifth attempt executes with six guest vCPUs/6144 MiB and eight outer
CPUs/10 GiB. Its result remains separate from the sixth cohort below.

## Separate failed CPU16 ARM observation

The 16-guest/16-outer-CPU resource cohort has closed as a sixth failed ARM case,
with 6144 MiB guest/10 GiB outer memory and unchanged binaries, deadlines and
guards. It fails before Enable-ready, capture, touch or packet qualification;
the mapped-window sample is after cancel. Its actual `ReceiveCommand`/`TimedOut`
context records original cutoff `42391806186` and error-only detection
`42436693683` BOOTTIME nanoseconds. Detection is 44,887,497 ns past the cutoff;
the derived 50,000,000 ns envelope-origin span is 94,887,497 ns. These values
qualify neither syscall timing nor a pre/post guard, scheduling or CPU-throttling
cause, and identify no cause or repair for earlier failures.

Before cancel, Main is live with an empty child vector and ready retained-worker
pidfd; this is not an atomic process snapshot. The worker's final actual wait is
natural exit 1, Main exits 0 and the assertion driver exits 101. First app signal
9/code null is observed after cancel. Controller custody qualifies; guest-case
closure remains false despite eventual ECHILD and one adopted actual exit 0.
All 52 scope waits/PIPE EOFs comprise 13 host, four boot, 29 guest and six
controller observations. QEMU/Xvfb exit 0, container wait 1/removal/absence on the
same daemon and independently rehashed unchanged inputs are recorded separately
in [the sixth-attempt report](native-settings-connectivity-window.md#sixth-attempt-cpu16-receivecommand-failure)
and curated provenance.

Repeated graphical ARM resource campaigns are parked with the production 50 ms
guard unchanged. Full background/reopen/Disable, other error-context branches,
matched Pixel behavior and all phone gates remain open. The separate
[headless wireless service](../tools/native-wireless-service/README.md) now has
passing ARM evidence; it does not qualify the complete graphical lifecycle.
